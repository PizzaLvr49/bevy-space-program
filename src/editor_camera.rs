use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy_egui::EguiContexts;

#[derive(Component)]
pub struct EditorCamera {
    pub target_y: f32,
    pub actual_y: f32,
    pub target_radius: f32,
    pub actual_radius: f32,
    pub target_yaw: f32,
    pub actual_yaw: f32,
    pub target_pitch: f32,
    pub actual_pitch: f32,
    pub smooth_speed: f32,
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self {
            target_y: 2.0,
            actual_y: 2.0,
            target_radius: 10.0,
            actual_radius: 10.0,
            target_yaw: 0.0,
            actual_yaw: 0.0,
            target_pitch: -0.2,
            actual_pitch: -0.2,
            smooth_speed: 10.0,
        }
    }
}

pub struct EditorCameraPlugin;

impl Plugin for EditorCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, editor_camera_system);
    }
}

fn editor_camera_system(
    mut contexts: EguiContexts,
    mut ev_motion: MessageReader<MouseMotion>,
    mut ev_scroll: MessageReader<MouseWheel>,
    input_mouse: Res<ButtonInput<MouseButton>>,
    input_keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut EditorCamera, &mut Transform)>,
) {
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_pointer_input()
    {
        ev_motion.clear();
        ev_scroll.clear();
        return;
    }

    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;

    for ev in ev_motion.read() {
        if input_mouse.pressed(MouseButton::Right) {
            rotation_move += ev.delta;
        }
    }

    for ev in ev_scroll.read() {
        scroll += match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y * 0.01,
        };
    }

    let is_shift =
        input_keyboard.pressed(KeyCode::ShiftLeft) || input_keyboard.pressed(KeyCode::ShiftRight);
    let dt = time.delta_secs();

    for (mut cam, mut transform) in query.iter_mut() {
        if scroll.abs() > 0.0 {
            if is_shift {
                cam.target_radius -= scroll * cam.target_radius * 0.2;
                cam.target_radius = cam.target_radius.clamp(1.0, 100.0);
            } else {
                cam.target_y += scroll * 0.5;
                cam.target_y = cam.target_y.clamp(0.0, 20.0);
            }
        }

        if rotation_move.length_squared() > 0.0 {
            cam.target_yaw -= rotation_move.x * 0.005;
            cam.target_pitch -= rotation_move.y * 0.005;

            cam.target_pitch = cam.target_pitch.clamp(
                -std::f32::consts::FRAC_PI_2 + 0.05,
                std::f32::consts::FRAC_PI_2 - 0.05,
            );
        }

        let decay = 1.0 - (-cam.smooth_speed * dt).exp();

        cam.actual_y += (cam.target_y - cam.actual_y) * decay;
        cam.actual_radius += (cam.target_radius - cam.actual_radius) * decay;
        cam.actual_yaw += (cam.target_yaw - cam.actual_yaw) * decay;
        cam.actual_pitch += (cam.target_pitch - cam.actual_pitch) * decay;

        let focus = Vec3::new(0.0, cam.actual_y, 0.0);
        let rotation = Quat::from_euler(EulerRot::YXZ, cam.actual_yaw, cam.actual_pitch, 0.0);

        transform.rotation = rotation;
        transform.translation = focus + rotation * Vec3::Z * cam.actual_radius;
    }
}
