# aiostreams-upscale

A small network that doubles the size of anime as it plays. It sharpens line art and cleans up compression, most of all in older and lower-resolution shows, and it is light enough for phones. It is what [AIOStreams](https://github.com/Viren070/AIOStreams)' apps run for **Anime upscaling**.

There are no modes to pick. It was trained on many kinds of damage at once, and it leaves a picture alone where there is nothing to fix.

## In mpv

Save [`shaders/aiostreams-upscale.glsl`](shaders/aiostreams-upscale.glsl) in mpv's `shaders` folder and add it in `mpv.conf`:

```
glsl-shaders-append="~~/shaders/aiostreams-upscale.glsl"
```

It runs with `vo=gpu-next` or `vo=gpu`, on any graphics API mpv uses. It only runs when the video is shown at least 1.2 times its size, so a 1080p video on a 1080p screen plays as it is. It is made for SDR video: leave it off for HDR.

## How it compares

PSNR to the clean frames, in dB, on 16 shows the network never saw. Each was copied with damage a real release has, then doubled back to full size. Higher is closer to the clean picture.

| Damage | Plain | Anime4K A | A HQ | B | C | This network |
| --- | --- | --- | --- | --- | --- | --- |
| Clean downscale to 540p | 37.30 | 35.16 | 36.07 | 36.77 | **39.15** | 38.81 |
| Web, 480p | 35.41 | 32.85 | 33.60 | 34.20 | 35.96 | **36.61** |
| Soft, 480p | 32.94 | 34.18 | 34.37 | 34.23 | 33.68 | **34.61** |
| DVD, MPEG-2 480p | 32.58 | 34.96 | **35.04** | 34.38 | 33.32 | 34.85 |
| Low bitrate, 432p | 31.26 | 31.48 | 31.76 | 31.81 | 31.75 | **32.19** |
| Ringing, 540p | **35.90** | 31.25 | 32.25 | 33.21 | 33.93 | 34.83 |
| Broadcast, 540p | 34.98 | 34.39 | 34.94 | 35.42 | **36.42** | 36.39 |
| Broadcast, 720p | 35.15 | 35.23 | 35.77 | 36.13 | **36.65** | 36.57 |
| **All** | 33.98 | 33.51 | 33.98 | 34.18 | 34.65 | **35.26** |

Plain is bilinear. Anime4K's presets are at their usual names: Mode A is Restore and Upscale CNN M, A HQ the same at VL, B uses Restore Soft M and C Upscale Denoise M. Each preset is best on the damage it was made for and worst away from it, which is why its users switch modes by hand.

What the numbers don't show: Anime4K's HQ preset draws crisper lines than this network does. This network keeps closer to the source, with fewer halos and none of the painted look Anime4K gives textures.

## Cost

About 740 multiply-adds per source pixel, against about 4,200 for Mode A and 2,100 for Mode C, in 11 passes of four values a pixel. Measured per frame while playing:

| Device | Source | Time |
| --- | --- | --- |
| NVIDIA RTX 5070 | 720x480 | 0.1 ms |
| AMD Ryzen 7 7800X3D's integrated graphics | 720x480 | 1.5 ms |
| Samsung Galaxy S24 (Xclipse 940) | 640x480 | 3.5 ms |

The time grows with the source's pixels, not the screen's.

## In an app

The crate has no dependencies. It gives the shader and the weights as constants:

```toml
[dependencies]
aiostreams-upscale = { git = "https://github.com/AIOStreams/media", rev = "<commit>" }
```

```rust
// The mpv shader, for a player that takes a file.
std::fs::write(&path, aiostreams_upscale::MPV_SHADER)?;
// The weights, for an app that builds its own shaders.
let network: serde_json::Value = serde_json::from_str(aiostreams_upscale::NETWORK)?;
// Below this much enlarging, the shader leaves the picture alone.
let min_scale = aiostreams_upscale::MIN_SCALE;
```

`NETWORK` is `{span, layers: [{weight: [out][in][y][x], bias}]}`. Each layer has a 3x3 kernel; the first reads luma, each later one reads the four features before it, and every layer's output is clamped to 0..1, which is its activation and lets the features live in 8-bit textures. The last layer's four values are the pixels at twice the size, as `(value - 0.5) * span` added to the bilinear picture.

## How it was made

It was trained on clean 1080p frames from 21 shows, with damaged copies of each as input, plus a smaller share of real old SD footage, for which Anime4K's Mode A was the target since no clean copy exists. The frames aren't included. Anime4K, by bloc97 under the MIT licence, was the teacher for that share and the comparison throughout.

After changing `network.json`, regenerate the shader with `cargo run -p aiostreams-upscale --example generate`.

## Licence

MIT, as in [`LICENSE`](LICENSE), unlike the rest of this repository.
