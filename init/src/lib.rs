use std::collections::HashMap;

use hearth_guest::{registry::RegistryResponse, *};
use petgraph::{algo::toposort, prelude::DiGraph};
use serde::{Deserialize, Serialize};

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

    let fs = REGISTRY.get_service("hearth.fs.Filesystem").unwrap();
    let mut graph = DiGraph::<Service, ()>::new();
    let mut names_to_idxs = HashMap::new();

    let search_dir = "init";
    for file in list_files(&fs, search_dir) {
        info!("file: {}", file.name);

        let config_path = format!("{}/{}/service.toml", search_dir, file.name);
        let config_data = read_file(&fs, &config_path);
        let config_str = String::from_utf8(config_data).unwrap();
        let config: ServiceConfig = toml::from_str(&config_str).unwrap();
        info!("config: {:?}", config);

        let deps = config.dependencies.need.clone();
        let name = file.name;
        let service = Service::new(name.clone(), config);
        let name = service.name.clone();
        let idx = graph.add_node(service);
        names_to_idxs.insert(name, idx);

        for dep in deps {
            let dep_idx = *names_to_idxs.get(&dep).unwrap();
            graph.add_edge(idx, dep_idx, ());
        }
    }

    // TODO start up init registry first and sandbox processes in it

    let sorted_services = toposort(&graph, None).unwrap();
    let mut targets: HashMap<String, HashMap<String, Process>> = HashMap::new();
    for service in sorted_services {
        let service = &mut graph[service];
        let name = service.get_name().to_string();
        let in_targets = service.get_config().targets.clone();
        let process = service.start();

        for target in in_targets.iter() {
            info!("adding {:?} to {:?} target", name, target);
            let process = process.clone();
            if let Some(existing) = targets.get_mut(target) {
                existing.insert(name.to_string(), process);
            } else {
                info!("initializing {:?} target", target);
                let mut registry = HashMap::new();
                registry.insert(name.to_string(), process);
                targets.insert(target.to_string(), registry);
            }
        }
    }

    let mut registries = HashMap::with_capacity(targets.len());
    for (name, services) in targets {
        let (service_names, caps): (Vec<String>, Vec<Process>) = services.into_iter().unzip();
        let caps: Vec<&Process> = caps.iter().collect();
        let config = RegistryConfig { service_names };
        let registry = Process::spawn(registry, None);
        registry.send_json(&config, &caps);
        registries.insert(name, registry);
    }

    let target_hook = |target_name: &str, hook_service: &str| {
        let Some(hook) = REGISTRY.get_service(hook_service) else {
            info!("Hook service {:?} is unavailable; skipping", hook_service);
            return;
        };

        let Some(target) = registries.get(target_name) else {
            info!("Hook target {:?} is unavailable; skipping", target_name);
            return;
        };

        info!("Hooking {:?} with {:?} target", hook_service, target_name);
        hook.send(&[], &[target]);
    };

    target_hook("server", "hearth.init.Server");
    target_hook("client", "hearth.init.Client");
    target_hook("daemon", "hearth.init.Daemon");
}

pub struct Service {
    name: String,
    process: Option<Process>,
    config: ServiceConfig,
}

impl Service {
    pub fn new(name: String, config: ServiceConfig) -> Self {
        Self {
            name,
            process: None,
            config,
        }
    }

    pub fn start(&mut self) -> &Process {
        self.process.get_or_insert_with(|| {
            info!("starting {:?}", self.name);

            let fs = REGISTRY.get_service("hearth.fs.Filesystem").unwrap();
            let lump = get_file(&fs, &format!("init/{}/service.wasm", self.name));

            let (_msg, mut caps) = WASM_SPAWNER.request(
                wasm::WasmSpawnInfo {
                    lump,
                    entrypoint: None,
                },
                &[REGISTRY.as_ref()],
            );

            caps.remove(0)
        })
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_config(&self) -> &ServiceConfig {
        &self.config
    }
}

#[derive(Deserialize, Serialize)]
pub struct RegistryConfig {
    pub service_names: Vec<String>,
}

fn registry() {
    let (config, service_list) = PARENT.recv_json::<RegistryConfig>();
    let mut services = HashMap::new();
    for (process, name) in service_list.into_iter().zip(config.service_names) {
        info!("now serving {:?}", name);
        services.insert(name, process);
    }

    loop {
        let (request, caps) = PARENT.recv_json::<registry::RegistryRequest>();
        let Some(reply) = caps.first() else { continue };

        use registry::RegistryRequest::*;
        let mut response_cap = vec![];
        let response = match request {
            Get { name } => match services.get(&name) {
                Some(service) => {
                    response_cap.push(service);
                    RegistryResponse::Get(true)
                }
                None => RegistryResponse::Get(false),
            },
            Register { .. } => RegistryResponse::Register(None),
            List => RegistryResponse::List(services.keys().map(|k| k.to_string()).collect()),
        };

        reply.send_json(&response, &response_cap);
    }
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

fn request_fs(fs: &Process, request: fs::Request) -> fs::Success {
    log!(ProcessLogLevel::Debug, "making fs request: {:?}", request);
    let reply = Mailbox::new();
    let reply_cap = reply.make_capability(Permissions::SEND);
    fs.send_json(&request, &[&reply_cap]);
    let (response, _caps) = reply.recv_json::<fs::Response>();
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

    let fs::Success::Get(lump) = success else {
        panic!("expected Success::Get, got {:?}", success)
    };

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

    let fs::Success::List(files) = success else {
        panic!("expected Success::List, got {:?}", success)
    };

    files
}
