use crate::engine::Application;
use crate::game::Game;
use crate::renderer::Renderer;
use egui::FontDefinitions;
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::Platform;
use log::info;
use std::sync::Arc;
use std::time::Instant;
use wgpu::{Device, TextureFormat};
use winit::event::Event;
use winit::window::Window;

pub trait EditorApplication: Application + Pause {}

pub trait Pause {
    fn pause(&mut self, paused: bool);
}

pub struct Editor<'a> {
    game: Option<Box<dyn EditorApplication>>,
    frames: usize,

    egui_platform: Option<Platform>,
    egui_render_pass: Option<egui_wgpu_backend::RenderPass>,
    start_time: Instant,

    game_scene_texture_size: Option<u32>,
    game_scene_texture_desc: Option<wgpu::TextureDescriptor<'a>>,
    game_scene_texture: Option<wgpu::Texture>,
    game_scene_texture_view: Option<wgpu::TextureView>,
    game_scene_output_buffer: Option<wgpu::Buffer>,
    game_scene_u32_size: Option<u32>,
}

impl<'a> Default for Editor<'a> {
    fn default() -> Self {
        Self {
            game: None,
            frames: 0,
            egui_platform: None,
            egui_render_pass: None,
            start_time: Instant::now(),
            game_scene_texture_size: None,
            game_scene_texture_desc: None,
            game_scene_texture: None,
            game_scene_texture_view: None,
            game_scene_output_buffer: None,
            game_scene_u32_size: None,
        }
    }
}

impl<'a> Application for Editor<'a> {
    fn on_start(&mut self, window: &Window, device: &Arc<Device>, surface_format: TextureFormat) {
        let size = window.inner_size();
        self.egui_platform = Some(egui_winit_platform::Platform::new(
            egui_winit_platform::PlatformDescriptor {
                physical_width: size.width as u32,
                physical_height: size.height as u32,
                scale_factor: window.scale_factor(),
                font_definitions: FontDefinitions::default(),
                style: Default::default(),
            },
        ));

        self.egui_render_pass = Some(egui_wgpu_backend::RenderPass::new(
            device,
            surface_format,
            1,
        ));

        self.start_time = Instant::now();

        ///// GAME ////////////////////////////
        let mut game = Game::default();
        game.on_start(window, device, surface_format);
        game.pause(true);
        self.game = Some(Box::new(game));

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
        let game_scene_texture = device.create_texture(&game_scene_texture_desc);
        let game_scene_texture_view = game_scene_texture.create_view(&Default::default());

        let game_scene_u32_size = std::mem::size_of::<u32>() as u32;

        let game_scene_output_buffer_size =
            (game_scene_u32_size * game_scene_texture_size * game_scene_texture_size)
                as wgpu::BufferAddress;
        let game_scene_output_buffer_desc = wgpu::BufferDescriptor {
            size: game_scene_output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST,
            label: None,
            mapped_at_creation: false,
        };
        let game_scene_output_buffer = device.create_buffer(&game_scene_output_buffer_desc);

        self.game_scene_texture_size = Some(game_scene_texture_size);
        self.game_scene_texture_desc = Some(game_scene_texture_desc);
        self.game_scene_texture = Some(game_scene_texture);
        self.game_scene_texture_view = Some(game_scene_texture_view);
        self.game_scene_u32_size = Some(game_scene_u32_size);
        self.game_scene_output_buffer = Some(game_scene_output_buffer);
    }

    fn on_event(&mut self, event: &Event<()>) {
        if let Some(egui_platform) = &mut self.egui_platform {
            egui_platform.handle_event(event);
        }
    }

    fn on_update(&mut self, window: &Window, renderer: &mut Renderer) {
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

        let game_scene_texture_view = self.game_scene_texture_view.take();
        renderer.render_to_texture(game_scene_texture_view);

        game.pause(!play_game);
        game.on_update(window, renderer);
        self.game_scene_texture_view = renderer.output_texture.take(); // TODO: Fix this API
        renderer.render_to_texture(None);

        // GUI (copy buffer to egui image)
        let game_scene_texture = self.game_scene_texture.as_ref().unwrap();
        let egui_render_pass = self.egui_render_pass.as_mut().unwrap();
        let game_scene_texture_id = egui_wgpu_backend::RenderPass::egui_texture_from_wgpu_texture(
            // internal,
            egui_render_pass,
            &renderer.device,
            game_scene_texture,
            wgpu::FilterMode::Linear,
        );

        // GUI (second render pass)
        let egui_platform = self.egui_platform.as_mut().unwrap();
        egui_platform.update_time(self.start_time.elapsed().as_secs_f64());
        egui_platform.begin_frame();

        let egui_ctx = egui_platform.context();
        egui::Window::new("Game Scene")
            .resizable(true)
            .show(&egui_ctx, |ui| {
                ui.image(game_scene_texture_id, egui::Vec2::new(640.0, 480.0));
            });

        let (_, paint_commands) = egui_platform.end_frame(Some(window));
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

            egui_render_pass.update_texture(&renderer.device, &renderer.queue, &font_image);
            egui_render_pass.update_user_textures(&renderer.device, &renderer.queue);
            egui_render_pass.update_buffers(
                &renderer.device,
                &renderer.queue,
                &paint_jobs,
                &screen_descriptor,
            );

            egui_render_pass
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

        // Ok(())

        self.frames += 1;
    }

    fn on_stop(&mut self) {
        if let Some(game) = &mut self.game {
            game.on_stop();
        }

        info!("EDITOR on_stop");
    }
}
