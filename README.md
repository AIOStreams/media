# AIOStreams media

Rust crates for seek-bar thumbnails and subtitle timelines of video files streamed over HTTP, from [AIOStreams](https://github.com/Viren070/AIOStreams)' desktop and Android apps. They read a file's index and only the frames they need, with range requests, so they work on a link to a large file without downloading it.

| Crate | Path | What it does |
| --- | --- | --- |
| `aiostreams-container` | `container/` | Reads the tracks, keyframes and subtitle timing of a Matroska/WebM or MP4/MOV file from its index (Matroska's SeekHead, Tracks and Cues, MP4's sample tables), over bounded HTTP range requests or from a local path. No FFmpeg. |
| `aiostreams-thumbs` | `thumbs/` | Thumbnails of single keyframes: decoded with FFmpeg's libavcodec, multi-slice HEVC as parallel strips, then scaled, tone mapped from HDR and Dolby Vision profile 5, and encoded as JPEG. H.264, HEVC, AV1, VP9 and MPEG-2. |

## Using them

They aren't on crates.io yet, so depend on a commit:

```toml
[dependencies]
aiostreams-container = { git = "https://github.com/AIOStreams/media", rev = "<commit>" }
aiostreams-thumbs = { git = "https://github.com/AIOStreams/media", rev = "<commit>" }
```

### Thumbnails

`aiostreams-thumbs` loads libavcodec and libavutil from FFmpeg 5 or later (libavcodec 59 to 66) when the first thumbnail decodes: first from the folders given to `library_dirs`, then from the system's search path. Building it needs no FFmpeg, and an app that ships no FFmpeg gets an error instead of a thumbnail.

`Thumbnailer` makes one at a time on the calling thread:

```rust
use aiostreams_container::Options;
use aiostreams_thumbs::{Settings, Thumbnailer};

let mut thumbs = Thumbnailer::open(url, Options::default(), Settings::default())?;
let thumbnail = thumbs.get(90_000)?; // the keyframe at or before 1:30
std::fs::write("thumb.jpg", &thumbnail.jpeg)?;
```

For a seek bar, `Session` keeps up with a moving pointer: it fetches several keyframes at once, renders only the newest asked for, caches what it made and frees its decoders when idle. `Thumbnails` is the same, but opens the file on a background thread when first asked:

```rust
use aiostreams_container::Options;
use aiostreams_thumbs::{Session, Settings};

let session = Session::open(url, Options::default(), Settings::default(), |thumbnail| {
    // runs on a background thread with each thumbnail made
})?;
if let Some(cached) = session.want(position_ms, showing_ms) {
    // made before, so it's here at once; otherwise the callback gets it
}
```

`Options` sets request headers, timeouts, a byte and request budget and the trusted roots; `Settings` the thumbnail width, decoder threads, JPEG quality and cache size.

### Subtitle timelines

```rust
use aiostreams_container::{RangeReader, index_options, subtitle_timelines};

let mut reader = RangeReader::new(url, index_options())?;
for track in subtitle_timelines(&mut reader)? {
    println!("track {} ({}): {} lines", track.number, track.codec, track.lines.len());
}
```

Each track's `lines` are when its lines start and end, in milliseconds, from the file's index alone: Matroska's Cues, with a PGS track's show and clear entries paired into lines, or an MP4 track's sample table. A track the index leaves out comes back empty. `index_options()` caps the reading at a few requests.

## Bench

```sh
cargo run --release -p aiostreams-thumbs --features bench --bin thumbs-bench -- <command>
```

with FFmpeg's libraries on the library search path.

- `fetch <url|file> --out <dir> [--n 12]` saves evenly spread keyframes as decoder packets.
- `decode <dir> [--split N] [--threads N] [--idle] [--sheet out.jpg]` times decoding them.
- `verify <dir>` decodes each keyframe whole and as strips and compares every sample.
- `scrub <url> [--workers N]` simulates a held seek key.
- `dovi-check <dir> <keyframe> <gbrp16le raw> [--out pair.jpg]` compares a Dolby Vision profile 5 thumbnail with a full-size PQ render.

## Licence

AGPL-3.0-only, as in `LICENSE`.
