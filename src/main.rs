
//use std::net::SocketAddr;
//use tokio::net::UDPSocket;
//use socket2::{Socket, Protocol, Domain, Type, SockAddr};
use socket2::{Socket, Protocol, Domain, Type};

async fn serve(_: usize) {

    let addr: std::net::SocketAddr = "0.0.0.0:4433".parse().unwrap();
    //let socket = Socket::new(domain, Type::dgram(), Some(Protocol::udp()))?;

    let socket = Socket::new(
        Domain::IPV4,
        Type::DGRAM,
        Some(Protocol::UDP),
    ).unwrap();
    /*
    let sock = socket2::Socket::new(
        match addr {
            SocketAddr::V4(_) => socket2::Domain::IPV4,
            SocketAddr::V6(_) => socket2::Domain::IPV6,
        },
        socket2::Type::STREAM,
        None,
    )
    .unwrap();
    */

    sock.set_reuse_address(true).unwrap();
    sock.set_reuse_port(true).unwrap();
    sock.set_nonblocking(true).unwrap();
    sock.bind(&addr.into()).unwrap();
    sock.listen(8192).unwrap();


    //sock.bind(&"127.0.0.1:4433".parse::<SocketAddr>().unwrap().into()).unwrap();
    //let _listener = sock.into_udp_socket();
}


fn main() {

    let mut handlers = Vec::new();

    for i in 0..10 {
        let h = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(serve(i));
        });

        handlers.push(h);
    }

    for h in handlers {
        h.join().unwrap();
    }
}
