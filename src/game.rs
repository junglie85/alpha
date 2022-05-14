use crate::editor::Pause;
use crate::engine::{Application, CreateApplication};
use crate::error::Error;
use crate::renderer::Renderer;
use log::info;
use winit::event::Event;
use winit::window::Window;

#[derive(Default)]
pub struct Game {
    paused: bool,
}

impl CreateApplication for Game {
    type App = Self;

    fn create(_window: &Window, _renderer: &Renderer) -> Result<Self::App, Error> {
        Ok(Game::default())
    }
}

impl Application for Game {
    fn on_start(&mut self) {
        info!("GAME on_start");
    }

    fn on_event(&mut self, _event: &Event<()>) {}

    fn on_update(&mut self, _window: &Window, renderer: &mut Renderer) -> Result<(), Error> {
        let paused_or_running = if self.paused { "paused" } else { "running" };
        info!("GAME on_update - {}", paused_or_running);

        renderer.draw_quad();

        Ok(())
    }

    fn on_stop(&mut self) {
        info!("GAME on_stop");
    }
}

impl Pause for Game {
    fn pause(&mut self, paused: bool) {
        self.paused = paused;
    }
}
