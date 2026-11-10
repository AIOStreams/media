//! The upscaling network, for apps that run it.

/// With less enlarging than this, the network only moves a sharp picture away from the original.
pub const MIN_SCALE: f32 = 1.2;

/// `{span, layers: [{weight: [out][in][y][x], bias}]}`, for an app that builds its own shaders. Every
/// 3x3 layer clamps to 0..1, which is its activation; the last one's four values are the pixels at
/// twice the size, as `(value - 0.5) * span` over the bilinear picture.
pub const NETWORK: &str = include_str!("../network.json");

/// The network as a shader for mpv and libplacebo. `cargo run --example generate` writes it.
pub const MPV_SHADER: &str = include_str!("../shaders/aiostreams-upscale.glsl");

/// The network as a pass file for WebGPU-Player's presenter.
pub const WEBGPU_PLAYER_PASS: &str = include_str!("../shaders/aiostreams-upscale.wgsl");
