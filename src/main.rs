mod math;

use std::sync::Arc;

use bevy::{
    prelude::*,
    window::{PresentMode, WindowMode},
};
use bevy_egui::{
    EguiContexts, EguiPlugin, EguiStartupSet,
    egui::{FontData, FontDefinitions, FontFamily},
};

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                present_mode: PresentMode::Mailbox,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_systems(
            PreStartup,
            setup_system.before(EguiStartupSet::InitContexts),
        )
        .add_systems(Startup, setup_fonts)
        .run()
}

fn setup_system(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_fonts(mut contexts: EguiContexts) -> Result {
    let ctx = contexts.ctx_mut()?;

    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "droid_sans".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../fonts/droid-sans.regular.ttf"
        ))),
    );

    fonts.font_data.insert(
        "droid_sans_mono".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../fonts/droid-sans-mono.regular.ttf"
        ))),
    );

    fonts
        .families
        .insert(FontFamily::Proportional, vec!["droid_sans".to_owned()]);

    fonts
        .families
        .insert(FontFamily::Monospace, vec!["droid_sans_mono".to_owned()]);

    ctx.set_fonts(fonts);

    Ok(())
}
