pub enum Command<State> {
    RemoveWindow(winit::window::WindowId),
    AddWindow(Box<dyn WindowHandler<State>>),
}

pub trait WindowHandler<State> {
    fn window_event(
        &mut self,
        app_state: &mut State,
        window: &winit::window::Window,
        event: winit::event::WindowEvent,
    ) -> Vec<Command<State>> {
        Vec::new()
    }

    fn window_attributes(&mut self) -> winit::window::WindowAttributes {
        winit::window::WindowAttributes::default()
    }
}

type InitFn<T, State> = fn(&mut State, window: &std::sync::Arc<winit::window::Window>) -> T;

pub struct App<State> {
    new_window_queue: Vec<(
        InitFn<Box<dyn WindowHandler<State>>, State>,
        winit::window::WindowAttributes,
    )>,
    windows: std::collections::HashMap<
        winit::window::WindowId,
        (
            std::sync::Arc<winit::window::Window>,
            Box<dyn WindowHandler<State>>,
        ),
    >,
    app_state: State,
}

impl<State> App<State> {
    pub fn new(
        app_state: State,
        window_handler: InitFn<Box<dyn WindowHandler<State>>, State>,
        window_attributes: winit::window::WindowAttributes,
    ) -> Self {
        Self {
            new_window_queue: vec![(window_handler, window_attributes)],
            windows: std::collections::HashMap::new(),
            app_state,
        }
    }
}

impl<State> winit::application::ApplicationHandler for App<State> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        for (window_handler, window_attributes) in self.new_window_queue.drain(..) {
            let window = std::sync::Arc::new(event_loop.create_window(window_attributes).unwrap());
            let window_handler = window_handler(&mut self.app_state, &window);

            self.windows.insert(window.id(), (window, window_handler));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some((window, handler)) = self.windows.get_mut(&window_id) else {
            return;
        };

        for command in handler.window_event(&mut self.app_state, window, event) {
            match command {
                Command::RemoveWindow(window_id) => {
                    drop(self.windows.remove(&window_id));
                }
                Command::AddWindow(mut new_handler) => {
                    let window = std::sync::Arc::new(
                        event_loop
                            .create_window(new_handler.window_attributes())
                            .unwrap(),
                    );

                    self.windows.insert(window.id(), (window, new_handler));
                }
            }
        }

        if self.windows.is_empty() {
            event_loop.exit();
        }
    }
}
