use hearth_guest::*;
use serde::Deserialize;

macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        ::hearth_guest::log(
            $level,
            ::core::module_path!(),
            &format!($($arg)*),
        )
    }
}

macro_rules! info {
    ($($arg:tt)*) => {
        log!(::hearth_guest::ProcessLogLevel::Info, $($arg)*);
    };
}

#[no_mangle]
pub extern "C" fn run() {
    hearth_guest::log(hearth_guest::ProcessLogLevel::Info, "init", "Hello world!");

    let fs = Process::get_service("hearth.fs.Filesystem").unwrap();

    for file in list_files(&fs, "") {
        info!("file: {}", file.name);

        let config_path = format!("{}/service.toml", file.name);
        let config_data = read_file(&fs, &config_path);
        let config_str = String::from_utf8(config_data).unwrap();
        let service: ServiceConfig = toml::from_str(&config_str).unwrap();
        info!("service: {:?}", service);
    }
}

fn request_fs(fs: &Process, request: fs::Request) -> fs::Success {
    fs.send_json(&request, &[&SELF]);
    let Signal::Message(msg) = Signal::recv() else { panic!("expected msg") };
    let response: fs::Response = serde_json::from_slice(&msg.data).unwrap();
    response.unwrap()
}

fn get_file(fs: &Process, path: &str) -> LumpId {
    let success = request_fs(
        fs,
        fs::Request {
            target: path.to_string(),
            kind: fs::RequestKind::Get,
        },
    );

    let fs::Success::Get(lump) = success else { panic!("expected Success::Get, got {:?}", success) };

    lump
}

fn read_file(fs: &Process, path: &str) -> Vec<u8> {
    let lump = get_file(fs, path);
    let lump = Lump::from_id(&lump);
    lump.get_data()
}

fn list_files(fs: &Process, path: &str) -> Vec<fs::FileInfo> {
    let success = request_fs(
        fs,
        fs::Request {
            target: path.to_string(),
            kind: fs::RequestKind::List,
        },
    );

    let fs::Success::List(files) = success else { panic!("expected Success::List, got {:?}", success) };

    files
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServiceConfig {
    pub description: Option<String>,

    #[serde(default)]
    pub license: Vec<License>,

    pub targets: Vec<String>,

    #[serde(default)]
    pub dependencies: Dependencies,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Dependencies {
    #[serde(default)]
    pub need: Vec<String>,

    #[serde(default)]
    pub milestone: Vec<String>,

    #[serde(default)]
    pub waits_for: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct License {
    pub name: String,
    pub file: String,
}
