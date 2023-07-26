use hearth_guest::*;

#[no_mangle]
pub extern "C" fn run() {
    hearth_guest::log(hearth_guest::ProcessLogLevel::Info, "init", "Hello world!");

    let child = Process::spawn(child_cb);
    child.send(b"Hello, child!", &[&SELF]);

    let Signal::Message(msg) = Signal::recv() else { panic!("expected msg") };
    log(ProcessLogLevel::Info, "parent", &format!("{:?}", msg));
}

fn child_cb() {
    let Signal::Message(msg) = Signal::recv() else { panic!("expected msg") };
    log(ProcessLogLevel::Info, "child", &format!("{:?}", msg));
    msg.caps[0].send(b"Hello, parent!", &[]);
}
