use crate::error::Error;
use crate::renderer::Renderer;
use crate::{logging, platform, renderer};
use std::sync::Arc;
use wgpu::{Device, TextureFormat};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit_input_helper::WinitInputHelper;

pub trait Application {
    fn on_start(&mut self, window: &Window, device: &Arc<Device>, surface_format: TextureFormat);

    fn on_event(&mut self, event: &Event<()>);

    fn on_update(&mut self, window: &Window, renderer: &mut Renderer);

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

        let renderer = renderer::init(&window)?;

        let engine = Engine {
            event_loop: Some(event_loop),
            input: Some(input),
            renderer: Some(renderer),
            window: Some(window),
        };

        Ok(engine)
    }

    pub fn run(&mut self, mut app: impl Application + 'static) -> Result<(), Error> {
        let event_loop = self.event_loop.take().unwrap();
        let mut input = self.input.take().unwrap();
        let mut renderer = self.renderer.take().unwrap();
        let window = self.window.take().unwrap();

        app.on_start(&window, &renderer.device, renderer.surface_config.format);

        event_loop.run(move |event, _, control_flow| {
            if let Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } = event
            {
                renderer.resize(size.width, size.height, window.scale_factor());
            }

            app.on_event(&event);

            let processed_all_events = input.update(&event);

            if processed_all_events {
                if input.quit() {
                    *control_flow = ControlFlow::Exit;
                    app.on_stop();
                    return;
                }

                app.on_update(&window, &mut renderer);
            }

            *control_flow = ControlFlow::Poll;
        });
    }
}