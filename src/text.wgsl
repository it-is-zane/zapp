@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

@group(1) @binding(0) var<uniform> color: vec4<f32>;

@group(2) @binding(0) var<uniform> transform: mat3x3<f32>;
@group(2) @binding(1) var<storage, read> pos_transforms: array<mat3x3<f32>>;
@group(2) @binding(3) var<storage, read> uv_transforms: array<mat3x3<f32>>;


struct Vertex {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @builtin(instance_index) instance: u32,
) -> Vertex {
    return Vertex(
        vec4(transform * pos_transforms[instance] * vec3(pos, 1.0), 1.0),
        (uv_transforms[instance] * vec3(uv, 1.0)).xy,
    );
}

@fragment
fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    return textureSample(texture, tex_sampler, in.uv) * color;
}
