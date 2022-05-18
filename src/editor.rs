use crate::engine::{Application, CreateApplication};
use crate::error::Error;
use crate::game::Game;
use crate::renderer::Renderer;
use egui::{FontDefinitions, Slider};
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::Platform;
use log::info;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Instant;
use std::{fs, path};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::window::Window;

pub trait Pause {
    fn pause(&mut self, paused: bool);
}

#[derive(Default)]
struct EditorState {
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
            egui_render_pass,
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

        egui::TopBottomPanel::top("toolbar").show(&egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                let save = ui.button("💾 Save").clicked();
                if save {
                    self.state.save_requested = true;
                }

                let build = ui.button("🛠 Build").clicked();
                if build {
                    self.state.build_requested = true;
                }
            });
        });

        egui::SidePanel::right("right pane").show(&egui_ctx, |ui| {
            egui::CollapsingHeader::new("Shape")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label("Color");
                    if ui
                        .color_edit_button_rgba_unmultiplied(&mut game.rects[0].color.to_array())
                        .changed()
                    {
                        self.state.changed_since_last_save = true;
                    }
                });

            egui::CollapsingHeader::new("Transform")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label("Position");
                    let slider = Slider::new(&mut game.rects[0].position[0], -2000.0..=2000.0)
                        .text("x")
                        .clamp_to_range(false);
                    ui.add(slider);
                    let slider = Slider::new(&mut game.rects[0].position[1], -2000.0..=2000.0)
                        .text("y")
                        .clamp_to_range(false);
                    ui.add(slider);

                    ui.label("Rotation");
                    let slider = Slider::new(&mut game.rects[0].rotation_degrees, 0.0..=360.0)
                        .clamp_to_range(false);
                    ui.add(slider);

                    ui.label("Size");
                    let slider = Slider::new(&mut game.rects[0].size.x, 0.0..=2000.0)
                        .text("width")
                        .clamp_to_range(false);
                    ui.add(slider);
                    let slider = Slider::new(&mut game.rects[0].size.y, 0.0..=2000.0)
                        .text("height")
                        .clamp_to_range(false);
                    ui.add(slider);
                });
        });

        egui::CentralPanel::default().show(&egui_ctx, |ui| {
            let size = ui.available_size_before_wrap();
            ui.image(game_scene_texture_id, size);

            if self.state.window_resized {
                let width = (size.x * window.scale_factor() as f32) as u32;
                let height = (size.y * window.scale_factor() as f32) as u32;
                let resize_event = Event::WindowEvent {
                    window_id: window.id(),
                    event: WindowEvent::Resized(PhysicalSize::new(width, height)),
                };
                game.on_event(&resize_event);
            }
        });

        if self.state.changed_since_last_save {
            window.set_title(&format!("{}*", self.state.editor_title));
        } else {
            window.set_title(&self.state.editor_title);
        }

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

        if self.state.save_requested {
            let x = game.rects[0].position[0];
            let y = game.rects[0].position[1];
            let width = game.rects[0].size[0];
            let height = game.rects[0].size[1];
            let rotation = game.rects[0].rotation_degrees;
            let transform = format!("{} {} {} {} {}", x, y, width, height, rotation);

            let r = game.rects[0].color[0];
            let g = game.rects[0].color[1];
            let b = game.rects[0].color[2];
            let a = game.rects[0].color[3];
            let color = format!("{} {} {} {}", r, g, b, a);

            let state = format!("{}\n{}\n", transform, color);
            let path = path::Path::new("alpha_game.alpha");
            fs::write(path, state).expect("Unable to write file alpha_game.alpha");

            self.state.save_requested = false;
            self.state.changed_since_last_save = false;
        }

        if self.state.build_requested {
            self.state.build_requested = false;
            let copy_src = path::Path::new("alpha_game.alpha");
            let copy_dst = path::Path::new("alpha_game.ini");
            fs::copy(copy_src, copy_dst)
                .expect("Unable to copy alpha_game.alpha to alpha_game.ini");
        }

        Ok(())
    }

    fn on_stop(&mut self) {
        if let Some(game) = &mut self.game {
            game.on_stop();
        }

        info!("EDITOR on_stop");
    }
}
