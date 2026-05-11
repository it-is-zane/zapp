pub enum Command<State> {
    RemoveWindow(winit::window::WindowId),
    AddWindow(Box<dyn WindowHandler<State>>),
    SetControlFlow(winit::event_loop::ControlFlow),
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

    fn new_events(
        &mut self,
        app_state: &mut State,
        window: &winit::window::Window,
        cause: winit::event::StartCause,
    ) -> Vec<Command<State>> {
        vec![]
    }
}

type InitFn<T, State> = fn(&mut State, window: &std::sync::Arc<winit::window::Window>) -> T;

type NewWindowQueue<State> = Vec<(
    InitFn<Box<dyn WindowHandler<State>>, State>,
    winit::window::WindowAttributes,
)>;

type Windows<State> = std::collections::HashMap<
    winit::window::WindowId,
    (
        std::sync::Arc<winit::window::Window>,
        Box<dyn WindowHandler<State>>,
    ),
>;

pub struct App<State> {
    new_window_queue: NewWindowQueue<State>,
    windows: Windows<State>,
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

fn handle_command<State>(
    command: Command<State>,
    windows: &mut Windows<State>,
    event_loop: &winit::event_loop::ActiveEventLoop,
) {
    match command {
        Command::RemoveWindow(window_id) => {
            drop(windows.remove(&window_id));
        }
        Command::AddWindow(mut new_handler) => {
            let window = std::sync::Arc::new(
                event_loop
                    .create_window(new_handler.window_attributes())
                    .unwrap(),
            );

            windows.insert(window.id(), (window, new_handler));
        }
        Command::SetControlFlow(flow) => {
            event_loop.set_control_flow(flow);
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
            handle_command(command, &mut self.windows, event_loop);
        }

        if self.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        for command in self
            .windows
            .values_mut()
            .flat_map(|(window, handler)| handler.new_events(&mut self.app_state, window, cause))
            .collect::<Vec<Command<State>>>()
        {
            handle_command(command, &mut self.windows, event_loop);
        }

        if self.windows.is_empty() && cause != winit::event::StartCause::Init {
            event_loop.exit();
        }
    }
}
