use crate::editor::Pause;
use crate::engine::{Application, CreateApplication};
use crate::error::Error;
use crate::renderer::camera::Camera;
use crate::renderer::{rect::Rect, Renderer};
use glam::{Vec2, Vec4};
use log::info;
use std::str::FromStr;
use std::{fs, path};
use winit::event::{Event, WindowEvent};
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
            let state: Vec<&str> = config.split('\n').collect();

            let transform: Vec<&str> = state[0].split_whitespace().collect();
            let x = f32::from_str(transform[0]).unwrap();
            let y = f32::from_str(transform[1]).unwrap();
            let width = f32::from_str(transform[2]).unwrap();
            let height = f32::from_str(transform[3]).unwrap();
            let rotation = f32::from_str(transform[4]).unwrap();

            let colors: Vec<&str> = state[1].split_whitespace().collect();
            let r = f32::from_str(colors[0]).unwrap();
            let g = f32::from_str(colors[1]).unwrap();
            let b = f32::from_str(colors[2]).unwrap();
            let a = f32::from_str(colors[3]).unwrap();
            let color = Vec4::new(r, g, b, a);

            let rect = Rect::new(Vec2::new(x, y), rotation, Vec2::new(width, height), color);
            self.rects.push(rect);

            let rect = Rect::new(
                Vec2::new(400.0, 400.0),
                0.0,
                Vec2::new(100.0, 100.0),
                Vec4::new(1.0, 0.0, 0.0, 1.0),
            );
            self.rects.push(rect);
        }
    }

    fn on_event(&mut self, event: &Event<()>) {
        if let Event::WindowEvent {
            event: WindowEvent::Resized(size),
            ..
        } = event
        {
            self.camera.resize(size.width, size.height);
        }
    }

    fn on_update(&mut self, _window: &Window, renderer: &mut Renderer) -> Result<(), Error> {
        let mut render_ctx = renderer.prepare();
        let mut scene = renderer.begin_scene(&self.camera);

        for rect in &self.rects {
            renderer.draw_rect(&mut scene, rect);
        }

        renderer.end_scene(scene, &mut render_ctx);
        renderer.finalise(render_ctx);

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
