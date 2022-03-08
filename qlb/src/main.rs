#![feature(maybe_uninit_uninit_array, maybe_uninit_slice)]

use std::error::Error;
use std::net::SocketAddr;
use socket2::{Socket, Protocol, Domain, Type};
use std::net::UdpSocket;

use std::mem::MaybeUninit;
use std::time::Duration;

pub const MAX_LEN: usize = 1300;

fn parse_packet(buf: [MaybeUninit<u8>; MAX_LEN]) {

    let pkt: [u8; MAX_LEN] = unsafe {
        std::mem::transmute::<_, [u8; MAX_LEN]>(buf)
    };

    if pkt.len() < 21 {
        return;
    }
    let long_hdr = (pkt[0] & 0b10000000) >> 7;
    if long_hdr == 1 {
        println!("long_hdr? {:#010b}", pkt[0]);
    }

    let dcid_len: usize = pkt[5].into();
    println!("DCID_Length: {}", dcid_len);

    for i in 6..6+dcid_len {
        println!("DCID: {} {:#010b}", i-6, pkt[i]);
    }
    //let mut _x = Pin::new(&buf[i]).get_ref();
    //let _y = buf.first_mut().unwrap();
    //let _x = unsafe { &*buf.as_ptr() };
    //let _y: u8 = _x.first_ptr();
}

fn forward(buf: [MaybeUninit<u8>; MAX_LEN]) -> Vec<u8> {

    let pkt: [u8; MAX_LEN] = unsafe {
        std::mem::transmute::<_, [u8; MAX_LEN]>(buf)
    };

    let remote: SocketAddr = "127.0.0.1:4444".parse()
        .expect("failed to parse addr");

    let client = UdpSocket::bind("127.0.0.1:0")
        .expect("failed to bind socket");

    client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    client.set_write_timeout(Some(Duration::from_secs(2))).unwrap();

    client.send_to(&pkt, &remote)
        .expect("failed to send data");

    let mut recv_buf = [0u8; MAX_LEN];

    loop {
        match client.recv_from(&mut recv_buf) {
            Ok((size, peer)) => {
                println!("received from backend: {}, {}", size, peer);
                //break;
                //let resp: Vec<u8> = recv_buf[..size].into()
                let resp: Vec<u8> = recv_buf[..size].to_vec();
                return resp;
            },
            Err(_e) => {
                println!("timeout");
                //break;
            },
        }
    }
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
                let resp = forward(recv_buf);

                let _amt = sock.send_to(&resp, &peer);
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
