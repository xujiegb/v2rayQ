#![forbid(unsafe_code)]

use std::{
    env,
    io::{self, Read},
    process::ExitCode,
};

use serde_json::{Value, json};
use v2rayQ::client::{
    Access, ClientData, ClientService, Command, Reply, default_data_path, language,
    runtime::ServiceStatus,
};

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
    let mut kind: Vec<_> = args.iter().map(String::as_str).collect();
    let explicit = if kind.first() == Some(&"--lang") && kind.len() >= 2 {
        Some(kind.drain(..2).nth(1).unwrap_or("en"))
    } else {
        None
    };
    let path = default_data_path();
    let saved = if path.exists() {
        ClientData::load(&path)
            .ok()
            .map(|data| data.settings.language)
    } else {
        None
    };
    let requested = explicit
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| saved.filter(|value| !value.is_empty()))
        .unwrap_or_else(v2rayQ::client::language::system_language);
    let tr = language::RESOURCES.translator(&requested);
    if matches!(kind.as_slice(), [] | ["help" | "--help" | "-h"]) {
        println!("{}", tr.text("CLI help"));
        return ExitCode::SUCCESS;
    }
    match parse(&kind).and_then(|cmd| service.call(cmd, Access::Terminal)) {
        Ok(reply) => {
            print_reply(reply, &kind, tr);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", tr.message(&error));
            ExitCode::FAILURE
        }
    }
}

fn read_input() -> Result<String, String> {
    const LIMIT: u64 = 8 * 1024 * 1024;
    let mut input = String::new();
    io::stdin()
        .take(LIMIT + 1)
        .read_to_string(&mut input)
        .map_err(|error| error.to_string())?;
    if input.len() as u64 > LIMIT {
        return Err("Command exceeds 8 MiB".into());
    }
    Ok(input)
}

fn parse(args: &[&str]) -> Result<Command, String> {
    let command: Value = match args {
        ["export-config"] => json!({ "action": "export_configuration" }),
        ["merge-config"] | ["merge-config", "--preferences"] => json!({
            "action": "merge_configuration", "content": read_input()?, "use_preferences": args.len() == 2
        }),
        ["language", language] => json!({ "action": "save_language", "language": language }),
        ["status"] => json!({ "action": "status" }),
        ["inspect-network"] => json!({ "action": "inspect_network" }),
        ["groups"] => json!({ "action": "profile" }),
        ["add-group", name] => json!({ "action": "save_node_group", "id": null, "name": name }),
        ["rename-group", id, name] => json!({ "action": "save_node_group", "id": id, "name": name }),
        ["delete-group", id] => json!({ "action": "remove_node_group", "id": id }),
        ["move-node", id] => json!({ "action": "assign_node_group", "node_id": id, "group_id": null }),
        ["move-node", id, group] => json!({ "action": "assign_node_group", "node_id": id, "group_id": group }),
        ["nodes" | "subscriptions" | "routes"] => json!({ "action": "profile" }),
        ["import" | "preview-import"] => {
            let action = if args[0] == "import" {
                "import_nodes"
            } else {
                "preview_import"
            };
            json!({ "action": action, "content": read_input()? })
        }
        ["update-subscription", id, name] => {
            json!({ "action": "update_subscription", "id": id, "name": name })
        }
        ["replace-subscription-url", id, name] => {
            json!({ "action": "update_subscription", "id": id, "name": name, "url": read_input()?.trim() })
        }
        ["add-link"] => json!({ "action": "add_link", "link": read_input()?.trim() }),
        ["add-node"] => {
            json!({ "action": "add_node", "node": serde_json::from_str::<Value>(&read_input()?)
            .map_err(|_| "Invalid manual node JSON")? })
        }
        ["subscribe", name] => {
            json!({ "action": "add_subscription", "name": name, "url": read_input()?.trim() })
        }
        ["edit-node", id, name, host, port] => {
            json!({ "action": "update_node", "id": id, "name": name, "host": host, "port": port })
        }
        ["copy-node", id] => json!({ "action": "copy_node", "id": id }),
        ["add-rule"] | ["edit-rule", _] => {
            let rule = serde_json::from_str::<Value>(&read_input()?)
                .map_err(|_| "Invalid route rule JSON")?;
            let id = args.get(1);
            json!({ "action": "save_route_rule", "id": id, "rule": rule })
        }
        ["delete-rule", id] => json!({ "action": "remove_route_rule", "id": id }),
        ["move-rule", id, direction] => {
            json!({ "action": "move_route_rule", "id": id, "direction": direction })
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

fn print_reply(reply: Reply, args: &[&str], tr: language::Translator) {
    if let Some(info) = reply.ip_info {
        println!("{}  {}  {}  {}", info.ip, terminal_text(&info.country), terminal_text(&info.city), terminal_text(&info.isp));
    }
    if let Some(config) = reply.configuration {
        println!("{config}");
    }
    if let Some(summary) = reply.merge {
        println!(
            "{}",
            tr.message(&format!(
                "{} nodes · {} subscriptions · {} rules · {} duplicates",
                summary.nodes, summary.subscriptions, summary.rules, summary.duplicates
            ))
        );
    }
    if let Some(message) = reply.message {
        println!("{}", tr.message(&message));
    }
    if let Some(status) = reply.runtime {
        println!("TUN               {}", tr.text(service_label(status.tun)));
        println!(
            "{} {}",
            tr.text("Transparent proxy"),
            tr.text(service_label(status.transparent))
        );
    }
    if let Some(profile) = reply.profile {
        if args == ["groups"] {
            for group in &profile.groups { println!("{}  {}  {}", group.id, terminal_text(&group.name), group.node_count); }
        } else if args == ["nodes"] {
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
                    "{}  {}  {}  {}",
                    item.id,
                    terminal_text(&item.name),
                    tr.message(&format!("{} nodes", item.node_count)),
                    tr.text(item.last_error.as_deref().unwrap_or("Ready"))
                );
            }
        } else if args == ["routes"] {
            for (index, rule) in profile.settings.route_rules.into_iter().enumerate() {
                println!(
                    "{}  {}  {}  {} → {}",
                    index + 1,
                    rule.id,
                    tr.text(rule.fields.kind.label()),
                    terminal_text(&rule.fields.value),
                    tr.text(rule.fields.target.label())
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
