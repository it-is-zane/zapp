#[derive(Debug)]
pub struct CharData {
    character: char,
    uv_transform: nalgebra::Matrix3<f32>,
    char_transform: nalgebra::Matrix3<f32>,
    advance: f32,
}

impl CharData {
    #[allow(clippy::cast_possible_truncation)]
    fn from_json_value(
        json: &serde_json::Value,
        texture_size: nalgebra::Vector2<f32>,
    ) -> Result<Self, &str> {
        let char = json.get("char").ok_or("could not find char")?;

        let character = char
            .get("value")
            .and_then(serde_json::Value::as_u64)
            .map(|n| n as u32)
            .and_then(char::from_u32)
            .ok_or("failed to parse value")?;

        let get_vec2 = |field: Option<&serde_json::Value>, x: &str, y: &str| {
            field
                .and_then(|o| Some((o.get(x)?, o.get(y)?)))
                .and_then(|(x, y)| Some((x.as_f64()? as f32, y.as_f64()? as f32)))
                .map(|(x, y)| nalgebra::Vector2::new(x, y))
        };

        let advance = char
            .get("advanceX")
            .and_then(serde_json::Value::as_f64)
            .ok_or("failed to parse advanceX")? as f32;

        let offset = get_vec2(char.get("offset"), "x", "y").ok_or("failed to parse offset")?;
        let position =
            get_vec2(json.get("position"), "x", "y").ok_or("failed to parse position")?;
        let source_size = get_vec2(json.get("sourceSize"), "width", "height")
            .ok_or("failed to parse sourceSize")?;

        let uv_transform = nalgebra::Matrix3::identity()
            .append_nonuniform_scaling(&source_size)
            .append_translation(&position)
            .append_nonuniform_scaling(&texture_size.map(|v| 1.0 / v));

        let char_transform = nalgebra::Matrix3::identity()
            .append_nonuniform_scaling(&source_size)
            .append_translation(&offset)
            .append_nonuniform_scaling(&nalgebra::Vector2::new(1.0, -1.0));

        Ok(Self {
            character,
            uv_transform,
            char_transform,
            advance,
        })
    }
}

pub struct FontData {
    characters: std::collections::HashMap<char, CharData>,
    font_size: f32,
    atlas_size: nalgebra::Vector2<f32>,
    bind_group: wgpu::BindGroup,
}
impl FontData {
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(
        json: serde_json::Value,
        image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
        bind_group_layout: &wgpu::BindGroupLayout,
        gpu: &crate::render::GpuContext,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let sprites = json
            .get("sprites")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten();

        let font_size = json
            .get("atlas")
            .and_then(|a| a.get("fontSize")?.as_f64())
            .map(|f| f as f32)
            .ok_or("failed to get fontSize")?;

        let image_extent = wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };

        let image_size = nalgebra::Vector2::new(image.width(), image.height()).cast::<f32>();

        let characters = sprites
            .map(|value| CharData::from_json_value(value, image_size))
            .filter_map(std::result::Result::ok)
            .map(|data| (data.character, data))
            .collect();

        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Font Atlas"),
            size: image_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * image.dimensions().0),
                rows_per_image: Some(image.dimensions().1),
            },
            image_extent,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Font Atlas Texture View"),
            ..Default::default()
        });

        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Font Atlas Sampler"),
            ..Default::default()
        });

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("A Font Atlas Bind Group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(Self {
            characters,
            font_size,
            bind_group,
            atlas_size: nalgebra::Vector2::new(image.width(), image.height()).cast::<f32>(),
        })
    }
}

pub struct GpuTextObject {
    pos: wgpu::Buffer,
    uv: wgpu::Buffer,
    range: std::ops::Range<u32>,
    transform: nalgebra::Matrix4<f32>,
    transform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl GpuTextObject {
    fn new(
        gpu: &crate::render::GpuContext,
        pos: wgpu::Buffer,
        uv: wgpu::Buffer,
        bind_group_layout: &wgpu::BindGroupLayout,
        range: std::ops::Range<u32>,
    ) -> Self {
        let transform = nalgebra::Matrix4::identity().scale(1.0 / 10000.0);

        let transform_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Text Transform"),
                contents: crate::util::as_bytes(&transform),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GpuTextObject"),
            layout: bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &transform_buffer,
                    offset: 0,
                    size: std::num::NonZero::new(
                        std::mem::size_of::<nalgebra::Matrix4<f32>>() as u64
                    ),
                }),
            }],
        });

        Self {
            pos,
            uv,
            range,
            transform,
            transform_buffer,
            bind_group,
        }
    }

    pub fn set_transform(&mut self, gpu: &crate::render::GpuContext, mat4: nalgebra::Matrix4<f32>) {
        self.transform = mat4;
        gpu.queue
            .write_buffer(&self.transform_buffer, 0, crate::util::as_bytes(&mat4));
        gpu.queue.submit([]);
    }
}

pub struct TextRenderer {
    font_data: FontData,
    render_pipeline: wgpu::RenderPipeline,
}

impl TextRenderer {
    pub fn new(
        json: serde_json::Value,
        image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
        gpu: &crate::render::GpuContext,
        format: wgpu::TextureFormat,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let module = gpu
            .device
            .create_shader_module(wgpu::include_wgsl!("text.wgsl"));

        let render_pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Text Render Pipeline"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vertex"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: 2 * 32 / 8,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &[wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 0,
                            }],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: 2 * 32 / 8,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &[wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 1,
                            }],
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fragment"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

        Ok(Self {
            font_data: FontData::new(json, image, &render_pipeline.get_bind_group_layout(1), gpu)?,
            render_pipeline,
        })
    }

    pub fn get_viewport_bind_group_layout(&self) -> wgpu::BindGroupLayout {
        self.render_pipeline.get_bind_group_layout(0)
    }

    pub fn create_gpu_text_object(
        &self,
        gpu: &crate::render::GpuContext,
        text: &str,
    ) -> GpuTextObject {
        let square = [
            nalgebra::Vector2::new(0.0, 0.0),
            nalgebra::Vector2::new(0.0, 1.0),
            nalgebra::Vector2::new(1.0, 1.0),
            nalgebra::Vector2::new(0.0, 0.0),
            nalgebra::Vector2::new(1.0, 1.0),
            nalgebra::Vector2::new(1.0, 0.0),
        ];

        let mut advance = nalgebra::Vector2::zeros();
        let mut vertex = Vec::new();
        let mut uv = Vec::new();

        for (c, data) in text.chars().map(|c| (c, self.font_data.characters.get(&c))) {
            let Some(data) = data else {
                if let Some(data) = data {
                    advance += nalgebra::Vector2::new(data.advance, 0.0);
                } else if c == '\n' {
                    advance = nalgebra::Vector2::new(0.0, advance.y - self.font_data.font_size);
                } else {
                    advance += nalgebra::Vector2::new(self.font_data.font_size, 0.0);
                }
                continue;
            };

            vertex.extend_from_slice(&square.map(|v| {
                data.char_transform
                    .transform_point(&nalgebra::OPoint::from(v))
                    + advance
            }));

            uv.extend_from_slice(&square.map(|v| {
                data.uv_transform
                    .transform_point(&nalgebra::OPoint::from(v))
            }));

            advance += nalgebra::Vector2::new(data.advance, 0.0);
        }

        GpuTextObject::new(
            gpu,
            wgpu::util::DeviceExt::create_buffer_init(
                &gpu.device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Character Vertex"),
                    contents: crate::util::slice_as_bytes(&vertex),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ),
            wgpu::util::DeviceExt::create_buffer_init(
                &gpu.device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Character UV"),
                    contents: crate::util::slice_as_bytes(&uv),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ),
            &self.render_pipeline.get_bind_group_layout(2),
            0..vertex.len() as u32,
        )
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass, text: &GpuTextObject) {
        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(1, &self.font_data.bind_group, &[]);
        pass.set_bind_group(2, Some(&text.bind_group), &[]);
        pass.set_vertex_buffer(0, text.pos.slice(..));
        pass.set_vertex_buffer(1, text.uv.slice(..));
        pass.draw(text.range.clone(), 0..1);
    }
}
