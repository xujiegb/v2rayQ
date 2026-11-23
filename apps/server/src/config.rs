use std::{io, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use v2rayq_storage::Storage;

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Section {
    Inbounds,
    Users,
    Outbounds,
    Nodes,
    Services,
    Tls,
    Reality,
    Routes,
    Dns,
    Administrators,
    Settings,
}

impl Section {
    const ALL: [Self; 11] = [
        Self::Inbounds,
        Self::Users,
        Self::Outbounds,
        Self::Nodes,
        Self::Services,
        Self::Tls,
        Self::Reality,
        Self::Routes,
        Self::Dns,
        Self::Administrators,
        Self::Settings,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Inbounds => "server/inbounds",
            Self::Users => "server/users",
            Self::Outbounds => "server/outbounds",
            Self::Nodes => "server/nodes",
            Self::Services => "server/services",
            Self::Tls => "server/tls",
            Self::Reality => "server/reality",
            Self::Routes => "server/routes",
            Self::Dns => "server/dns",
            Self::Administrators => "server/administrators",
            Self::Settings => "server/settings",
        }
    }

    fn list(self) -> bool {
        !matches!(self, Self::Dns | Self::Settings)
    }

    fn blank(self) -> Draft {
        Draft {
            revision: 0,
            value: if self.list() { json!([]) } else { json!({}) },
        }
    }

    fn accepts(self, value: &Value) -> bool {
        if self.list() {
            value
                .as_array()
                .is_some_and(|items| items.len() <= 1000 && items.iter().all(Value::is_object))
        } else {
            value.is_object()
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Draft {
    revision: u64,
    value: Value,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Command {
    Overview,
    System,
    SecurityProfiles,
    Theme,
    SaveTheme {
        theme: String,
    },
    Language,
    SaveLanguage {
        language: String,
    },
    Load {
        section: Section,
    },
    Save {
        section: Section,
        revision: u64,
        value: Value,
    },
}

pub fn dispatch(input: &[u8]) -> String {
    let result = serde_json::from_slice::<Command>(input)
        .map_err(|_| "Invalid server command".to_owned())
        .and_then(|command| run(command).map_err(|error| error.to_string()));
    match result {
        Ok(result) => json!({"result": result}),
        Err(error) => json!({"error": error}),
    }
    .to_string()
}

fn run(command: Command) -> v2rayq_storage::StorageResult<Value> {
    if matches!(command, Command::System) {
        return Ok(json!({
            "os": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "logical_cpus": std::thread::available_parallelism().ok().map(|count| count.get()),
            "version": env!("CARGO_PKG_VERSION"),
        }));
    }
    let store = Storage::open(data_path())?;
    match command {
        Command::Overview => {
            let drafts = store.list::<Draft>("server/")?;
            let mut counts = serde_json::Map::new();
            for section in Section::ALL {
                let count = drafts
                    .iter()
                    .find(|(key, _)| key == section.key())
                    .and_then(|(_, draft)| draft.value.as_array())
                    .map_or(0, Vec::len);
                let Some(name) = section.key().strip_prefix("server/") else {
                    continue;
                };
                counts.insert(name.into(), json!(count));
            }
            Ok(json!({ "counts": counts, "core_connected": false }))
        }
        Command::Theme => Ok(
            json!({ "theme": store.get::<String>("panel/theme")?.unwrap_or_else(|| "system".into()) }),
        ),
        Command::SaveTheme { theme } => {
            if !matches!(theme.as_str(), "system" | "light" | "dark") {
                return Err("Unsupported theme".into());
            }
            store.put("panel/theme", &theme)?;
            Ok(json!({}))
        }
        Command::Language => {
            Ok(json!({ "language": store.get::<String>("panel/language")?.unwrap_or_default() }))
        }
        Command::SaveLanguage { language } => {
            if !language.is_empty()
                && !crate::language::RESOURCES
                    .languages()
                    .iter()
                    .any(|item| item.code == language)
            {
                return Err("Unsupported language".into());
            }
            store.put("panel/language", &language)?;
            Ok(json!({}))
        }
        Command::SecurityProfiles => Ok(json!({
            "tls": profile_names(&store, Section::Tls)?,
            "reality": profile_names(&store, Section::Reality)?,
        })),
        Command::Load { section } => Ok(json!(
            store
                .get::<Draft>(section.key())?
                .unwrap_or_else(|| section.blank())
        )),
        Command::Save {
            section,
            revision,
            value,
        } => {
            if !section.accepts(&value) {
                return Err(if section.list() {
                    "Use a JSON array of up to 1,000 objects"
                } else {
                    "Use one JSON object"
                }
                .into());
            }
            store.update(section.key(), |old: Option<Draft>| {
                if old.as_ref().map_or(0, |draft| draft.revision) != revision {
                    return Err(
                        "The draft changed in another session. Reload and reapply your edits."
                            .into(),
                    );
                }
                let revision = revision
                    .checked_add(1)
                    .ok_or_else(|| io::Error::other("Invalid draft revision"))?;
                Ok((Draft { revision, value }, json!({ "revision": revision })))
            })
        }
        Command::System => unreachable!("System handled before opening storage"),
    }
}

fn profile_names(store: &Storage, section: Section) -> v2rayq_storage::StorageResult<Vec<String>> {
    let draft = store.get::<Draft>(section.key())?;
    let mut names: Vec<_> = draft
        .as_ref()
        .and_then(|draft| draft.value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("name").and_then(Value::as_str))
        .filter(|name| !name.trim().is_empty())
        .map(str::to_owned)
        .collect();
    names.sort_unstable();
    names.dedup();
    Ok(names)
}

fn data_path() -> PathBuf {
    if let Some(path) = std::env::var_os("V2RAYQ_SERVER_DB") {
        return path.into();
    }
    #[cfg(target_os = "windows")]
    if let Some(dir) = std::env::var_os("APPDATA") {
        return PathBuf::from(dir).join("v2rayQ").join("server.redb");
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join("Library/Application Support/v2rayQ/server.redb");
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join("v2rayQ/server.redb");
    }
    std::env::var_os("HOME").map_or_else(
        || PathBuf::from("v2rayQ-server.redb"),
        |home| PathBuf::from(home).join(".config/v2rayQ/server.redb"),
    )
}
