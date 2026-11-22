#![forbid(unsafe_code)]

use std::{collections::BTreeMap, sync::OnceLock};

pub struct Catalog {
    pub code: &'static str,
    pub name: String,
    messages: BTreeMap<String, String>,
}

pub struct Resources {
    source: &'static [(&'static str, &'static str)],
    catalogs: OnceLock<Vec<Catalog>>,
}

impl Resources {
    pub const fn new(source: &'static [(&'static str, &'static str)]) -> Self {
        Self {
            source,
            catalogs: OnceLock::new(),
        }
    }

    pub fn languages(&'static self) -> &'static [Catalog] {
        self.catalogs.get_or_init(|| {
            self.source
                .iter()
                .filter_map(|(code, content)| {
                    let mut messages: BTreeMap<String, String> =
                        serde_json::from_str(content).ok()?;
                    let name = messages
                        .remove("_name")
                        .unwrap_or_else(|| (*code).to_owned());
                    Some(Catalog {
                        code,
                        name,
                        messages,
                    })
                })
                .collect()
        })
    }

    pub fn translator(&'static self, requested: &str) -> Translator {
        let languages = self.languages();
        let code = choose_language(requested, languages.iter().map(|item| item.code));
        Translator {
            catalog: languages.iter().find(|catalog| catalog.code == code),
            english: languages.iter().find(|catalog| catalog.code == "en"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Translator {
    catalog: Option<&'static Catalog>,
    english: Option<&'static Catalog>,
}

impl Translator {
    pub fn code(self) -> &'static str {
        self.catalog.map_or("en", |catalog| catalog.code)
    }

    pub fn text<'a>(self, key: &'a str) -> &'a str {
        self.catalog
            .and_then(|catalog| catalog.messages.get(key))
            .or_else(|| self.english.and_then(|catalog| catalog.messages.get(key)))
            .map_or(key, String::as_str)
    }

    pub fn message(self, message: &str) -> String {
        if let Some(catalog) = self.catalog {
            if let Some(text) = catalog.messages.get(message) {
                return text.to_owned();
            }
            for pattern in self.english.unwrap_or(catalog).messages.keys() {
                if pattern.contains('{')
                    && let Some(values) = capture(pattern, message)
                {
                    let mut result = self.text(pattern).to_owned();
                    for (name, value) in values {
                        result = result.replace(&format!("{{{name}}}"), self.text(value));
                    }
                    return result;
                }
            }
        }
        message.to_owned()
    }
}

fn capture<'a>(mut pattern: &'a str, mut text: &'a str) -> Option<Vec<(&'a str, &'a str)>> {
    let mut values = Vec::new();
    while let Some((prefix, rest)) = pattern.split_once('{') {
        text = text.strip_prefix(prefix)?;
        let (name, remainder) = rest.split_once('}')?;
        let next_literal = remainder.split('{').next()?;
        let count = if remainder.is_empty() {
            text.len()
        } else {
            text.find(next_literal)?
        };
        if count == 0 {
            return None;
        }
        values.push((name, &text[..count]));
        text = &text[count..];
        pattern = remainder;
    }
    (text == pattern).then_some(values)
}

pub fn choose_language<'a>(requested: &str, languages: impl Iterator<Item = &'a str>) -> &'a str {
    let codes: Vec<_> = languages.collect();
    let requested = requested.replace('_', "-").to_ascii_lowercase();
    let requested = requested.split('.').next().unwrap_or("en");
    if let Some(code) = codes
        .iter()
        .find(|code| code.eq_ignore_ascii_case(requested))
    {
        return code;
    }
    let candidate = if requested.starts_with("zh") {
        if requested.contains("hant")
            || requested.ends_with("-tw")
            || requested.ends_with("-hk")
            || requested.ends_with("-mo")
        {
            "zh-TW"
        } else {
            "zh-CN"
        }
    } else {
        requested.split('-').next().unwrap_or("en")
    };
    codes
        .into_iter()
        .find(|code| code.eq_ignore_ascii_case(candidate))
        .unwrap_or("en")
}

pub fn system_language() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
        .unwrap_or_else(|| "en".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    static RESOURCES: Resources = Resources::new(&[
        (
            "en",
            r#"{"_name":"English","Ready":"Ready","Imported {count} nodes: {status}":"Imported {count} nodes: {status}"}"#,
        ),
        (
            "fr",
            r#"{"_name":"Français","Ready":"Prêt","Imported {count} nodes: {status}":"{count} nœuds importés : {status}"}"#,
        ),
        ("zh-CN", r#"{"_name":"简体中文","Ready":"就绪"}"#),
        ("zh-TW", r#"{"_name":"繁體中文","Ready":"就緒"}"#),
    ]);

    #[test]
    fn resolves_regional_languages_and_translates_dynamic_messages() {
        let french = RESOURCES.translator("fr_CA.UTF-8");
        assert_eq!(french.code(), "fr");
        assert_eq!(
            french.message("Imported 12 nodes: Ready"),
            "12 nœuds importés : Prêt"
        );
        assert_eq!(french.text("User supplied name"), "User supplied name");
        assert_eq!(RESOURCES.translator("zh-Hans").text("Ready"), "就绪");
        assert_eq!(RESOURCES.translator("zh-HK").text("Ready"), "就緒");
        assert_eq!(RESOURCES.translator("de-DE").text("Ready"), "Ready");
        assert_eq!(
            RESOURCES
                .translator("zh-TW")
                .message("Imported 2 nodes: Ready"),
            "Imported 2 nodes: 就緒"
        );
    }
}
