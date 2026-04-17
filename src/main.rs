mod assembly;
mod editor_camera;
mod log;
mod math;

use avian3d::prelude::*;
use bevy::{
    log::LogPlugin,
    prelude::*,
    window::{PresentMode, WindowMode},
};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use bevy_steamworks::SteamworksPlugin;
use big_space::prelude::*;
use tracing::Level;

use assembly::{debug_visual_tree, spawn_tank, tank_ui, update_tank};
use editor_camera::EditorCameraPlugin;

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(LogPlugin {
        fmt_layer: log::fmt_layer,
        custom_layer: log::custom_layer,
        level: Level::DEBUG,
        ..default()
    });
    match SteamworksPlugin::init() {
        Ok(plugin) => {
            app.add_plugins(plugin);
            info!("Steamworks succesfully initialized")
        }
        Err(err) => warn!("Steamworks failed to initialize: {err}"),
    }
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    present_mode: PresentMode::Mailbox,
                    ..default()
                }),
                ..default()
            })
            .disable::<LogPlugin>()
            .disable::<TransformPlugin>(),
    )
    .add_plugins(MeshPickingPlugin)
    .add_plugins(PhysicsPlugins::default())
    .add_plugins(BigSpaceDefaultPlugins)
    .add_plugins(EguiPlugin::default())
    .add_systems(Startup, spawn_tank)
    .add_systems(Update, update_tank)
    .add_systems(EguiPrimaryContextPass, (tank_ui, debug_visual_tree))
    .add_plugins(EditorCameraPlugin)
    .run()
}
