//! Prints the frames of one ethertype heard on an interface, to see what
//! a platform lets a program hear.
//!
//! cargo run -p avb-net --example listen -- <interface> <ethertype> [seconds] [group]

use std::time::{Duration, Instant};

use avb_net::{MacAddress, Socket};

fn main() -> std::io::Result<()> {
    let mut arguments = std::env::args().skip(1);
    let usage = "usage: listen <interface> <ethertype in hex> [seconds] [group MAC]";
    let interface = arguments.next().expect(usage);
    let ethertype =
        u16::from_str_radix(arguments.next().expect(usage).trim_start_matches("0x"), 16)
            .expect(usage);
    let seconds: u64 = arguments
        .next()
        .map_or(5, |text| text.parse().expect(usage));
    let socket = Socket::open(&interface, ethertype)?;
    if let Some(group) = arguments.next() {
        let group: MacAddress = group.parse().expect(usage);
        socket.join_multicast(group)?;
    }
    let started = Instant::now();
    let until = started + Duration::from_secs(seconds);
    let mut buffer = [0; 1500];
    let mut count = 0;
    while let Some(left) = until.checked_duration_since(Instant::now()) {
        if let Some(received) = socket.receive(&mut buffer, Some(left))? {
            count += 1;
            let shown = received.length.min(16);
            println!(
                "{:8.3}s  {}  {}{} octets  {:02x?}",
                started.elapsed().as_secs_f64(),
                received.source,
                if received.group { "group, " } else { "" },
                received.length,
                &buffer[..shown]
            );
        }
    }
    println!("{count} frames");
    Ok(())
}
