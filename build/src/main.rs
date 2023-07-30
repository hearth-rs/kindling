use std::{
    path::Path,
    process::{Command, Stdio},
};

use cargo_metadata::Package;

fn main() {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("failed to get cargo metadata");

    let target_path = metadata.target_directory.as_std_path().to_owned();
    let root_path = target_path.join("kindling-root");

    let is_clean = touch_dir(&root_path);

    build_wasm("init", &root_path.join("init.wasm"), is_clean);

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

        build_service(&root_path, &package);
    }
}

/// Returns true if the directory is freshly created.
fn touch_dir(path: &Path) -> bool {
    eprintln!("touching directory {:?}", path);
    // TODO ignore already exists error, catch everything else
    let _ = std::fs::create_dir_all(path);
    true
}

fn get_cargo() -> String {
    std::env::var("CARGO").expect("CARGO env var isn't set")
}

fn build_service(root_path: &Path, package: &Package) {
    let service_path = root_path.join("init").join(&package.name);
    let is_clean = touch_dir(&service_path);
    let module_path = service_path.join("service.wasm");
    build_wasm(&package.name, &module_path, is_clean);
}

fn build_wasm(package: &str, path: &Path, force_copy: bool) {
    let mut command = Command::new(get_cargo());
    command
        .stdout(Stdio::piped())
        .arg("build")
        .arg("--message-format=json-render-diagnostics")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--package")
        .arg(&package);

    eprintln!("executing command: {:?}", command);

    let mut child = command.spawn().expect("failed to run cargo command");

    let reader = std::io::BufReader::new(child.stdout.take().unwrap());
    for message in cargo_metadata::Message::parse_stream(reader) {
        use cargo_metadata::Message;
        match message.unwrap() {
            Message::CompilerArtifact(artifact) => {
                for file in artifact.filenames {
                    if file.as_str().ends_with(".wasm") {
                        if artifact.fresh && !force_copy {
                            continue;
                        }

                        eprintln!("copying {:?} to {:?}", file, path);
                        std::fs::copy(file, path).unwrap();
                    }
                }
            }
            _ => {}
        }
    }

    child.wait().unwrap();
}
