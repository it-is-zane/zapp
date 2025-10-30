@group(0) @binding(0) var<uniform> screen_size: vec2<f32>;
@group(1) @binding(0) var atlas_texture: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;
@group(2) @binding(0) var<uniform> transform: mat4x4<f32>;

@vertex
fn vertex(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
) -> VertexOut {
    return VertexOut (
        transform * vec4(position / screen_size, 0.0, 1.0),
        vec2(uv.x,uv.y),
    );
}

struct VertexOut {
    @builtin(position) possition: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let alpha = textureSample(atlas_texture, atlas_sampler, uv).w;
    return vec4(0.0, 0.0, 0.0, alpha);
}
