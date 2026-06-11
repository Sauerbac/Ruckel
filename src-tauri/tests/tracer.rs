//! THE TRACER GATE (ADR-0027, ADR-0029 — v2 issue 02).
//!
//! Links rsmpeg against the self-built minimal static FFmpeg in `ffmpeg-libs/` and
//! transcodes the committed h264+aac/mp4 fixture **in-process** to a valid mp4. This is
//! the project's risk-retirement gate: it proves, in one place, that
//!   - the MSVC-built static libs link into a Rust binary (MSVC-link risk),
//!   - rusty_ffmpeg's libs-dir discovery finds them (discovery risk),
//!   - the `--disable-everything` build still contains everything the in-process
//!     pipeline needs: mov demux, h264 decode, the minimal filter graph
//!     (buffer/null/buffersink), libx264 encode, mp4 mux, file protocol
//!     (disable-everything risk).
//!
//! Issues 03 (probe swap) and 04 (encoder swap) did not start until this was green.

// Deliberately link the full app lib even though nothing here calls it: the gate must
// prove that libav and the Tauri app link together into one binary, and it is the lib
// target that carries build.rs's x264/dav1d/bcrypt link directives.
extern crate ruckel_lib;

use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avfilter::{AVFilter, AVFilterContextMut, AVFilterGraph, AVFilterInOut},
    avformat::{AVFormatContextInput, AVFormatContextOutput},
    avutil::{av_inv_q, av_rescale_q, AVFrame},
    error::RsmpegError,
    ffi,
};
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};

fn cstr_path(path: &Path) -> CString {
    CString::new(path.to_str().expect("utf-8 path")).unwrap()
}

#[test]
fn tracer_in_process_h264_transcode() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("h264-aac.mp4");
    let output = std::env::temp_dir().join("ruckel-tracer-out.mp4");
    let _ = std::fs::remove_file(&output);

    transcode_video(&cstr_path(&fixture), &cstr_path(&output));

    // The gate's assertion: the output opens in-process and has a video stream.
    let probe =
        AVFormatContextInput::open(&cstr_path(&output)).expect("transcoded output must open");
    assert!(
        probe
            .streams()
            .into_iter()
            .any(|s| s.codecpar().codec_type().is_video()),
        "transcoded output must contain a video stream"
    );
    assert!(probe.duration > 0, "transcoded output must have a duration");
}

/// Minimal but real in-process transcode of the fixture's video stream:
/// demux -> h264 decode -> buffer/null/buffersink graph -> libx264 encode -> mp4 mux.
/// Audio is deliberately ignored; the production loop is issue 04's job.
fn transcode_video(input: &CStr, output: &CStr) {
    let mut ifmt = AVFormatContextInput::open(input).expect("open input");

    let (video_index, mut dec) = {
        let (i, stream) = ifmt
            .streams()
            .into_iter()
            .enumerate()
            .find(|(_, s)| s.codecpar().codec_type().is_video())
            .expect("fixture has a video stream");
        let codecpar = stream.codecpar();
        let decoder =
            AVCodec::find_decoder(codecpar.codec_id).expect("h264 decoder compiled in");
        let mut dec = AVCodecContext::new(&decoder);
        dec.apply_codecpar(&codecpar).unwrap();
        dec.set_pkt_timebase(stream.time_base);
        if let Some(framerate) = stream.guess_framerate() {
            dec.set_framerate(framerate);
        }
        dec.open(None).expect("open h264 decoder");
        (i, dec)
    };

    let encoder = AVCodec::find_encoder_by_name(c"libx264").expect("libx264 compiled in");
    let mut enc = AVCodecContext::new(&encoder);
    enc.set_width(dec.width);
    enc.set_height(dec.height);
    enc.set_sample_aspect_ratio(dec.sample_aspect_ratio);
    enc.set_pix_fmt(ffi::AV_PIX_FMT_YUV420P);
    enc.set_time_base(av_inv_q(dec.framerate));

    let mut ofmt = AVFormatContextOutput::create(output).expect("create mp4 output");
    if ofmt.oformat().flags & ffi::AVFMT_GLOBALHEADER as i32 != 0 {
        enc.set_flags(enc.flags | ffi::AV_CODEC_FLAG_GLOBAL_HEADER as i32);
    }
    enc.open(None).expect("open libx264 encoder");
    {
        let mut out_stream = ofmt.new_stream();
        out_stream.set_codecpar(enc.extract_codecpar());
        out_stream.set_time_base(enc.time_base);
    }
    ofmt.write_header(&mut None).expect("write mp4 header");

    // Minimal filter graph — proves ADR-0027's minimal libavfilter works.
    let graph = AVFilterGraph::new();
    let (mut buffersrc_ctx, mut buffersink_ctx) = init_null_filter(&graph, &dec, &enc);

    loop {
        let packet = match ifmt.read_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(e) => panic!("read_packet failed: {e}"),
        };
        if packet.stream_index as usize != video_index {
            continue;
        }
        dec.send_packet(Some(&packet)).expect("send packet to decoder");
        receive_filter_encode_write(
            &mut dec,
            &mut buffersrc_ctx,
            &mut buffersink_ctx,
            &mut enc,
            &mut ofmt,
        );
    }

    // Drain decoder, then filter graph, then encoder.
    dec.send_packet(None).expect("flush decoder");
    receive_filter_encode_write(
        &mut dec,
        &mut buffersrc_ctx,
        &mut buffersink_ctx,
        &mut enc,
        &mut ofmt,
    );
    filter_encode_write(None, &mut buffersrc_ctx, &mut buffersink_ctx, &mut enc, &mut ofmt);
    encode_write(None, &mut enc, &mut ofmt);

    ofmt.write_trailer().expect("write mp4 trailer");
}

fn init_null_filter<'graph>(
    graph: &'graph AVFilterGraph,
    dec: &AVCodecContext,
    enc: &AVCodecContext,
) -> (AVFilterContextMut<'graph>, AVFilterContextMut<'graph>) {
    let buffersrc = AVFilter::get_by_name(c"buffer").expect("buffer filter compiled in");
    let buffersink = AVFilter::get_by_name(c"buffersink").expect("buffersink filter compiled in");

    let args = CString::new(format!(
        "video_size={}x{}:pix_fmt={}:time_base={}/{}:pixel_aspect={}/{}",
        dec.width,
        dec.height,
        dec.pix_fmt,
        dec.pkt_timebase.num,
        dec.pkt_timebase.den,
        dec.sample_aspect_ratio.num,
        dec.sample_aspect_ratio.den,
    ))
    .unwrap();

    let mut buffersrc_ctx = graph
        .create_filter_context(&buffersrc, c"in", Some(&args))
        .expect("create buffer source");
    let mut buffersink_ctx = graph
        .create_filter_context(&buffersink, c"out", None)
        .expect("create buffer sink");
    buffersink_ctx
        .opt_set_bin(c"pix_fmts", &enc.pix_fmt)
        .expect("set sink pix_fmt");

    let outputs = AVFilterInOut::new(c"in", &mut buffersrc_ctx, 0);
    let inputs = AVFilterInOut::new(c"out", &mut buffersink_ctx, 0);
    graph
        .parse_ptr(c"null", Some(inputs), Some(outputs))
        .expect("parse null filter graph");
    graph.config().expect("configure filter graph");

    (buffersrc_ctx, buffersink_ctx)
}

fn receive_filter_encode_write(
    dec: &mut AVCodecContext,
    buffersrc_ctx: &mut AVFilterContextMut,
    buffersink_ctx: &mut AVFilterContextMut,
    enc: &mut AVCodecContext,
    ofmt: &mut AVFormatContextOutput,
) {
    loop {
        let mut frame = match dec.receive_frame() {
            Ok(frame) => frame,
            Err(RsmpegError::DecoderDrainError) | Err(RsmpegError::DecoderFlushedError) => break,
            Err(e) => panic!("receive_frame failed: {e}"),
        };
        frame.set_pts(frame.best_effort_timestamp);
        filter_encode_write(Some(frame), buffersrc_ctx, buffersink_ctx, enc, ofmt);
    }
}

fn filter_encode_write(
    frame: Option<AVFrame>,
    buffersrc_ctx: &mut AVFilterContextMut,
    buffersink_ctx: &mut AVFilterContextMut,
    enc: &mut AVCodecContext,
    ofmt: &mut AVFormatContextOutput,
) {
    buffersrc_ctx
        .buffersrc_add_frame(frame, None)
        .expect("feed frame to filter graph");
    loop {
        let mut filtered = match buffersink_ctx.buffersink_get_frame(None) {
            Ok(frame) => frame,
            Err(RsmpegError::BufferSinkDrainError) | Err(RsmpegError::BufferSinkEofError) => break,
            Err(e) => panic!("buffersink_get_frame failed: {e}"),
        };
        filtered.set_time_base(buffersink_ctx.get_time_base());
        filtered.set_pict_type(ffi::AV_PICTURE_TYPE_NONE);
        encode_write(Some(filtered), enc, ofmt);
    }
}

fn encode_write(mut frame: Option<AVFrame>, enc: &mut AVCodecContext, ofmt: &mut AVFormatContextOutput) {
    if let Some(frame) = frame.as_mut() {
        if frame.pts != ffi::AV_NOPTS_VALUE {
            frame.set_pts(av_rescale_q(frame.pts, frame.time_base, enc.time_base));
        }
    }
    enc.send_frame(frame.as_ref()).expect("send frame to encoder");
    loop {
        let mut packet = match enc.receive_packet() {
            Ok(packet) => packet,
            Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => break,
            Err(e) => panic!("receive_packet failed: {e}"),
        };
        packet.set_stream_index(0);
        packet.rescale_ts(enc.time_base, ofmt.streams()[0].time_base);
        ofmt.interleaved_write_frame(&mut packet)
            .expect("interleaved write");
    }
}
