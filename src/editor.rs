use crate::engine::{Application, CreateApplication};
use crate::error::Error;
use crate::game::Game;
use crate::renderer::Renderer;
use egui::FontDefinitions;
use egui_winit_platform::Platform;
use log::info;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Instant;
use winit::event::{Event, WindowEvent};
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
}

pub struct Editor {
    game: Option<Game>,
    frames: usize,

    state: EditorState,

    egui_start_time: Instant,
    egui_platform: Platform,
    game_scene_texture: wgpu::Texture,
    game_scene_texture_view: Arc<RefCell<wgpu::TextureView>>, // TODO: Does this need to be an Arc with interior mutability in the Renderer?
}

impl CreateApplication for Editor {
    type App = Self;

    fn create(window: &Window, renderer: &Renderer) -> Result<Self::App, Error> {
        let size = window.inner_size();
        let egui_platform =
            egui_winit_platform::Platform::new(egui_winit_platform::PlatformDescriptor {
                physical_width: size.width as u32,
                physical_height: size.height as u32,
                scale_factor: window.scale_factor(),
                font_definitions: FontDefinitions::default(),
                style: Default::default(),
            });

        let start_time = Instant::now();

        let game = Game::create(window, renderer)?;

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
        let game_scene_texture_view = Arc::new(RefCell::new(
            game_scene_texture.create_view(&Default::default()),
        ));

        let editor = Editor {
            game: Some(game),
            frames: 0,
            state,
            egui_platform,
            egui_start_time: start_time,
            game_scene_texture,
            game_scene_texture_view,
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
        self.egui_platform.handle_event(event);
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

        renderer.render_to_texture(Some(self.game_scene_texture_view.clone()));

        game.pause(!play_game);
        game.on_update(window, renderer)
            .expect("Handle error - game crash should not crash editor"); // TODO
        renderer.render_to_texture(None);

        let game_scene_texture_id =
            renderer.egui_texture_from_wgpu_texture(&self.game_scene_texture);

        let (egui_ctx, egui_paint_commands) = gui::update(
            &mut self.egui_platform,
            self.egui_start_time,
            game_scene_texture_id,
            game,
            &mut self.state,
            window,
        );

        let render_ctx = renderer.prepare();
        renderer.begin_egui(&render_ctx, egui_ctx, egui_paint_commands);
        renderer.finalise(render_ctx);

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
