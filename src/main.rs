use bevy::{asset::RenderAssetUsages, color::palettes::tailwind::*, prelude::*, render::mesh::{Indices, PrimitiveTopology}};
use csgrs::csg::CSG;
use transform_gizmo_bevy::{prelude::*, GizmoHotkeys};


const CSG_OFFSET_VALUE: f32 = 0.0;
const CSG_COMBINED_OFFSET_VALUE: f32 = 8.0;
const SHAPES_X_EXTENT: f32 = 7.0;

const CUBE_SIZE: f64 = 2.0;
const SPHERE_RADIUS: f64 = 1.0;

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
struct SphereCSG(CSG);

#[derive(Resource)]
struct CubeCSG(CSG);

#[derive(Resource)]
struct CubeLastPos(Transform);

#[derive(Component)]
struct CubeMarker;

#[derive(Component)]
struct CSGCombined;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
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
            ..default()
        })
        .insert_resource(SphereCSG(CSG::sphere(SPHERE_RADIUS, 20, 12, None)))
        .insert_resource(CubeCSG(CSG::cube(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE, None).center()))
        .insert_resource(CubeLastPos(Transform::default()))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, respawn_combined_csg_when_cube_moved)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sphere_csg: Res<SphereCSG>,
    cube_csg: Res<CubeCSG>,
) {
    let white_matl = materials.add(Color::WHITE);
    let ground_matl = materials.add(Color::Srgba(GRAY_300));

    // Spawn CSG Shapes
    let cube_mesh_handle = meshes.add(csg_to_mesh(&cube_csg.0));
    let sphere_mesh_handle = meshes.add(csg_to_mesh(&sphere_csg.0));

    // cube
    commands.spawn((
        Mesh3d(cube_mesh_handle),
        MeshMaterial3d(white_matl.clone()),
        Transform::from_xyz(
            -3.0,
            2.0 + CSG_OFFSET_VALUE,
            0.0,
        ),
        CubeMarker,
        CSGShape { csg: cube_csg.0.clone() },
        GizmoTarget::default(),
    ));

    // sphere
    commands.spawn((
        Mesh3d(sphere_mesh_handle),
        MeshMaterial3d(white_matl.clone()),
        Transform::from_xyz(
            0.0,
            2.0 + CSG_OFFSET_VALUE,
            0.0,
        ),
        CSGShape { csg: sphere_csg.0.clone() },
        GizmoTarget::default(),
    ));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(8))),
        MeshMaterial3d(ground_matl.clone()),
        PickingBehavior::IGNORE, 
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(7.0, 13.0, 30.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        GizmoCamera,
    ));
}

fn respawn_combined_csg_when_cube_moved(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sphere_csg: Res<SphereCSG>,
    cube_csg: Res<CubeCSG>,
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
    let white_matl = materials.add(Color::WHITE); // todo put in res

    let transformed_cube = &cube_csg.0.translate(translation.x as f64, translation.y as f64, translation.z as f64);
    // this is not right way to do it. there has to be a way to set the transform of the shape while creating it itself.
    let transformed_sphere = &sphere_csg.0.translate(0f64, 2f64, 0f64); 

    let new_combined_csg = transformed_cube.difference(transformed_sphere);

    let new_mesh = csg_to_mesh(&new_combined_csg);
    let combined_mesh_handle = meshes.add(new_mesh);

    commands.spawn((
        Mesh3d(combined_mesh_handle),
        MeshMaterial3d(white_matl.clone()),
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

        debug_assert!(poly.vertices.len() == 3);
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
