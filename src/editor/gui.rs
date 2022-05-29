use crate::components::{compute_inverse_transformation_matrix, Script, Shape, Tag, Transform};
use crate::editor::EditorState;
use crate::engine::Application;
use crate::game::Game;
use egui::{FullOutput, Image, PointerButton, Pos2, Sense, Slider, TextureId, Ui, Widget};
use glam::{Vec2, Vec4, Vec4Swizzles};
use hecs::Entity;
use std::{fs, path};
use wgpu::{Device, Texture};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::window::Window;
use winit_input_helper::WinitInputHelper;

struct EntityDetails<'a> {
    id: Entity,
    tag: &'a String,
}

impl<'a> EntityDetails<'a> {
    fn ui(&mut self, ui: &mut Ui, state: &mut EditorState) {
        if ui.button(self.tag).clicked() {
            state.active_entity = Some(self.id);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update(
    egui_ctx: &egui::Context,
    egui_platform: &mut egui_winit::State,
    game_scene_texture_id: TextureId,
    game: &mut Game,
    state: &mut EditorState,
    window: &Window,
    input: &WinitInputHelper,
    game_scene_texture: &mut Texture,
    device: &Device,
) -> FullOutput {
    let egu_input = egui_platform.take_egui_input(window);
    egui_ctx.begin_frame(egu_input);

    egui::TopBottomPanel::top("Menu Bar").show(&egui_ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            let save = ui.button("💾 Save").clicked();
            if save {
                state.save_requested = true;
            }

            let build = ui.button("🛠 Build").clicked();
            if build {
                state.build_requested = true;
            }
        });
    });

    egui::SidePanel::left("Scene Hierarchy").show(egui_ctx, |ui| {
        for entity_ref in game.world.iter() {
            let entity = entity_ref.entity();
            let tag = game
                .world
                .get::<Tag>(entity)
                .expect("All entities should have tags");

            let mut entity_details = EntityDetails {
                id: entity,
                tag: &tag.0,
            };
            entity_details.ui(ui, state);
        }
    });

    egui::SidePanel::right("Properties Panel").show(egui_ctx, |ui| {
        if let Some(entity) = state.active_entity {
            if let Ok(mut tag) = game.world.get_mut::<Tag>(entity) {
                egui::CollapsingHeader::new("Tag")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label("Tag");
                        if ui.text_edit_singleline(&mut tag.0).changed() {
                            state.changed_since_last_save = true;
                        }
                    });
            };

            if let Ok(mut transform) = game.world.get_mut::<Transform>(entity) {
                egui::CollapsingHeader::new("Transform")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label("Position");
                        let slider = Slider::new(&mut transform.position.x, -2000.0..=2000.0)
                            .text("x")
                            .clamp_to_range(false);
                        if ui.add(slider).changed() {
                            state.changed_since_last_save = true;
                        }
                        let slider = Slider::new(&mut transform.position.y, -2000.0..=2000.0)
                            .text("y")
                            .clamp_to_range(false);
                        if ui.add(slider).changed() {
                            state.changed_since_last_save = true;
                        }

                        ui.label("Rotation");
                        let slider =
                            Slider::new(&mut transform.rotation, 0.0..=360.0).clamp_to_range(false);
                        if ui.add(slider).changed() {
                            state.changed_since_last_save = true;
                        }

                        ui.label("Size");
                        let slider = Slider::new(&mut transform.size.x, 0.0..=2000.0)
                            .text("width")
                            .clamp_to_range(false);
                        if ui.add(slider).changed() {
                            state.changed_since_last_save = true;
                        }
                        let slider = Slider::new(&mut transform.size.y, 0.0..=2000.0)
                            .text("height")
                            .clamp_to_range(false);
                        if ui.add(slider).changed() {
                            state.changed_since_last_save = true;
                        }
                    });
            }

            if let Ok(mut shape) = game.world.get_mut::<Shape>(entity) {
                egui::CollapsingHeader::new("Shape")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label("Color");

                        let mut color = shape.color.to_array();

                        if ui.color_edit_button_rgba_unmultiplied(&mut color).changed() {
                            shape.color = Vec4::from_slice(&color);
                            state.changed_since_last_save = true;
                        }
                    });
            }

            if let Ok(mut script) = game.world.get_mut::<Script>(entity) {
                egui::CollapsingHeader::new("Script")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label("File");
                        ui.label(script.filepath.as_ref().unwrap());

                        if ui.button("Choose file…").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                script.filepath = Some(path.display().to_string());
                            }
                        }
                    });
            }
        };
    });

    egui::CentralPanel::default().show(egui_ctx, |ui| {
        egui::TopBottomPanel::bottom("Scene Info Bar").show_inside(ui, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::SidePanel::right("").show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        if input.mouse().is_some() {
                            ui.label(format!(
                                "Window: ({:.2}, {:.2}),",
                                state.mouse_window_pos.x, state.mouse_window_pos.y
                            ));
                            ui.label(format!(
                                "Viewport: ({:.2}, {:.2}),",
                                state.mouse_viewport_pos.x, state.mouse_viewport_pos.y
                            ));
                            ui.label(format!(
                                "World: ({:.2}, {:.2})",
                                state.mouse_world_pos.x, state.mouse_world_pos.y
                            ));
                        } else {
                            ui.label("Window: (-, -)");
                            ui.label("Viewport: (-, -)");
                            ui.label("World: (-, -)");
                        }
                    });
                });
            });
        });

        let size = ui.available_size_before_wrap();
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.spacing_mut().item_spacing.y = 0.0;
        let scene = Image::new(game_scene_texture_id, size)
            .sense(Sense::hover())
            .sense(Sense::click())
            .ui(ui);

        if let Some(Pos2 {
            x: mouse_x,
            y: mouse_y,
        }) = scene.hover_pos()
        {
            state.mouse_window_pos.x = mouse_x * window.scale_factor() as f32;
            state.mouse_window_pos.y = mouse_y * window.scale_factor() as f32;

            state.mouse_viewport_pos.x =
                state.mouse_window_pos.x - scene.rect.min.x * window.scale_factor() as f32; //viewport_x;
            state.mouse_viewport_pos.y =
                state.mouse_window_pos.y - scene.rect.min.y * window.scale_factor() as f32; //viewport_y;

            let viewport_width = size.x * window.scale_factor() as f32;
            let viewport_height = size.y * window.scale_factor() as f32;
            let viewport_dims = Vec2::new(viewport_width, viewport_height);
            let mut ndc = ((state.mouse_viewport_pos / viewport_dims) * 2.0) - 1.0;
            ndc.y *= -1.0; // TODO: Why is this even necessary?
            let ndc = Vec4::from((ndc, 1.0, 1.0));

            let inverse_projection = game.camera.get_projection().inverse();
            let inverse_view = game.camera.get_view().inverse();

            let world = inverse_view * inverse_projection * ndc;
            state.mouse_world_pos.x = world.x;
            state.mouse_world_pos.y = world.y;
        }

        if scene.clicked_by(PointerButton::Primary) {
            for (id, (transform,)) in game.world.query::<(&Transform,)>().iter() {
                let inverse = compute_inverse_transformation_matrix(transform);
                let test_point = (inverse * Vec4::from((state.mouse_world_pos, 0.0, 1.0))).xy();

                if test_point.x >= 0.0
                    && test_point.x <= 1.0
                    && test_point.y >= 0.0
                    && test_point.y <= 1.0
                {
                    state.active_entity = Some(id);
                }
            }
        }

        if scene.clicked_by(PointerButton::Secondary) {
            let tag = Tag(String::from("Entity"));

            let transform = Transform {
                position: state.mouse_world_pos,
                size: Vec2::new(100.0, 100.0),
                rotation: 0.0,
            };

            let shape = Shape {
                color: Vec4::new(1.0, 0.0, 0.0, 1.0),
            };

            game.world.spawn((tag, transform, shape));
        }

        if state.window_resized {
            let width = (size.x * window.scale_factor() as f32) as u32;
            let height = (size.y * window.scale_factor() as f32) as u32;
            let resize_event = Event::WindowEvent {
                window_id: window.id(),
                event: WindowEvent::Resized(PhysicalSize::new(width, height)),
            };
            game.on_event(&resize_event);

            let game_scene_texture_desc = wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: size.x as u32,
                    height: size.y as u32,
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
            *game_scene_texture = device.create_texture(&game_scene_texture_desc);
        }
    });

    if state.changed_since_last_save {
        window.set_title(&format!("{}*", state.editor_title));
    } else {
        window.set_title(&state.editor_title);
    }

    if state.save_requested {
        let mut editor_state = String::default();

        for e_ref in game.world.iter() {
            // We know we only have 2 entities, both with the same components, so let's hack this in for now.
            // TODO: Implement hecs serde.
            let entity = e_ref.entity();

            let tag = game.world.get::<Tag>(entity).unwrap();
            let tag = tag.0.to_string();

            let transform = game.world.get::<Transform>(entity).unwrap();
            let x = transform.position.x;
            let y = transform.position.y;
            let width = transform.size.x;
            let height = transform.size.y;
            let rotation = transform.rotation;
            let transform = format!("{} {} {} {} {}", x, y, width, height, rotation);

            let shape = game.world.get::<Shape>(entity).unwrap();
            let r = shape.color.x;
            let g = shape.color.y;
            let b = shape.color.z;
            let a = shape.color.w;
            let color = format!("{} {} {} {}", r, g, b, a);

            let wasm = if let Ok(script) = game.world.get::<Script>(entity) {
                format!("\n{}", script.filepath.as_ref().unwrap())
            } else {
                String::from("")
            };

            editor_state = format!(
                "{}{}\n{}\n{}{}\n---\n",
                editor_state, tag, transform, color, wasm
            );
        }

        let path = path::Path::new("alpha_game.alpha");
        fs::write(path, editor_state).expect("Unable to write file alpha_game.alpha");

        state.save_requested = false;
        state.changed_since_last_save = false;
    }

    if state.build_requested {
        state.build_requested = false;
        let copy_src = path::Path::new("alpha_game.alpha");
        let copy_dst = path::Path::new("alpha_game.ini");
        fs::copy(copy_src, copy_dst).expect("Unable to copy alpha_game.alpha to alpha_game.ini");
    }

    egui_ctx.end_frame()
}
