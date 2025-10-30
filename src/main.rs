#![warn(clippy::pedantic, clippy::all, clippy::nursery)]

mod app;
mod render;
mod text;
mod util;

// create views from data
// views can be reused and interpolated
// 1. data changes
// 2. get new view
// 3. interpolate against old view
// if interupted set old view to the interpolated view?
//
// there should be multiple types of simple views
// each view only needs to interpolate against itself
// list, and grid views are some examples

struct DevWindow<'a> {
    window_id: winit::window::WindowId,
    window: std::sync::Arc<winit::window::Window>,
    surface: wgpu::Surface<'a>,
    surface_config: wgpu::SurfaceConfiguration,
    command_encoder_descriptor: wgpu::CommandEncoderDescriptor<'a>,
    texture_view_descriptor: wgpu::TextureViewDescriptor<'a>,
    text_renderer: text::TextRenderer,
    text: text::GpuTextObject,
    window_size: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,
    window_creation: std::time::Instant,
}

impl app::WindowHandler for DevWindow<'_> {
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: winit::event::WindowEvent,
        gpu: &crate::render::GpuContext,
    ) -> app::Command {
        match event {
            winit::event::WindowEvent::MouseInput {
                state: winit::event::ElementState::Pressed,
                ..
            } => {
                _ = self.window.drag_window();
            }
            winit::event::WindowEvent::Resized(size) => {
                self.surface_config.width = size.width;
                self.surface_config.height = size.height;

                self.surface.configure(&gpu.device, &self.surface_config);

                gpu.queue.write_buffer(
                    &self.window_size,
                    0,
                    util::as_bytes(&nalgebra::Vector2::new(size.width, size.height).cast::<f32>()),
                );
            }
            winit::event::WindowEvent::RedrawRequested => {
                let mut encoder = gpu
                    .device
                    .create_command_encoder(&self.command_encoder_descriptor);

                let texture = self.surface.get_current_texture().unwrap();
                let view = texture.texture.create_view(&self.texture_view_descriptor);

                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                ..Default::default()
                            },
                        })],
                        ..Default::default()
                    });

                    pass.set_bind_group(0, Some(&self.viewport_bind_group), &[]);

                    self.text_renderer.render(&mut pass, &self.text);
                }

                gpu.queue.submit([encoder.finish()]);
                texture.present();
            }
            winit::event::WindowEvent::CloseRequested => {
                return app::Command::RemoveWindow(self.window_id);
            }
            _ => {
                println!("{event:?}");
            }
        }

        app::Command::Nothing
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
    let gpu = util::block_on(render::GpuContext::new());

    let mut app = app::App::new(
        gpu,
        Box::new(|gpu, event_loop| {
            let window = std::sync::Arc::new(
                event_loop
                    .create_window(
                        winit::window::WindowAttributes::default().with_decorations(false),
                    )
                    .expect("failed to create window"),
            );

            let window_id = window.id();
            let size = window.inner_size();

            let surface = gpu
                .instance
                .create_surface(window.clone())
                .expect("failed to create surface");

            let surface_config = surface
                .get_default_config(&gpu.adapter, size.width, size.height)
                .unwrap();

            surface.configure(&gpu.device, &surface_config);

            let command_encoder_descriptor = wgpu::CommandEncoderDescriptor {
                label: Some("DevWindow Command Encoder"),
            };

            let texture_view_descriptor = wgpu::TextureViewDescriptor {
                label: Some("DevWindow Texture View"),
                ..Default::default()
            };

            let text_renderer = text::TextRenderer::new(
                serde_json::from_str(include_str!("atlas.json")).unwrap(),
                image::load_from_memory(include_bytes!("atlas.png"))
                    .unwrap()
                    .to_rgba8(),
                gpu,
                surface_config.format,
            )
            .unwrap();

            let mut text = text_renderer.create_gpu_text_object(gpu, "Happy Birthday\nMom!");

            text.set_transform(
                gpu,
                nalgebra::Matrix4::identity()
                    .append_translation(&nalgebra::Vector3::new(-1.0, 1.0, 0.0)),
            );

            let window_size = wgpu::util::DeviceExt::create_buffer_init(
                &gpu.device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Window Size Vector"),
                    contents: util::as_bytes(
                        &nalgebra::Vector2::new(size.width, size.height).cast::<f32>(),
                    ),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                },
            );

            let viewport_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Viewport Info"),
                layout: &text_renderer.get_viewport_bind_group_layout(),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &window_size,
                        offset: 0,
                        size: std::num::NonZero::new(
                            std::mem::size_of::<nalgebra::Vector2<f32>>() as u64
                        ),
                    }),
                }],
            });

            (
                window_id,
                Box::new(DevWindow {
                    window_id,
                    window,
                    surface,
                    surface_config,
                    command_encoder_descriptor,
                    texture_view_descriptor,
                    text_renderer,
                    text,
                    window_size,
                    viewport_bind_group,
                    window_creation: std::time::Instant::now(),
                }),
            )
        }),
    );

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    _ = event_loop.run_app(&mut app);

    std::process::exit(0);
}
