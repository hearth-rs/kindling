use std::{collections::HashSet, f32::consts::PI, sync::Mutex};

use glam::{vec2, vec3, Mat3, Vec2, Vec3};
use hearth_guest::{
    debug_draw::{DebugDrawMesh, DebugDrawUpdate, DebugDrawVertex},
    log,
    terminal::{FactoryRequest, TerminalState, TerminalUpdate},
    Color, Process, Signal, SELF,
};
use obj::IndexTuple;

static LINKED: Mutex<Vec<Process>> = Mutex::new(Vec::new());

fn link(process: Process) {
    LINKED.lock().unwrap().push(process);
}

fn unlink() {
    for child in LINKED.lock().unwrap().drain(..) {
        log(hearth_guest::ProcessLogLevel::Info, "wm", "killing child");
        child.kill();
    }
}

fn layout_split(vertical: bool, count: usize, available: Vec2) -> (Vec2, Vec<Vec2>) {
    let size = if vertical {
        vec2(available.x, available.y / count as f32)
    } else {
        vec2(available.x / count as f32, available.y)
    };

    let offset = if vertical {
        vec2(0.0, size.y)
    } else {
        vec2(size.x, 0.0)
    };

    let half_size = size / 2.0;
    let mut tiles = Vec::new();
    let mut cursor = half_size - available / 2.0;

    for _ in 0..count {
        tiles.push(cursor);
        cursor += offset;
    }

    (half_size - 0.025, tiles)
}

#[no_mangle]
pub extern "C" fn run() {
    spawn_surface();

    let dd_factory = Process::get_service("hearth.DebugDrawFactory").unwrap();
    spawn_grids(&dd_factory);
    spawn_room(&dd_factory);

    Signal::recv();

    unlink();
}

fn recv_process() -> Process {
    let signal = Signal::recv();
    let Signal::Message(mut msg) = signal else {
        panic!("received a non-message");
    };

    let process = msg.caps.remove(0);
    link(process.clone());
    process
}

fn spawn_surface() {
    let term_factory = Process::get_service("hearth.terminal.TerminalFactory").unwrap();

    let programs = &[
        "pipes",
        "unimatrix -l aAcCk -s 96",
        "hollywood",
        "macchina -t Lithium",
    ];

    let mut program_index = 0;

    let mut get_program = || {
        program_index += 1;

        if program_index >= programs.len() {
            program_index = 0;
        }

        programs[program_index]
    };

    let surface_size = vec2(6.0, 4.0);

    let mut spawn_tiles = |size, tiles: Vec<Vec2>, offset: Vec2| {
        for tile in tiles {
            spawn_terminal(&term_factory, tile + offset, size, get_program());
        }
    };

    let split = true;
    let (hori_size, hori_tiles) = layout_split(split, 2, surface_size);
    let (left_size, left_tiles) = layout_split(!split, 5, hori_size * 2.0);
    let (right_size, right_tiles) = layout_split(!split, 2, hori_size * 2.0);

    spawn_tiles(left_size, left_tiles, hori_tiles[0]);
    spawn_tiles(right_size, right_tiles, hori_tiles[1]);
}

fn spawn_terminal(factory: &Process, position: Vec2, half_size: Vec2, command: &str) {
    let state = TerminalState {
        position: position.extend(0.0),
        orientation: Default::default(),
        half_size,
        opacity: 1.0,
        padding: Vec2::splat(0.1),
        units_per_em: 0.05,
    };

    factory.send_json(&FactoryRequest::CreateTerminal(state.clone()), &[&SELF]);

    let term = recv_process();
    term.send_json(&TerminalUpdate::Input(format!("{}\n", command)), &[&SELF]);
}

fn spawn_grids(dd_factory: &Process) {
    let size = 100;

    spawn_grid(
        dd_factory,
        size,
        Color::from_rgb(0, 32, 0),
        |x: i32, y: i32| vec3(x as f32, -8.0, y as f32),
    );

    spawn_grid(
        dd_factory,
        size,
        Color::from_rgb(32, 0, 32),
        |x: i32, y: i32| vec3(x as f32, y as f32, -0.01) * 0.25,
    );
}

fn spawn_grid(
    dd_factory: &Process,
    size: i32,
    color: Color,
    grid_to_pos: impl Fn(i32, i32) -> Vec3,
) -> Process {
    let mut vertices = Vec::new();

    for x in -size..=size {
        vertices.push(DebugDrawVertex {
            position: grid_to_pos(x, -size),
            color,
        });

        vertices.push(DebugDrawVertex {
            position: grid_to_pos(x, size),
            color,
        });
    }

    for y in -size..=size {
        vertices.push(DebugDrawVertex {
            position: grid_to_pos(-size, y),
            color,
        });

        vertices.push(DebugDrawVertex {
            position: grid_to_pos(size, y),
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

    dd
}

fn spawn_room(dd_factory: &Process) {
    let obj = include_bytes!("viking_room.obj");
    let model = obj::ObjData::load_buf(obj.as_slice()).unwrap();

    let color = Color::from_rgb(0, 255, 0);
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
