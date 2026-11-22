pub use v2rayq_i18n::{Translator, system_language};

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

pub static RESOURCES: v2rayq_i18n::Resources = v2rayq_i18n::Resources::new(LOCALES);
