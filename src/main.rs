use bevy::{asset::RenderAssetUsages, prelude::*, render::mesh::{Indices, PrimitiveTopology}};
use csgrs::csg::CSG;
use transform_gizmo_bevy::{prelude::*, GizmoHotkeys};


const CSG_OFFSET_VALUE: f32 = 0.0;
const CSG_COMBINED_OFFSET_VALUE: f32 = 8.0;
const SHAPES_X_EXTENT: f32 = 7.0;

const CUBE_SIZE: f64 = 4.0;
const SPHERE_RADIUS: f64 = 2.0;

#[derive(Component)]
struct CSGShape {
    csg: CSG<()>
}

#[derive(Event)]
pub struct TransformCSGShapesEvent {
    pub entity: Entity,
    pub name: String,
    pub transform: Transform,
}

#[derive(Resource)]
struct SphereCSG(CSG<()>);

#[derive(Resource)]
struct CubeCSG(CSG<()>);

#[derive(Resource)]
struct CubeLastPos(Transform);

#[derive(Resource)]
struct CombinedCSGMaterial(Handle<StandardMaterial>);

#[derive(Component)]
struct CubeMarker;

#[derive(Component)]
struct CSGCombined;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
        .add_plugins(bevy_editor_cam::DefaultEditorCamPlugins)
        .add_event::<TransformCSGShapesEvent>()
        .insert_resource(GizmoOptions {
            hotkeys: Some(GizmoHotkeys::default()),
            group_targets: false,
            gizmo_modes: {
                let mut modes = EnumSet::empty();
                modes.insert(GizmoMode::TranslateX);
                modes.insert(GizmoMode::TranslateY);
                modes.insert(GizmoMode::TranslateZ);
                modes.insert(GizmoMode::TranslateView);
                modes.insert(GizmoMode::ScaleX);
                modes.insert(GizmoMode::ScaleY);
                modes.insert(GizmoMode::ScaleZ);
                modes.insert(GizmoMode::RotateX);
                modes.insert(GizmoMode::RotateY);
                modes.insert(GizmoMode::RotateZ);
                modes
            },
            visuals: GizmoVisuals { inactive_alpha: 0.1, ..default() },
            ..default()
        })
        .insert_resource(SphereCSG(CSG::sphere(SPHERE_RADIUS, 20, 12, None)))
        .insert_resource(CubeCSG(CSG::cube(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE, None).center()))
        .insert_resource(CubeLastPos(Transform::default()))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, respawn_combined_csg_when_cube_moved)
        .add_systems(Update, toggle_bottom_light)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sphere_csg: Res<SphereCSG>,
    cube_csg: Res<CubeCSG>,
) {
    let shape_mat = materials.add(Color::WHITE);
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::Srgba(bevy::color::palettes::tailwind::GRAY_300),
        perceptual_roughness: 0.6,
        thickness: 1.0,
        ior: 1.6,
        reflectance: 0.55,
        diffuse_transmission: 0.25,
        ..default()
    });
    commands.insert_resource(
        CombinedCSGMaterial(materials.add(const { Color::srgb(0.0, 0.0, 0.9) }))
    );

    // Spawn CSG Shapes
    let cube_mesh_handle = meshes.add(csg_to_mesh(&cube_csg.0));
    let sphere_mesh_handle = meshes.add(csg_to_mesh(&sphere_csg.0));

    // cube
    commands.spawn((
        Mesh3d(cube_mesh_handle),
        MeshMaterial3d(shape_mat.clone()),
        Transform::from_xyz(
            -6.0,
            4.0 + CSG_OFFSET_VALUE,
            0.0,
        ),
        CubeMarker,
        CSGShape { csg: cube_csg.0.clone() },
        GizmoTarget::default(),
    ));

    // sphere
    commands.spawn((
        Mesh3d(sphere_mesh_handle),
        MeshMaterial3d(shape_mat.clone()),
        Transform::from_xyz(
            0.0,
            4.0 + CSG_OFFSET_VALUE,
            0.0,
        ),
        CSGShape { csg: sphere_csg.0.clone() },
        GizmoTarget::default(),
    ));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(128.0, 128.0).subdivisions(8))),
        MeshMaterial3d(ground_mat),
        PickingBehavior::IGNORE, 
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 30_000_000.,
            range: 110.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(14.0, 32.0, 14.0),
    ));
    commands.spawn((
        PointLight {
            shadows_enabled: false,
            intensity: 25_000_000.,
            range: 100.0,
            ..default()
        },
        Transform::from_xyz(0.0, -30.0, 0.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(12.0, 22.0, 45.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        GizmoCamera,
        bevy_editor_cam::controller::component::EditorCam {
            zoom_limits: bevy_editor_cam::prelude::zoom::ZoomLimits { min_size_per_pixel: 1e-6, max_size_per_pixel: 0.5, zoom_through_objects: false },
            enabled_motion: bevy_editor_cam::prelude::EnabledMotion { pan: true, orbit: false, zoom: true },
            sensitivity: bevy_editor_cam::prelude::Sensitivity { orbit: Vec2::ZERO, zoom: 2.0 },
            ..default()
        },
    ));
}

fn toggle_bottom_light(light_query: Query<(Entity, &PointLight, &Transform)>, keyboard_events: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keyboard_events.just_pressed(KeyCode::Space) {
        // toggle any lights with negative heights
        // sorry just don't even read this please
        if !(light_query.iter().any(|(entity, _, transform)| {
            if transform.translation.y < 0.0 {
                commands.entity(entity).despawn();
                true
            } else { false }
        })) {
            commands.spawn((
                PointLight {
                    shadows_enabled: false,
                    intensity: 25_000_000.,
                    range: 100.0,
                    ..default()
                },
                Transform::from_xyz(0.0, -30.0, 0.0),
            ));
        }
    }
}

fn respawn_combined_csg_when_cube_moved(
    mut meshes: ResMut<Assets<Mesh>>,
    sphere_csg: Res<SphereCSG>,
    cube_csg: Res<CubeCSG>,
    combined_csg_material: Res<CombinedCSGMaterial>,
    mut cube_last_pos: ResMut<CubeLastPos>,
    mut commands: Commands,
    csg_combined_query: Option<Single<(Entity, &mut Transform, &mut CSGShape, &Mesh3d), (With<CSGCombined>, Without<CubeMarker>)>>,
    csg_cube_entity: Single<(Entity, &mut Transform, &mut CSGShape, &Mesh3d), With<CubeMarker>>,
) {
    // skip if cube did not move
    if *csg_cube_entity.1 == cube_last_pos.0 {
        return;
    }

    if let Some(csg_combined_entity) = csg_combined_query {
        commands.entity(csg_combined_entity.0).despawn();
    }

    let transform = *csg_cube_entity.1;

    cube_last_pos.0 = transform;
    let translation = transform.translation;

    let transformed_cube = &cube_csg.0.translate(translation.x as f64, translation.y as f64, translation.z as f64);
    // this is not right way to do it. there has to be a way to set the transform of the shape while creating it itself.
    let transformed_sphere = &sphere_csg.0.translate(2.0, 5.0, 0.0); 

    let new_combined_csg = transformed_cube.difference(transformed_sphere);

    let new_mesh = csg_to_mesh(&new_combined_csg);
    let combined_mesh_handle = meshes.add(new_mesh);

    commands.spawn((
        Mesh3d(combined_mesh_handle),
        MeshMaterial3d(combined_csg_material.0.clone_weak()),
        transform,
        // Transform::from_xyz(0.0, CSG_COMBINED_OFFSET_VALUE, 0.0),
        CSGCombined,
        CSGShape { csg: new_combined_csg },
    ));
}

/// This tessellates the `CSG` then convert it to a [`Mesh`]
fn csg_to_mesh(csg: &CSG<()>) -> Mesh {
    let polygons = &csg.tessellate().unwrap().polygons;

    // Prepare buffers
    let vertices_len = polygons.iter().map(|poly| poly.vertices.len()).sum();
    let mut positions_32 = Vec::with_capacity(vertices_len);
    let mut normals_32   = Vec::with_capacity(vertices_len);
    let mut indices = Vec::with_capacity(vertices_len);

    let mut index_start = 0u32;

    // Each polygon is assumed to have exactly 3 vertices after tessellation.
    for poly in polygons {
        // skip any degenerate polygons
        if poly.vertices.len() != 3 {
            continue;
        }

        // push 3 positions/normals
        for v in &poly.vertices {
            positions_32.push([v.pos.x as f32, v.pos.y as f32, v.pos.z as f32]);
            normals_32.push([v.normal.x as f32, v.normal.y as f32, v.normal.z as f32]);
        }

        // triangle indices
        indices.push(index_start);
        indices.push(index_start + 1);
        indices.push(index_start + 2);
        index_start += 3;
    }

    // Create the mesh with the new 2-argument constructor
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());

    // Insert attributes. Note the `<Vec<[f32; 3]>>` usage.
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions_32);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals_32);

    // Insert triangle indices
    mesh.insert_indices(Indices::U32(indices));

    mesh
}
