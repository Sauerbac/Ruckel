# Third-party licenses

Ruckel is licensed under the GPLv3 (see [LICENSE](LICENSE), decision in
[ADR-0003](docs/adr/0003-gplv3-license.md)). `ruckel.exe` statically links the
following third-party libraries (ADR-0027). The exact source tarballs, their
SHA-256 hashes, and the full configure flags for all three are pinned in
[`src-tauri/ffmpeg-build-manifest.json`](src-tauri/ffmpeg-build-manifest.json) —
that manifest is the compliance record of what is in the binary (ADR-0029).

## FFmpeg

Licensed under the **GPL version 3** (built with `--enable-gpl`; the GPL text in
[LICENSE](LICENSE) covers it). Source: the FFmpeg release pinned in the build
manifest, from <https://ffmpeg.org/>.

## x264

Licensed under the **GPL version 2 or later** (distributed here under GPLv3 as
part of the combined work). Source: the x264 stable-branch commit pinned in the
build manifest, from <https://www.videolan.org/developers/x264.html>.

## dav1d

Licensed under the **BSD 2-Clause License**. Source: the dav1d release pinned in
the build manifest, from <https://www.videolan.org/projects/dav1d.html>.

```
Copyright © 2018-2025, VideoLAN and dav1d authors
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```
