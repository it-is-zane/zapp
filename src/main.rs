#![warn(clippy::pedantic, clippy::all, clippy::nursery)]

mod app;
mod render;
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
    surface: wgpu::Surface<'a>,
    surface_config: wgpu::SurfaceConfiguration,
    command_encoder_descriptor: wgpu::CommandEncoderDescriptor<'a>,
    texture_view_descriptor: wgpu::TextureViewDescriptor<'a>,

    font_system: glyphon::FontSystem,
    swach_cache: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    text_renderer: glyphon::TextRenderer,
    text_buffer: glyphon::Buffer,

    window: std::sync::Arc<winit::window::Window>,
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

                self.viewport.update(
                    &gpu.queue,
                    glyphon::Resolution {
                        width: size.width,
                        height: size.height,
                    },
                );
            }
            winit::event::WindowEvent::RedrawRequested => {
                self.text_renderer
                    .prepare(
                        &gpu.device,
                        &gpu.queue,
                        &mut self.font_system,
                        &mut self.atlas,
                        &self.viewport,
                        [glyphon::TextArea {
                            buffer: &self.text_buffer,
                            left: 10.0,
                            top: 10.0,
                            scale: 1.0,
                            bounds: glyphon::TextBounds {
                                left: 0,
                                top: 0,
                                right: self.surface_config.width as i32,
                                bottom: self.surface_config.height as i32,
                            },
                            default_color: glyphon::Color::rgb(255, 255, 255),
                            custom_glyphs: &[],
                        }],
                        &mut self.swach_cache,
                    )
                    .unwrap();

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
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                ..Default::default()
                            },
                        })],
                        ..Default::default()
                    });

                    self.text_renderer
                        .render(&self.atlas, &self.viewport, &mut pass)
                        .unwrap();
                }

                self.window.pre_present_notify();
                gpu.queue.submit([encoder.finish()]);
                texture.present();
            }
            winit::event::WindowEvent::CloseRequested => {
                return app::Command::RemoveWindow(self.window_id);
            }
            _ => {
                // println!("{event:?}");
            }
        }

        app::Command::Nothing
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
    let gpu = util::block_on(render::GpuContext::new()).unwrap();

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

            let mut font_system = glyphon::FontSystem::new();
            font_system.db_mut().load_system_fonts();

            let swach_cache = glyphon::SwashCache::new();
            let cache = glyphon::Cache::new(&gpu.device);
            let viewport = glyphon::Viewport::new(&gpu.device, &cache);
            let mut atlas =
                glyphon::TextAtlas::new(&gpu.device, &gpu.queue, &cache, surface_config.format);
            let text_renderer = glyphon::TextRenderer::new(
                &mut atlas,
                &gpu.device,
                wgpu::MultisampleState::default(),
                None,
            );
            let mut text_buffer =
                glyphon::Buffer::new(&mut font_system, glyphon::Metrics::new(30.0, 42.0));

            text_buffer.set_size(
                &mut font_system,
                Some((window.inner_size().width as f64 * window.scale_factor()) as f32),
                Some((window.inner_size().height as f64 * window.scale_factor()) as f32),
            );
            text_buffer.set_text(
                &mut font_system,
                "Zane Gant!☻",
                &glyphon::Attrs::new().family(glyphon::Family::SansSerif),
                glyphon::Shaping::Advanced,
            );
            text_buffer.shape_until_scroll(&mut font_system, false);

            (
                window_id,
                Box::new(DevWindow {
                    window_id,
                    surface,
                    surface_config,
                    command_encoder_descriptor,
                    texture_view_descriptor,
                    font_system,
                    swach_cache,
                    viewport,
                    atlas,
                    text_renderer,
                    text_buffer,
                    window,
                }),
            )
        }),
    );

    event_loop.set_control_flow(winit::event_loop::ControlFlow::wait_duration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));

    _ = event_loop.run_app(&mut app);

    std::process::exit(0);
}
