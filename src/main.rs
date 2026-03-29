mod log;
mod math;

use bevy::{
    log::LogPlugin,
    prelude::*,
    window::{PresentMode, WindowMode},
};
use bevy_steamworks::SteamworksPlugin;
use tracing::Level;

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
                primary_window: None,
                ..default()
            })
            .disable::<LogPlugin>(),
    )
    .add_systems(Startup, spawn_window)
    .run()
}

fn spawn_window(mut commands: Commands) {
    commands.spawn(Window {
        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        present_mode: PresentMode::Mailbox,
        ..default()
    });
}
