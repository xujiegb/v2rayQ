#![forbid(unsafe_code)]

use std::{
    env,
    io::{self, Read},
    process::ExitCode,
};

use serde_json::{Value, json};
use v2rayQ::client::{Access, ClientService, Command, Reply, runtime::ServiceStatus};

const HELP: &str = "v2rayq-cli
  status | nodes | subscriptions
  add-link                          Read a share link from stdin
  add-node                          Read a manual node JSON object from stdin
  subscribe NAME                    Read an HTTPS subscription URL from stdin
  select ID | delete ID
  refresh ID | unsubscribe ID
  routing direct|global|rules [DNS]
  tun on|off | transparent on|off";

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    let service = ClientService::default();
    if let [entry] = args.as_slice() {
        let access = match entry.as_str() {
            "--rpc" | "--luci-rpc" => Some(Access::ProfileOnly),
            "--service-rpc" | "--luci-service-rpc" => Some(Access::System),
            _ => None,
        };
        if let Some(access) = access {
            let result = read_input()
                .and_then(|body| {
                    if entry.starts_with("--luci-") {
                        let wrapper: Value =
                            serde_json::from_str(&body).map_err(|_| "Invalid LuCI request")?;
                        let command = wrapper
                            .get("command")
                            .and_then(Value::as_str)
                            .ok_or("Missing LuCI command")?;
                        return serde_json::from_str(command)
                            .map_err(|_| "Invalid command JSON".into());
                    }
                    serde_json::from_str::<Command>(&body)
                        .map_err(|_| "Invalid command JSON".into())
                })
                .and_then(|cmd| service.call(cmd, access));
            let response = match result {
                Ok(value) => json!({ "result": value }),
                Err(error) => json!({ "error": error }),
            };
            println!("{response}");
            return ExitCode::SUCCESS;
        }
    }
    let kind: Vec<_> = args.iter().map(String::as_str).collect();
    if matches!(kind.as_slice(), [] | ["help" | "--help" | "-h"]) {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    match parse(&kind).and_then(|cmd| service.call(cmd, Access::Terminal)) {
        Ok(reply) => {
            print_reply(reply, &kind);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn read_input() -> Result<String, String> {
    const LIMIT: u64 = 128 * 1024;
    let mut input = String::new();
    io::stdin()
        .take(LIMIT + 1)
        .read_to_string(&mut input)
        .map_err(|error| error.to_string())?;
    if input.len() as u64 > LIMIT {
        return Err("Command exceeds 128 KiB".into());
    }
    Ok(input)
}

fn parse(args: &[&str]) -> Result<Command, String> {
    let command: Value = match args {
        ["status"] => json!({ "action": "status" }),
        ["nodes" | "subscriptions"] => json!({ "action": "profile" }),
        ["add-link"] => json!({ "action": "add_link", "link": read_input()?.trim() }),
        ["add-node"] => {
            json!({ "action": "add_node", "node": serde_json::from_str::<Value>(&read_input()?)
            .map_err(|_| "Invalid manual node JSON")? })
        }
        ["subscribe", name] => {
            json!({ "action": "add_subscription", "name": name, "url": read_input()?.trim() })
        }
        ["select", id] => json!({ "action": "select_node", "id": id }),
        ["delete", id] => json!({ "action": "remove_node", "id": id }),
        ["refresh", id] => json!({ "action": "refresh_subscription", "id": id }),
        ["unsubscribe", id] => json!({ "action": "remove_subscription", "id": id }),
        ["routing", mode] => {
            json!({ "action": "save_routing", "route_mode": mode, "dns_server": "" })
        }
        ["routing", mode, dns] => {
            json!({ "action": "save_routing", "route_mode": mode, "dns_server": dns })
        }
        [mode @ ("tun" | "transparent"), state @ ("on" | "off")] => {
            json!({ "action": "set_network", "mode": mode, "enabled": *state == "on" })
        }
        _ => return Err("Invalid arguments. Run v2rayq-cli --help.".into()),
    };
    serde_json::from_value(command).map_err(|_| "Invalid arguments. Run v2rayq-cli --help.".into())
}

fn print_reply(reply: Reply, args: &[&str]) {
    if let Some(message) = reply.message {
        println!("{message}");
    }
    if let Some(status) = reply.runtime {
        println!("TUN               {}", service_label(status.tun));
        println!("Transparent proxy {}", service_label(status.transparent));
    }
    if let Some(profile) = reply.profile {
        if args == ["nodes"] {
            for node in profile.nodes {
                let selected = if profile.selected_node == Some(node.id) {
                    "*"
                } else {
                    " "
                };
                println!(
                    "{selected}  {}  {:12}  {}  {}:{}",
                    node.id,
                    node.protocol.label(),
                    terminal_text(&node.name),
                    terminal_text(&node.host),
                    node.port
                );
            }
        } else if args == ["subscriptions"] {
            for item in profile.subscriptions {
                println!(
                    "{}  {}  {}",
                    item.id,
                    terminal_text(&item.name),
                    item.last_error.as_deref().unwrap_or("Ready")
                );
            }
        }
    }
}

fn terminal_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .collect()
}

fn service_label(status: ServiceStatus) -> &'static str {
    match (status.installed, status.running) {
        (false, _) => "Service missing",
        (true, Some(true)) => "Running",
        (true, Some(false)) => "Stopped",
        (true, None) => "Unknown",
    }
}
