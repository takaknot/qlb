#![feature(maybe_uninit_uninit_array, maybe_uninit_slice)]

use std::error::Error;
use std::net::SocketAddr;
use socket2::{Socket, Protocol, Domain, Type};

use std::mem::MaybeUninit;

pub const MAX_LEN: usize = 1024;

fn parse_packet(buf: [MaybeUninit<u8>; MAX_LEN]) {

    let pkt: [u8; MAX_LEN] = unsafe {
        std::mem::transmute::<_, [u8; MAX_LEN]>(buf)
    };

    if pkt.len() < 21 {
        return;
    }
    for i in 0..4 {
        println!("{:?}", pkt[i]);
    }

    //let mut _x = Pin::new(&buf[i]).get_ref();
    //let _y = buf.first_mut().unwrap();
    //let _x = unsafe { &*buf.as_ptr() };
    //let _y: u8 = _x.first_ptr();
}

async fn serve(i: usize) -> Result<(), std::io::Error> {

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

    let send_buf: Vec<u8> = vec![97; MAX_LEN];

    /*
    let mut recv_buf: [MaybeUninit<u8>; 1024] = unsafe {
        //MaybeUninit::uninit().assume_init()
        MaybeUninit::uninit_array()
    };
    */
    let mut recv_buf: [MaybeUninit<u8>; MAX_LEN] = MaybeUninit::uninit_array();

    loop {
        match sock.recv_from(&mut recv_buf) {
            Ok((size, peer)) => {
                println!("echo - proc: {} size: {}", i, size);

                parse_packet(recv_buf);

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
