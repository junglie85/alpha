use crate::components::{Shape, Tag, Transform};
use crate::editor::EditorState;
use crate::engine::Application;
use crate::game::Game;
use egui::{FullOutput, Slider, TextureId, Ui};
use glam::Vec4;
use hecs::Entity;
use std::{fs, path};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::window::Window;

pub(crate) fn update(
    egui_ctx: &egui::Context,
    egui_platform: &mut egui_winit::State,
    game_scene_texture_id: TextureId,
    game: &mut Game,
    state: &mut EditorState,
    window: &Window,
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
        };
    });

    egui::CentralPanel::default().show(egui_ctx, |ui| {
        let size = ui.available_size_before_wrap();
        ui.image(game_scene_texture_id, size);

        if state.window_resized {
            let width = (size.x * window.scale_factor() as f32) as u32;
            let height = (size.y * window.scale_factor() as f32) as u32;
            let resize_event = Event::WindowEvent {
                window_id: window.id(),
                event: WindowEvent::Resized(PhysicalSize::new(width, height)),
            };
            game.on_event(&resize_event);
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

            editor_state = format!("{}{}\n{}\n{}\n---\n", editor_state, tag, transform, color);
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
    // egui_platform.handle_platform_output(window, &egui_ctx, egui_ctx.output().deref());
}
