use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU8, Ordering};

mod lang;
pub use lang::Language;

// Language code: 0 = English, 1 = Russian
static CURRENT_LANG: AtomicU8 = AtomicU8::new(0);

// Embedded translations
type TransMap = HashMap<String, String>;

static EN: LazyLock<TransMap> = LazyLock::new(|| {
    serde_json::from_str(include_str!("translations/en.json"))
        .expect("Failed to parse English translations")
});

static RU: LazyLock<TransMap> = LazyLock::new(|| {
    serde_json::from_str(include_str!("translations/ru.json"))
        .expect("Failed to parse Russian translations")
});

pub fn set_language(lang: Language) {
    CURRENT_LANG.store(lang as u8, Ordering::Relaxed);
}

pub fn language() -> Language {
    match CURRENT_LANG.load(Ordering::Relaxed) {
        1 => Language::Russian,
        _ => Language::English,
    }
}

/// Get a translation for the given key.
pub fn translate(key: &str) -> &str {
    let map = match CURRENT_LANG.load(Ordering::Relaxed) {
        1 => &*RU,
        _ => &*EN,
    };
    map.get(key).map(String::as_str).unwrap_or(key)
}

/// Get a translation with variable substitution.
/// Variables in the string are `{name}` placeholders.
pub fn translate_with(key: &str, vars: &[(&str, &str)]) -> String {
    let mut s = translate(key).to_string();
    for (k, v) in vars {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

/// Macro for simple translation lookup.
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::translate($key)
    };
}

/// Detect system language from environment variables (LANG, LANGUAGE, LC_ALL).
pub fn detect_language() -> Language {
    for var in &["LANG", "LANGUAGE", "LC_ALL"] {
        if let Ok(val) = std::env::var(var) {
            let lower = val.to_lowercase();
            if lower.starts_with("ru") || lower.contains("ru_ru") {
                return Language::Russian;
            }
        }
    }
    Language::English
}

/// Initialize i18n: detect language and set it.
pub fn init() {
    let lang = detect_language();
    set_language(lang);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_language_is_english() {
        assert_eq!(translate("cli.new.creating"), "Creating new world...");
    }

    #[test]
    fn test_russian_language() {
        set_language(Language::Russian);
        assert_eq!(translate("cli.new.creating"), "Создание нового мира...");
        set_language(Language::English);
    }

    #[test]
    fn test_missing_key_returns_key() {
        assert_eq!(translate("nonexistent.key"), "nonexistent.key");
    }

    #[test]
    fn test_translate_with_vars() {
        let result = translate_with("cli.new.created", &[("name", "test"), ("tiles", "9")]);
        assert!(result.contains("test"));
        assert!(result.contains("9"));
    }
}
