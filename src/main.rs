#![warn(clippy::pedantic, clippy::all, clippy::nursery)]

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

trait WindowHandler {
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: winit::event::WindowEvent,
    ) {
    }
}

struct Menu<'a> {
    window: std::sync::Arc<winit::window::Window>,
    surface: wgpu::Surface<'a>,
}

impl Menu<'_> {
    fn new(
        instace: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        window: std::sync::Arc<winit::window::Window>,
    ) -> Self {
        let winit::dpi::PhysicalSize { width, height } = window.inner_size();

        let surface = instace.create_surface(window.clone()).unwrap();
        let config = surface.get_default_config(adapter, width, height).unwrap();
        surface.configure(device, &config);

        Self { window, surface }
    }
}

impl WindowHandler for Menu<'_> {}

struct App {
    windows: std::collections::HashMap<winit::window::WindowId, Box<dyn WindowHandler>>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl App {
    async fn new() -> Self {
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
            windows: std::collections::HashMap::new(),
            instance,
            adapter,
            device,
            queue,
        }
    }
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let Self {
            instance,
            adapter,
            device,
            ..
        } = &self;

        let window = event_loop
            .create_window(winit::window::WindowAttributes::default())
            .unwrap();

        let id = window.id();

        let window_handler = Menu::new(instance, adapter, device, std::sync::Arc::new(window));

        self.windows.insert(id, Box::new(window_handler));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(handler) = self.windows.get_mut(&window_id) {
            handler.window_event(event_loop, event);
        }
    }
}

fn main() {
    block_on(async {
        let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
        let app = App::new();

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        event_loop.run_app(&mut app.await).unwrap();
    });
}
