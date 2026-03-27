#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::float_cmp)]

mod math;

use bevy::prelude::*;
use bevy_egui::{
    EguiContexts, EguiPlugin, EguiPrimaryContextPass,
    egui::{self, FontId, RichText},
};

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup_camera_system)
        .add_systems(EguiPrimaryContextPass, ui_example_system)
        .insert_resource(UiState {
            is_window_open: true,
        })
        .run()
}

#[derive(Resource)]
struct UiState {
    is_window_open: bool,
}

fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn ui_example_system(mut contexts: EguiContexts, mut ui_state: ResMut<UiState>) -> Result {
    egui::Window::new("Hello")
        .vscroll(true)
        .open(&mut ui_state.is_window_open)
        .show(contexts.ctx_mut()?, |ui| {
            ui.heading("Orbit");

            ui.separator();

            ui.label(RichText::new("Apoapsis: 120km").font(FontId::monospace(14.0)));
            ui.label(RichText::new("Periapsis: 80km").font(FontId::monospace(14.0)));
            ui.label(RichText::new("Eccentricity: 0.17").font(FontId::monospace(14.0)));
        });
    Ok(())
}
