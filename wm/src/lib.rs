use hearth_guest::{
    log,
    terminal::{FactoryRequest, FactoryResponse, TerminalState, TerminalUpdate},
    RequestResponse, REGISTRY,
};

pub type TerminalFactory = RequestResponse<FactoryRequest, FactoryResponse>;

#[no_mangle]
pub extern "C" fn run() {
    let term_factory = REGISTRY
        .get_service("hearth.terminal.TerminalFactory")
        .unwrap();
    let term_factory = TerminalFactory::new(term_factory);

    spawn_terminal(&term_factory, -1, 1, "pipes");
    spawn_terminal(&term_factory, 0, 0, "unimatrix -l aAcCk -s 96");
    spawn_terminal(&term_factory, 1, -1, "hollywood");
    spawn_terminal(&term_factory, 1, 1, "macchina -t Lithium");
    spawn_terminal(&term_factory, -1, -1, "notcurses-demo");
    spawn_terminal(&term_factory, 0, -1, "chocolate-doom Downloads/DOOM.WAD");
}

fn spawn_terminal(factory: &TerminalFactory, x: i32, y: i32, command: &str) {
    let request = FactoryRequest::CreateTerminal(TerminalState {
        position: (x as f32 * 2.1, y as f32 * 2.1, 0.0).into(),
        orientation: Default::default(),
        half_size: (1.0, 1.0).into(),
        opacity: 1.0,
        padding: Default::default(),
        units_per_em: 0.06,
    });

    log(
        hearth_guest::ProcessLogLevel::Info,
        "wm",
        &format!("spawning terminal: {:?}", request),
    );

    let (msg, mut caps) = factory.request(request, &[]);
    msg.unwrap();

    let term = caps.remove(0);
    term.send_json(&TerminalUpdate::Input(format!("{}\n", command)), &[]);
}
