extern crate nalgebra;
#[macro_use] extern crate slog;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate bincode;
extern crate udpcon;

pub mod message;

use {
    std::{
        collections::{HashMap},
        thread,
        time::{Duration}
    },

    nalgebra::{Vector2, Point3},
    slog::{Logger},
    udpcon::{Peer, Event, Reliability},

    message::{ClientMessage, ServerMessage, PlayerUpdate},
};

pub const PROTOCOL: &str = concat!("blockgame-", env!("CARGO_PKG_VERSION"));

pub fn run(log: &Logger) {
    info!(log, "Starting Server");

    let mut players = HashMap::new();

    let address = "0.0.0.0:25566".parse().unwrap();
    let mut peer = Peer::start(Some(address), PROTOCOL);

    const DESIRED_FPS: u32 = 30;
    const DELTA: f32 = 1.0 / DESIRED_FPS as f32;
    loop {
        for event in peer.poll() {
            match event {
                Event::NewPeer { address } => {
                    players.insert(address, Player::new());
                    info!(log, "Client Connected: {}", address)
                },
                Event::PeerTimedOut { address } => {
                    players.remove(&address);
                    info!(log, "Client Disconnected: {}", address)
                },
                Event::Message { source, data } => {
                    // TODO: Drop clients sending invalid packets
                    if let Some(message) = ClientMessage::deserialize(&data) {
                        match message {
                            ClientMessage::PlayerFrame(player_frame) =>
                                players.get_mut(&source).unwrap().input = player_frame.input,
                        }
                    }
                },
            }
        }

        for (address, player) in &mut players {
            const SPEED: f32 = 2.0;

            let mut input = player.input;
            if input.x != 0.0 || input.y != 0.0 {
                input = input.normalize();
            }

            player.position.x += input.x * DELTA * SPEED;
            player.position.z += input.y * DELTA * SPEED;

            let message = ServerMessage::PlayerUpdate(PlayerUpdate {
                position: player.position,
            });
            peer.send(*address, message.serialize(), Reliability::Sequenced).unwrap();
        }

        thread::sleep(Duration::from_millis((DELTA * 1000.0).floor() as u64));
    }
}

struct Player {
    input: Vector2<f32>,
    position: Point3<f32>,
}

impl Player {
    pub fn new() -> Self {
        Player {
            input: Vector2::new(0.0, 0.0),
            position: Point3::new(0.0, 40.0, 0.0),
        }
    }
}
