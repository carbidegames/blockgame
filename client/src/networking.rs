use {
    mio::{
        net::{UdpSocket},
        Events, Ready, Poll, PollOpt, Token,
    },
};

pub fn send_msg() {
    const WRITE: Token = Token(1);

    let write_socket = UdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap();
    write_socket.connect("127.0.0.1:25566".parse().unwrap()).unwrap();

    let poll = Poll::new().unwrap();
    poll.register(&write_socket, WRITE, Ready::writable(), PollOpt::edge()).unwrap();

    let mut events = Events::with_capacity(128);
    loop {
        poll.poll(&mut events, None).unwrap();
        for event in events.iter() {
            match event.token() {
                WRITE => {
                    write_socket.send(&[0, 1, 2, 3]).unwrap();
                    return
                }
                _ => unreachable!()
            }
        }
    }
}
