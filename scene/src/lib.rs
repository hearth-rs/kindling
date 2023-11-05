use std::f32::consts::PI;

use glam::{Mat4, Vec2, Vec3};
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
