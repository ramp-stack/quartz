/// Built-in post-processing shaders registered by Quartz into the
/// wgpu_canvas `ShaderRegistry` via the `FrameEnvelope` system.
///
/// These follow the `EffectParams` convention:
///   group(0) binding(0) = scene texture
///   group(0) binding(1) = sampler
///   group(1) binding(0) = EffectParams uniform (p0..p3 vec4s + screen vec4)

/// The shader ID used for the built-in bloom post-processing shader.
pub const BLOOM_SHADER_ID: &str = "__builtin_bloom";

/// Bloom post-processing shader (EffectParams convention).
///
/// Params:
///   p0.x = threshold (luminance cutoff for bright extraction)
///   p0.y = strength  (bloom intensity multiplier)
///   screen.x = width, screen.y = height
pub const BLOOM_WGSL: &str = r#"
struct EffectParams {
    p0: vec4<f32>,
    p1: vec4<f32>,
    p2: vec4<f32>,
    p3: vec4<f32>,
    screen: vec4<f32>,
}

@group(0) @binding(0) var t_scene: texture_2d<f32>;
@group(0) @binding(1) var s_scene: sampler;
@group(1) @binding(0) var<uniform> params: EffectParams;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );

    let p = positions[vi];

    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
    return out;
}

fn bright_extract(color: vec3<f32>, threshold: f32) -> vec3<f32> {
    let lum = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    let knee = threshold * 0.7;
    let soft = clamp(lum - threshold + knee, 0.0, 2.0 * knee);
    let contrib = soft * soft / (4.0 * knee + 0.0001);
    let brightness = max(contrib, lum - threshold) / max(lum, 0.0001);
    return color * clamp(brightness, 0.0, 1.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let threshold = params.p0.x;
    let strength  = params.p0.y;

    let original = textureSample(t_scene, s_scene, in.uv);
    let tx = 1.0 / params.screen.x;
    let ty = 1.0 / params.screen.y;

    var bloom = vec3<f32>(0.0);
    var offsets = array<f32, 4>(1.0, 2.0, 4.0, 6.0);
    var weights = array<f32, 4>(0.20, 0.15, 0.10, 0.05);
    let center_weight: f32 = 0.20;

    bloom += bright_extract(original.rgb, threshold) * center_weight;

    for (var i = 0u; i < 4u; i = i + 1u) {
        let o = offsets[i];
        let w = weights[i];
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>( tx * o, 0.0)).rgb, threshold) * w;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>(-tx * o, 0.0)).rgb, threshold) * w;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>(0.0,  ty * o)).rgb, threshold) * w;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>(0.0, -ty * o)).rgb, threshold) * w;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>( tx * o,  ty * o)).rgb, threshold) * w * 0.5;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>(-tx * o,  ty * o)).rgb, threshold) * w * 0.5;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>( tx * o, -ty * o)).rgb, threshold) * w * 0.5;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>(-tx * o, -ty * o)).rgb, threshold) * w * 0.5;
    }

    return vec4<f32>(original.rgb + bloom * strength, original.a);
}
"#;

/// Vignette post-processing shader.
///
/// Params:
///   p0.x = strength (0.0 = no vignette, 1.0 = full dark corners)
///   p0.y = radius   (0.0..1.0, how far from center the darkening starts)
///   p0.z = softness (how gradual the falloff is)
pub const VIGNETTE_SHADER_ID: &str = "__builtin_vignette";
pub const VIGNETTE_WGSL: &str = r#"
struct EffectParams {
    p0: vec4<f32>,
    p1: vec4<f32>,
    p2: vec4<f32>,
    p3: vec4<f32>,
    screen: vec4<f32>,
}

@group(0) @binding(0) var t_scene: texture_2d<f32>;
@group(0) @binding(1) var s_scene: sampler;
@group(1) @binding(0) var<uniform> params: EffectParams;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );
    let p = positions[vi];
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let strength = params.p0.x;
    let radius   = params.p0.y;
    let softness = params.p0.z;

    let original = textureSample(t_scene, s_scene, in.uv);
    let center   = vec2<f32>(0.5, 0.5);
    let dist     = distance(in.uv, center);
    let vignette = 1.0 - smoothstep(radius, radius + softness, dist) * strength;

    return vec4<f32>(original.rgb * vignette, original.a);
}
"#;

/// Chromatic aberration post-processing shader.
///
/// Params:
///   p0.x = intensity (pixel offset for R/B channel shift, e.g. 2.0..6.0)
pub const CHROMATIC_ABERRATION_SHADER_ID: &str = "__builtin_chromatic_aberration";
pub const CHROMATIC_ABERRATION_WGSL: &str = r#"
struct EffectParams {
    p0: vec4<f32>,
    p1: vec4<f32>,
    p2: vec4<f32>,
    p3: vec4<f32>,
    screen: vec4<f32>,
}

@group(0) @binding(0) var t_scene: texture_2d<f32>;
@group(0) @binding(1) var s_scene: sampler;
@group(1) @binding(0) var<uniform> params: EffectParams;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );
    let p = positions[vi];
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let intensity = params.p0.x;
    let tx = intensity / params.screen.x;
    let ty = intensity / params.screen.y;

    let dir = normalize(in.uv - vec2<f32>(0.5, 0.5));
    let offset = dir * vec2<f32>(tx, ty);

    let r = textureSample(t_scene, s_scene, in.uv + offset).r;
    let g = textureSample(t_scene, s_scene, in.uv).g;
    let b = textureSample(t_scene, s_scene, in.uv - offset).b;
    let a = textureSample(t_scene, s_scene, in.uv).a;

    return vec4<f32>(r, g, b, a);
}
"#;

/// Night-mode combined shader: bloom + vignette + subtle chromatic aberration.
///
/// Params:
///   p0.x = bloom threshold
///   p0.y = bloom strength
///   p0.z = vignette strength (0.0..1.0)
///   p0.w = vignette radius
///   p1.x = vignette softness
///   p1.y = chromatic aberration intensity (pixels)
pub const NIGHT_MODE_SHADER_ID: &str = "__builtin_night_mode";
pub const NIGHT_MODE_WGSL: &str = r#"
struct EffectParams {
    p0: vec4<f32>,
    p1: vec4<f32>,
    p2: vec4<f32>,
    p3: vec4<f32>,
    screen: vec4<f32>,
}

@group(0) @binding(0) var t_scene: texture_2d<f32>;
@group(0) @binding(1) var s_scene: sampler;
@group(1) @binding(0) var<uniform> params: EffectParams;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );
    let p = positions[vi];
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
    return out;
}

fn bright_extract(color: vec3<f32>, threshold: f32) -> vec3<f32> {
    let lum = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    let knee = threshold * 0.7;
    let soft = clamp(lum - threshold + knee, 0.0, 2.0 * knee);
    let contrib = soft * soft / (4.0 * knee + 0.0001);
    let brightness = max(contrib, lum - threshold) / max(lum, 0.0001);
    return color * clamp(brightness, 0.0, 1.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let bloom_threshold = params.p0.x;
    let bloom_strength  = params.p0.y;
    let vig_strength    = params.p0.z;
    let vig_radius      = params.p0.w;
    let vig_softness    = params.p1.x;
    let ca_intensity    = params.p1.y;

    let tx = 1.0 / params.screen.x;
    let ty = 1.0 / params.screen.y;

    // ── Chromatic aberration ────────────────────────────────────────────
    let ca_tx = ca_intensity / params.screen.x;
    let ca_ty = ca_intensity / params.screen.y;
    let ca_dir = normalize(in.uv - vec2<f32>(0.5, 0.5));
    let ca_off = ca_dir * vec2<f32>(ca_tx, ca_ty);

    let r_sample = textureSample(t_scene, s_scene, in.uv + ca_off).r;
    let g_sample = textureSample(t_scene, s_scene, in.uv).g;
    let b_sample = textureSample(t_scene, s_scene, in.uv - ca_off).b;
    let a_sample = textureSample(t_scene, s_scene, in.uv).a;
    let original = vec4<f32>(r_sample, g_sample, b_sample, a_sample);

    // ── Bloom ───────────────────────────────────────────────────────────
    var bloom = vec3<f32>(0.0);
    var offsets = array<f32, 4>(1.0, 2.0, 4.0, 6.0);
    var weights = array<f32, 4>(0.20, 0.15, 0.10, 0.05);
    let center_weight: f32 = 0.20;

    bloom += bright_extract(original.rgb, bloom_threshold) * center_weight;

    for (var i = 0u; i < 4u; i = i + 1u) {
        let o = offsets[i];
        let w = weights[i];
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>( tx * o, 0.0)).rgb, bloom_threshold) * w;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>(-tx * o, 0.0)).rgb, bloom_threshold) * w;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>(0.0,  ty * o)).rgb, bloom_threshold) * w;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>(0.0, -ty * o)).rgb, bloom_threshold) * w;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>( tx * o,  ty * o)).rgb, bloom_threshold) * w * 0.5;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>(-tx * o,  ty * o)).rgb, bloom_threshold) * w * 0.5;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>( tx * o, -ty * o)).rgb, bloom_threshold) * w * 0.5;
        bloom += bright_extract(textureSample(t_scene, s_scene, in.uv + vec2<f32>(-tx * o, -ty * o)).rgb, bloom_threshold) * w * 0.5;
    }

    let lit = original.rgb + bloom * bloom_strength;

    // ── Vignette ────────────────────────────────────────────────────────
    let center   = vec2<f32>(0.5, 0.5);
    let dist     = distance(in.uv, center);
    let vignette = 1.0 - smoothstep(vig_radius, vig_radius + vig_softness, dist) * vig_strength;

    return vec4<f32>(lit * vignette, original.a);
}
"#;
