#![warn(clippy::pedantic, clippy::all, clippy::nursery)]

// https://doc.rust-lang.org/beta/std/task/trait.Wake.html
fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    struct ThreadWaker(std::thread::Thread);

    impl std::task::Wake for ThreadWaker {
        fn wake(self: std::sync::Arc<Self>) {
            self.0.unpark();
        }
    }

    let mut fut = std::pin::pin!(future);

    let t = std::thread::current();
    let waker = std::sync::Arc::new(ThreadWaker(t)).into();
    let mut cx = std::task::Context::from_waker(&waker);

    loop {
        match fut.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(res) => return res,
            std::task::Poll::Pending => std::thread::park(),
        }
    }
}

type WindowHandlerInitilizer = Box<
    dyn Fn(
        &Gpu,
        &winit::event_loop::ActiveEventLoop,
    ) -> (winit::window::WindowId, Box<dyn WindowHandler>),
>;

enum Command {
    Nothing,
    RemoveWindow(winit::window::WindowId),
    AddWindow(WindowHandlerInitilizer),
}

trait WindowHandler {
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: winit::event::WindowEvent,
        gpu: &Gpu,
    ) -> Command {
        Command::Nothing
    }
}

struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        Self {
            instance,
            adapter,
            device,
            queue,
        }
    }
}

struct App {
    windows: std::collections::HashMap<winit::window::WindowId, Box<dyn WindowHandler>>,
    gpu: Gpu,
    resumed_fn: WindowHandlerInitilizer,
}

impl App {
    fn new(gpu: Gpu, resumed_fn: WindowHandlerInitilizer) -> Self {
        Self {
            windows: std::collections::HashMap::new(),
            gpu,
            resumed_fn,
        }
    }
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let Self {
            gpu,
            windows,
            resumed_fn,
        } = self;

        let (id, window_handler) = (resumed_fn)(gpu, event_loop);

        windows.insert(id, window_handler);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(handler) = self.windows.get_mut(&window_id) else {
            return;
        };

        match handler.window_event(event_loop, event, &self.gpu) {
            Command::Nothing => {}
            Command::RemoveWindow(window_id) => {
                _ = self.gpu.device.poll(wgpu::PollType::Wait);
                drop(self.windows.remove(&window_id));
            }
            Command::AddWindow(f) => {
                let (id, window_handler) = (f)(&self.gpu, event_loop);

                _ = self.windows.insert(id, window_handler);
            }
        }

        if self.windows.is_empty() {
            _ = self.gpu.device.poll(wgpu::PollType::Wait);
            event_loop.exit();
        }
    }
}

fn create_text_pipline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    blend: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    let module = &device.create_shader_module(wgpu::include_wgsl!("text.wgsl"));

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("text render pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<nalgebra::Vector2<f32>>() as u64 * 2,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: std::mem::size_of::<nalgebra::Vector2<f32>>() as u64,
                        shader_location: 1,
                    },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

fn create_bind_group(gpu: &Gpu) -> wgpu::BindGroup {
    gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: todo!(),
        entries: todo!(),
    });
}

struct Menu<'a> {
    window: std::sync::Arc<winit::window::Window>,
    surface: wgpu::Surface<'a>,
    pipeline: wgpu::RenderPipeline,
    texture_group: wgpu::BindGroup,
    color_group: wgpu::BindGroup,
}

impl Menu<'_> {
    fn new(gpu: &Gpu, window: std::sync::Arc<winit::window::Window>) -> Self {
        let winit::dpi::PhysicalSize { width, height } = window.inner_size();

        let surface = gpu.instance.create_surface(window.clone()).unwrap();
        let config = surface
            .get_default_config(&gpu.adapter, width, height)
            .unwrap();
        surface.configure(&gpu.device, &config);

        let pipeline = create_text_pipline(&gpu.device, config.format, wgpu::BlendState::REPLACE);

        Self {
            window,
            surface,
            pipeline,
            texture_group: todo!(),
            color_group: todo!(),
        }
    }
}

impl WindowHandler for Menu<'_> {
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: winit::event::WindowEvent,
        gpu: &Gpu,
    ) -> Command {
        if event == winit::event::WindowEvent::RedrawRequested {
            let Ok(texture) = self.surface.get_current_texture() else {
                return Command::Nothing;
            };

            let view = &texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let mut ce = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            {
                let mut pass = ce.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });

                pass.set_pipeline(&self.pipeline);
            }

            gpu.queue.submit([ce.finish()]);
            texture.present();
        }

        if let winit::event::WindowEvent::MouseInput {
            device_id: _,
            state: winit::event::ElementState::Pressed,
            button: winit::event::MouseButton::Left,
        } = event
        {
            return Command::AddWindow(Box::new(|gpu, event_loop| {
                let window = std::sync::Arc::new(
                    event_loop
                        .create_window(winit::window::WindowAttributes::default())
                        .unwrap(),
                );

                let id = window.id();
                let window_handler = Box::new(Menu::new(gpu, window));

                (id, window_handler)
            }));
        }

        if event == winit::event::WindowEvent::CloseRequested {
            return Command::RemoveWindow(self.window.id());
        }

        Command::Nothing
    }
}

fn main() {
    block_on(async {
        let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();

        let mut app = App::new(
            Gpu::new().await,
            Box::new(|gpu, event_loop| {
                let window = std::sync::Arc::new(
                    event_loop
                        .create_window(winit::window::WindowAttributes::default())
                        .unwrap(),
                );

                let id = window.id();
                let window_handler = Box::new(Menu::new(gpu, window));

                (id, window_handler)
            }),
        );

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        event_loop.run_app(&mut app).unwrap();
    });

    println!("exit");
}
