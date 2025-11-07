pub enum Command<State> {
    Nothing,
    RemoveWindow(winit::window::WindowId),
    AddWindow(WindowHandlerInitilizer<State>),
}

pub trait WindowHandler<State> {
    fn window_event(
        &mut self,
        app_state: &mut State,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: winit::event::WindowEvent,
    ) -> Command<State> {
        Command::Nothing
    }
}

pub type WindowHandlerInitilizer<State> = Box<
    dyn Fn(
        &mut State,
        &winit::event_loop::ActiveEventLoop,
    ) -> (winit::window::WindowId, Box<dyn WindowHandler<State>>),
>;

pub struct AppState {
    pub gpu: crate::render::GpuContext,
    pub font_system: glyphon::FontSystem,
}

pub struct App<State> {
    pub windows: std::collections::HashMap<winit::window::WindowId, Box<dyn WindowHandler<State>>>,
    pub resumed_fn: WindowHandlerInitilizer<State>,
    app_state: State,
}

impl<State> App<State> {
    pub fn new(app_state: State, resumed_fn: WindowHandlerInitilizer<State>) -> Self {
        Self {
            windows: std::collections::HashMap::new(),
            resumed_fn,
            app_state,
        }
    }
}

impl<State> winit::application::ApplicationHandler for App<State> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let (id, window_handler) = (self.resumed_fn)(&mut self.app_state, event_loop);

        self.windows.insert(id, window_handler);
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

        match handler.window_event(&mut self.app_state, event_loop, event) {
            Command::Nothing => {}
            Command::RemoveWindow(window_id) => {
                drop(self.windows.remove(&window_id));
            }
            Command::AddWindow(f) => {
                let (id, window_handler) = (f)(&mut self.app_state, event_loop);

                assert!(
                    self.windows.insert(id, window_handler).is_none(),
                    "tried to add a new window with an already existing id"
                );
            }
        }

        if self.windows.is_empty() {
            event_loop.exit();
        }
    }
}
