use crate::editor::Pause;
use crate::engine::{Application, CreateApplication};
use crate::error::Error;
use crate::renderer::{Camera, Rect, Renderer};
use log::info;
use std::str::FromStr;
use std::{fs, path};
use winit::event::Event;
use winit::window::Window;

pub struct Game {
    paused: bool,
    pub rects: Vec<Rect>,
    pub camera: Camera,
}

impl Game {
    pub fn new(_window: &Window, renderer: &Renderer) -> Self {
        let paused = false;
        let rects = Vec::new();
        let camera = Camera::new(renderer.width, renderer.height);

        Self {
            paused,
            rects,
            camera,
        }
    }
}

impl CreateApplication for Game {
    type App = Self;

    fn create(window: &Window, renderer: &Renderer) -> Result<Self::App, Error> {
        Ok(Game::new(window, renderer))
    }
}

impl Application for Game {
    fn on_start(&mut self, config_filename: Option<&str>) {
        let filename = match config_filename {
            Some(filename) => filename,
            None => "alpha_game.ini",
        };

        let path = path::Path::new(filename);
        let file = fs::read_to_string(path);

        if let Ok(config) = file {
            let colors: Vec<&str> = config.split_whitespace().collect();
            let r = f32::from_str(colors[0]).unwrap();
            let g = f32::from_str(colors[1]).unwrap();
            let b = f32::from_str(colors[2]).unwrap();
            let a = f32::from_str(colors[3]).unwrap();
            let color = [r, g, b, a];

            let mut rect = Rect::new([100.0, 100.0], color);
            rect.scale = [300.0, 300.0];
            rect.rotation_degrees = 10.0;
            self.rects.push(rect);
        }
    }

    fn on_event(&mut self, _event: &Event<()>) {}

    fn on_update(&mut self, _window: &Window, renderer: &mut Renderer) -> Result<(), Error> {
        let paused_or_running = if self.paused { "paused" } else { "running" };
        info!("GAME on_update - {}", paused_or_running);

        renderer.draw_rect(&self.rects[0], &self.camera);

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
