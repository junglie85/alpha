use crate::engine::{Application, CreateApplication};
use crate::error::Error;
use crate::game::Game;
use crate::renderer::Renderer;
use hecs::Entity;
use log::info;
use wgpu::TextureViewDescriptor;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::Window;

mod gui;

pub trait Pause {
    fn pause(&mut self, paused: bool);
}

#[derive(Default)]
pub(crate) struct EditorState {
    pub editor_title: String,
    pub changed_since_last_save: bool,
    pub save_requested: bool,
    pub build_requested: bool,
    pub window_resized: bool,
    pub active_entity: Option<Entity>,
}

pub struct Editor {
    game: Option<Game>,
    frames: usize,

    state: EditorState,

    egui_ctx: egui::Context,
    egui_platform: egui_winit::State,
    game_scene_texture: wgpu::Texture,
}

impl CreateApplication for Editor {
    type App = Self;

    fn create(
        window: &Window,
        event_loop: &EventLoop<()>,
        renderer: &Renderer,
    ) -> Result<Self::App, Error> {
        let egui_winit_state = egui_winit::State::new(event_loop);

        let egui_ctx = egui::Context::default();

        let game = Game::create(window, event_loop, renderer)?;

        let mut state = EditorState::default();
        state.editor_title = String::from("Alpha Editor");
        state.window_resized = true;

        // TODO: Recreate this texture whenever we resize the editor/scene view window.
        let game_scene_texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: 1280,
                height: 720,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
        };
        let game_scene_texture = renderer.device.create_texture(&game_scene_texture_desc);

        let editor = Editor {
            game: Some(game),
            frames: 0,
            state,
            egui_platform: egui_winit_state,
            egui_ctx,
            game_scene_texture,
        };

        Ok(editor)
    }
}

impl Application for Editor {
    fn on_start(&mut self, _config_filename: Option<&str>) {
        if let Some(game) = &mut self.game {
            game.on_start(Some("alpha_game.alpha"));
            game.pause(true);
        }
    }

    fn on_event(&mut self, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            // TODO: deal with event handled (returns false).
            self.egui_platform.on_event(&self.egui_ctx, event);
        }

        if let Event::WindowEvent {
            event: WindowEvent::Resized(_size),
            ..
        } = event
        {
            self.state.window_resized = true;
        }
    }

    fn on_update(&mut self, window: &Window, renderer: &mut Renderer) -> Result<(), Error> {
        let game = self.game.as_mut().unwrap();

        let play_game = match self.frames {
            1 => {
                info!("Simulate start playing game in the editor");
                true
            }
            2 => true,
            3 => {
                info!("Simulate stop playing game in the editor");
                false
            }
            _ => false,
        };

        let game_scene_texture_view = self.game_scene_texture.create_view(&Default::default());
        renderer.render_to_texture(Some(game_scene_texture_view));

        game.pause(!play_game);
        game.on_update(window, renderer)
            .expect("Handle error - game crash should not crash editor"); // TODO
        renderer.render_to_texture(None);

        let tv = self
            .game_scene_texture
            .create_view(&TextureViewDescriptor::default());
        let game_scene_texture_id = renderer.egui_texture_from_wgpu_texture(&tv);

        self.egui_platform
            .set_pixels_per_point(window.scale_factor() as f32);
        let egui_output = gui::update(
            &self.egui_ctx,
            &mut self.egui_platform,
            game_scene_texture_id,
            game,
            &mut self.state,
            window,
        );

        let render_ctx = renderer.prepare();
        renderer.begin_egui(&render_ctx, &self.egui_ctx, &egui_output);
        renderer.finalise(render_ctx);

        self.egui_platform.handle_platform_output(
            window,
            &self.egui_ctx,
            egui_output.platform_output,
        );

        self.frames += 1;

        Ok(())
    }

    fn on_stop(&mut self) {
        if let Some(game) = &mut self.game {
            game.on_stop();
        }

        info!("EDITOR on_stop");
    }
}
