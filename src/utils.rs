use bevy::math::Vec3;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::transform::components::Transform;

pub fn combine_meshes<'a>(
    meshes: impl IntoIterator<Item = &'a (Mesh, Transform)>,
    use_normals: bool,
    use_tangents: bool,
    use_uvs: bool,
    use_colors: bool,
) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut tangets: Vec<[f32; 4]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut indices_offset = 0;

    for (mesh, transform) in meshes {
        if let Some(Indices::U32(mesh_indices)) = mesh.indices() {
            let matrix = transform.compute_matrix();

            let positions_len = if let Some(VertexAttributeValues::Float32x3(vert_positions)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            {
                for pos in vert_positions {
                    positions.push(matrix.transform_point3(Vec3::from(*pos)).into());
                }
                vert_positions.len()
            } else {
                0
            };

            if use_uvs {
                if let Some(VertexAttributeValues::Float32x2(vert_uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
                    for uv in vert_uvs {
                        uvs.push(*uv);
                    }
                }
            }

            if use_normals {
                // Comment below taken from mesh_normal_local_to_world() in mesh_functions.wgsl regarding
                // transform normals from local to world coordinates:

                // NOTE: The mikktspace method of normal mapping requires that the world normal is
                // re-normalized in the vertex shader to match the way mikktspace bakes vertex tangents
                // and normal maps so that the exact inverse process is applied when shading. Blender, Unity,
                // Unreal Engine, Godot, and more all use the mikktspace method. Do not change this code
                // unless you really know what you are doing.
                // http://www.mikktspace.com/

                // let inverse_transpose_model = matrix.inverse().transpose();
                // let inverse_transpose_model = Mat3 {
                //     x_axis: inverse_transpose_model.x_axis.xyz(),
                //     y_axis: inverse_transpose_model.y_axis.xyz(),
                //     z_axis: inverse_transpose_model.z_axis.xyz(),
                // };

                if let Some(VertexAttributeValues::Float32x3(vert_normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
                    for norm in vert_normals {
                        normals.push(*norm);
                        //     inverse_transpose_model
                        //         .mul_vec3(Vec3::from(*norm))
                        //         .normalize_or_zero()
                        //         .into(),
                        // );
                    }
                }
            }

            if use_tangents {
                if let Some(VertexAttributeValues::Float32x4(vert_tangets)) = mesh.attribute(Mesh::ATTRIBUTE_TANGENT) {
                    for tan in vert_tangets {
                        tangets.push(*tan);
                    }
                }
            }

            if use_colors {
                if let Some(VertexAttributeValues::Float32x4(vert_colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
                    for color in vert_colors {
                        colors.push(*color);
                    }
                }
            }

            for idx in mesh_indices {
                indices.push(*idx + indices_offset);
            }
            indices_offset += positions_len as u32;
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

    if use_normals {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }

    if use_tangents {
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangets);
    }

    if use_uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }

    if use_colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }

    mesh.insert_indices(Indices::U32(indices));

    mesh
}
