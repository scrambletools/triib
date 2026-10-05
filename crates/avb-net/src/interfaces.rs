//! The Ethernet interfaces of this computer and what each can do.

use crate::MacAddress;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub mac: MacAddress,
    /// The link is up.
    pub up: bool,
    /// Link speed in Mb/s, when the driver reports it.
    pub speed: Option<u32>,
    /// Backed by hardware, not a bridge, tunnel or other virtual device.
    pub physical: bool,
    pub wireless: bool,
    /// Index of the PTP hardware clock, on interfaces that timestamp in
    /// hardware.
    pub hardware_clock: Option<u32>,
}

/// The Ethernet interfaces, physical and up first, then by name.
pub fn interfaces() -> Vec<Interface> {
    let mut found = platform::interfaces();
    found.sort_by(|left, right| {
        (!left.physical, !left.up, &left.name).cmp(&(!right.physical, !right.up, &right.name))
    });
    found
}

/// The MAC address of the interface named `name`.
pub fn mac_of(name: &str) -> Option<MacAddress> {
    platform::interfaces()
        .into_iter()
        .find(|interface| interface.name == name)
        .map(|interface| interface.mac)
}

/// The kernel's index for the interface named `name`.
#[cfg(target_os = "linux")]
pub(crate) fn index_of(name: &str) -> Option<i32> {
    if name.contains('/') {
        return None;
    }
    std::fs::read_to_string(format!("/sys/class/net/{name}/ifindex"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

#[cfg(target_os = "linux")]
mod platform {
    use std::fs;
    use std::path::Path;

    use super::Interface;

    /// `ARPHRD_ETHER`, the link type of Ethernet and Wi-Fi interfaces.
    const LINK_TYPE_ETHERNET: u32 = 1;

    pub fn interfaces() -> Vec<Interface> {
        let Ok(entries) = fs::read_dir("/sys/class/net") else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter_map(|entry| read(&entry.path(), entry.file_name().to_str()?))
            .collect()
    }

    fn read(dir: &Path, name: &str) -> Option<Interface> {
        let value = |file: &str| fs::read_to_string(dir.join(file)).ok();
        let link_type: u32 = value("type")?.trim().parse().ok()?;
        if link_type != LINK_TYPE_ETHERNET {
            return None;
        }
        let mac = value("address")?.trim().parse().ok()?;
        Some(Interface {
            name: name.to_owned(),
            mac,
            up: value("operstate").is_some_and(|state| state.trim() == "up"),
            // Negative or unreadable while the link is down.
            speed: value("speed").and_then(|speed| speed.trim().parse().ok()),
            physical: dir.join("device").exists(),
            wireless: dir.join("wireless").exists() || dir.join("phy80211").exists(),
            hardware_clock: hardware_clock(dir),
        })
    }

    /// The `ptpN` the driver registered for the interface, as `ethtool -T`
    /// reports: under the PCI device for most drivers (igb, igc, mlx5),
    /// under the interface itself for some (atlantic).
    pub(super) fn hardware_clock(dir: &Path) -> Option<u32> {
        let clocks = |folder: std::path::PathBuf| {
            fs::read_dir(folder)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|entry| {
                    entry
                        .file_name()
                        .to_str()?
                        .strip_prefix("ptp")?
                        .parse::<u32>()
                        .ok()
                })
        };
        clocks(dir.join("device/ptp"))
            .chain(clocks(dir.to_path_buf()))
            .min()
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use super::Interface;

    /// Not listed on this system yet.
    pub fn interfaces() -> Vec<Interface> {
        Vec::new()
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn lists_without_loopback() {
        assert!(interfaces().iter().all(|found| found.name != "lo"));
    }

    #[test]
    fn finds_clocks_under_the_device_or_the_interface() {
        let root = std::env::temp_dir().join(format!("avb-net-clocks-{}", std::process::id()));
        let device = root.join("igc0");
        let own = root.join("atlantic0");
        let none = root.join("r8169");
        for folder in [
            device.join("device/ptp/ptp2"),
            own.join("ptp0"),
            none.join("device"),
        ] {
            std::fs::create_dir_all(folder).unwrap();
        }
        // An entry that only starts like a clock is not one.
        std::fs::create_dir_all(own.join("ptpx")).unwrap();
        assert_eq!(platform::hardware_clock(&device), Some(2));
        assert_eq!(platform::hardware_clock(&own), Some(0));
        assert_eq!(platform::hardware_clock(&none), None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
