pub enum Command {
    Nothing,
    RemoveWindow(winit::window::WindowId),
    AddWindow(WindowHandlerInitilizer),
}

pub trait WindowHandler {
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: winit::event::WindowEvent,
        gpu: &crate::render::GpuContext,
    ) -> Command {
        Command::Nothing
    }
}

pub type WindowHandlerInitilizer = Box<
    dyn Fn(
        &crate::render::GpuContext,
        &winit::event_loop::ActiveEventLoop,
    ) -> (winit::window::WindowId, Box<dyn WindowHandler>),
>;

pub struct App {
    pub windows: std::collections::HashMap<winit::window::WindowId, Box<dyn WindowHandler>>,
    pub gpu: crate::render::GpuContext,
    pub resumed_fn: WindowHandlerInitilizer,
}

impl App {
    pub fn new(gpu: crate::render::GpuContext, resumed_fn: WindowHandlerInitilizer) -> Self {
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
                _ = self.gpu.device.poll(wgpu::PollType::wait_indefinitely());
                drop(self.windows.remove(&window_id));
            }
            Command::AddWindow(f) => {
                let (id, window_handler) = (f)(&self.gpu, event_loop);

                assert!(
                    self.windows.insert(id, window_handler).is_none(),
                    "tried to add a new window with an already existing id"
                );
            }
        }

        if self.windows.is_empty() {
            _ = self.gpu.device.poll(wgpu::PollType::wait_indefinitely());
            event_loop.exit();
        }
    }
}
