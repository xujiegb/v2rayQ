//! Service control for administrator-provisioned proxy profiles.
use std::{
    path::Path,
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};

use super::Access;

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    Tun,
    Transparent,
}

#[derive(Default, Deserialize, Serialize)]
pub struct ServiceStatus {
    pub installed: bool,
    pub running: Option<bool>,
    pub can_change: bool,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Runtime {
    #[serde(default)]
    pub traffic: Traffic,
    pub tun: ServiceStatus,
    pub transparent: ServiceStatus,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Traffic {
    pub core_connected: bool,
    pub upload_bps: Option<u64>,
    pub download_bps: Option<u64>,
}

pub fn status(access: Access) -> Runtime {
    Runtime {
        traffic: Traffic::default(),
        tun: service(NetworkMode::Tun)
            .map_or_else(ServiceStatus::default, |item| item.status(access)),
        transparent: service(NetworkMode::Transparent)
            .map_or_else(ServiceStatus::default, |item| item.status(access)),
    }
}

pub fn set(mode: NetworkMode, enabled: bool, access: Access) -> Result<(), String> {
    if matches!(access, Access::ProfileOnly) {
        return Err("Use an authorized service control session".into());
    }
    let service = service(mode).ok_or("No service manager for this platform")?;
    if !service.status(access).installed {
        return Err("Ask an administrator to install the proxy service for this mode".into());
    }
    let (program, args) = service.action(enabled);
    let result = match access {
        Access::System => Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
        Access::Terminal | Access::Desktop => authorize(program, args, access),
        Access::ProfileOnly => return Err("Use an authorized service control session".into()),
    };
    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err("Authorization or service operation failed".into()),
        Err(error) => Err(format!("Could not start service authorization: {error}")),
    }
}

#[cfg(unix)]
fn authorize(
    program: &str,
    args: Vec<&str>,
    access: Access,
) -> std::io::Result<std::process::ExitStatus> {
    let mut command = Command::new(sudo_path());
    command.arg("-k");
    if matches!(access, Access::Desktop) {
        command.arg("-A").stdin(Stdio::null());
    }
    command
        .arg("--")
        .arg(program)
        .args(args)
        .stdout(Stdio::null())
        .status()
}

fn sudo_path() -> &'static str {
    if cfg!(target_os = "freebsd") {
        "/usr/local/bin/sudo"
    } else {
        "/usr/bin/sudo"
    }
}

#[cfg(not(unix))]
fn authorize(
    program: &str,
    args: Vec<&str>,
    _: Access,
) -> std::io::Result<std::process::ExitStatus> {
    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .status()
}

#[derive(Clone, Copy)]
enum Manager {
    Systemd,
    Procd,
    Launchd,
    Rc,
    Windows,
}

struct Service {
    manager: Manager,
    name: &'static str,
    definition: &'static str,
}

fn service(mode: NetworkMode) -> Option<Service> {
    let tun = matches!(mode, NetworkMode::Tun);
    if cfg!(target_os = "linux") {
        return if Path::new("/run/systemd/system").is_dir() {
            Some(Service {
                manager: Manager::Systemd,
                name: if tun {
                    "v2rayq-tun.service"
                } else {
                    "v2rayq-transparent.service"
                },
                definition: "",
            })
        } else {
            let script = if tun {
                "/etc/init.d/v2rayq-tun"
            } else {
                "/etc/init.d/v2rayq-transparent"
            };
            Some(Service {
                manager: Manager::Procd,
                name: script,
                definition: script,
            })
        };
    }
    if cfg!(target_os = "macos") {
        return Some(Service {
            manager: Manager::Launchd,
            name: if tun {
                "system/org.v2rayq.tun"
            } else {
                "system/org.v2rayq.transparent"
            },
            definition: "",
        });
    }
    if cfg!(target_os = "freebsd") {
        return Some(if tun {
            Service {
                manager: Manager::Rc,
                name: "v2rayq_tun",
                definition: "/usr/local/etc/rc.d/v2rayq_tun",
            }
        } else {
            Service {
                manager: Manager::Rc,
                name: "v2rayq_transparent",
                definition: "/usr/local/etc/rc.d/v2rayq_transparent",
            }
        });
    }
    if cfg!(target_os = "windows") {
        return Some(Service {
            manager: Manager::Windows,
            name: if tun {
                "v2rayq-tun"
            } else {
                "v2rayq-transparent"
            },
            definition: "",
        });
    }
    None
}

impl Service {
    fn status(&self, access: Access) -> ServiceStatus {
        let (program, args) = match self.manager {
            Manager::Systemd => (
                "/usr/bin/systemctl",
                vec!["show", "-p", "LoadState", "-p", "ActiveState", self.name],
            ),
            Manager::Procd => (self.name, vec!["running"]),
            Manager::Launchd => ("/bin/launchctl", vec!["print", self.name]),
            Manager::Rc => ("/usr/sbin/service", vec![self.name, "onestatus"]),
            Manager::Windows => ("sc.exe", vec!["query", self.name]),
        };
        if !self.definition.is_empty() && !Path::new(self.definition).is_file() {
            return ServiceStatus::default();
        }
        let Ok(output) = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .output()
        else {
            return ServiceStatus::default();
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let (installed, running) = match self.manager {
            Manager::Systemd => (
                text.lines().any(|line| line == "LoadState=loaded"),
                text.lines().find_map(|line| {
                    line.strip_prefix("ActiveState=")
                        .map(|state| state == "active")
                }),
            ),
            Manager::Launchd => (
                output.status.success(),
                Some(text.lines().any(|line| line.trim() == "state = running")),
            ),
            Manager::Procd | Manager::Rc => (true, Some(output.status.success())),
            Manager::Windows => (
                output.status.success(),
                text.lines().find_map(|line| {
                    let (field, value) = line.split_once(':')?;
                    (field.trim() == "STATE").then(|| value.split_whitespace().next() == Some("4"))
                }),
            ),
        };
        let can_change = installed
            && match access {
                Access::ProfileOnly => false,
                Access::Desktop if cfg!(unix) => Path::new(sudo_path()).is_file(),
                Access::Desktop | Access::System | Access::Terminal => true,
            };
        ServiceStatus {
            installed,
            running: if installed { running } else { None },
            can_change,
        }
    }

    fn action(&self, enabled: bool) -> (&'static str, Vec<&str>) {
        let action = if enabled { "start" } else { "stop" };
        match self.manager {
            Manager::Systemd => ("/usr/bin/systemctl", vec![action, self.name]),
            Manager::Procd => (self.name, vec![action]),
            Manager::Launchd if enabled => ("/bin/launchctl", vec!["kickstart", self.name]),
            Manager::Launchd => ("/bin/launchctl", vec!["kill", "SIGTERM", self.name]),
            Manager::Rc => (
                "/usr/sbin/service",
                vec![self.name, if enabled { "onestart" } else { "onestop" }],
            ),
            Manager::Windows => ("sc.exe", vec![action, self.name]),
        }
    }
}
