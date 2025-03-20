use std::f32::consts::PI;
use csgrs::csg::CSG;
use bevy_editor_pls::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        mesh::{Indices, PrimitiveTopology},
    },
};

fn main() {
    App::new()
        // Unlike UiPickingPlugin, MeshPickingPlugin is not a default plugin
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            MeshPickingPlugin,
            //#[cfg(not(target_arch = "wasm32"))]
            //WireframePlugin,
        ))
        .add_plugins(EditorPlugin::default())
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                #[cfg(not(target_arch = "wasm32"))]
                toggle_wireframe,
            ),
        )
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
        let debug_material = materials.add(StandardMaterial {
            base_color_texture: Some(images.add(uv_debug_texture())),
            ..default()
        });

        //------------------------------------------------------
        // 1) Build or load your desired CSG shape
        //------------------------------------------------------
        // Example: build a simple cube of size 2.0 x 2.0 x 2.0
        let cube = CSG::cube(2.0, 2.0, 2.0, None).center();
        let sphere = CSG::sphere(1.3, 16, 8, None);
        let csg_shape = cube.difference(&sphere);
    
        // Optionally transform, union, difference, etc. For instance:
        // let sphere = CSG::sphere(1.0, 16, 8, None);
        // let csg_shape = csg_shape.union(&sphere.translate(1.0, 0.0, 0.0));
    
        // 2) Tessellate the CSG to produce only triangles
        let csg_tessellated = csg_shape.tessellate();
    
        // 3) Convert the tessellated CSG into a Bevy Mesh
        let mesh_handle = meshes.add(csg_to_mesh(&csg_tessellated));
    
        // Spawn the CSG mesh
        commands.spawn((
            // For Bevy 0.11+, use `Mesh3d` and `MeshMaterial3d` if you’re using the
            // new “3D mesh” APIs. If you’re still on older Bevy, you can use
            // `PbrBundle { mesh: mesh_handle, material: debug_material, ... }`
            Mesh3d(mesh_handle),
            MeshMaterial3d(debug_material.clone()),
            Transform::from_xyz(0.0, 0.0, 2.0).with_rotation(Quat::from_rotation_x(-PI / 4.)),
        ))
        .observe(on_drag_rotate);

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    #[cfg(not(target_arch = "wasm32"))]
    commands.spawn((
        Text::new("Press space to toggle wireframes"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn on_drag_rotate(drag: Trigger<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
    if let Ok(mut transform) = transforms.get_mut(drag.entity()) {
        transform.rotate_y(drag.delta.x * 0.02);
        transform.rotate_x(drag.delta.y * 0.02);
    }
}

/// A utility function to convert a tessellated CSG into a Bevy `Mesh`.
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

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        wireframe_config.global = !wireframe_config.global;
    }
}
