use hearth_guest::{log, ProcessLogLevel};

#[no_mangle]
pub extern "C" fn run() {
    panic!("panic handler works!");
}
