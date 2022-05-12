use crate::error::Error;
use crate::renderer::Renderer;
use crate::{logging, platform};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit_input_helper::WinitInputHelper;

pub trait Application {
    fn on_start(&mut self);

    fn on_update(&mut self, renderer: &mut Renderer);

    fn on_stop(&mut self);
}

pub struct Engine {
    event_loop: Option<EventLoop<()>>,
    input: Option<WinitInputHelper>,
    renderer: Option<Renderer>,
    window: Option<Window>,
}

impl Engine {
    pub fn init() -> Result<Self, Error> {
        logging::init("info")?;
        let (event_loop, window, input) = platform::init()?;

        let renderer = Renderer::default();

        let engine = Engine {
            event_loop: Some(event_loop),
            input: Some(input),
            renderer: Some(renderer),
            window: Some(window),
        };

        Ok(engine)
    }

    pub fn run(&mut self, mut app: impl Application + 'static) -> Result<(), Error> {
        app.on_start();

        let event_loop = self.event_loop.take().unwrap();
        let mut input = self.input.take().unwrap();
        let mut renderer = self.renderer.take().unwrap();
        let mut _window = self.window.take().unwrap();

        event_loop.run(move |event, _, control_flow| {
            let processed_all_events = input.update(&event);

            if processed_all_events {
                if input.quit() {
                    *control_flow = ControlFlow::Exit;
                    app.on_stop();
                    return;
                }

                app.on_update(&mut renderer);
            }

            *control_flow = ControlFlow::Poll;
        });
    }
}
