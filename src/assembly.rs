use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use std::collections::HashMap;

use crate::editor_camera::EditorCamera;

#[derive(Component)]
#[relationship(relationship_target = AttachedChildren)]
pub struct AttachedTo {
    #[relationship]
    pub parent: Entity,
    pub parent_point: usize,
    pub child_point: usize,
}

#[derive(Component, Default)]
#[relationship_target(relationship = AttachedTo)]
pub struct AttachedChildren(Vec<Entity>);

#[derive(Component)]
pub struct FuelTank {
    height: f32,
    radius: f32,
}

#[derive(Component)]
pub struct RocketPart {
    attach_points: Vec<Vec3>,
}

#[derive(Component)]
struct PartMaterials {
    normal: Handle<StandardMaterial>,
    highlighted: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct SnapTarget {
    entity: Entity,
    local_offset: Vec3,
    parent_point: usize,
    child_entity: Entity,
    child_point: usize,
}

#[derive(Component, Clone, Copy)]
struct DragState {
    cursor_offset: Vec3,
    plane_origin: Vec3,
    plane_normal: Vec3,
}

fn on_drag_start(
    mut drag: On<Pointer<DragStart>>,
    global_transforms: Query<&GlobalTransform>,
    camera: Single<(&Camera, &GlobalTransform)>,
    children_query: Query<&Children>,
    part_materials: Query<&PartMaterials>,
    mut commands: Commands,
) {
    if drag.button != PointerButton::Primary {
        return;
    }

    drag.propagate(false);

    let entity = drag.entity;
    let (camera, camera_transform) = camera.into_inner();
    let part_world = global_transforms.get(entity).unwrap().translation();
    let plane_normal = camera_transform.forward().as_vec3();

    let cursor_world = {
        let screen = drag.pointer_location.position;
        let ray = camera.viewport_to_world(camera_transform, screen).unwrap();
        let denom = ray.direction.dot(plane_normal);
        let t = (part_world - ray.origin).dot(plane_normal) / denom;
        ray.origin + ray.direction * t
    };

    commands.entity(entity).insert(DragState {
        cursor_offset: part_world - cursor_world,
        plane_origin: part_world,
        plane_normal,
    });

    let mut queue = vec![entity];
    while let Some(e) = queue.pop() {
        if let Ok(mats) = part_materials.get(e) {
            commands
                .entity(e)
                .insert(MeshMaterial3d(mats.highlighted.clone()));
        }
        if let Ok(children) = children_query.get(e) {
            queue.extend(children.iter());
        }
    }

    commands
        .entity(entity)
        .remove_parent_in_place()
        .remove::<AttachedTo>();
}

fn on_drag_end(
    mut drag: On<Pointer<DragEnd>>,
    children_query: Query<&Children>,
    part_materials: Query<&PartMaterials>,
    snap: Query<&SnapTarget>,
    mut transforms: Query<&mut Transform>,
    mut commands: Commands,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    drag.propagate(false);

    let entity = drag.entity;

    let mut queue = vec![entity];
    while let Some(e) = queue.pop() {
        if let Ok(mats) = part_materials.get(e) {
            commands
                .entity(e)
                .insert(MeshMaterial3d(mats.normal.clone()));
        }
        if let Ok(children) = children_query.get(e) {
            queue.extend(children.iter());
        }
    }

    commands.entity(entity).remove::<DragState>();

    if let Ok(target) = snap.get(entity) {
        if let Ok(mut transform) = transforms.get_mut(entity) {
            transform.translation = target.local_offset;
        }

        commands
            .entity(target.child_entity)
            .insert(ChildOf(target.entity))
            .insert(AttachedTo {
                parent: target.entity,
                parent_point: target.parent_point,
                child_point: target.child_point,
            });

        commands.entity(entity).remove::<SnapTarget>();
    }
}

fn on_drag(
    mut drag: On<Pointer<Drag>>,
    mut transforms: Query<&mut Transform>,
    drag_states: Query<&DragState>,
    parts: Query<(Entity, &RocketPart, &GlobalTransform)>,
    camera: Single<(&Camera, &GlobalTransform)>,
    parents: Query<&ChildOf>,
    children_query: Query<&Children>,
    attached: Query<&AttachedTo>,
    mut commands: Commands,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    drag.propagate(false);

    let entity = drag.entity;
    let (camera, camera_transform) = camera.into_inner();

    let Ok(drag_state) = drag_states.get(entity) else {
        return;
    };

    let DragState {
        cursor_offset,
        plane_origin,
        plane_normal,
    } = *drag_state;

    let intersect = |screen: Vec2| -> Option<Vec3> {
        let ray = camera.viewport_to_world(camera_transform, screen).ok()?;
        let denom = ray.direction.dot(plane_normal);
        if denom.abs() < f32::EPSILON {
            return None;
        }
        let t = (plane_origin - ray.origin).dot(plane_normal) / denom;
        Some(ray.origin + ray.direction * t)
    };

    let Some(world_curr) = intersect(drag.pointer_location.position) else {
        return;
    };

    let mut next = world_curr + cursor_offset;
    next.x = next.x.clamp(-20.0, 20.0);
    next.y = next.y.clamp(-100.0, 100.0);
    next.z = next.z.clamp(-20.0, 20.0);

    const SNAP_RADIUS: f32 = 0.75;

    let root_global = parts.get(entity).unwrap().2.translation();

    let mut subassembly = Vec::new();
    let mut stack = vec![entity];

    while let Some(e) = stack.pop() {
        subassembly.push(e);
        if let Ok(children) = children_query.get(e) {
            stack.extend(children.iter());
        }
    }

    let mut subassembly_points = Vec::new();

    for e in &subassembly {
        if let Ok((_, part, global)) = parts.get(*e) {
            let offset_from_root = global.translation() - root_global;

            for (i, ap) in part.attach_points.iter().enumerate() {
                let used = attached
                    .iter()
                    .any(|a| a.parent == *e && a.parent_point == i);

                if !used {
                    let local = offset_from_root + *ap;
                    subassembly_points.push((*e, i, local));
                }
            }
        }
    }

    let mut best: Option<(f32, Vec3, Entity, Entity, Vec3, usize, usize)> = None;

    for (other_entity, other_part, other_global) in parts.iter() {
        if subassembly.contains(&other_entity) {
            continue;
        }

        let mut is_descendant = false;
        let mut current_check = other_entity;

        while let Ok(parent) = parents.get(current_check) {
            if subassembly.contains(&parent.parent()) {
                is_descendant = true;
                break;
            }
            current_check = parent.parent();
        }

        if is_descendant {
            continue;
        }

        let other_world = other_global.translation();

        for (own_entity, own_i, local) in &subassembly_points {
            let own_world = next + *local;

            for (other_i, &other_ap) in other_part.attach_points.iter().enumerate() {
                let other_ap_world = other_world + other_ap;
                let dist = own_world.distance(other_ap_world);

                if dist < SNAP_RADIUS && best.is_none_or(|(d, ..)| dist < d) {
                    let snapped_root = other_ap_world - *local;

                    let own_part = parts.get(*own_entity).unwrap().1;
                    let local_offset = other_ap - own_part.attach_points[*own_i];

                    best = Some((
                        dist,
                        snapped_root,
                        other_entity,
                        *own_entity,
                        local_offset,
                        other_i,
                        *own_i,
                    ));
                }
            }
        }
    }

    if let Some((_, snapped_world, parent, child_entity, local_offset, parent_i, child_i)) = best {
        transforms.get_mut(entity).unwrap().translation = snapped_world;

        commands.entity(entity).insert(SnapTarget {
            entity: parent,
            local_offset,
            parent_point: parent_i,
            child_entity,
            child_point: child_i,
        });
    } else {
        transforms.get_mut(entity).unwrap().translation = next;
        commands.entity(entity).remove::<SnapTarget>();
    }
}

fn spawn_part(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    transform: Transform,
) {
    let normal_mat = materials.add(StandardMaterial::default());
    let highlighted_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 1.0, 0.2, 0.4),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands
        .spawn((
            FuelTank {
                height: 5.0,
                radius: 1.0,
            },
            RocketPart {
                attach_points: vec![Vec3::new(0.0, 2.5, 0.0), Vec3::new(0.0, -2.5, 0.0)],
            },
            Name::new("Fuel Tank"),
            Mesh3d(meshes.add(Cylinder::new(1.0, 5.0))),
            MeshMaterial3d(normal_mat.clone()),
            PartMaterials {
                normal: normal_mat,
                highlighted: highlighted_mat,
            },
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
        Transform::from_xyz(0.0, 7.5, 0.0),
    );
    spawn_part(
        &mut commands,
        &mut meshes,
        &mut materials,
        Transform::from_xyz(0.0, 22.5, 0.0),
    );
    spawn_part(
        &mut commands,
        &mut meshes,
        &mut materials,
        Transform::from_xyz(0.0, 15.0, 0.0),
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

pub fn debug_visual_tree(
    mut contexts: EguiContexts,
    roots: Query<Entity, (Without<ChildOf>, With<FuelTank>)>,
    children_query: Query<&Children, With<FuelTank>>,
    names: Query<&Name>,
) {
    egui::Window::new("Visual Hierarchy").show(contexts.ctx_mut().unwrap(), |ui| {
        let (rect, _) = ui.allocate_at_least(ui.available_size(), egui::Sense::hover());
        let painter = ui.painter_at(rect);

        let mut positions = HashMap::new();
        let start_pos = rect.left_top() + egui::vec2(50.0, 50.0);

        let mut y_offset = 0.0;
        for root in roots.iter() {
            draw_node_recursive(
                root,
                start_pos + egui::vec2(0.0, y_offset),
                0,
                &mut y_offset,
                &painter,
                &mut positions,
                &children_query,
                &names,
            );
        }
    });
}

fn draw_node_recursive(
    entity: Entity,
    pos: egui::Pos2,
    depth: usize,
    y_accumulator: &mut f32,
    painter: &egui::Painter,
    positions: &mut HashMap<Entity, egui::Pos2>,
    children_query: &Query<&Children, With<FuelTank>>,
    names: &Query<&Name>,
) {
    let x_step = 120.0;
    let y_step = 60.0;
    let current_pos = egui::pos2(pos.x + (depth as f32 * x_step), pos.y + *y_accumulator);
    positions.insert(entity, current_pos);

    let label = names.get(entity).map(|n| n.as_str()).unwrap_or("Entity");
    let color = if depth == 0 {
        egui::Color32::LIGHT_BLUE
    } else {
        egui::Color32::WHITE
    };

    painter.circle_filled(current_pos, 20.0, egui::Color32::from_black_alpha(150));
    painter.circle_stroke(current_pos, 20.0, egui::Stroke::new(2.0, color));
    painter.text(
        current_pos,
        egui::Align2::CENTER_CENTER,
        format!("{}\n{:?}", label, entity),
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
    );

    if let Ok(children) = children_query.get(entity) {
        let parent_pos = current_pos;
        for child in children.iter() {
            draw_node_recursive(
                child,
                pos,
                depth + 1,
                y_accumulator,
                painter,
                positions,
                children_query,
                names,
            );

            if let Some(&child_pos) = positions.get(&child) {
                painter.line_segment(
                    [
                        parent_pos + egui::vec2(20.0, 0.0),
                        child_pos - egui::vec2(20.0, 0.0),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::GRAY),
                );
            }
            *y_accumulator += y_step;
        }
    }
}

use bevy::ecs::system::ParamSet;

pub fn update_tank(
    mut meshes: ResMut<Assets<Mesh>>,
    mut tank_params: ParamSet<(
        Query<
            (
                &Mesh3d,
                &FuelTank,
                &mut RocketPart,
                Option<&AttachedChildren>,
            ),
            Changed<FuelTank>,
        >,
        Query<&RocketPart>,
    )>,
    attached: Query<&AttachedTo>,
    mut transforms: Query<&mut Transform>,
) {
    for (mesh_handle, tank, mut rocket_part, _) in tank_params.p0().iter_mut() {
        if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
            *mesh = Cylinder::new(tank.radius, tank.height).mesh().build();
        }

        rocket_part.attach_points = vec![
            Vec3::new(0.0, tank.height / 2.0, 0.0),
            Vec3::new(0.0, -tank.height / 2.0, 0.0),
        ];
    }

    let mut updates = Vec::new();
    {
        let p0 = tank_params.p0();
        for (_, _, part, children) in p0.iter() {
            if let Some(attached_children) = children {
                let child_list: Vec<Entity> = attached_children.iter().collect();
                updates.push((part.attach_points.clone(), child_list));
            }
        }
    }

    for (parent_points, children_entities) in updates {
        for child in children_entities {
            let Ok(attached_data) = attached.get(child) else {
                continue;
            };

            let parent_pos = parent_points[attached_data.parent_point];

            let final_pos = if let Ok(child_part) = tank_params.p1().get(child) {
                parent_pos - child_part.attach_points[attached_data.child_point]
            } else {
                parent_pos
            };

            if let Ok(mut transform) = transforms.get_mut(child) {
                transform.translation = final_pos;
            }
        }
    }
}
