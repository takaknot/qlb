
use std::io;
use std::error::Error;
use std::net::SocketAddr;
use socket2::{Socket, Protocol, Domain, Type};

use std::mem::MaybeUninit;


async fn serve(i: usize) -> Result<(), io::Error> {

    let addr: SocketAddr = "0.0.0.0:4433".parse().unwrap();

    let sock = Socket::new(
        Domain::for_address(addr),
        Type::DGRAM,
        Some(Protocol::UDP),
    ).unwrap();

    sock.set_nonblocking(true).unwrap();
    sock.set_reuse_address(true).unwrap();
    sock.set_reuse_port(true).unwrap();
    //sock.listen(4).unwrap();
    
    sock.bind(&addr.into()).unwrap();

    let send_buf: Vec<u8> = vec![128; 1024];

    let mut recv_buf: [MaybeUninit<u8>; 1024] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    loop {
        match sock.recv_from(&mut recv_buf) {
            Ok((size, peer)) => {
                println!("echo - proc: {} size: {}", i, size);

                let _amt = sock.send_to(&send_buf[..size], &peer);
            },
            Err(_e) => {},
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let mut handlers = Vec::new();

    for i in 0..2 {
        let h = std::thread::spawn(move || {
            let _r = tokio::runtime::Builder::new_current_thread()
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

    Ok(())
}
