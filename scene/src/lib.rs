use std::f32::consts::PI;

use glam::{uvec2, vec3, Mat4, Vec2, Vec3, Vec4};
use hearth_guest::{renderer::*, Lump, LumpId, RequestResponse, REGISTRY};
use image::GenericImageView;
use serde::Serialize;

pub type Renderer = RequestResponse<RendererRequest, RendererResponse>;

#[no_mangle]
pub extern "C" fn run() {
    let ren = REGISTRY.get_service("hearth.Renderer").unwrap();
    let ren = Renderer::new(ren);

    ren.request(
        RendererRequest::SetSkybox {
            texture: load_skybox(include_bytes!("skybox.png")),
        },
        &[],
    )
    .0
    .unwrap();

    ren.request(
        RendererRequest::AddObject {
            mesh: load_obj(include_bytes!("viking_room.obj")),
            material: load_albedo_material(include_bytes!("viking_room.png")),
            skeleton: None,
            transform: Mat4::from_rotation_y(PI / -2.0)
                * Mat4::from_rotation_x(PI / -2.0)
                * Mat4::from_scale(Vec3::splat(3.0)),
        },
        &[],
    )
    .0
    .unwrap();

    ren.request(
        RendererRequest::AddDirectionalLight {
            initial_state: DirectionalLightState {
                color: Vec3::ONE,
                intensity: 10.0,
                direction: Vec3::new(-2.0, -2.0, -1.0),
                distance: 400.0,
            },
        },
        &[],
    )
    .0
    .unwrap();

    spawn_gltf(
        &ren,
        include_bytes!("korakoe.vrm"),
        Mat4::from_translation(vec3(0.0, 0.0, 1.7)) * Mat4::from_rotation_y(PI),
    );
}

pub fn load_albedo_material(texture: &[u8]) -> LumpId {
    json_lump(&MaterialData {
        albedo: load_texture(texture),
    })
}

pub fn load_obj(model: &[u8]) -> LumpId {
    let model = obj::ObjData::load_buf(model).unwrap();
    let mesh = &model.objects[0].groups[0];

    let mut mesh_data = MeshData {
        positions: Vec::new(),
        normals: Vec::new(),
        tangents: Vec::new(),
        uv0: Vec::new(),
        uv1: Vec::new(),
        colors: Vec::new(),
        joint_indices: Vec::new(),
        joint_weights: Vec::new(),
        indices: Vec::new(),
    };

    for face in mesh.polys.iter() {
        let mut push_vertex = |v: obj::IndexTuple| {
            let position = Vec3::from_slice(&model.position[v.0]);
            let uv0 = Vec2::from_slice(&model.texture[v.1.unwrap()]);
            let normal = Vec3::from_slice(&model.normal[v.2.unwrap()]);

            let texture = Vec2::new(uv0.x, 1.0 - uv0.y);

            let idx = mesh_data.positions.len() as u32;
            mesh_data.positions.push(position);
            mesh_data.uv0.push(texture);
            mesh_data.normals.push(normal);

            idx
        };

        mesh_data.indices.push(push_vertex(face.0[0]));
        mesh_data.indices.push(push_vertex(face.0[1]));
        mesh_data.indices.push(push_vertex(face.0[2]));
    }

    let len = mesh_data.positions.len();
    mesh_data.tangents.resize(len, Default::default());
    mesh_data.uv1.resize(len, Default::default());
    mesh_data.colors.resize(len, Default::default());
    mesh_data.joint_indices.resize(len, Default::default());
    mesh_data.joint_weights.resize(len, Default::default());

    json_lump(&mesh_data)
}

pub fn spawn_gltf(ren: &Renderer, src: &[u8], transform: Mat4) {
    use gltf::*;

    let (document, buffers, images) = import_slice(src).unwrap();

    let images: Vec<_> = images
        .into_iter()
        .map(|image| {
            json_lump(&TextureData {
                label: None,
                data: image.pixels.clone(),
                size: uvec2(image.width, image.height),
            })
        })
        .collect();

    let materials: Vec<_> = document
        .materials()
        .map(|material| {
            let pbr = material.pbr_metallic_roughness();
            let base = pbr.base_color_texture().unwrap();
            let base = base.texture().source();
            let albedo = images[base.index()];
            json_lump(&MaterialData { albedo })
        })
        .collect();

    let mut objects = Vec::new();

    for mesh in document.meshes() {
        for prim in mesh.primitives() {
            let reader = prim.reader(|buffer| Some(&buffers[buffer.index()]));

            let positions: Vec<_> = reader
                .read_positions()
                .expect("glTF primitive has no positions")
                .map(Vec3::from)
                .collect();

            let len = positions.len();

            let mut mesh_data = MeshData {
                positions,
                normals: vec![Default::default(); len],
                tangents: vec![Default::default(); len],
                uv0: vec![Default::default(); len],
                uv1: vec![Default::default(); len],
                colors: vec![Default::default(); len],
                joint_indices: vec![Default::default(); len],
                joint_weights: vec![Default::default(); len],
                indices: Vec::new(),
            };

            if let Some(normals) = reader.read_normals() {
                mesh_data.normals.clear();
                mesh_data.normals.extend(normals.map(Vec3::from));
            }

            if let Some(tangents) = reader.read_tangents() {
                mesh_data.tangents.clear();

                mesh_data
                    .tangents
                    .extend(tangents.map(|t| Vec3::from_slice(&t)));
            }

            if let Some(uv0) = reader.read_tex_coords(0) {
                mesh_data.uv0.clear();
                mesh_data.uv0.extend(uv0.into_f32().map(Vec2::from));
            }

            if let Some(uv1) = reader.read_tex_coords(1) {
                mesh_data.uv1.clear();
                mesh_data.uv1.extend(uv1.into_f32().map(Vec2::from));
            }

            if let Some(colors) = reader.read_colors(0) {
                mesh_data.colors.clear();
                mesh_data.colors.extend(colors.into_rgba_u8());
            }

            if let Some(joints) = reader.read_joints(0) {
                mesh_data.joint_indices.clear();
                mesh_data.joint_indices.extend(joints.into_u16());
            }

            if let Some(weights) = reader.read_weights(0) {
                mesh_data.joint_weights.clear();
                mesh_data
                    .joint_weights
                    .extend(weights.into_f32().map(Vec4::from));
            }

            if let Some(indices) = reader.read_indices() {
                mesh_data.indices.extend(indices.into_u32());
            }

            let mesh = json_lump(&mesh_data);
            let material = materials[prim.material().index().unwrap()];

            objects.push(RendererRequest::AddObject {
                mesh,
                skeleton: None,
                material,
                transform,
            });
        }
    }

    for object in objects {
        ren.request(object, &[]).0.unwrap();
    }
}

pub fn load_skybox(src: &[u8]) -> LumpId {
    let image = image::load_from_memory(src).unwrap();
    let size = image.dimensions().into();

    let data = std::iter::repeat(image.into_rgba8().into_vec())
        .take(6)
        .flatten()
        .collect();

    json_lump(&TextureData {
        label: None,
        data,
        size,
    })
}

pub fn load_texture(src: &[u8]) -> LumpId {
    let image = image::load_from_memory(src).unwrap();
    let size = image.dimensions().into();
    let data = image.into_rgba8().into_vec();

    json_lump(&TextureData {
        label: None,
        data,
        size,
    })
}

pub fn json_lump(data: &impl Serialize) -> LumpId {
    let data = serde_json::to_vec(data).unwrap();
    Lump::load(&data).get_id()
}
