use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

const WALK_SPEED: f32 = 7.0;
const MOUSE_SENSITIVITY: f32 = 0.0025;
const INTERACT_DISTANCE: f32 = 4.0;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct ResourceNode {
    kind: ResourceKind,
    amount: u32,
}

#[derive(Clone, Copy)]
enum ResourceKind { Wood, Stone }

#[derive(Resource, Default)]
struct Inventory {
    wood: u32,
    stone: u32,
}

#[derive(Component)]
struct Hud;

#[derive(Resource)]
struct LookAngles { yaw: f32, pitch: f32 }

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.48, 0.68, 0.82)))
        .init_resource::<Inventory>()
        .insert_resource(LookAngles { yaw: 0.0, pitch: 0.0 })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Shoreline — Rust survival prototype".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            capture_cursor,
            player_look,
            player_move,
            gather_resources,
            update_hud,
        ))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(220.0, 220.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.20, 0.36, 0.22))),
        Transform::default(),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(600.0, 600.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.06, 0.30, 0.48))),
        Transform::from_xyz(0.0, -0.18, 0.0),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-30.0, 50.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.7, 12.0),
        Player,
    ));

    for (x, z, scale) in [
        (-8.0, -4.0, 1.3), (5.0, -8.0, 1.0), (12.0, 3.0, 1.5),
        (-13.0, 8.0, 1.1), (2.0, 11.0, 0.9),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.4 * scale, 1.0 * scale, 1.2 * scale))),
            MeshMaterial3d(materials.add(Color::srgb(0.38, 0.39, 0.40))),
            Transform::from_xyz(x, 0.55 * scale, z)
                .with_rotation(Quat::from_rotation_y(x * 0.13)),
            ResourceNode { kind: ResourceKind::Stone, amount: 5 },
        ));
    }

    for (x, z) in [
        (-16.0, -12.0), (-11.0, -16.0), (9.0, -15.0), (17.0, -9.0),
        (15.0, 13.0), (-17.0, 15.0), (0.0, -20.0), (21.0, 4.0),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.35, 2.5))),
            MeshMaterial3d(materials.add(Color::srgb(0.30, 0.17, 0.08))),
            Transform::from_xyz(x, 1.25, z),
            ResourceNode { kind: ResourceKind::Wood, amount: 8 },
        ));
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.6).mesh().ico(3).unwrap())),
            MeshMaterial3d(materials.add(Color::srgb(0.08, 0.28, 0.10))),
            Transform::from_xyz(x, 3.0, z),
        ));
    }

    commands.spawn((
        Text::new("SHORELINE | Выживание\nWASD — движение, мышь — обзор\nE — добыть ресурс рядом\nENTER — захватить мышь, ESC — освободить"),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Px(16.0),
            ..default()
        },
        Hud,
    ));

    commands.spawn((
        Text::new("+"),
        TextFont { font_size: 30.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(48.0),
            ..default()
        },
    ));
}

fn capture_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = windows.get_single_mut() {
        if keys.just_pressed(KeyCode::Escape) {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
        }
        if keys.just_pressed(KeyCode::Enter) {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        }
    }
}

fn player_look(
    mut motion: EventReader<MouseMotion>,
    mut player: Query<&mut Transform, With<Player>>,
    mut angles: ResMut<LookAngles>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let locked = windows.get_single()
        .map(|w| w.cursor_options.grab_mode == CursorGrabMode::Locked)
        .unwrap_or(false);
    if !locked { motion.clear(); return; }

    let delta: Vec2 = motion.read().map(|m| m.delta).sum();
    angles.yaw -= delta.x * MOUSE_SENSITIVITY;
    angles.pitch = (angles.pitch - delta.y * MOUSE_SENSITIVITY).clamp(-1.45, 1.45);

    if let Ok(mut transform) = player.get_single_mut() {
        transform.rotation =
            Quat::from_rotation_y(angles.yaw) * Quat::from_rotation_x(angles.pitch);
    }
}

fn player_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player: Query<&mut Transform, With<Player>>,
    angles: Res<LookAngles>,
) {
    let Ok(mut transform) = player.get_single_mut() else { return; };
    let mut direction = Vec3::ZERO;
    let forward = Vec3::new(-angles.yaw.sin(), 0.0, -angles.yaw.cos());
    let right = Vec3::new(angles.yaw.cos(), 0.0, -angles.yaw.sin());

    if keys.pressed(KeyCode::KeyW) { direction += forward; }
    if keys.pressed(KeyCode::KeyS) { direction -= forward; }
    if keys.pressed(KeyCode::KeyD) { direction += right; }
    if keys.pressed(KeyCode::KeyA) { direction -= right; }

    if direction.length_squared() > 0.0 {
        transform.translation += direction.normalize() * WALK_SPEED * time.delta_secs();
    }
    transform.translation.x = transform.translation.x.clamp(-95.0, 95.0);
    transform.translation.z = transform.translation.z.clamp(-95.0, 95.0);
    transform.translation.y = 1.7;
}

fn gather_resources(
    keys: Res<ButtonInput<KeyCode>>,
    player: Query<&Transform, With<Player>>,
    mut nodes: Query<(Entity, &Transform, &mut ResourceNode)>,
    mut inventory: ResMut<Inventory>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::KeyE) { return; }
    let Ok(player) = player.get_single() else { return; };

    let nearest = nodes.iter_mut()
        .filter_map(|(entity, transform, node)| {
            let distance = player.translation.distance(transform.translation);
            (distance <= INTERACT_DISTANCE && node.amount > 0).then_some((entity, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1));

    if let Some((entity, _)) = nearest {
        if let Ok((_, _, mut node)) = nodes.get_mut(entity) {
            match node.kind {
                ResourceKind::Wood => inventory.wood += 1,
                ResourceKind::Stone => inventory.stone += 1,
            }
            node.amount -= 1;
            if node.amount == 0 { commands.entity(entity).despawn(); }
        }
    }
}

fn update_hud(
    inventory: Res<Inventory>,
    mut hud: Query<&mut Text, With<Hud>>,
) {
    if !inventory.is_changed() { return; }
    if let Ok(mut text) = hud.get_single_mut() {
        text.0 = format!(
            "SHORELINE | Выживание\nWASD — движение, мышь — обзор\nE — добыть ресурс рядом\nENTER — захватить мышь, ESC — освободить\n\nИнвентарь: дерево {} | камень {}",
            inventory.wood, inventory.stone
        );
    }
}
