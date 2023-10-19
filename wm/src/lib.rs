use std::{collections::HashSet, f32::consts::PI};

use glam::{vec3, Mat3};
use hearth_guest::{
    debug_draw::{DebugDrawMesh, DebugDrawUpdate, DebugDrawVertex},
    terminal::{FactoryRequest, TerminalState, TerminalUpdate},
    Process, Signal, SELF,
};
use obj::IndexTuple;

#[no_mangle]
pub extern "C" fn run() {
    let term_factory = Process::get_service("hearth.terminal.TerminalFactory").unwrap();

    spawn_terminal(&term_factory, -1, 1, "pipes");
    spawn_terminal(&term_factory, 0, 0, "unimatrix -l aAcCk -s 96");
    spawn_terminal(&term_factory, 1, -1, "hollywood");
    spawn_terminal(&term_factory, 1, 1, "macchina -t Lithium");

    let dd_factory = Process::get_service("hearth.DebugDrawFactory").unwrap();

    spawn_grid(&dd_factory);
    spawn_room(&dd_factory);
}

fn recv_process() -> Process {
    let signal = Signal::recv();
    let Signal::Message(mut msg) = signal else {
        panic!("received a non-message");
    };

    msg.caps.remove(0)
}

fn spawn_terminal(factory: &Process, x: i32, y: i32, command: &str) {
    factory.send_json(
        &FactoryRequest::CreateTerminal(TerminalState {
            position: (x as f32 * 1.2, y as f32 * 1.2, 0.0).into(),
            orientation: Default::default(),
            half_size: (1.0, 1.0).into(),
            opacity: 1.0,
            padding: Default::default(),
            units_per_em: 0.04,
        }),
        &[&SELF],
    );

    let term = recv_process();
    term.send_json(&TerminalUpdate::Input(format!("{}\n", command)), &[&SELF]);
}

fn spawn_grid(dd_factory: &Process) {
    let mut vertices = Vec::new();

    let size = 100;
    let scale = 1.0;
    let color = [0, 255, 0];

    for x in -size..size {
        vertices.push(DebugDrawVertex {
            position: vec3(x as f32, 0.0, -size as f32) * scale,
            color,
        });

        vertices.push(DebugDrawVertex {
            position: vec3(x as f32, 0.0, size as f32) * scale,
            color,
        });
    }

    for y in -size..size {
        vertices.push(DebugDrawVertex {
            position: vec3(-size as f32, 0.0, y as f32) * scale,
            color,
        });

        vertices.push(DebugDrawVertex {
            position: vec3(size as f32, 0.0, y as f32) * scale,
            color,
        });
    }

    dd_factory.send_json(&(), &[&SELF]);
    let dd = recv_process();

    dd.send_json::<Process>(
        &DebugDrawUpdate::Contents(DebugDrawMesh {
            indices: (0..(vertices.len() as u32)).collect(),
            vertices,
        }),
        &[&SELF],
    );
}

fn spawn_room(dd_factory: &Process) {
    let obj = include_bytes!("viking_room.obj");
    let model = obj::ObjData::load_buf(obj.as_slice()).unwrap();

    let color = [255, 0, 255];
    let mesh = &model.objects[0].groups[0];
    let rotate = Mat3::from_rotation_y(PI / -2.0) * Mat3::from_rotation_x(PI / -2.0) * 3.0;

    let vertices = model
        .position
        .iter()
        .map(|v| DebugDrawVertex {
            position: rotate * vec3(v[0], v[1], v[2]),
            color,
        })
        .collect();

    let mut edges = HashSet::new();
    let mut indices = Vec::new();

    for face in mesh.polys.iter() {
        let mut make_edge = |v0: IndexTuple, v1: IndexTuple| {
            let v0 = v0.0 as u32;
            let v1 = v1.0 as u32;

            let (v0, v1) = if v0 < v1 { (v0, v1) } else { (v1, v0) };

            if edges.insert((v0, v1)) {
                indices.push(v0);
                indices.push(v1);
            }
        };

        let face = &face.0;
        make_edge(face[0], face[1]);
        make_edge(face[0], face[2]);
        make_edge(face[1], face[2]);
    }

    dd_factory.send_json(&(), &[&SELF]);
    let dd = recv_process();

    dd.send_json(
        &DebugDrawUpdate::Contents(DebugDrawMesh { vertices, indices }),
        &[&SELF],
    );
}
