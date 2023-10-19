use hearth_guest::{
    terminal::{FactoryRequest, TerminalState, TerminalUpdate},
    Process, Signal, SELF,
};

#[no_mangle]
pub extern "C" fn run() {
    let term_factory = Process::get_service("hearth.terminal.TerminalFactory").unwrap();

    spawn_terminal(&term_factory, -1, 1, "pipes");
    spawn_terminal(&term_factory, 0, 0, "unimatrix -l aAcCk -s 96");
    spawn_terminal(&term_factory, 1, -1, "hollywood");
    spawn_terminal(&term_factory, 1, 1, "macchina -t Lithium");

    spawn_grid();
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

fn spawn_grid() {
    use glam::vec3;
    use hearth_guest::debug_draw::*;

    let debug_factory = Process::get_service("hearth.DebugDrawFactory").unwrap();
    debug_factory.send_json(&(), &[&SELF]);
    let dd = recv_process();

    let mut vertices = Vec::new();

    let size = 100;
    let scale = 0.1;
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

    dd.send_json::<Process>(
        &DebugDrawUpdate::Contents(DebugDrawMesh {
            indices: (0..(vertices.len() as u32)).collect(),
            vertices,
        }),
        &[&SELF],
    );
}
