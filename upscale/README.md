# aiostreams-upscale

A small network that doubles the size of anime as it plays. It draws line art crisp and cleans up compression, most of all in older and lower-resolution shows, and it is light enough for phones. It is what [AIOStreams](https://github.com/Viren070/AIOStreams)' apps run for **Anime upscaling**.

There are no modes to pick. It was trained on many kinds of damage at once, and it leaves a picture alone where there is nothing to fix.

## In mpv

Save [`shaders/aiostreams-upscale.glsl`](shaders/aiostreams-upscale.glsl) in mpv's `shaders` folder and add it in `mpv.conf`:

```
glsl-shaders-append="~~/shaders/aiostreams-upscale.glsl"
```

It runs with `vo=gpu-next` or `vo=gpu`, on any graphics API mpv uses. It only runs when the video is shown at least 1.2 times its size, so a 1080p video on a 1080p screen plays as it is. It is made for SDR video: leave it off for HDR.

## In WebGPU-Player

[`shaders/aiostreams-upscale.wgsl`](shaders/aiostreams-upscale.wgsl) is the same network as a pass file for [WebGPU-Player](https://github.com/alchemyyy/WebGPU-Player). Drop it on the sandbox's shader panel, or give its text to the presenter's `setPresentationPreset`. It runs when the mpv shader would, and leaves HDR video alone by itself.

## How it looks

After the network doubles the picture, a last step moves each pixel along an S-curve toward the darker or lighter end of its source pixel's 3x3 luma range, only where that range is as wide as a line's. Edges get steeper without moving and without halos, and flat areas and texture stay as they are. The network was trained through that step, toward clean frames given the same steeper edges, so it draws lines crisper than the source holds rather than the closest match to a clean copy.

## Cost

About 260 multiply-adds per source pixel in four layers, plus one 3x3 luma range per source pixel, in 6 passes. Measured per frame:

| Device | Source | Time |
| --- | --- | --- |
| NVIDIA RTX 5070 | 640x480, shown at 1440x1080 | 0.04 ms |
| Samsung Galaxy S24 (Xclipse 940) | 640x480, shown at 1440x1080 | 1.9 ms |

The S24's is the median of 300 frames, its graphics clock left to vary.

## In an app

The crate has no dependencies. It gives the shader and the weights as constants:

```toml
[dependencies]
aiostreams-upscale = { git = "https://github.com/AIOStreams/media", rev = "<commit>" }
```

```rust
// The mpv shader, for a player that takes a file.
std::fs::write(&path, aiostreams_upscale::MPV_SHADER)?;
// The same network as a WebGPU-Player pass file.
let pass = aiostreams_upscale::WEBGPU_PLAYER_PASS;
// The weights, for an app that builds its own shaders.
let network: serde_json::Value = serde_json::from_str(aiostreams_upscale::NETWORK)?;
// Below this much enlarging, the shader leaves the picture alone.
let min_scale = aiostreams_upscale::MIN_SCALE;
```

`NETWORK` is `{span, layers: [{weight: [out][in][y][x], bias, step?}], steepen}`. Each layer has a 3x3 kernel whose taps are `step` pixels apart (1 when absent); the first reads luma, each later one reads the four features before it, and every layer's output is clamped to 0..1, which is its activation and lets the features live in 8-bit textures. The last layer's four values are the pixels at twice the size, as `(value - 0.5) * span` added to the bilinear picture. `steepen` holds the last step's numbers; the documentation on `NETWORK` gives its formula.

## How it was made

It was trained on clean 1080p frames from 13 shows, with damaged copies of each as input, from clean downscales to heavily compressed SD encodes, plus a share of real old SD footage, for which Anime4K's Mode A was the target since no clean copy exists. The frames aren't included. Anime4K, by bloc97 under the MIT licence, was the teacher for that share and the comparison throughout.

After changing `network.json`, regenerate the shaders with `cargo run -p aiostreams-upscale --example generate`.

## Licence

MIT, as in [`LICENSE`](LICENSE), unlike the rest of this repository.
