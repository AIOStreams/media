//! Writes the shaders in `shaders/` from `network.json`.

use aiostreams_upscale::{MIN_SCALE, NETWORK};
use serde::Deserialize;

#[derive(Deserialize)]
struct Network {
    span: f32,
    layers: Vec<Layer>,
    steepen: Option<Steepen>,
}

#[derive(Deserialize)]
struct Layer {
    weight: Vec<Vec<Vec<Vec<f32>>>>,
    bias: Vec<f32>,
    #[serde(default = "one")]
    step: i32,
}

#[derive(Deserialize)]
struct Steepen {
    strength: f32,
    low: f32,
    high: f32,
    darken: f32,
}

fn one() -> i32 {
    1
}

impl Layer {
    /// How much input channel `channel` at this tap adds to each output.
    fn column(&self, y: usize, x: usize, channel: usize) -> Vec<f32> {
        self.weight.iter().map(|out| out[channel][y][x]).collect()
    }

    fn is_empty(&self, y: usize, x: usize) -> bool {
        self.weight.iter().flatten().all(|input| input[y][x] == 0.0)
    }

    fn offset(&self, y: usize, x: usize) -> (i32, i32) {
        ((x as i32 - 1) * self.step, (y as i32 - 1) * self.step)
    }
}

fn numbers(values: &[f32]) -> String {
    let shown: Vec<String> = values.iter().map(|v| format!("{v:?}")).collect();
    shown.join(",")
}

const RANGE: &str = "UPSCALE_RANGE";

/// The shader's last lines, which read `c` and `range`.
fn curve(steepen: &Steepen, wgsl: bool) -> String {
    let Steepen {
        strength,
        low,
        high,
        darken,
    } = steepen;
    let scale = 1.0 / strength.tanh();
    let (float, darker) = if wgsl {
        ("let", format!("select(0.0, {darken:?}, t < 0.0)"))
    } else {
        ("float", format!("(t < 0.0 ? {darken:?} : 0.0)"))
    };
    let luma = if wgsl {
        "vec3f(0.299, 0.587, 0.114)"
    } else {
        "vec3(0.299, 0.587, 0.114)"
    };
    format!(
        "    {float} y = dot(c.rgb, {luma});\n\
         \x20   {float} gate = clamp((range.y - range.x - {low:?}) / {:?}, 0.0, 1.0);\n\
         \x20   {float} mid = (range.x + range.y) * 0.5;\n\
         \x20   {float} reach = max((range.y - range.x) * 0.5, 1e-4);\n\
         \x20   {float} t = clamp((y - mid) / reach, -4.0, 4.0);\n\
         \x20   {float} s = mid + (reach + {darker}) * tanh({strength:?} * t) * {scale:?};\n\
         \x20   return {}(c.rgb + gate * (s - y), c.a);\n",
        high - low,
        if wgsl { "vec4f" } else { "vec4" },
    )
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
                let (dx, dy) = layer.offset(y, x);
                let offset = format!("vec2({dx}.0, {dy}.0)");
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
    if network.steepen.is_some() {
        out += &format!(
            "//!DESC upscale range\n//!HOOK MAIN\n//!BIND MAIN\n//!SAVE {RANGE}\n\
             //!WIDTH MAIN.w\n//!HEIGHT MAIN.h\n//!COMPONENTS 2\n{when}\n\
             vec4 hook() {{\n    float low = 1.0, high = 0.0;\n\
             \x20   for (int y = -1; y <= 1; y++) {{\n        for (int x = -1; x <= 1; x++) {{\n\
             \x20           float l = dot(MAIN_texOff(vec2(x, y)).rgb, vec3(0.299, 0.587, 0.114));\n\
             \x20           low = min(low, l);\n            high = max(high, l);\n        }}\n    }}\n\
             \x20   return vec4(low, high, 0.0, 0.0);\n}}\n\n"
        );
    }
    let range_bind = if network.steepen.is_some() {
        format!("//!BIND {RANGE}\n")
    } else {
        String::new()
    };
    out += &format!(
        "//!DESC upscale\n//!HOOK MAIN\n//!BIND MAIN\n//!BIND {last}\n{range_bind}//!SAVE MAIN\n\
         //!WIDTH {last}.w 2 *\n//!HEIGHT {last}.h 2 *\n{when}\n\
         vec4 hook() {{\n    vec2 f = fract({last}_pos * {last}_size);\n    ivec2 i = ivec2(f * vec2(2.0));\n\
         \x20   vec2 centre = (vec2(0.5) - f) * {last}_pt + {last}_pos;\n\
         \x20   float e = {last}_tex(centre)[i.y * 2 + i.x];\n\
         \x20   vec4 c = MAIN_tex(MAIN_pos) + vec4(vec3((e - 0.5) * {span:?}), 0.0);\n"
    );
    out += &match &network.steepen {
        Some(steepen) => format!(
            "    vec2 range = {RANGE}_tex(centre).xy;\n{}",
            curve(steepen, false)
        ),
        None => "    return c;\n".to_string(),
    };
    out += "}\n";
    out
}

fn webgpu_pass(network: &Network) -> String {
    // Only blank lines may come before the first header, so the notice opens the first body.
    let notice = "// aiostreams-upscale, https://github.com/AIOStreams/media/tree/main/upscale\n\
                  // SPDX-License-Identifier: MIT\n\n";
    let head = format!(
        "//! shape: kernel\n//! hook: PRESCALE\n//! domain: source\n\
         //! when: output.height / source.height > {MIN_SCALE} && !hdr.source\n"
    );
    let mut out = String::new();
    let mut previous: Option<String> = None;
    for (i, layer) in network.layers.iter().enumerate() {
        let target = format!("UPSCALE_{i}");
        let input = previous.as_deref().unwrap_or("HOOKED");
        out += &format!(
            "//! name: upscale layer {i}\n{head}//! input: {input}\n//! save: {target}\n\
             //! output: HOOKED.size, rgba8unorm\n\n{}\
             fn kernel(coordinate: vec2f) -> vec4f {{\n    var r = vec4f({});\n",
            if i == 0 { notice } else { "" },
            numbers(&layer.bias)
        );
        for y in 0..3 {
            for x in 0..3 {
                if layer.is_empty(y, x) {
                    continue;
                }
                let (dx, dy) = layer.offset(y, x);
                let offset = format!("vec2f({dx}.0, {dy}.0)");
                out += &match &previous {
                    None => format!(
                        "    r += vec4f({}) * dot(HOOKED_texOff(coordinate, {offset}).rgb, vec3f(0.299, 0.587, 0.114));\n",
                        numbers(&layer.column(y, x, 0))
                    ),
                    Some(previous) => {
                        let columns: Vec<f32> =
                            (0..4).flat_map(|c| layer.column(y, x, c)).collect();
                        format!(
                            "    r += mat4x4f({}) * {previous}_texOff(coordinate, {offset});\n",
                            numbers(&columns)
                        )
                    }
                };
            }
        }
        out += "    return clamp(r, vec4f(0.0), vec4f(1.0));\n}\n\n";
        previous = Some(target);
    }
    let last = previous.unwrap_or_default();
    let span = network.span;
    if network.steepen.is_some() {
        out += &format!(
            "//! name: upscale range\n{head}//! input: HOOKED\n//! save: {RANGE}\n\
             //! output: HOOKED.size, rg16float\n\n\
             fn kernel(coordinate: vec2f) -> vec4f {{\n    var low = 1.0;\n    var high = 0.0;\n\
             \x20   for (var y = -1; y <= 1; y++) {{\n        for (var x = -1; x <= 1; x++) {{\n\
             \x20           let l = dot(HOOKED_texOff(coordinate, vec2f(f32(x), f32(y))).rgb, vec3f(0.299, 0.587, 0.114));\n\
             \x20           low = min(low, l);\n            high = max(high, l);\n        }}\n    }}\n\
             \x20   return vec4f(low, high, 0.0, 0.0);\n}}\n\n"
        );
    }
    let range_input = if network.steepen.is_some() {
        format!("//! input: {RANGE}\n")
    } else {
        String::new()
    };
    out += &format!(
        "//! name: upscale\n{head}//! input: HOOKED\n//! input: {last}\n{range_input}\
         //! output: HOOKED.width * 2, HOOKED.height * 2, rgba16float\n\n\
         fn kernel(coordinate: vec2f) -> vec4f {{\n    let f = fract(coordinate * {last}_size());\n\
         \x20   let i = vec2i(f * 2.0);\n\
         \x20   let centre = (vec2f(0.5) - f) * {last}_pt() + coordinate;\n\
         \x20   var e = {last}_tex(centre);\n\
         \x20   let c = HOOKED_tex(coordinate) + vec4f(vec3f((e[i.y * 2 + i.x] - 0.5) * {span:?}), 0.0);\n"
    );
    out += &match &network.steepen {
        Some(steepen) => format!(
            "    let range = {RANGE}_tex(centre).xy;\n{}",
            curve(steepen, true)
        ),
        None => "    return c;\n".to_string(),
    };
    out += "}\n";
    out
}

fn main() {
    let network: Network = serde_json::from_str(NETWORK).expect("network.json");
    let shaders = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders");
    std::fs::write(
        format!("{shaders}/aiostreams-upscale.glsl"),
        shader(&network),
    )
    .expect("write the mpv shader");
    std::fs::write(
        format!("{shaders}/aiostreams-upscale.wgsl"),
        webgpu_pass(&network),
    )
    .expect("write the WebGPU-Player pass");
}
