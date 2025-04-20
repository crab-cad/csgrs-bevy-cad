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


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
        .add_event::<TransformCSGShapesEvent>()
        .add_systems(Startup, setup_scene)
        .add_systems(Update, respawn_combined_csg_when_cube_moved)
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
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let white_matl = materials.add(Color::WHITE);
    let ground_matl = materials.add(Color::Srgba(GRAY_300));

    // Spawn CSG Shapes
    let cube_csg = CSG::cube(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE, None).center();
    let sphere_csg = CSG::sphere(SPHERE_RADIUS, 16, 8, None);

    let cube_tesselation = cube_csg.tessellate();
    let sphere_tesselation = sphere_csg.tessellate();

    let cube_mesh_handle = meshes.add(csg_to_mesh(&cube_tesselation));
    let sphere_mesh_handle = meshes.add(csg_to_mesh(&sphere_tesselation));

    commands.spawn((
        Mesh3d(cube_mesh_handle),
        MeshMaterial3d(white_matl.clone()),
        Transform::from_xyz(
            -3.0,
            2.0 + CSG_OFFSET_VALUE,
            0.0,
        ),
        Name::new("cube".to_string()),
        CSGShape { csg: cube_csg },
        GizmoTarget::default(),
    ));

    commands.spawn((
        Mesh3d(sphere_mesh_handle),
        MeshMaterial3d(white_matl.clone()),
        Transform::from_xyz(
            0.0,
            2.0 + CSG_OFFSET_VALUE,
            0.0,
        ),
        Name::new("sphere".to_string()),
        CSGShape { csg: sphere_csg },
        // GizmoTarget::default(),
    ));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
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
    mut commands: Commands,
    csg_shape_query: Query<(Entity, &Name, &mut Transform, &mut CSGShape, &Mesh3d)>,
) {
    for (entity, name, transform, mut _csg_shape, _mesh_handle) in csg_shape_query.iter() {
        if name.as_str() == "CSG Combined" {
            commands.entity(entity).despawn();
        }

        let cube_csg = CSG::cube(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE, None).center();
        let sphere_csg = CSG::sphere(SPHERE_RADIUS, 16, 8, None);

        if name.as_str() == "cube" {
            let translation = transform.translation;
            let white_matl = materials.add(Color::WHITE);

            let transformed_cube = cube_csg.translate(translation.x as f64, translation.y as f64, translation.z as f64);
            //this is not right way to do it. there has to be a way to set the transform of the shape while creating it itself.
            let transformed_sphere = sphere_csg.translate(0 as f64, 2.0 as f64, 0 as f64); 

            let new_combined_csg = transformed_cube.difference(&transformed_sphere);

            let new_tessellation = new_combined_csg.tessellate();
            let new_mesh = csg_to_mesh(&new_tessellation);
            let combined_mesh_handle = meshes.add(new_mesh);

            commands.spawn((
                Mesh3d(combined_mesh_handle),
                MeshMaterial3d(white_matl.clone()),
                Transform::from_xyz(0.0, CSG_COMBINED_OFFSET_VALUE, 0.0),
                Name::new("CSG Combined".to_string()),
                CSGShape { csg: new_combined_csg },
            ));
        }
    }
}

fn csg_to_mesh(csg: &CSG<()>) -> Mesh {
    let polygons = &csg.polygons;

    // Prepare buffers
    let mut positions_32 = Vec::new();
    let mut normals_32   = Vec::new();
    let mut indices      = Vec::new();

    let mut index_start = 0u32;

    // Each polygon is assumed to have exactly 3 vertices after tessellation.
    // If not, be sure to handle polygons with more than 3 vertices, or call `csg.tessellate()`.
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