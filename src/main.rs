mod log;
mod math;

use avian3d::prelude::*;
use bevy::{
    log::LogPlugin,
    prelude::*,
    window::{PresentMode, WindowMode},
};
use bevy_steamworks::SteamworksPlugin;
use big_space::prelude::*;
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
    .add_plugins(PhysicsPlugins::default())
    .add_plugins(BigSpaceDefaultPlugins)
    .run()
}
