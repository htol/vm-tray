use virt::connect::Connect;
use virt::domain::DomainState;
use virt::enumutil::Enum;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Running,
    Paused,
    Off,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            State::Running => "running",
            State::Paused => "paused",
            State::Off => "shut off",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Vm {
    pub name: String,
    pub state: State,
}

#[derive(Clone, Copy)]
pub enum Action {
    Start,
    Stop,
    Resume,
}

pub fn list(conn: &Connect) -> Result<Vec<Vm>, String> {
    let domains = conn.list_all_domains(0).map_err(|e| e.to_string())?;
    let mut vms = Vec::with_capacity(domains.len());
    for d in domains {
        let name = d.name().map_err(|e| e.to_string())?;
        let state = match d.state().map_err(|e| e.to_string())?.0 {
            Enum::Known(DomainState::Running | DomainState::Blocked | DomainState::Shutdown) => {
                State::Running
            }
            Enum::Known(DomainState::Paused | DomainState::PMSuspended) => State::Paused,
            _ => State::Off,
        };
        vms.push(Vm { name, state });
    }
    vms.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(vms)
}

/// Runs an action on its own connection, so it can be called off the UI thread.
pub fn run(uri: &str, name: &str, action: Action) -> Result<(), String> {
    let conn = Connect::open(Some(uri)).map_err(|e| e.to_string())?;
    let dom = conn
        .lookup_domain_by_name(name)
        .map_err(|e| e.to_string())?;
    match action {
        Action::Start => dom.create(),
        Action::Stop => dom.shutdown(),
        Action::Resume => dom.resume(),
    }
    .map_err(|e| e.to_string())
}

pub fn open_viewer(uri: &str, name: &str) -> Result<(), String> {
    std::process::Command::new("virt-viewer")
        .args(["--attach", "--connect", uri, name])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("virt-viewer: {e}"))
}
