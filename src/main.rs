use avian3d::prelude::*;
use bevy::{
    log::LogPlugin,
    prelude::*,
    window::{MonitorSelection, PresentMode, WindowMode},
};
use bevy_steamworks::{Client, SteamworksPlugin};
use big_space::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(LogPlugin::default());

    match SteamworksPlugin::init() {
        Ok(steam_plugin) => {
            app.add_plugins(steam_plugin);
            info!("Steamworks initialized");
        }
        Err(error) => {
            info!("Steamworks failed to initilize, {error:?}, running standalone",);
        }
    }

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Space Program".to_string(),
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    present_mode: PresentMode::Mailbox,
                    ..default()
                }),
                ..default()
            })
            .build()
            .disable::<TransformPlugin>()
            .disable::<LogPlugin>(),
    )
    .add_plugins(BigSpaceDefaultPlugins)
    .add_plugins(PhysicsPlugins::default())
    .add_systems(Startup, test_steamworks)
    .run();
}

fn test_steamworks(client: If<Res<Client>>) {
    info!(
        "Player: {}, id: {:?}",
        client.friends().name(),
        client.user().steam_id(),
    );
}
