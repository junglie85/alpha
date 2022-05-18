use crate::editor::EditorState;
use crate::engine::Application;
use crate::game::Game;
use egui::epaint::ClippedShape;
use egui::{CtxRef, Slider, TextureId};
use egui_winit_platform::Platform;
use std::time::Instant;
use std::{fs, path};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::window::Window;

pub(crate) fn update(
    egui_platform: &mut Platform,
    egui_start_time: Instant,
    game_scene_texture_id: TextureId,
    game: &mut Game,
    state: &mut EditorState,
    window: &Window,
) -> (CtxRef, Vec<ClippedShape>) {
    egui_platform.update_time(egui_start_time.elapsed().as_secs_f64());
    egui_platform.begin_frame();

    let egui_ctx = egui_platform.context();

    egui::TopBottomPanel::top("toolbar").show(&egui_ctx, |ui| {
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

    egui::SidePanel::right("right pane").show(&egui_ctx, |ui| {
        egui::CollapsingHeader::new("Shape")
            .default_open(true)
            .show(ui, |ui| {
                ui.label("Color");
                if ui
                    .color_edit_button_rgba_unmultiplied(&mut game.rects[0].color.to_array())
                    .changed()
                {
                    state.changed_since_last_save = true;
                }
            });

        egui::CollapsingHeader::new("Transform")
            .default_open(true)
            .show(ui, |ui| {
                ui.label("Position");
                let slider = Slider::new(&mut game.rects[0].position[0], -2000.0..=2000.0)
                    .text("x")
                    .clamp_to_range(false);
                if ui.add(slider).changed() {
                    state.changed_since_last_save = true;
                }
                let slider = Slider::new(&mut game.rects[0].position[1], -2000.0..=2000.0)
                    .text("y")
                    .clamp_to_range(false);
                if ui.add(slider).changed() {
                    state.changed_since_last_save = true;
                }

                ui.label("Rotation");
                let slider = Slider::new(&mut game.rects[0].rotation_degrees, 0.0..=360.0)
                    .clamp_to_range(false);
                if ui.add(slider).changed() {
                    state.changed_since_last_save = true;
                }

                ui.label("Size");
                let slider = Slider::new(&mut game.rects[0].size.x, 0.0..=2000.0)
                    .text("width")
                    .clamp_to_range(false);
                if ui.add(slider).changed() {
                    state.changed_since_last_save = true;
                }
                let slider = Slider::new(&mut game.rects[0].size.y, 0.0..=2000.0)
                    .text("height")
                    .clamp_to_range(false);
                if ui.add(slider).changed() {
                    state.changed_since_last_save = true;
                }
            });
    });

    egui::CentralPanel::default().show(&egui_ctx, |ui| {
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
        let x = game.rects[0].position.x;
        let y = game.rects[0].position.y;
        let width = game.rects[0].size.x;
        let height = game.rects[0].size.y;
        let rotation = game.rects[0].rotation_degrees;
        let transform = format!("{} {} {} {} {}", x, y, width, height, rotation);

        let r = game.rects[0].color.x;
        let g = game.rects[0].color.y;
        let b = game.rects[0].color.z;
        let a = game.rects[0].color.w;
        let color = format!("{} {} {} {}", r, g, b, a);

        let editor_state = format!("{}\n{}\n", transform, color);
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

    let (_, paint_commands) = egui_platform.end_frame(Some(window));

    (egui_ctx, paint_commands)
}
