use crate::engine::{Application, CreateApplication};
use crate::error::Error;
use crate::game::Game;
use crate::renderer::Renderer;
use egui::FontDefinitions;
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::Platform;
use log::info;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Instant;
use winit::event::Event;
use winit::window::Window;

pub trait Pause {
    fn pause(&mut self, paused: bool);
}

pub struct Editor {
    game: Option<Game>,
    frames: usize,

    egui_start_time: Instant,
    egui_platform: Platform,
    egui_render_pass: egui_wgpu_backend::RenderPass,

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

        let egui_render_pass =
            egui_wgpu_backend::RenderPass::new(&renderer.device, renderer.surface_config.format, 1);

        let start_time = Instant::now();

        ///// GAME ////////////////////////////
        let game = Game::create(window, renderer)?;

        ////// Texture to render GAME into ////////
        let game_scene_texture_size = 256u32;

        let game_scene_texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: game_scene_texture_size,
                height: game_scene_texture_size,
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
            egui_platform,
            egui_render_pass,
            egui_start_time: start_time,
            game_scene_texture,
            game_scene_texture_view,
        };

        Ok(editor)
    }
}

impl Application for Editor {
    fn on_start(&mut self) {
        if let Some(game) = &mut self.game {
            game.on_start();
            game.pause(true);
        }
    }

    fn on_event(&mut self, event: &Event<()>) {
        self.egui_platform.handle_event(event);
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

        // GUI (copy buffer to egui image)
        let game_scene_texture_id = egui_wgpu_backend::RenderPass::egui_texture_from_wgpu_texture(
            &mut self.egui_render_pass,
            &renderer.device,
            &self.game_scene_texture,
            wgpu::FilterMode::Linear,
        );

        // GUI (second render pass)
        self.egui_platform
            .update_time(self.egui_start_time.elapsed().as_secs_f64());
        self.egui_platform.begin_frame();

        let egui_ctx = self.egui_platform.context();
        egui::Window::new("Game Scene")
            .resizable(true)
            .show(&egui_ctx, |ui| {
                ui.image(game_scene_texture_id, egui::Vec2::new(640.0, 480.0));
            });

        let (_, paint_commands) = self.egui_platform.end_frame(Some(window));
        let paint_jobs = egui_ctx.tessellate(paint_commands);
        let font_image = egui_ctx.font_image();

        // TODO: move all this egui rendering logic into renderer?
        let output = renderer
            .surface
            .get_current_texture()
            .expect("Handle Errors");
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let screen_descriptor = ScreenDescriptor {
                physical_width: renderer.surface_config.width,
                physical_height: renderer.surface_config.height,
                scale_factor: renderer.scale_factor as f32,
            };

            self.egui_render_pass
                .update_texture(&renderer.device, &renderer.queue, &font_image);
            self.egui_render_pass
                .update_user_textures(&renderer.device, &renderer.queue);
            self.egui_render_pass.update_buffers(
                &renderer.device,
                &renderer.queue,
                &paint_jobs,
                &screen_descriptor,
            );

            self.egui_render_pass
                .execute(
                    &mut encoder,
                    &view,
                    &paint_jobs,
                    &screen_descriptor,
                    Some(wgpu::Color::BLACK),
                )
                .unwrap();
        }

        renderer.queue.submit(std::iter::once(encoder.finish()));
        output.present();

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
