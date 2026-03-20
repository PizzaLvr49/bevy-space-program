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
            warn!("Steamworks failed to initilize, {error:?}, running standalone",);
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
    .add_plugins(PhysicsDebugPlugin)
    .add_systems(Startup, (test_steamworks, test_big_space))
    .add_systems(PostStartup, check_precision)
    .run();
}

fn test_steamworks(client: If<Res<Client>>) {
    info!(
        "Player: {}, id: {:?}",
        client.friends().name(),
        client.user().steam_id(),
    );
}

fn test_big_space(mut commands: Commands) {
    commands.spawn_big_space(Grid::new(10_000.0, 0.01), |root| {
        root.spawn_spatial((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            FloatingOrigin,
            Name::new("Camera"),
        ));

        root.spawn_spatial((
            Transform::from_xyz(0.0, 0.0, 0.0),
            CellCoord::new(0, 0, 3),
            Name::new("Test Object"),
        ));
    });
}

fn check_precision(grid: Single<&Grid>, objects: Query<(&CellCoord, &Transform, &Name)>) {
    for (cell, transform, name) in &objects {
        let pos = grid.grid_position_double(cell, transform);
        info!("{} is at world pos: {:?}", name, pos);
    }
}
