use crate::editor::{EditorApplication, Pause};
use crate::engine::Application;
use crate::renderer::Renderer;
use log::info;

#[derive(Default)]
pub struct Game {
    paused: bool,
}

impl Application for Game {
    fn on_start(&mut self) {
        info!("GAME on_start");
    }

    fn on_update(&mut self, renderer: &mut Renderer) {
        let paused_or_running = if self.paused { "paused" } else { "running" };
        info!("GAME on_update - {}", paused_or_running);
        renderer.render("GAME");
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

impl EditorApplication for Game {}
