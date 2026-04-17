use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::editor_camera::EditorCamera;

#[derive(Component)]
pub struct FuelTank {
    height: f32,
    radius: f32,
}

#[derive(Component)]
pub struct RocketPart;

fn on_drag_start(
    drag: On<Pointer<DragStart>>,
    materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut assets: ResMut<Assets<StandardMaterial>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if let Ok(mat) = materials.get(drag.entity)
        && let Some(material) = assets.get_mut(&mat.0)
    {
        material.base_color = Color::srgba(0.2, 1.0, 0.2, 0.2);
        material.alpha_mode = AlphaMode::Blend;
    }
}

fn on_drag_end(
    drag: On<Pointer<DragEnd>>,
    materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut assets: ResMut<Assets<StandardMaterial>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if let Ok(mat) = materials.get(drag.entity)
        && let Some(material) = assets.get_mut(&mat.0)
    {
        material.base_color = Color::WHITE;
        material.alpha_mode = AlphaMode::Opaque;
    }
}

fn on_drag(
    drag: On<Pointer<Drag>>,
    mut transforms: Query<&mut Transform>,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }

    let mut transform = transforms.get_mut(drag.entity).unwrap();
    let (camera, camera_transform) = camera.into_inner();

    let plane_normal = camera_transform.forward().as_vec3();
    let plane_point = transform.translation;

    let ray = |p| camera.viewport_to_world(camera_transform, p).unwrap();

    let intersect = |ray: Ray3d| {
        let denom = ray.direction.dot(plane_normal);
        if denom.abs() < 1e-6 {
            return None;
        }
        let t = (plane_point - ray.origin).dot(plane_normal) / denom;
        Some(ray.origin + ray.direction * t)
    };

    let curr = drag.pointer_location.position;
    let prev = curr - drag.delta;

    if let (Some(world_curr), Some(world_prev)) = (intersect(ray(curr)), intersect(ray(prev))) {
        let mut next = transform.translation + (world_curr - world_prev);
        next.x = next.x.clamp(-20.0, 20.0);
        next.y = next.y.clamp(-100.0, 100.0);
        next.z = next.z.clamp(-20.0, 20.0);
        transform.translation = next;
    }
}

fn spawn_part(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    transform: Transform,
) {
    commands
        .spawn((
            FuelTank {
                height: 5.0,
                radius: 1.0,
            },
            RocketPart,
            Mesh3d(meshes.add(Cylinder::new(0.5, 2.0))),
            MeshMaterial3d(materials.add(StandardMaterial::default())),
            Pickable::default(),
            transform,
        ))
        .observe(on_drag_start)
        .observe(on_drag_end)
        .observe(on_drag);
}

pub fn spawn_tank(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            range: 200.0,
            intensity: 50000000.0,
            ..default()
        },
        Transform::from_xyz(20.0, 40.0, 20.0),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        EditorCamera::default(),
    ));

    spawn_part(
        &mut commands,
        &mut meshes,
        &mut materials,
        Transform::from_xyz(0.0, 10.0, 0.0),
    );
    spawn_part(
        &mut commands,
        &mut meshes,
        &mut materials,
        Transform::default(),
    );
}
pub fn tank_ui(mut contexts: EguiContexts, mut query: Query<(Entity, &mut FuelTank)>) {
    for (entity, mut tank) in query.iter_mut() {
        egui::Window::new("Fuel Tank")
            .id(egui::Id::new(entity))
            .show(contexts.ctx_mut().unwrap(), |ui| {
                ui.add(egui::Slider::new(&mut tank.height, 0.5..=10.0).text("Height"));
                ui.add(egui::Slider::new(&mut tank.radius, 0.1..=5.0).text("Radius"));
            });
    }
}

pub fn update_tank(
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(&Mesh3d, &FuelTank), Changed<FuelTank>>,
) {
    for (mesh_handle, tank) in &query {
        if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
            *mesh = Cylinder::new(tank.radius, tank.height).mesh().build();
        }
    }
}
