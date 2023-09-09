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
}

fn spawn_terminal(factory: &Process, x: i32, y: i32, command: &str) {
    factory.send_json(
        &FactoryRequest::CreateTerminal(TerminalState {
            position: (x as f32 * 1.2, y as f32 * 1.2, 0.0).into(),
            orientation: Default::default(),
            half_size: (1.0, 1.0).into(),
            opacity: 1.0,
            padding: Default::default(),
        }),
        &[&SELF],
    );

    let signal = Signal::recv();
    let Signal::Message(mut msg) = signal else {
        panic!("received a non-message");
    };

    let term = msg.caps.remove(0);
    term.send_json(&TerminalUpdate::Input(format!("{}\n", command)), &[&SELF]);
}
