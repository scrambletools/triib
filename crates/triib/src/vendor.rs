//! Vendor names for the first three octets of a MAC address, from the
//! IEEE list a Linux system keeps, when it has one.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{Mutex, OnceLock};

/// Where distributions keep the list.
const LISTS: [&str; 3] = [
    "/usr/share/hwdata/oui.txt",
    "/usr/share/ieee-data/oui.txt",
    "/usr/share/misc/oui.txt",
];

static NAMES: OnceLock<Mutex<HashMap<[u8; 3], Option<String>>>> = OnceLock::new();

/// The company the IEEE assigned `oui` to, without its legal form, read
/// once and remembered.
pub fn name(oui: [u8; 3]) -> Option<String> {
    let names = NAMES.get_or_init(Mutex::default);
    if let Some(found) = names.lock().ok()?.get(&oui) {
        return found.clone();
    }
    let found = look_up(oui);
    names.lock().ok()?.insert(oui, found.clone());
    found
}

fn look_up(oui: [u8; 3]) -> Option<String> {
    let prefix = format!("{:02X}-{:02X}-{:02X}", oui[0], oui[1], oui[2]);
    LISTS.iter().find_map(|path| {
        let file = File::open(path).ok()?;
        BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .find_map(|line| {
                let name = line
                    .strip_prefix(&prefix)?
                    .trim_start()
                    .strip_prefix("(hex)")?;
                Some(short(name))
            })
    })
}

/// A company name without its legal form, as "Mark of the Unicorn, Inc."
/// becomes "Mark of the Unicorn".
fn short(name: &str) -> String {
    const FORMS: [&str; 12] = [
        "Inc.",
        "Inc",
        "LLC",
        "Ltd.",
        "Ltd",
        "Limited",
        "GmbH",
        "AG",
        "S.A.",
        "Corp.",
        "Corporation",
        "Co.",
    ];
    let mut name = name.trim();
    loop {
        let before = name;
        for form in FORMS {
            if let Some(rest) = name.strip_suffix(form)
                && rest.ends_with([' ', ','])
            {
                name = rest.trim_end_matches([' ', ',']);
            }
        }
        if name == before {
            return name.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_forms_go() {
        assert_eq!(short("Mark of the Unicorn, Inc."), "Mark of the Unicorn");
        assert_eq!(short("Example Co., Ltd."), "Example");
        assert_eq!(short("Acme"), "Acme");
        assert_eq!(short("Incorporated Things"), "Incorporated Things");
    }
}
