use crate::components::{Shape, Tag, Transform};
use crate::editor::Pause;
use crate::engine::{Application, CreateApplication};
use crate::error::Error;
use crate::renderer::camera::Camera;
use crate::renderer::{rect::Rect, Renderer};
use glam::{Vec2, Vec4};
use hecs::World;
use log::info;
use std::str::FromStr;
use std::{fs, path};
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::Window;

pub struct Game {
    paused: bool,
    pub camera: Camera,
    pub world: World,
}

impl Game {
    pub fn new(_window: &Window, renderer: &Renderer) -> Self {
        let paused = false;
        let camera = Camera::new(renderer.width, renderer.height);

        let world = World::new();

        Self {
            paused,
            camera,
            world,
        }
    }
}

impl CreateApplication for Game {
    type App = Self;

    fn create(
        window: &Window,
        _event_loop: &EventLoop<()>,
        renderer: &Renderer,
    ) -> Result<Self::App, Error> {
        Ok(Game::new(window, renderer))
    }
}

impl Application for Game {
    fn on_start(&mut self, config_filename: Option<&str>) {
        let filename = config_filename.unwrap_or("alpha_game.ini");

        let path = path::Path::new(filename);
        let file = fs::read_to_string(path);

        if let Ok(config) = file {
            let entities: Vec<&str> = config
                .trim()
                .split("---\n")
                .filter(|e| !e.is_empty())
                .collect();

            for entity in entities {
                let components: Vec<&str> = entity.split('\n').collect();

                let tag = components[0].to_string();

                let transform: Vec<&str> = components[1].split_whitespace().collect();
                let x = f32::from_str(transform[0]).unwrap();
                let y = f32::from_str(transform[1]).unwrap();
                let width = f32::from_str(transform[2]).unwrap();
                let height = f32::from_str(transform[3]).unwrap();
                let rotation = f32::from_str(transform[4]).unwrap();

                let colors: Vec<&str> = components[2].split_whitespace().collect();
                let r = f32::from_str(colors[0]).unwrap();
                let g = f32::from_str(colors[1]).unwrap();
                let b = f32::from_str(colors[2]).unwrap();
                let a = f32::from_str(colors[3]).unwrap();
                let color = Vec4::new(r, g, b, a);

                let tag = Tag(tag);
                let transform = Transform {
                    position: Vec2::new(x, y),
                    size: Vec2::new(width, height),
                    rotation,
                };
                let shape = Shape { color };
                self.world.spawn((tag, transform, shape));
            }
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
        system_render(&self.world, &self.camera, renderer);

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

fn system_render(world: &World, camera: &Camera, renderer: &mut Renderer) {
    let mut render_ctx = renderer.prepare();
    let mut scene = renderer.begin_scene(camera); // TODO: Add camera as a resource in the World.

    for (_id, (transform, shape)) in world.query::<(&Transform, &Shape)>().iter() {
        let rect = Rect::new(
            transform.position,
            transform.rotation,
            transform.size,
            shape.color,
        );
        renderer.draw_rect(&mut scene, &rect);
    }

    renderer.end_scene(scene, &mut render_ctx);
    renderer.finalise(render_ctx);
}
