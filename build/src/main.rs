use std::process::Command;

fn main() {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("failed to get cargo metadata");

    for package_id in metadata.workspace_members.iter() {
        let package = &metadata[package_id];

        let mut is_lib = false;
        for target in package.targets.iter() {
            if target.kind.contains(&"cdylib".to_string()) {
                is_lib = true;
                break;
            }
        }

        if !is_lib {
            eprintln!("{:?} is not a lib; skipping", package.name);
            continue;
        }

        build_service(&package.name);
    }
}

fn get_cargo() -> String {
    std::env::var("CARGO").expect("CARGO env var isn't set")
}

fn build_service(package: &str) {
    let mut command = Command::new(get_cargo());
    command
        .arg("build")
        .arg("--message-format=json-render-diagnostics")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--package")
        .arg(package);

    eprintln!("executing command: {:?}", command);

    let mut child = command.spawn().expect("failed to run cargo command");

    child.wait().unwrap();
}
