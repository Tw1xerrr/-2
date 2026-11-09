use bevy::{
    asset::RenderAssetUsages,
    input::mouse::MouseMotion,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
    window::{CursorGrabMode, PrimaryWindow},
};

const WALK_SPEED: f32 = 7.0;
const MOUSE_SENSITIVITY: f32 = 0.0022;
const INTERACT_DISTANCE: f32 = 4.0;
const ISLAND_RADIUS: f32 = 92.0;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct ResourceNode { kind: ResourceKind, amount: u32 }

#[derive(Clone, Copy)]
enum ResourceKind { Wood, Stone }

#[derive(Resource, Default)]
struct Inventory { wood: u32, stone: u32 }

#[derive(Component)]
struct Hud;

#[derive(Resource)]
struct LookAngles { yaw: f32, pitch: f32 }

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.48, 0.62, 0.72)))
        .insert_resource(AmbientLight {
            color: Color::srgb(0.72, 0.80, 0.92),
            brightness: 180.0,
        })
        .init_resource::<Inventory>()
        .insert_resource(LookAngles { yaw: 0.0, pitch: 0.0 })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "SHORELINE | Survival".into(),
                resolution: (1440.0, 810.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            capture_cursor, player_look, player_move, gather_resources, update_hud,
        ))
        .run();
}

fn terrain_height(x: f32, z: f32) -> f32 {
    let r = (x * x + z * z).sqrt() / ISLAND_RADIUS;
    let falloff = (1.0 - r).max(0.0);
    let hills = falloff.powf(0.62) * 7.8;
    let rolling = (x * 0.075).sin() * (z * 0.061).cos() * 1.15
        + (x * 0.13 + z * 0.08).sin() * 0.48;
    -1.35 + hills + rolling * falloff
}

fn hash01(x: i32, z: i32) -> f32 {
    let n = (x.wrapping_mul(374761393) ^ z.wrapping_mul(668265263))
        .wrapping_mul(1274126177);
    ((n ^ (n >> 16)) as u32) as f32 / u32::MAX as f32
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Physically based materials: rough land, glossy ocean, matte foliage and stone.
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.96,
        ..default()
    });
    let water_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.035, 0.22, 0.30, 0.88),
        perceptual_roughness: 0.18,
        metallic: 0.12,
        reflectance: 0.82,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let bark_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.105, 0.055),
        perceptual_roughness: 0.94,
        ..default()
    });
    let leaf_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.035, 0.16, 0.075),
        perceptual_roughness: 0.92,
        ..default()
    });
    let leaf_light = materials.add(StandardMaterial {
        base_color: Color::srgb(0.075, 0.25, 0.105),
        perceptual_roughness: 0.9,
        ..default()
    });
    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.32, 0.32),
        perceptual_roughness: 0.88,
        ..default()
    });
    let grass_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.13, 0.25, 0.095),
        perceptual_roughness: 1.0,
        ..default()
    });
    let sand_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.60, 0.49, 0.32),
        perceptual_roughness: 1.0,
        ..default()
    });

    // A subdivided island mesh gives the shoreline and hills a real silhouette.
    let terrain = make_terrain_mesh();
    commands.spawn((Mesh3d(meshes.add(terrain)), MeshMaterial3d(ground_mat)));

    // Large ocean plane; the shoreline is shaped into the terrain mesh.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(520.0, 520.0))),
        MeshMaterial3d(water_mat),
        Transform::from_xyz(0.0, -1.18, 0.0),
    ));

    // A pale sandy band follows the coast as a series of natural irregular patches.
    let sand_mesh = meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap());
    for i in 0..170 {
        let angle = i as f32 * 2.399_963;
        let radius = 77.0 + hash01(i, 4) * 8.0;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        let y = terrain_height(x, z) + 0.02;
        let scale = 1.2 + hash01(i, 8) * 2.8;
        commands.spawn((
            Mesh3d(sand_mesh.clone()),
            MeshMaterial3d(sand_mat.clone()),
            Transform::from_xyz(x, y, z)
                .with_scale(Vec3::new(scale * 1.8, 0.035, scale * 1.2))
                .with_rotation(Quat::from_rotation_y(angle)),
        ));
    }

    // Sunlight and a soft fill light create depth and readable shadows.
    commands.spawn((
        DirectionalLight {
            illuminance: 17_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-65.0, 90.0, 45.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Shared meshes keep the scene inexpensive to render.
    let trunk_mesh = meshes.add(Cylinder::new(0.34, 4.2));
    let crown_a = meshes.add(Cone::new(2.35, 4.3));
    let crown_b = meshes.add(Cone::new(1.85, 3.7));
    let crown_c = meshes.add(Cone::new(1.30, 3.0));
    let rock_mesh = meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap());
    let grass_blade = meshes.add(Cuboid::new(0.075, 0.75, 0.10));

    // Forest: clustered trees with layered crowns and varied scale/rotation.
    for gx in -10..=10 {
        for gz in -10..=10 {
            let x = gx as f32 * 8.2 + (hash01(gx, gz) - 0.5) * 4.0;
            let z = gz as f32 * 8.2 + (hash01(gx + 31, gz - 11) - 0.5) * 4.0;
            let r = (x * x + z * z).sqrt();
            if r < 19.0 || r > 72.0 || hash01(gx * 3, gz * 7) < 0.27 { continue; }
            let y = terrain_height(x, z);
            let s = 0.78 + hash01(gx + 9, gz + 4) * 0.72;
            let rot = hash01(gx - 7, gz + 17) * 6.28;
            commands.spawn((
                Mesh3d(trunk_mesh.clone()),
                MeshMaterial3d(bark_mat.clone()),
                Transform::from_xyz(x, y + 2.0 * s, z)
                    .with_scale(Vec3::new(s, s, s))
                    .with_rotation(Quat::from_rotation_y(rot)),
            ));
            commands.spawn((
                Mesh3d(crown_a.clone()),
                MeshMaterial3d(leaf_dark.clone()),
                Transform::from_xyz(x, y + 4.0 * s, z)
                    .with_scale(Vec3::new(s, s, s)),
            ));
            commands.spawn((
                Mesh3d(crown_b.clone()),
                MeshMaterial3d(leaf_light.clone()),
                Transform::from_xyz(x, y + 5.15 * s, z)
                    .with_scale(Vec3::new(s * 0.86, s * 0.9, s * 0.86)),
            ));
            commands.spawn((
                Mesh3d(crown_c.clone()),
                MeshMaterial3d(leaf_dark.clone()),
                Transform::from_xyz(x, y + 6.1 * s, z)
                    .with_scale(Vec3::new(s * 0.72, s * 0.78, s * 0.72)),
            ));
        }
    }

    // Natural rock outcrops and harvestable stone deposits.
    for i in 0..100 {
        let angle = hash01(i, 15) * 6.283;
        let radius = 24.0 + hash01(i, 21) * 57.0;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        let y = terrain_height(x, z);
        let s = 0.7 + hash01(i, 29) * 2.0;
        commands.spawn((
            Mesh3d(rock_mesh.clone()),
            MeshMaterial3d(rock_mat.clone()),
            Transform::from_xyz(x, y + s * 0.45, z)
                .with_scale(Vec3::new(s * 1.35, s * 0.82, s))
                .with_rotation(Quat::from_euler(
                    EulerRot::XYZ,
                    hash01(i, 31) * 0.7,
                    angle,
                    hash01(i, 37) * 0.5,
                )),
        ));
    }

    for (x, z) in [
        (-28.0, -17.0), (22.0, -32.0), (42.0, 13.0),
        (-43.0, 22.0), (8.0, 49.0), (-13.0, -52.0),
    ] {
        let y = terrain_height(x, z);
        commands.spawn((
            Mesh3d(rock_mesh.clone()),
            MeshMaterial3d(rock_mat.clone()),
            Transform::from_xyz(x, y + 1.0, z).with_scale(Vec3::new(2.1, 1.5, 1.8)),
            ResourceNode { kind: ResourceKind::Stone, amount: 12 },
        ));
    }

    // Low grass tufts add detail near the player without thousands of draw calls.
    for i in 0..240 {
        let x = (hash01(i, 101) - 0.5) * 130.0;
        let z = (hash01(i, 103) - 0.5) * 130.0;
        if (x * x + z * z).sqrt() > 76.0 { continue; }
        let y = terrain_height(x, z);
        let rot = hash01(i, 107) * 6.28;
        commands.spawn((
            Mesh3d(grass_blade.clone()),
            MeshMaterial3d(grass_mat.clone()),
            Transform::from_xyz(x, y + 0.35, z)
                .with_rotation(Quat::from_rotation_y(rot))
                .with_scale(Vec3::new(0.8, 0.7 + hash01(i, 109), 0.8)),
        ));
    }

    // Player starts on the high ground, facing toward the forest.
    let start_y = terrain_height(0.0, 12.0) + 1.72;
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, start_y, 12.0),
        Player,
    ));

    commands.spawn((
        Text::new(
            "SHORELINE | SURVIVAL\nWASD — движение   Мышь — обзор\nE — добыть ресурс   ENTER — захватить мышь   ESC — курсор\n\nИнвентарь: дерево 0 | камень 0"
        ),
        TextFont { font_size: 17.0, ..default() },
        TextColor(Color::srgb(0.96, 0.97, 0.94)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(18.0),
            left: Val::Px(20.0),
            ..default()
        },
        Hud,
    ));

    commands.spawn((
        Text::new("+"),
        TextFont { font_size: 28.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(48.0),
            ..default()
        },
    ));
}

fn make_terrain_mesh() -> Mesh {
    let resolution = 150usize;
    let size = 210.0_f32;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((resolution + 1) * (resolution + 1));
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity((resolution + 1) * (resolution + 1));
    let mut indices: Vec<u32> = Vec::with_capacity(resolution * resolution * 6);

    for iz in 0..=resolution {
        for ix in 0..=resolution {
            let x = (ix as f32 / resolution as f32 - 0.5) * size;
            let z = (iz as f32 / resolution as f32 - 0.5) * size;
            let y = terrain_height(x, z);
            positions.push([x, y, z]);

            let slope_noise = (x * 0.17).sin() * (z * 0.13).cos();
            let color = if y < -0.15 {
                [0.40, 0.36, 0.25, 1.0] // wet shoreline sand
            } else if y < 1.1 {
                [0.67, 0.57, 0.39, 1.0] // dry sand
            } else if y > 5.8 || slope_noise > 0.78 {
                [0.34, 0.35, 0.32, 1.0] // exposed rock
            } else {
                [0.20 + slope_noise * 0.025, 0.34, 0.16, 1.0] // coastal grass
            };
            colors.push(color);
        }
    }

    for z in 0..resolution {
        for x in 0..resolution {
            let a = (z * (resolution + 1) + x) as u32;
            let b = a + 1;
            let c = a + (resolution + 1) as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_normals();
    mesh
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
    transform.translation.x = transform.translation.x.clamp(-96.0, 96.0);
    transform.translation.z = transform.translation.z.clamp(-96.0, 96.0);
    transform.translation.y = terrain_height(transform.translation.x, transform.translation.z) + 1.72;
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

fn update_hud(inventory: Res<Inventory>, mut hud: Query<&mut Text, With<Hud>>) {
    if !inventory.is_changed() { return; }
    if let Ok(mut text) = hud.get_single_mut() {
        text.0 = format!(
            "SHORELINE | SURVIVAL\nWASD — движение   Мышь — обзор\nE — добыть ресурс   ENTER — захватить мышь   ESC — курсор\n\nИнвентарь: дерево {} | камень {}",
            inventory.wood, inventory.stone
        );
    }
}
