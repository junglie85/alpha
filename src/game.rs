use crate::editor::{EditorApplication, Pause};
use crate::engine::Application;
use crate::renderer::Renderer;
use log::info;
use std::sync::Arc;
use wgpu::{Device, TextureFormat};
use winit::event::Event;
use winit::window::Window;

#[derive(Default)]
pub struct Game {
    paused: bool,
}

impl Application for Game {
    fn on_start(
        &mut self,
        _window: &Window,
        _device: &Arc<Device>,
        _surface_format: TextureFormat,
    ) {
        info!("GAME on_start");
    }

    fn on_event(&mut self, _event: &Event<()>) {}

    fn on_update(&mut self, _window: &Window, renderer: &mut Renderer) {
        let paused_or_running = if self.paused { "paused" } else { "running" };
        info!("GAME on_update - {}", paused_or_running);

        renderer.draw_quad();
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
