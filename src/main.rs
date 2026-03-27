#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::float_cmp)]

mod math;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup_camera_system)
        .add_systems(EguiPrimaryContextPass, ui_example_system)
        .insert_resource(UiState {
            is_window_open: true,
            apoapsis: 120.0,
            periapsis: 80.0,
            eccentricity: 0.17,
        })
        .run()
}

#[derive(Resource)]
struct UiState {
    is_window_open: bool,
    apoapsis: f64,
    periapsis: f64,
    eccentricity: f64,
}

fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn ui_example_system(mut contexts: EguiContexts, mut ui_state: ResMut<UiState>) -> Result {
    let UiState {
        is_window_open,
        apoapsis,
        periapsis,
        eccentricity,
    } = &mut *ui_state;

    egui::SidePanel::left("toolbar")
        .resizable(false)
        .default_width(140.0)
        .show(contexts.ctx_mut()?, |ui| {
            ui.heading("Widgets");
            ui.separator();

            ui.toggle_value(is_window_open, "Orbit");
        });

    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Orbit Widget")
        .constrain_to(ctx.available_rect())
        .vscroll(true)
        .open(is_window_open)
        .show(ctx, |ui| {
            ui.heading("Orbit");

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Apoapsis:");
                ui.add(
                    egui::DragValue::new(apoapsis)
                        .range(0.0..=f64::INFINITY)
                        .suffix(" km"),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Periapsis:");
                ui.add(
                    egui::DragValue::new(periapsis)
                        .range(0.0..=f64::INFINITY)
                        .suffix(" km"),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Eccentricity:");
                ui.add(
                    egui::DragValue::new(eccentricity)
                        .speed(0.01)
                        .range(0.0..=10.0),
                );
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Calculations");
                if ui.button("Calculate Something").clicked() {
                    info!("Calculating Something");
                }
            });
        });

    Ok(())
}
