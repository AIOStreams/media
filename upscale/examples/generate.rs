//! Writes `shaders/aiostreams-upscale.glsl` from `network.json`.

use aiostreams_upscale::{MIN_SCALE, NETWORK};
use serde::Deserialize;

#[derive(Deserialize)]
struct Network {
    span: f32,
    layers: Vec<Layer>,
}

#[derive(Deserialize)]
struct Layer {
    weight: Vec<Vec<Vec<Vec<f32>>>>,
    bias: Vec<f32>,
}

impl Layer {
    /// How much input channel `channel` at this tap adds to each output.
    fn column(&self, y: usize, x: usize, channel: usize) -> Vec<f32> {
        self.weight.iter().map(|out| out[channel][y][x]).collect()
    }

    fn is_empty(&self, y: usize, x: usize) -> bool {
        self.weight.iter().flatten().all(|input| input[y][x] == 0.0)
    }
}

fn numbers(values: &[f32]) -> String {
    let shown: Vec<String> = values.iter().map(|v| format!("{v:?}")).collect();
    shown.join(",")
}

fn shader(network: &Network) -> String {
    let when = format!("//!WHEN OUTPUT.h MAIN.h / {MIN_SCALE} >");
    let mut out = String::from(
        "// aiostreams-upscale, https://github.com/AIOStreams/media/tree/main/upscale\n\
         // SPDX-License-Identifier: MIT\n\n",
    );
    let mut previous: Option<String> = None;
    for (i, layer) in network.layers.iter().enumerate() {
        let target = format!("UPSCALE_{i}");
        let bound = previous.as_deref().unwrap_or("MAIN");
        out += &format!(
            "//!DESC upscale layer {i}\n//!HOOK MAIN\n//!BIND {bound}\n//!SAVE {target}\n\
             //!WIDTH MAIN.w\n//!HEIGHT MAIN.h\n//!COMPONENTS 4\n{when}\n\
             vec4 hook() {{\n    vec4 r = vec4({});\n",
            numbers(&layer.bias)
        );
        for y in 0..3 {
            for x in 0..3 {
                if layer.is_empty(y, x) {
                    continue;
                }
                let offset = format!("vec2({}.0, {}.0)", x as i32 - 1, y as i32 - 1);
                out += &match &previous {
                    None => format!(
                        "    r += vec4({}) * dot(MAIN_texOff({offset}).rgb, vec3(0.299, 0.587, 0.114));\n",
                        numbers(&layer.column(y, x, 0))
                    ),
                    Some(previous) => {
                        let columns: Vec<f32> =
                            (0..4).flat_map(|c| layer.column(y, x, c)).collect();
                        format!(
                            "    r += mat4({}) * {previous}_texOff({offset});\n",
                            numbers(&columns)
                        )
                    }
                };
            }
        }
        // These textures can hold anything, and the network relies on 8-bit's clamp.
        out += "    return clamp(r, 0.0, 1.0);\n}\n\n";
        previous = Some(target);
    }
    let last = previous.unwrap_or_default();
    let span = network.span;
    out += &format!(
        "//!DESC upscale\n//!HOOK MAIN\n//!BIND MAIN\n//!BIND {last}\n//!SAVE MAIN\n\
         //!WIDTH {last}.w 2 *\n//!HEIGHT {last}.h 2 *\n{when}\n\
         vec4 hook() {{\n    vec2 f = fract({last}_pos * {last}_size);\n    ivec2 i = ivec2(f * vec2(2.0));\n\
         \x20   float e = {last}_tex((vec2(0.5) - f) * {last}_pt + {last}_pos)[i.y * 2 + i.x];\n\
         \x20   return MAIN_tex(MAIN_pos) + vec4(vec3((e - 0.5) * {span:?}), 0.0);\n}}\n"
    );
    out
}

fn main() {
    let network: Network = serde_json::from_str(NETWORK).expect("network.json");
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/shaders/aiostreams-upscale.glsl"
    );
    std::fs::write(path, shader(&network)).expect("write the shader");
}
