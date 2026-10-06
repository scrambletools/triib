//! The interface's text in the user's language.
//!
//! Every string triib shows lives in `i18n/<language>/triib.ftl` at the top
//! of the repository, in Project Fluent's format. `i18n/en` is the source:
//! the `fl!` macro checks each key against it when triib is compiled, and
//! other languages fall back to it for keys they lack. Adding a language
//! is adding its folder; no code changes. The words of the standards stay
//! as they write them, as `docs/GLOSSARY.md` sets out.

use std::collections::BTreeSet;
use std::sync::{LazyLock, Mutex, PoisonError, RwLock};

use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::unic_langid::{CharacterDirection, LanguageIdentifier};
use i18n_embed::{DesktopLanguageRequester, LanguageLoader};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../i18n/"]
struct Localizations;

/// The loaded text, in the first language the user asks for that triib
/// has, else English.
pub static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader = fluent_language_loader!();
    select(&loader);
    loader
});

/// The language chosen in Settings; none follows the system.
static CHOSEN: RwLock<Option<LanguageIdentifier>> = RwLock::new(None);

/// Loads the best language for the user's requests into `loader`.
fn select(loader: &FluentLanguageLoader) {
    let _ = i18n_embed::select(loader, &Localizations, &requested_languages());
    // Values placed in a sentence are wrapped in direction marks so a
    // Latin entity name keeps its order inside right to left text; left to
    // right languages need no marks.
    let right_to_left = direction_of(&loader.current_language()) == CharacterDirection::RTL;
    loader.set_use_isolating(right_to_left);
    scramble_ui::dir::set_right_to_left(right_to_left);
}

/// The languages the user asks for, most wanted first: `TRIIB_LANG` (a
/// language tag such as `he` or `pt-BR`, for trying translations), then
/// the one chosen in Settings, then the system's.
fn requested_languages() -> Vec<LanguageIdentifier> {
    let chosen = CHOSEN
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    // Tests compare English text, whatever the machine's language, unless
    // one picks a language, as the pictures do.
    if cfg!(test) {
        return vec![chosen.unwrap_or_else(|| "en".parse().expect("a language tag"))];
    }
    let mut languages: Vec<LanguageIdentifier> = std::env::var("TRIIB_LANG")
        .ok()
        .and_then(|tag| tag.parse().ok())
        .into_iter()
        .chain(chosen)
        .collect();
    languages.extend(DesktopLanguageRequester::requested_languages());
    with_aliases(languages)
}

/// `languages` with, after each macrolanguage tag, the language triib has
/// for it: systems name Norwegian `no`, which triib's `nb` answers.
fn with_aliases(languages: Vec<LanguageIdentifier>) -> Vec<LanguageIdentifier> {
    let mut out = Vec::with_capacity(languages.len());
    for language in languages {
        let alias = (language.language.as_str() == "no").then(|| {
            let mut bokmal = language.clone();
            bokmal.language = "nb".parse().expect("a language subtag");
            bokmal
        });
        out.push(language);
        out.extend(alias);
    }
    out
}

/// Shows the interface in the language tagged `tag` (such as `he`), or in
/// the system's language for `None` or a tag triib has no text for. Text
/// shown from then on is in that language.
pub fn set_language(tag: Option<&str>) {
    *CHOSEN.write().unwrap_or_else(PoisonError::into_inner) = tag.and_then(|tag| tag.parse().ok());
    select(&LOADER);
    // scramble-ui's components name their own buttons.
    scramble_ui::labels::set(scramble_ui::labels::Labels {
        close: crate::fl!("common-close"),
        more: crate::fl!("common-more"),
        keep_toolbar_shown: crate::fl!("common-keep-toolbar-shown"),
        auto_hide_toolbar: crate::fl!("common-auto-hide-toolbar"),
    });
}

/// The languages triib has text for: each one's tag and its name in its
/// own language, such as ("he", "עברית"), sorted by tag.
pub fn languages() -> &'static [(String, String)] {
    static LANGUAGES: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
        let mut languages: Vec<(String, String)> = available()
            .into_iter()
            .map(|language| (language.to_string(), name_of(&language)))
            .collect();
        languages.sort();
        languages
    });
    &LANGUAGES
}

/// `language`'s name for itself, from its own file.
fn name_of(language: &LanguageIdentifier) -> String {
    let loader = fluent_language_loader!();
    let _ = loader.load_languages(&Localizations, std::slice::from_ref(language));
    loader.get("language-name")
}

/// The name of the language the system asks for, as triib would show it
/// when following the system.
pub fn system_language_name() -> String {
    let loader = fluent_language_loader!();
    let _ = i18n_embed::select(
        &loader,
        &Localizations,
        &DesktopLanguageRequester::requested_languages(),
    );
    loader.get("language-name")
}

/// Text that must outlive the view showing it, such as a tab's label or a
/// placeholder iced borrows. Each distinct text is kept once for the life
/// of triib, so after a language change the new text is kept too.
pub fn lasting(text: String) -> &'static str {
    static KEPT: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());
    let mut kept = KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(found) = kept.get(text.as_str()) {
        return found;
    }
    let text: &'static str = Box::leak(text.into_boxed_str());
    kept.insert(text);
    text
}

/// `text` in capitals as the interface's language writes them.
pub fn uppercase(text: &str) -> String {
    uppercase_in(LOADER.current_language().language.as_str(), text)
}

/// `text` with a capital first letter, as the interface's language writes
/// it.
pub fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => {
            uppercase_in(
                LOADER.current_language().language.as_str(),
                &first.to_string(),
            ) + characters.as_str()
        }
        None => String::new(),
    }
}

/// `text` in capitals in the language with code `language`: Turkish and
/// Azerbaijani capitalize i as İ, and Greek drops its accents in capitals.
fn uppercase_in(language: &str, text: &str) -> String {
    match language {
        "tr" | "az" => text.replace('i', "İ").to_uppercase(),
        "el" => text
            .to_uppercase()
            .chars()
            .filter(|&character| character != '\u{301}')
            .map(|character| match character {
                'Ά' => 'Α',
                'Έ' => 'Ε',
                'Ή' => 'Η',
                'Ί' => 'Ι',
                'Ό' => 'Ο',
                'Ύ' => 'Υ',
                'Ώ' => 'Ω',
                other => other,
            })
            .collect(),
        _ => text.to_uppercase(),
    }
}

/// A number formatted with a point, written with the language's decimal
/// mark: "44.1" as "44,1" in German.
pub fn decimal(text: String) -> String {
    let mark = crate::fl!("common-decimal-separator");
    if mark == "." {
        text
    } else {
        text.replace('.', &mark)
    }
}

/// Parts of a description joined as the language lists them, such as
/// "enp6s0, up, hardware clock ptp0".
pub fn list(parts: impl IntoIterator<Item = String>) -> String {
    parts
        .into_iter()
        .collect::<Vec<_>>()
        .join(&crate::fl!("common-list-separator"))
}

fn direction_of(language: &LanguageIdentifier) -> CharacterDirection {
    language.character_direction()
}

/// The languages triib has text for.
pub fn available() -> Vec<LanguageIdentifier> {
    LOADER
        .available_languages(&Localizations)
        .unwrap_or_default()
}

/// Text for `key` from `i18n/<language>/triib.ftl`, with Fluent arguments:
/// `fl!("entity-count", count = entities)`. Checked against the English
/// file at compile time.
#[macro_export]
macro_rules! fl {
    ($key:literal) => {{
        i18n_embed_fl::fl!($crate::i18n::LOADER, $key)
    }};
    ($key:literal, $($args:tt)*) => {{
        i18n_embed_fl::fl!($crate::i18n::LOADER, $key, $($args)*)
    }};
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../i18n")
    }

    /// Each message's key and its text, attributes and continuation lines
    /// joined to it.
    fn messages(language: &str) -> BTreeMap<String, String> {
        let path = root().join(language).join("triib.ftl");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("reading {path:?}"));
        let mut messages: BTreeMap<String, String> = BTreeMap::new();
        let mut current: Option<String> = None;
        for line in text.lines() {
            let starts_message =
                line.split_once('=')
                    .map(|(key, _)| key.trim_end())
                    .filter(|key| {
                        !line.starts_with([' ', '\t', '#', '.'])
                            && key.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                            && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                    });
            if let Some(key) = starts_message {
                assert!(
                    !messages.contains_key(key),
                    "{language}: `{key}` is defined twice"
                );
                let value = line.split_once('=').map_or("", |(_, value)| value);
                messages.insert(key.to_owned(), value.to_owned());
                current = Some(key.to_owned());
            } else if line.is_empty() || line.starts_with('#') {
                current = None;
            } else if let Some(key) = &current {
                let value = messages.get_mut(key).unwrap();
                value.push('\n');
                value.push_str(line);
            }
        }
        messages
    }

    /// The `{ $variables }` a message's text uses.
    fn variables(text: &str) -> BTreeSet<String> {
        text.split('$')
            .skip(1)
            .map(|piece| {
                piece
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                    .collect::<String>()
            })
            .filter(|name| !name.is_empty())
            .collect()
    }

    /// The words of the standards, the names and the units in a message,
    /// which every language keeps as written: AVB, gPTP, CRF, AECP, Milan,
    /// kHz, Mb/s. They are the Latin words with two capitals or a capital
    /// after a small letter, the names and the units, each ending where
    /// another script starts, as in "triibは"; Fluent's selector lines are
    /// left out.
    fn kept_words(text: &str) -> BTreeSet<String> {
        const NAMES: [&str; 2] = ["Milan", "triib"];
        const UNITS: [&str; 8] = ["kHz", "Hz", "Mb/s", "Gb/s", "kb/s", "µs", "ns", "ms"];
        text.lines()
            .filter(|line| {
                let line = line.trim_start();
                !(line.starts_with('[') || line.starts_with("*["))
            })
            .flat_map(|line| {
                line.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | 'µ')))
            })
            .map(|word| word.trim_matches(|c| c == '.' || c == '/'))
            .filter(|word| {
                let capitals = word.chars().filter(char::is_ascii_uppercase).count();
                let starts_small = word.chars().next().is_some_and(|c| c.is_ascii_lowercase());
                NAMES.contains(word)
                    || UNITS.contains(word)
                    || (word.is_ascii() && (capitals >= 2 || (starts_small && capitals >= 1)))
            })
            .map(str::to_owned)
            .collect()
    }

    fn languages() -> Vec<String> {
        let mut languages: Vec<String> = std::fs::read_dir(root())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().join("triib.ftl").is_file())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        languages.sort();
        languages
    }

    #[test]
    fn norwegian_asks_for_bokmal() {
        let asked: Vec<LanguageIdentifier> = vec!["no-NO".parse().unwrap(), "en".parse().unwrap()];
        let tags: Vec<String> = with_aliases(asked).iter().map(|l| l.to_string()).collect();
        assert_eq!(tags, ["no-NO", "nb-NO", "en"]);
    }

    #[test]
    fn every_language_loads() {
        for language in languages() {
            let id: LanguageIdentifier = language.parse().expect("folder names are language tags");
            let loader = fluent_language_loader!();
            loader
                .load_languages(&Localizations, &[id])
                .unwrap_or_else(|error| panic!("{language}: {error}"));
        }
    }

    /// Translations may lack keys, which then show in English, but may not
    /// have keys English lacks (renamed or removed ones), values English
    /// does not provide, or the standards' words and units changed.
    #[test]
    fn translations_match_english() {
        let english = messages("en");
        assert!(!english.is_empty());
        for language in languages().into_iter().filter(|language| language != "en") {
            let translated = messages(&language);
            assert!(
                translated.contains_key("language-name"),
                "{language}: `language-name` names the language in the Settings list"
            );
            for (key, text) in &translated {
                let Some(source) = english.get(key) else {
                    panic!("{language}: `{key}` is not in the English file");
                };
                let unknown: Vec<_> = variables(text)
                    .difference(&variables(source))
                    .cloned()
                    .collect();
                assert!(
                    unknown.is_empty(),
                    "{language}: `{key}` uses {unknown:?}, which English does not provide"
                );
                if key == "language-name" {
                    continue;
                }
                let lost: Vec<_> = kept_words(source)
                    .difference(&kept_words(text))
                    .cloned()
                    .collect();
                assert!(
                    lost.is_empty(),
                    "{language}: `{key}` loses {lost:?}, which stay as written in every language"
                );
            }
            let missing = english
                .keys()
                .filter(|key| !translated.contains_key(*key))
                .count();
            if missing > 0 {
                println!(
                    "{language}: {missing} of {} keys not translated yet",
                    english.len()
                );
            }
        }
    }

    /// Count messages render in every language and pick the forms their
    /// plural rules give: in Russian, 2 and 3 take one form and 5 another.
    /// Larger counts show their number; small ones may say it in words,
    /// as Arabic says one and two.
    #[test]
    fn counts_pick_their_plural_forms() {
        let render = |language: &str, key: &str, count: i64| {
            let loader = fluent_language_loader!();
            let id: LanguageIdentifier = language.parse().unwrap();
            loader.load_languages(&Localizations, &[id]).unwrap();
            loader.set_use_isolating(false);
            let mut args = std::collections::HashMap::new();
            args.insert("count", count);
            loader.get_args(key, args)
        };
        for language in languages() {
            for key in [
                "status-entities",
                "matrix-hidden",
                "netmap-passing-count",
                "presets-connections",
            ] {
                for count in [0, 1, 2, 5, 21] {
                    let text = render(&language, key, count);
                    assert!(
                        (count < 3 || text.contains(&count.to_string()))
                            && !text.trim().is_empty()
                            && !text.contains("No localization"),
                        "{language}: `{key}` with {count} gave {text:?}"
                    );
                }
            }
        }
        let russian = |count| render("ru", "status-entities", count);
        assert_eq!(russian(2).replace('2', "3"), russian(3));
        assert_ne!(russian(2).replace('2', "5"), russian(5));
    }

    #[test]
    fn capitals_follow_the_language() {
        assert_eq!(uppercase_in("tr", "gPTP ağacı içinde"), "GPTP AĞACI İÇİNDE");
        assert_eq!(uppercase_in("el", "εκτός δέντρου"), "ΕΚΤΟΣ ΔΕΝΤΡΟΥ");
        assert_eq!(uppercase_in("en", "not on the tree"), "NOT ON THE TREE");
        assert_eq!(capitalized("talker 2"), "Talker 2");
    }

    #[test]
    fn the_standards_words_and_units_are_found() {
        let words = kept_words(
            "Milan 1.3 over AVB with gPTP, 48 kHz at 100 Mb/s, 3 µs; AECP said so in triibで",
        );
        let expected: BTreeSet<String> =
            ["Milan", "AVB", "gPTP", "kHz", "Mb/s", "µs", "AECP", "triib"]
                .into_iter()
                .map(str::to_owned)
                .collect();
        assert_eq!(words, expected);
    }
}
