//! The in-process transcode loop (ADR-0027): one Conversion Job → one mp4,
//! using the libav* API directly through rsmpeg.
//!
//! The Tauri-facing batch loop lives in `commands.rs`; this module owns the
//! single-job encode so it is testable without a running Tauri app. The encode
//! settings come from the pure [`super::args::encoder_config`] mapping (B2).
//!
//! The pipeline, per the PRD "transcode loop" spec:
//! demux → decode → minimal video filter graph (autorotate · `scale=-2:H` ·
//! mandatory `format=yuv420p` · `fps=N`) → libx264 (profile high, preset medium,
//! crf from options, no level) + native AAC via swresample (or no audio) → mp4
//! mux with `+faststart`, writing to the ADR-0012 temp path, atomic rename on
//! success.

use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rsmpeg::avcodec::{AVCodec, AVCodecContext};
use rsmpeg::avfilter::{AVFilter, AVFilterContextMut, AVFilterGraph, AVFilterInOut};
use rsmpeg::avformat::{AVFormatContextInput, AVFormatContextOutput};
use rsmpeg::avutil::{av_rescale_q, AVAudioFifo, AVDictionary, AVFrame};
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;
use rsmpeg::swresample::SwrContext;

use super::args::{encoder_config, AudioConfig, EncoderConfig};
use super::progress::percent_of;
use crate::preflight::plan::ConversionJob;

/// A live progress update for one job, already resolved to a real percentage.
#[derive(Debug, Clone, PartialEq)]
pub struct EncodeUpdate {
    pub percent: f64,
    pub fps: f64,
    pub speed: f64,
}

/// Distinguishes a clean cancel from a real failure so the batch loop can
/// emit the right event (ADR-0013, ADR-0015).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    Cancelled,
    Failed(String),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::Cancelled => write!(f, "cancelled"),
            EncodeError::Failed(msg) => write!(f, "{msg}"),
        }
    }
}

/// Encode one job in-process, invoking `on_progress` at ~2 Hz. Returns the final
/// output path on success.
///
/// Output safety (ADR-0012): bytes are written to `<stem>.tmp.mp4` in the
/// destination folder; only after a clean finish is it renamed atomically to the
/// final `<stem>_ppt.mp4` (same volume, truly atomic). A failure or cancel
/// deletes the temp, so the destination is only ever replaced by a complete,
/// valid file and no partial `_ppt.mp4` is ever left behind.
///
/// `should_cancel` is polled once per demuxed packet; it is a predicate, not a
/// token, so the caller decides *why* to stop — a whole-batch abort or this one
/// job's individual cancel (ADR-0025). The encoder only asks "should I stop?".
pub fn encode<F>(
    job: &ConversionJob,
    duration_secs: f64,
    should_cancel: &dyn Fn() -> bool,
    mut on_progress: F,
) -> Result<PathBuf, EncodeError>
where
    F: FnMut(EncodeUpdate),
{
    let final_path = PathBuf::from(&job.output_path);
    let temp_path = temp_path_for(&final_path);
    let config = encoder_config(&job.options);

    let input = path_cstr(Path::new(&job.source_path))?;
    let output = path_cstr(&temp_path)?;

    match transcode(
        &input,
        &output,
        &config,
        duration_secs,
        should_cancel,
        &mut on_progress,
    ) {
        Ok(()) => {
            // Atomic promote: temp lives in the destination folder, so this is a
            // same-volume rename (ADR-0012). On failure, drop the temp.
            std::fs::rename(&temp_path, &final_path).map_err(|e| {
                let _ = std::fs::remove_file(&temp_path);
                EncodeError::Failed(format!("failed to finalize output: {e}"))
            })?;
            Ok(final_path)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&temp_path);
            Err(e)
        }
    }
}

/// The whole libav transcode, temp-path in, no rename (the caller promotes).
fn transcode<F: FnMut(EncodeUpdate)>(
    input: &CStr,
    output: &CStr,
    config: &EncoderConfig,
    duration_secs: f64,
    should_cancel: &dyn Fn() -> bool,
    on_progress: &mut F,
) -> Result<(), EncodeError> {
    let mut ifmt = AVFormatContextInput::open(input)
        .map_err(|e| EncodeError::Failed(format!("open input: {e}")))?;

    // --- Select streams + open decoders (borrow of ifmt ends in this block) ---
    let mut audio_index: Option<usize> = None;
    let (video_index, rotation, mut dec) = {
        let (i, vstream) = ifmt
            .streams()
            .iter()
            .enumerate()
            .find(|(_, s)| s.codecpar().codec_type().is_video())
            .ok_or_else(|| EncodeError::Failed("input has no video stream".into()))?;
        let codecpar = vstream.codecpar();
        let decoder = AVCodec::find_decoder(codecpar.codec_id)
            .ok_or_else(|| EncodeError::Failed("no decoder for this video codec".into()))?;
        let mut dec = AVCodecContext::new(&decoder);
        dec.apply_codecpar(&codecpar)
            .map_err(|e| EncodeError::Failed(format!("apply video codecpar: {e}")))?;
        dec.set_pkt_timebase(vstream.time_base);
        if let Some(fr) = vstream.guess_framerate() {
            dec.set_framerate(fr);
        }
        dec.open(None)
            .map_err(|e| EncodeError::Failed(format!("open video decoder: {e}")))?;
        let rotation = stream_rotation_degrees(&codecpar);

        // First audio stream, if the job keeps audio.
        if matches!(config.audio, AudioConfig::Aac { .. }) {
            audio_index = ifmt
                .streams()
                .iter()
                .position(|s| s.codecpar().codec_type().is_audio());
        }
        (i, rotation, dec)
    };

    // The container's start time (AV_TIME_BASE units). MPEG-PS/TS streams begin
    // at a nonzero PTS (~0.44s for VOB, ~1.4s for TS); the ffmpeg CLI shifts all
    // output timestamps so the file starts at 0, and so do we — otherwise the
    // output duration inflates and audio (whose PTS we synthesize) drifts out of
    // sync with video.
    let input_start_time = if ifmt.start_time == ffi::AV_NOPTS_VALUE { 0 } else { ifmt.start_time };
    // The same offset in the video stream's packet time base, for frame PTS.
    let video_pts_offset = av_rescale_q(
        input_start_time,
        ffi::AVRational { num: 1, den: ffi::AV_TIME_BASE as i32 },
        dec.pkt_timebase,
    );

    let mut adec: Option<AVCodecContext> = None;
    let mut audio: Option<AudioPipe> = None;

    let mut ofmt = AVFormatContextOutput::create(output)
        .map_err(|e| EncodeError::Failed(format!("create output: {e}")))?;
    let global_header = ofmt.oformat().flags & ffi::AVFMT_GLOBALHEADER as i32 != 0;

    if let (AudioConfig::Aac { bitrate_bps }, Some(ai)) = (config.audio, audio_index) {
        // A source with audio that we cannot wire up is a job failure, not a
        // silent audio drop — v1 (full-build ffmpeg) would have transcoded it,
        // and zero behavior change is the bar (ADR-0016/0031).
        audio = Some(AudioPipe::new(
            &ifmt,
            ai,
            bitrate_bps,
            global_header,
            input_start_time,
            &mut adec,
        )?);
    }
    let audio_active = audio.is_some();

    // --- Video filter graph + encoder, both lazy. The graph is built from the
    //     first decoded frame (its real pixel format/size/SAR — some codecs only
    //     report these after decoding, not from codecpar); the encoder + output
    //     streams + mp4 header follow on the first *filtered* frame, when the
    //     post-filter dimensions are known. Rotation comes from the stream.
    //     Audio frames decoded before the header accumulate in the FIFO. ---
    let graph = AVFilterGraph::new();
    let mut filters: Option<(AVFilterContextMut, AVFilterContextMut)> = None;
    let mut enc: Option<AVCodecContext> = None;
    let mut header_written = false;
    let mut progress = ProgressState::new();

    loop {
        if should_cancel() {
            return Err(EncodeError::Cancelled);
        }
        let packet = match ifmt.read_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(e) => return Err(EncodeError::Failed(format!("read packet: {e}"))),
        };
        let idx = packet.stream_index as usize;

        if idx == video_index {
            dec.send_packet(Some(&packet))
                .map_err(|e| EncodeError::Failed(format!("send video packet: {e}")))?;
            drain_decoder_into_filter(
                &mut dec,
                &graph,
                &mut filters,
                rotation,
                video_pts_offset,
                &mut enc,
                &mut ofmt,
                &mut header_written,
                audio.as_ref(),
                config,
                global_header,
                duration_secs,
                &mut progress,
                on_progress,
            )?;
            // A freshly written header means buffered audio can now flow.
            if header_written {
                if let Some(ap) = audio.as_mut() {
                    ap.pump(&mut ofmt, false)?;
                }
            }
        } else if audio_active && Some(idx) == audio_index {
            let adec = adec.as_mut().unwrap();
            adec.send_packet(Some(&packet))
                .map_err(|e| EncodeError::Failed(format!("send audio packet: {e}")))?;
            let ap = audio.as_mut().unwrap();
            loop {
                let frame = match adec.receive_frame() {
                    Ok(f) => f,
                    Err(RsmpegError::DecoderDrainError) | Err(RsmpegError::DecoderFlushedError) => {
                        break
                    }
                    Err(e) => return Err(EncodeError::Failed(format!("decode audio: {e}"))),
                };
                ap.push_decoded(&frame)?;
            }
            if header_written {
                ap.pump(&mut ofmt, false)?;
            }
        }
    }

    // --- Drain: flush video decoder → filter graph → encoder ---
    dec.send_packet(None)
        .map_err(|e| EncodeError::Failed(format!("flush video decoder: {e}")))?;
    drain_decoder_into_filter(
        &mut dec,
        &graph,
        &mut filters,
        rotation,
        video_pts_offset,
        &mut enc,
        &mut ofmt,
        &mut header_written,
        audio.as_ref(),
        config,
        global_header,
        duration_secs,
        &mut progress,
        on_progress,
    )?;
    // Flush the filter graph itself (EOF frame), if it was ever built.
    if let Some((src, sink)) = filters.as_mut() {
        src.buffersrc_add_frame(None, None)
            .map_err(|e| EncodeError::Failed(format!("flush filter graph: {e}")))?;
        pull_filtered_and_encode(
            sink,
            &mut enc,
            &mut ofmt,
            &mut header_written,
            audio.as_ref(),
            config,
            global_header,
            duration_secs,
            &mut progress,
            on_progress,
        )?;
    }

    if !header_written {
        return Err(EncodeError::Failed("no video frames were decoded".into()));
    }

    // Flush the video encoder.
    if let Some(e) = enc.as_mut() {
        encode_and_mux_video(e, &mut ofmt, None, duration_secs, &mut progress, on_progress)?;
    }
    // Drain remaining audio: flush decoder, then resampler + FIFO + encoder.
    if let (Some(adec), Some(ap)) = (adec.as_mut(), audio.as_mut()) {
        let _ = adec.send_packet(None);
        while let Ok(f) = adec.receive_frame() {
            ap.push_decoded(&f)?;
        }
        ap.pump(&mut ofmt, true)?;
    }

    ofmt.write_trailer()
        .map_err(|e| EncodeError::Failed(format!("write trailer: {e}")))?;

    // Final terminal-value progress emit.
    progress.emit(duration_secs, on_progress);
    Ok(())
}

/// Pull every decodable frame and run it through the video filter + encoder.
/// The filter graph is built lazily from the first frame (its real format/size).
#[allow(clippy::too_many_arguments)]
fn drain_decoder_into_filter<'g, F: FnMut(EncodeUpdate)>(
    dec: &mut AVCodecContext,
    graph: &'g AVFilterGraph,
    filters: &mut Option<(AVFilterContextMut<'g>, AVFilterContextMut<'g>)>,
    rotation: i32,
    pts_offset: i64,
    enc: &mut Option<AVCodecContext>,
    ofmt: &mut AVFormatContextOutput,
    header_written: &mut bool,
    audio: Option<&AudioPipe>,
    config: &EncoderConfig,
    global_header: bool,
    duration_secs: f64,
    progress: &mut ProgressState,
    on_progress: &mut F,
) -> Result<(), EncodeError> {
    loop {
        let mut frame = match dec.receive_frame() {
            Ok(f) => f,
            Err(RsmpegError::DecoderDrainError) | Err(RsmpegError::DecoderFlushedError) => break,
            Err(e) => return Err(EncodeError::Failed(format!("decode video: {e}"))),
        };
        // Shift to a zero-based timeline (CLI parity, see input_start_time).
        if frame.best_effort_timestamp != ffi::AV_NOPTS_VALUE {
            frame.set_pts(frame.best_effort_timestamp - pts_offset);
        } else {
            frame.set_pts(frame.best_effort_timestamp);
        }
        if filters.is_none() {
            *filters = Some(build_video_graph(graph, &frame, dec.pkt_timebase, rotation, config)?);
        }
        let (buffersrc, buffersink) = filters.as_mut().unwrap();
        buffersrc
            .buffersrc_add_frame(Some(frame), None)
            .map_err(|e| EncodeError::Failed(format!("feed filter graph: {e}")))?;
        pull_filtered_and_encode(
            buffersink,
            enc,
            ofmt,
            header_written,
            audio,
            config,
            global_header,
            duration_secs,
            progress,
            on_progress,
        )?;
    }
    Ok(())
}

/// Pull filtered frames from the sink; lazily open the encoder + write the mp4
/// header on the first one; encode and mux the rest.
#[allow(clippy::too_many_arguments)]
fn pull_filtered_and_encode<F: FnMut(EncodeUpdate)>(
    buffersink: &mut AVFilterContextMut,
    enc: &mut Option<AVCodecContext>,
    ofmt: &mut AVFormatContextOutput,
    header_written: &mut bool,
    audio: Option<&AudioPipe>,
    config: &EncoderConfig,
    global_header: bool,
    duration_secs: f64,
    progress: &mut ProgressState,
    on_progress: &mut F,
) -> Result<(), EncodeError> {
    loop {
        let mut filtered = match buffersink.buffersink_get_frame(None) {
            Ok(f) => f,
            Err(RsmpegError::BufferSinkDrainError) | Err(RsmpegError::BufferSinkEofError) => break,
            Err(e) => return Err(EncodeError::Failed(format!("filter graph: {e}"))),
        };
        filtered.set_time_base(buffersink.get_time_base());
        filtered.set_pict_type(ffi::AV_PICTURE_TYPE_NONE);

        if enc.is_none() {
            let opened = open_video_encoder(&filtered, buffersink.get_time_base(), config, global_header)?;
            *enc = Some(opened);
            // Output streams must be declared before the header: video first
            // (index 0), then audio (index 1) if present.
            {
                let e = enc.as_ref().unwrap();
                let mut s = ofmt.new_stream();
                s.set_codecpar(e.extract_codecpar());
                s.set_time_base(e.time_base);
            }
            if let Some(ap) = audio {
                let mut s = ofmt.new_stream();
                s.set_codecpar(ap.aenc.extract_codecpar());
                s.set_time_base(ap.aenc.time_base);
            }
            let mut opts = if config.faststart {
                Some(AVDictionary::new(c"movflags", c"+faststart", 0))
            } else {
                None
            };
            ofmt.write_header(&mut opts)
                .map_err(|e| EncodeError::Failed(format!("write mp4 header: {e}")))?;
            *header_written = true;
        }

        let e = enc.as_mut().unwrap();
        encode_and_mux_video(e, ofmt, Some(filtered), duration_secs, progress, on_progress)?;
    }
    Ok(())
}

/// Encode one filtered video frame (or flush with `None`) and mux the packets,
/// updating progress from the muxed PTS.
fn encode_and_mux_video<F: FnMut(EncodeUpdate)>(
    enc: &mut AVCodecContext,
    ofmt: &mut AVFormatContextOutput,
    frame: Option<AVFrame>,
    duration_secs: f64,
    progress: &mut ProgressState,
    on_progress: &mut F,
) -> Result<(), EncodeError> {
    let mut frame = frame;
    if let Some(f) = frame.as_mut() {
        if f.pts != ffi::AV_NOPTS_VALUE {
            let pts = av_rescale_q(f.pts, f.time_base, enc.time_base);
            f.set_pts(pts);
        }
    }
    enc.send_frame(frame.as_ref())
        .map_err(|e| EncodeError::Failed(format!("encode video: {e}")))?;
    loop {
        let mut pkt = match enc.receive_packet() {
            Ok(p) => p,
            Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => break,
            Err(e) => return Err(EncodeError::Failed(format!("receive video packet: {e}"))),
        };
        pkt.set_stream_index(0);
        let out_tb = ofmt.streams()[0].time_base;
        pkt.rescale_ts(enc.time_base, out_tb);
        if pkt.pts != ffi::AV_NOPTS_VALUE {
            progress.update_out_time(pkt.pts as f64 * out_tb.num as f64 / out_tb.den as f64);
        }
        progress.video_frames += 1;
        ofmt.interleaved_write_frame(&mut pkt)
            .map_err(|e| EncodeError::Failed(format!("mux video: {e}")))?;
        progress.maybe_emit(duration_secs, on_progress);
    }
    Ok(())
}

/// Open a libx264 encoder for the given filtered frame and time base, applying
/// the ADR-0006 contract (profile high, preset medium, crf, no level).
fn open_video_encoder(
    filtered: &AVFrame,
    time_base: ffi::AVRational,
    config: &EncoderConfig,
    global_header: bool,
) -> Result<AVCodecContext, EncodeError> {
    let codec = AVCodec::find_encoder_by_name(c"libx264")
        .ok_or_else(|| EncodeError::Failed("libx264 encoder is missing".into()))?;
    let mut enc = AVCodecContext::new(&codec);
    enc.set_width(filtered.width);
    enc.set_height(filtered.height);
    enc.set_pix_fmt(config.pix_fmt);
    enc.set_sample_aspect_ratio(filtered.sample_aspect_ratio);
    enc.set_time_base(time_base);
    if global_header {
        enc.set_flags(enc.flags | ffi::AV_CODEC_FLAG_GLOBAL_HEADER as i32);
    }
    let crf = CString::new(config.crf.to_string()).unwrap();
    let preset = CString::new(config.preset).unwrap();
    let profile = CString::new(config.profile).unwrap();
    let opts = AVDictionary::new(c"preset", &preset, 0)
        .set(c"crf", &crf, 0)
        .set(c"profile", &profile, 0);
    enc.open(Some(opts))
        .map_err(|e| EncodeError::Failed(format!("open libx264: {e}")))?;
    Ok(enc)
}

/// The audio transcode pipe: decode (elsewhere) → swresample to fltp at a
/// supported rate → FIFO → native AAC in fixed-size frames. Owns the encoder,
/// resampler, and FIFO; the muxer is passed in so the pipe never aliases it.
struct AudioPipe {
    aenc: AVCodecContext,
    swr: SwrContext,
    fifo: AVAudioFifo,
    out_ch_layout: ffi::AVChannelLayout,
    out_sample_rate: i32,
    frame_size: i32,
    next_pts: i64,
    /// The audio stream's time base — for mapping the first decoded frame's
    /// timestamp onto the output sample clock.
    in_time_base: ffi::AVRational,
    /// Container start time (AV_TIME_BASE units) subtracted from all timestamps
    /// (zero-based output timeline, CLI parity).
    start_time: i64,
    /// Set once the first decoded frame has anchored `next_pts`.
    anchored: bool,
}

impl AudioPipe {
    /// Set up the whole audio path from the input's audio stream. Any failure
    /// is a hard error: the source has audio the user asked to keep, so we
    /// must not silently strip it (zero behavior change vs v1).
    fn new(
        ifmt: &AVFormatContextInput,
        audio_index: usize,
        bitrate_bps: i64,
        global_header: bool,
        start_time: i64,
        adec_out: &mut Option<AVCodecContext>,
    ) -> Result<Self, EncodeError> {
        let astream = &ifmt.streams()[audio_index];
        let codecpar = astream.codecpar();
        let decoder = AVCodec::find_decoder(codecpar.codec_id)
            .ok_or_else(|| EncodeError::Failed("no decoder for this audio codec".into()))?;
        let mut adec = AVCodecContext::new(&decoder);
        adec.apply_codecpar(&codecpar)
            .map_err(|e| EncodeError::Failed(format!("apply audio codecpar: {e}")))?;
        adec.set_pkt_timebase(astream.time_base);
        adec.open(None)
            .map_err(|e| EncodeError::Failed(format!("open audio decoder: {e}")))?;

        let encoder = AVCodec::find_encoder_by_name(c"aac")
            .ok_or_else(|| EncodeError::Failed("aac encoder is missing".into()))?;
        let out_sample_rate = choose_sample_rate(&encoder, adec.sample_rate);

        // Preserve the source channel layout (CLI parity). Containers like AVI
        // carry only a channel count, leaving the layout order UNSPEC — the aac
        // encoder and swresample both reject that, so normalize to the default
        // layout for the same channel count (exactly what the CLI does).
        // Standard layouts are mask-based with no heap, so the bit-copies below
        // are safe; the Drop impl frees any custom map.
        let mut out_ch_layout: ffi::AVChannelLayout = unsafe { std::mem::zeroed() };
        if adec.ch_layout.order == ffi::AV_CHANNEL_ORDER_UNSPEC {
            unsafe {
                ffi::av_channel_layout_default(&mut out_ch_layout, adec.ch_layout.nb_channels)
            };
        } else {
            unsafe { ffi::av_channel_layout_copy(&mut out_ch_layout, &adec.ch_layout) };
        }

        let mut aenc = AVCodecContext::new(&encoder);
        aenc.set_sample_fmt(ffi::AV_SAMPLE_FMT_FLTP);
        aenc.set_sample_rate(out_sample_rate);
        let mut enc_ch_layout: ffi::AVChannelLayout = unsafe { std::mem::zeroed() };
        unsafe { ffi::av_channel_layout_copy(&mut enc_ch_layout, &out_ch_layout) };
        aenc.set_ch_layout(enc_ch_layout);
        aenc.set_bit_rate(bitrate_bps);
        aenc.set_time_base(ffi::AVRational { num: 1, den: out_sample_rate });
        if global_header {
            aenc.set_flags(aenc.flags | ffi::AV_CODEC_FLAG_GLOBAL_HEADER as i32);
        }
        aenc.open(None)
            .map_err(|e| EncodeError::Failed(format!("open aac encoder: {e}")))?;

        // Input side uses the same normalized layout: for an UNSPEC source the
        // samples are positional either way, and swresample rejects UNSPEC.
        let mut swr = SwrContext::new(
            &out_ch_layout,
            ffi::AV_SAMPLE_FMT_FLTP,
            out_sample_rate,
            &out_ch_layout,
            adec.sample_fmt,
            adec.sample_rate,
        )
        .map_err(|e| EncodeError::Failed(format!("audio resampler setup: {e}")))?;
        swr.init()
            .map_err(|e| EncodeError::Failed(format!("audio resampler init: {e}")))?;

        let frame_size = if aenc.frame_size > 0 { aenc.frame_size } else { 1024 };
        let fifo = AVAudioFifo::new(
            ffi::AV_SAMPLE_FMT_FLTP,
            out_ch_layout.nb_channels,
            frame_size.max(1),
        );

        let in_time_base = astream.time_base;
        *adec_out = Some(adec);
        Ok(AudioPipe {
            aenc,
            swr,
            fifo,
            out_ch_layout,
            out_sample_rate,
            frame_size,
            next_pts: 0,
            in_time_base,
            start_time,
            anchored: false,
        })
    }

    /// Resample one decoded audio frame into the FIFO.
    fn push_decoded(&mut self, frame: &AVFrame) -> Result<(), EncodeError> {
        // Anchor the output sample clock to the first frame's real position on
        // the zero-based timeline, so audio stays aligned with video when the
        // container doesn't start at t=0 (VOB/TS). Subsequent PTS continue by
        // sample count (gapless, matching the resampler's output).
        if !self.anchored {
            if frame.best_effort_timestamp != ffi::AV_NOPTS_VALUE {
                let out_tb = ffi::AVRational { num: 1, den: self.out_sample_rate };
                let pos = av_rescale_q(frame.best_effort_timestamp, self.in_time_base, out_tb)
                    - av_rescale_q(
                        self.start_time,
                        ffi::AVRational { num: 1, den: ffi::AV_TIME_BASE as i32 },
                        out_tb,
                    );
                self.next_pts = pos.max(0);
            }
            self.anchored = true;
        }
        let out_samples = self.swr.get_out_samples(frame.nb_samples);
        if out_samples <= 0 {
            return Ok(());
        }
        let mut tmp = self.new_fltp_frame(out_samples)?;
        let converted = unsafe {
            self.swr.convert(
                tmp.data_mut().as_mut_ptr(),
                out_samples,
                frame.data.as_ptr() as *const *const u8,
                frame.nb_samples,
            )
        }
        .map_err(|e| EncodeError::Failed(format!("resample audio: {e}")))?;
        if converted > 0 {
            unsafe { self.fifo.write(tmp.data_mut().as_ptr(), converted) }
                .map_err(|e| EncodeError::Failed(format!("audio fifo write: {e}")))?;
        }
        Ok(())
    }

    /// Drain the FIFO into the encoder in `frame_size` chunks. When `finalize`,
    /// also flush the resampler tail, the last partial chunk, and the encoder.
    fn pump(&mut self, ofmt: &mut AVFormatContextOutput, finalize: bool) -> Result<(), EncodeError> {
        if finalize {
            self.flush_resampler()?;
        }
        while self.fifo.size() >= self.frame_size {
            let frame = self.read_chunk(self.frame_size)?;
            self.encode(ofmt, Some(frame))?;
        }
        if finalize {
            if self.fifo.size() > 0 {
                let n = self.fifo.size();
                let frame = self.read_chunk(n)?;
                self.encode(ofmt, Some(frame))?;
            }
            self.encode(ofmt, None)?;
        }
        Ok(())
    }

    /// Pull the resampler's buffered tail into the FIFO (sample-rate conversion
    /// leaves a delay behind the last input).
    fn flush_resampler(&mut self) -> Result<(), EncodeError> {
        loop {
            let out_samples = self.swr.get_out_samples(0);
            if out_samples <= 0 {
                break;
            }
            let mut tmp = self.new_fltp_frame(out_samples)?;
            let n = unsafe {
                self.swr
                    .convert(tmp.data_mut().as_mut_ptr(), out_samples, std::ptr::null(), 0)
            }
            .map_err(|e| EncodeError::Failed(format!("flush resampler: {e}")))?;
            if n <= 0 {
                break;
            }
            unsafe { self.fifo.write(tmp.data_mut().as_ptr(), n) }
                .map_err(|e| EncodeError::Failed(format!("audio fifo write: {e}")))?;
        }
        Ok(())
    }

    /// Read `nb` samples from the FIFO into a fresh fltp frame, stamping its PTS.
    fn read_chunk(&mut self, nb: i32) -> Result<AVFrame, EncodeError> {
        let mut frame = self.new_fltp_frame(nb)?;
        let got = unsafe { self.fifo.read(frame.data_mut().as_ptr(), nb) }
            .map_err(|e| EncodeError::Failed(format!("audio fifo read: {e}")))?;
        frame.set_nb_samples(got);
        frame.set_pts(self.next_pts);
        self.next_pts += got as i64;
        Ok(frame)
    }

    /// Encode one audio frame (or flush with `None`) and mux to stream index 1.
    fn encode(
        &mut self,
        ofmt: &mut AVFormatContextOutput,
        frame: Option<AVFrame>,
    ) -> Result<(), EncodeError> {
        self.aenc
            .send_frame(frame.as_ref())
            .map_err(|e| EncodeError::Failed(format!("encode audio: {e}")))?;
        loop {
            let mut pkt = match self.aenc.receive_packet() {
                Ok(p) => p,
                Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => break,
                Err(e) => return Err(EncodeError::Failed(format!("receive audio packet: {e}"))),
            };
            pkt.set_stream_index(1);
            let out_tb = ofmt.streams()[1].time_base;
            pkt.rescale_ts(self.aenc.time_base, out_tb);
            ofmt.interleaved_write_frame(&mut pkt)
                .map_err(|e| EncodeError::Failed(format!("mux audio: {e}")))?;
        }
        Ok(())
    }

    /// Allocate an fltp frame matching the encoder's layout/rate with `nb` slots.
    fn new_fltp_frame(&self, nb: i32) -> Result<AVFrame, EncodeError> {
        let mut frame = AVFrame::new();
        frame.set_format(ffi::AV_SAMPLE_FMT_FLTP);
        let mut layout: ffi::AVChannelLayout = unsafe { std::mem::zeroed() };
        unsafe { ffi::av_channel_layout_copy(&mut layout, &self.out_ch_layout) };
        frame.set_ch_layout(layout);
        frame.set_sample_rate(self.out_sample_rate);
        frame.set_nb_samples(nb);
        frame
            .alloc_buffer()
            .map_err(|e| EncodeError::Failed(format!("alloc audio frame: {e}")))?;
        Ok(frame)
    }
}

impl Drop for AudioPipe {
    fn drop(&mut self) {
        unsafe { ffi::av_channel_layout_uninit(&mut self.out_ch_layout) };
    }
}

/// Native AAC's supported sample rate nearest the source (CLI parity: keep the
/// source rate when supported, else resample to the closest the encoder allows).
fn choose_sample_rate(codec: &AVCodec, src_rate: i32) -> i32 {
    match codec.supported_samplerates() {
        Some(rates) => rates
            .iter()
            .copied()
            .filter(|&r| r > 0)
            .min_by_key(|&r| (r - src_rate).abs())
            .unwrap_or(src_rate),
        None => src_rate,
    }
}

/// Build the buffer → [autorotate] → [scale] → format=yuv420p → [fps] →
/// buffersink graph from the first decoded frame's real properties. Rotation is
/// the stream's display-matrix angle (degrees); `pkt_tb` is the decoder packet
/// time base (the frame timestamps' base).
fn build_video_graph<'g>(
    graph: &'g AVFilterGraph,
    frame: &AVFrame,
    pkt_tb: ffi::AVRational,
    rotation: i32,
    config: &EncoderConfig,
) -> Result<(AVFilterContextMut<'g>, AVFilterContextMut<'g>), EncodeError> {
    let buffersrc =
        AVFilter::get_by_name(c"buffer").ok_or_else(|| EncodeError::Failed("buffer filter".into()))?;
    let buffersink = AVFilter::get_by_name(c"buffersink")
        .ok_or_else(|| EncodeError::Failed("buffersink filter".into()))?;

    let sar = frame.sample_aspect_ratio;
    let (sar_n, sar_d) = if sar.num > 0 && sar.den > 0 { (sar.num, sar.den) } else { (1, 1) };
    let tb = if pkt_tb.num > 0 && pkt_tb.den > 0 { pkt_tb } else { ffi::AVRational { num: 1, den: 90_000 } };
    let args = CString::new(format!(
        "video_size={}x{}:pix_fmt={}:time_base={}/{}:pixel_aspect={}/{}",
        frame.width, frame.height, frame.format, tb.num, tb.den, sar_n, sar_d,
    ))
    .unwrap();

    let mut src = graph
        .create_filter_context(&buffersrc, c"in", Some(&args))
        .map_err(|e| EncodeError::Failed(format!("create buffer source: {e}")))?;
    let mut sink = graph
        .create_filter_context(&buffersink, c"out", None)
        .map_err(|e| EncodeError::Failed(format!("create buffer sink: {e}")))?;
    sink.opt_set_bin(c"pix_fmts", &config.pix_fmt)
        .map_err(|e| EncodeError::Failed(format!("set sink pix_fmt: {e}")))?;

    let desc = CString::new(video_filter_desc(rotation, config)).unwrap();
    let outputs = AVFilterInOut::new(c"in", &mut src, 0);
    let inputs = AVFilterInOut::new(c"out", &mut sink, 0);
    graph
        .parse_ptr(&desc, Some(inputs), Some(outputs))
        .map_err(|e| EncodeError::Failed(format!("parse filter graph: {e}")))?;
    graph
        .config()
        .map_err(|e| EncodeError::Failed(format!("configure filter graph: {e}")))?;
    Ok((src, sink))
}

/// The comma-separated filter chain: autorotate, optional scale, mandatory
/// yuv420p normalization, optional CFR fps cap. Always non-empty.
fn video_filter_desc(rotation: i32, config: &EncoderConfig) -> String {
    let mut parts: Vec<String> = Vec::new();
    match rotation {
        90 => parts.push("transpose=clock".into()),
        180 => {
            parts.push("hflip".into());
            parts.push("vflip".into());
        }
        270 => parts.push("transpose=cclock".into()),
        _ => {}
    }
    if let Some(h) = config.scale_height {
        parts.push(format!("scale=-2:{h}"));
    }
    parts.push("format=yuv420p".into());
    if let Some(f) = config.fps_cap {
        parts.push(format!("fps={f}"));
    }
    parts.join(",")
}

/// The video stream's display-matrix rotation, normalized to 0/90/180/270
/// degrees. Matches the ffmpeg CLI's default autorotate (`-autorotate`).
fn stream_rotation_degrees(codecpar: &rsmpeg::avcodec::AVCodecParametersRef) -> i32 {
    unsafe {
        let coded = codecpar.coded_side_data;
        let n = codecpar.nb_coded_side_data;
        if coded.is_null() || n <= 0 {
            return 0;
        }
        let sd = ffi::av_packet_side_data_get(coded as *const _, n, ffi::AV_PKT_DATA_DISPLAYMATRIX);
        if sd.is_null() {
            return 0;
        }
        let data = (*sd).data as *const i32;
        if data.is_null() {
            return 0;
        }
        normalize_rotation(-ffi::av_display_rotation_get(data))
    }
}

/// Normalize a rotation in degrees to the nearest of 0/90/180/270.
fn normalize_rotation(theta: f64) -> i32 {
    let mut t = theta % 360.0;
    if t < 0.0 {
        t += 360.0;
    }
    ((((t / 90.0).round() as i64) % 4) * 90) as i32
}

/// Throttled progress emitter (ADR-0013): muxed video PTS over probed duration,
/// plus an fps/speed estimate from wall-clock time, at ~2 Hz.
struct ProgressState {
    start: Instant,
    last_emit: Option<Instant>,
    video_frames: u64,
    out_time_secs: f64,
}

impl ProgressState {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            last_emit: None,
            video_frames: 0,
            out_time_secs: 0.0,
        }
    }

    fn update_out_time(&mut self, secs: f64) {
        if secs > self.out_time_secs {
            self.out_time_secs = secs;
        }
    }

    /// Emit if at least ~500 ms have elapsed since the last emit.
    fn maybe_emit<F: FnMut(EncodeUpdate)>(&mut self, duration_secs: f64, on_progress: &mut F) {
        let due = self
            .last_emit
            .map_or(true, |t| t.elapsed() >= Duration::from_millis(500));
        if due {
            self.emit(duration_secs, on_progress);
        }
    }

    fn emit<F: FnMut(EncodeUpdate)>(&mut self, duration_secs: f64, on_progress: &mut F) {
        self.last_emit = Some(Instant::now());
        let elapsed = self.start.elapsed().as_secs_f64();
        let fps = if elapsed > 0.0 { self.video_frames as f64 / elapsed } else { 0.0 };
        let speed = if elapsed > 0.0 { self.out_time_secs / elapsed } else { 0.0 };
        let percent = percent_of((self.out_time_secs * 1_000_000.0) as i64, duration_secs);
        on_progress(EncodeUpdate { percent, fps, speed });
    }
}

/// The in-progress temp path beside the final output (ADR-0012):
/// `<stem>.tmp.mp4` in the destination folder, keeping the success rename
/// same-volume and therefore atomic.
fn temp_path_for(final_path: &Path) -> PathBuf {
    let stem = final_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".into());
    let dir = final_path.parent().unwrap_or_else(|| Path::new("."));
    dir.join(format!("{stem}.tmp.mp4"))
}

/// A path as a libav-ready `CString`, erroring on non-UTF-8 or interior NULs.
fn path_cstr(path: &Path) -> Result<CString, EncodeError> {
    let s = path
        .to_str()
        .ok_or_else(|| EncodeError::Failed(format!("path is not valid UTF-8: {}", path.display())))?;
    CString::new(s).map_err(|e| EncodeError::Failed(format!("path has an interior NUL: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preflight::plan::ConversionOptions;

    fn config(options: ConversionOptions) -> EncoderConfig {
        encoder_config(&options)
    }

    #[test]
    fn temp_path_sits_beside_final_as_tmp_mp4() {
        let temp = temp_path_for(Path::new("/videos/clip_ppt.mp4"));
        assert_eq!(temp.file_name().unwrap(), "clip_ppt.tmp.mp4");
        assert_eq!(temp.parent().unwrap(), Path::new("/videos"));
    }

    #[test]
    fn normalize_rotation_snaps_to_quadrants() {
        assert_eq!(normalize_rotation(0.0), 0);
        assert_eq!(normalize_rotation(90.0), 90);
        assert_eq!(normalize_rotation(-90.0), 270);
        assert_eq!(normalize_rotation(180.0), 180);
        assert_eq!(normalize_rotation(-270.0), 90);
        assert_eq!(normalize_rotation(360.0), 0);
        assert_eq!(normalize_rotation(89.4), 90);
    }

    #[test]
    fn filter_desc_always_normalizes_to_yuv420p() {
        let desc = video_filter_desc(0, &config(ConversionOptions::PRESENTATION));
        assert_eq!(desc, "format=yuv420p");
    }

    #[test]
    fn filter_desc_adds_scale_and_fps_when_capped() {
        let desc = video_filter_desc(0, &config(ConversionOptions::COMPACT));
        assert_eq!(desc, "scale=-2:720,format=yuv420p,fps=30");
    }

    #[test]
    fn filter_desc_prepends_rotation() {
        let c = config(ConversionOptions::PRESENTATION);
        assert_eq!(video_filter_desc(90, &c), "transpose=clock,format=yuv420p");
        assert_eq!(video_filter_desc(270, &c), "transpose=cclock,format=yuv420p");
        assert_eq!(video_filter_desc(180, &c), "hflip,vflip,format=yuv420p");
    }
}
