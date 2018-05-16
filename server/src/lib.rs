#[macro_use] extern crate slog;
extern crate mio;

use {
    slog::{Logger},
    mio::{
        net::{UdpSocket},
        Events, Ready, Poll, PollOpt, Token,
    },
};

pub fn run(log: &Logger) {
    // Remember, recommended UDP packet size is: 512

    info!(log, "Starting Server");

    const READ: Token = Token(1);

    let read_socket = UdpSocket::bind(&"0.0.0.0:25566".parse().unwrap()).unwrap();

    let poll = Poll::new().unwrap();
    poll.register(&read_socket, READ, Ready::readable(), PollOpt::edge()).unwrap();

    let mut buffer = [0; 4];

    let mut events = Events::with_capacity(128);
    loop {
        poll.poll(&mut events, None).unwrap();
        for event in events.iter() {
            match event.token() {
                READ => {
                    let from = read_socket.recv_from(&mut buffer).unwrap().1;
                    println!("Data {:?} from {:?}", buffer, from);
                }
                _ => unreachable!()
            }
        }
    }
}
