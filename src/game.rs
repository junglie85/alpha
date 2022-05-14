use crate::editor::Pause;
use crate::engine::{Application, CreateApplication};
use crate::error::Error;
use crate::renderer::{Rect, Renderer};
use log::info;
use winit::event::Event;
use winit::window::Window;

#[derive(Default)]
pub struct Game {
    paused: bool,
    rects: Vec<Rect>,
}

impl CreateApplication for Game {
    type App = Self;

    fn create(_window: &Window, _renderer: &Renderer) -> Result<Self::App, Error> {
        Ok(Game::default())
    }
}

impl Application for Game {
    fn on_start(&mut self) {
        let rect = Rect::new([0.0, 0.0], [1.0, 0.0, 0.0, 1.0]);
        self.rects.push(rect);
    }

    fn on_event(&mut self, _event: &Event<()>) {}

    fn on_update(&mut self, _window: &Window, renderer: &mut Renderer) -> Result<(), Error> {
        let paused_or_running = if self.paused { "paused" } else { "running" };
        info!("GAME on_update - {}", paused_or_running);

        renderer.draw_rect(&self.rects[0]);

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
