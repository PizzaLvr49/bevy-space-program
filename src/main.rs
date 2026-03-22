#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

use avian3d::prelude::*;
use bevy::{
    log::{Level, LogPlugin},
    prelude::*,
    window::{PresentMode, WindowMode},
};
use bevy_steamworks::{Client, SteamworksPlugin};
use big_space::prelude::*;
use semver::Version;
use std::sync::LazyLock;

use space::{CelestialBody, KeplerOrbit};

use keplerian_sim::{Orbit, OrbitTrait};

mod log;
mod space;

pub static GAME_VERSION: LazyLock<Version> = LazyLock::new(|| Version::parse("0.0.0").unwrap());

fn main() -> AppExit {
    let mut app = App::new();

    app.add_plugins(LogPlugin {
        level: Level::DEBUG,
        custom_layer: log::custom_layer,
        fmt_layer: log::fmt_layer,
        ..default()
    });

    match SteamworksPlugin::init() {
        Ok(steam_plugin) => {
            app.add_plugins(steam_plugin);
            info!("Steamworks initialized");
        }
        Err(error) => {
            warn!("Steamworks failed to initialize, {error:?}, running standalone");
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
    .add_systems(Update, log_orbit)
    .insert_resource(Gravity::ZERO);

    app.run()
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
            FloatingOrigin,
            KeplerOrbit::from(Orbit::new_flat_circular(3_000.0, 0.0, 10_000.0)),
            Name::new("Orbiter"),
        ));

        root.spawn_spatial((CelestialBody::new(10_000.0), Name::new("Test Object")));
    });
}

fn log_orbit(time: Res<Time>, orbits: Query<(&KeplerOrbit, &Name)>) {
    for (orbit, name) in &orbits {
        let pos = orbit.0.get_position_at_time(time.elapsed_secs_f64());
        let vel = orbit
            .0
            .get_velocity_at_time(time.elapsed_secs_f64())
            .length();
        info!("Orbit: {name} is at: {pos:.2?}, with vel: {vel:.4?}");
    }
}
