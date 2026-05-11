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

enum StartStop {
    Start(std::time::Instant),
    Stop(std::time::Instant),
}

impl std::fmt::Debug for StartStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start(_) => f.debug_tuple("Start").finish(),
            Self::Stop(_) => f.debug_tuple("Stop").finish(),
        }
    }
}

struct DevWindow<'a> {
    window_id: winit::window::WindowId,
    surface: wgpu::Surface<'a>,
    surface_config: wgpu::SurfaceConfiguration,
    command_encoder_descriptor: wgpu::CommandEncoderDescriptor<'a>,
    texture_view_descriptor: wgpu::TextureViewDescriptor<'a>,

    viewport: glyphon::Viewport,
    text_buffer: glyphon::Buffer,

    starts_and_stops: Vec<StartStop>,
}

impl DevWindow<'_> {
    fn new(app_state: &mut AppState, window: &std::sync::Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();

        let surface = app_state
            .gpu
            .instance
            .create_surface(window.clone())
            .expect("failed to create surface");

        let mut surface_config = surface
            .get_default_config(&app_state.gpu.adapter, size.width, size.height)
            .unwrap();

        surface_config.format = wgpu::TextureFormat::Rgba8Unorm;
        surface.configure(&app_state.gpu.device, &surface_config);

        let command_encoder_descriptor = wgpu::CommandEncoderDescriptor {
            label: Some("DevWindow Command Encoder"),
        };

        let texture_view_descriptor = wgpu::TextureViewDescriptor {
            label: Some("DevWindow Texture View"),
            ..Default::default()
        };

        let viewport = glyphon::Viewport::new(&app_state.gpu.device, &app_state.cache);

        let text_buffer = glyphon::Buffer::new(
            &mut app_state.font_system,
            glyphon::Metrics::new(30.0, 42.0),
        );

        DevWindow {
            window_id: window.id(),
            surface,
            surface_config,
            command_encoder_descriptor,
            texture_view_descriptor,
            viewport,
            text_buffer,
            starts_and_stops: Vec::new(),
        }
    }
}

impl app::WindowHandler<AppState> for DevWindow<'_> {
    fn window_event(
        &mut self,
        app_state: &mut AppState,
        window: &winit::window::Window,
        event: winit::event::WindowEvent,
    ) -> Vec<app::Command<AppState>> {
        match event {
            winit::event::WindowEvent::MouseInput {
                state: winit::event::ElementState::Pressed,
                ..
            } => {
                _ = window.drag_window();
            }
            winit::event::WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        text: Some(text),
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => match text.as_str() {
                " " => {
                    let total: std::time::Duration = self
                        .starts_and_stops
                        .chunks(2)
                        .map(|s| match s {
                            [StartStop::Start(a), StartStop::Stop(b)] => b.duration_since(*a),
                            [StartStop::Start(a)] => a.elapsed(),
                            a => {
                                todo!("was not expecting {a:?}")
                            }
                        })
                        .sum();

                    let (start_stop, control_flow) = match self.starts_and_stops.last() {
                        Some(StartStop::Start(_)) => {
                            let seconds = total.as_secs() % 60;
                            let minutes = (total.as_secs() / 60) % 60;
                            let hours = (total.as_secs() / 60) / 60;

                            println!("total {hours}h {minutes}m {seconds}s");

                            (
                                StartStop::Stop(std::time::Instant::now()),
                                winit::event_loop::ControlFlow::Wait,
                            )
                        }
                        Some(StartStop::Stop(stopped)) => {
                            let stopped = stopped.elapsed();

                            let seconds = stopped.as_secs() % 60;
                            let minutes = (stopped.as_secs() / 60) % 60;
                            let hours = (stopped.as_secs() / 60) / 60;

                            println!("stopped for {hours}h {minutes}m {seconds}s");

                            (
                                StartStop::Start(std::time::Instant::now()),
                                winit::event_loop::ControlFlow::wait_duration(
                                    std::time::Duration::from_secs_f32(
                                        2.0 - total.as_secs_f32() % 1.0,
                                    ),
                                ),
                            )
                        }
                        None => (
                            StartStop::Start(std::time::Instant::now()),
                            winit::event_loop::ControlFlow::wait_duration(
                                std::time::Duration::from_secs(1),
                            ),
                        ),
                    };

                    self.starts_and_stops.push(start_stop);

                    window.request_redraw();

                    return vec![app::Command::SetControlFlow(control_flow)];
                }
                "r" => {
                    self.starts_and_stops.clear();
                    window.request_redraw();
                }
                _ => (),
            },
            winit::event::WindowEvent::Resized(size) => {
                self.surface_config.width = size.width;
                self.surface_config.height = size.height;
                self.surface
                    .configure(&app_state.gpu.device, &self.surface_config);

                self.viewport.update(
                    &app_state.gpu.queue,
                    glyphon::Resolution {
                        width: size.width,
                        height: size.height,
                    },
                );
            }
            winit::event::WindowEvent::RedrawRequested => {
                let total: std::time::Duration = self
                    .starts_and_stops
                    .chunks(2)
                    .map(|s| match s {
                        [StartStop::Start(a), StartStop::Stop(b)] => b.duration_since(*a),
                        [StartStop::Start(a)] => a.elapsed(),
                        a => {
                            todo!("was not expecting {a:?}")
                        }
                    })
                    .sum();

                let seconds = total.as_secs() % 60;
                let minutes = (total.as_secs() / 60) % 60;
                let hours = (total.as_secs() / 60) / 60;

                self.text_buffer.set_text(
                    &mut app_state.font_system,
                    format!("{hours}:{minutes}:{seconds}\n{:?}", self.starts_and_stops).as_str(),
                    &glyphon::Attrs::new(),
                    glyphon::Shaping::Advanced,
                    Some(glyphon::cosmic_text::Align::Center),
                );

                let size = window.inner_size().cast::<f32>();
                let scale_factor = window.scale_factor() as f32;

                self.text_buffer.set_size(
                    &mut app_state.font_system,
                    Some(size.width * scale_factor),
                    Some(size.height * scale_factor),
                );

                app_state
                    .text_renderer
                    .prepare(
                        &app_state.gpu.device,
                        &app_state.gpu.queue,
                        &mut app_state.font_system,
                        &mut app_state.atlas,
                        &self.viewport,
                        [glyphon::TextArea {
                            buffer: &self.text_buffer,
                            left: 0.0,
                            top: 0.0,
                            scale: scale_factor,
                            bounds: glyphon::TextBounds {
                                left: 0,
                                top: 0,
                                right: self.surface_config.width as i32,
                                bottom: self.surface_config.height as i32,
                            },
                            default_color: glyphon::Color::rgb(255, 255, 255),
                            custom_glyphs: &[],
                        }],
                        &mut app_state.swach_cache,
                    )
                    .unwrap();

                let mut encoder = app_state
                    .gpu
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

                    app_state
                        .text_renderer
                        .render(&app_state.atlas, &self.viewport, &mut pass)
                        .unwrap();
                }

                window.pre_present_notify();
                app_state.gpu.queue.submit([encoder.finish()]);
                texture.present();
            }
            winit::event::WindowEvent::CloseRequested => {
                return vec![app::Command::RemoveWindow(self.window_id)];
            }
            _ => {
                // println!("{event:?}");
            }
        }

        vec![]
    }

    fn window_attributes(&mut self) -> winit::window::WindowAttributes {
        winit::window::WindowAttributes::default().with_decorations(false)
    }

    fn new_events(
        &mut self,
        app_state: &mut AppState,
        window: &winit::window::Window,
        cause: winit::event::StartCause,
    ) -> Vec<app::Command<AppState>> {
        match cause {
            winit::event::StartCause::ResumeTimeReached {
                start: _,
                requested_resume: _,
            } => {
                window.request_redraw();

                let total: std::time::Duration = self
                    .starts_and_stops
                    .chunks(2)
                    .map(|s| match s {
                        [StartStop::Start(a), StartStop::Stop(b)] => b.duration_since(*a),
                        [StartStop::Start(a)] => a.elapsed(),
                        a => {
                            todo!("was not expecting {a:?}")
                        }
                    })
                    .sum();

                vec![app::Command::SetControlFlow(
                    winit::event_loop::ControlFlow::wait_duration(
                        std::time::Duration::from_secs_f32(1.0 - total.as_secs_f32() % 1.0),
                    ),
                )]
            }
            _ => vec![],
        }
    }
}

struct AppState {
    font_system: glyphon::FontSystem,
    swach_cache: glyphon::SwashCache,
    cache: glyphon::Cache,
    atlas: glyphon::TextAtlas,
    text_renderer: glyphon::TextRenderer,
    gpu: render::GpuContext,
}

impl AppState {
    async fn new() -> Self {
        let gpu = render::GpuContext::new();

        let font_system = smol::unblock(glyphon::FontSystem::new);
        let swach_cache = glyphon::SwashCache::new();

        let gpu = gpu.await.unwrap();

        let cache = glyphon::Cache::new(&gpu.device);

        let mut atlas = glyphon::TextAtlas::new(
            &gpu.device,
            &gpu.queue,
            &cache,
            wgpu::TextureFormat::Rgba8Unorm,
        );

        let text_renderer = glyphon::TextRenderer::new(
            &mut atlas,
            &gpu.device,
            wgpu::MultisampleState::default(),
            None,
        );

        Self {
            font_system: font_system.await,
            swach_cache,
            cache,
            atlas,
            text_renderer,
            gpu,
        }
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let mut app = app::App::new(
        smol::block_on(AppState::new()),
        |app_state, window| Box::new(DevWindow::new(app_state, window)),
        winit::window::WindowAttributes::default().with_decorations(false),
    );

    event_loop.run_app(&mut app).unwrap();

    std::process::exit(0);
}
