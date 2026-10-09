//! Prints what each interface does about EEE and PAUSE.

#[cfg(target_os = "linux")]
fn main() {
    for interface in avb_net::interfaces() {
        if let Ok(power) = avb_net::interfaces::link_power(&interface.name) {
            println!(
                "{}: EEE {}, PAUSE {}",
                interface.name, power.eee, power.pause
            );
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("only on Linux");
}
