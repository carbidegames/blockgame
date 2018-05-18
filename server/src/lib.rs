#[macro_use] extern crate slog;
extern crate udpcon;

use {
    std::{thread, time::{Duration}},
    slog::{Logger},
    udpcon::{Peer, Event},
};

pub const PROTOCOL: &str = concat!("blockgame-", env!("CARGO_PKG_VERSION"));

pub fn run(log: &Logger) {
    info!(log, "Starting Server");

    let address = "0.0.0.0:25566".parse().unwrap();
    let mut server = Peer::start(Some(address), PROTOCOL);

    loop {
        for event in server.poll() {
            match event {
                Event::Packet { source, data } =>
                    info!(log, "Data: {:?} from {}", data, source),
                Event::NewPeer { address } =>
                    info!(log, "Client Connected: {}", address),
                Event::PeerTimedOut { address } =>
                    info!(log, "Client Disconnected: {}", address),
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
}
