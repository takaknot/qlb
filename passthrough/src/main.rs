#![feature(maybe_uninit_uninit_array, maybe_uninit_slice)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::collections::HashMap;
use std::str::FromStr;

use pnet::datalink::{linux, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{FanoutOption, FanoutType};

use pnet::packet::Packet;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::MutableIpv4Packet;
use pnet::packet::udp::MutableUdpPacket;

use pnet::transport::transport_channel;
use pnet::transport::TransportChannelType::Layer3;

use pnet::packet::ethernet::MutableEthernetPacket;
use pnet::util::MacAddr;

pub const ETHER_SIZE: usize = 42;

#[derive(Clone)]
pub struct LB {
    pub listen_ip: Ipv4Addr,
    pub listen_port: u16,
    pub iface: NetworkInterface,
}

impl LB {

    pub fn new() -> Option<LB> {

        let mut backend_servers = HashMap::new();

        let listen_addr: SocketAddr = FromStr::from_str("127.0.0.1:4433")
            .ok()
            .expect("Failed to parse listen host:port string");

        let addr1: SocketAddr = FromStr::from_str("127.0.0.2:4003")
            .ok()
            .expect("");

        backend_servers.insert(addr1, 100);

        match listen_addr.ip() {
            IpAddr::V4(ip4) => {

                let interface = match find_interface(ip4) {
                    Some(interface) => interface,
                    None => {
                        return None;
                    }
                };
                let new_lb = LB {
                    listen_ip: ip4,
                    listen_port: listen_addr.port(),
                    iface: interface,
                };
                Some(new_lb)
            }
            _ => {
                None
            }
        }
    }
}

pub struct Server {

    pub lbs: Vec<LB>,
}

impl Server {

    pub fn new() -> Server {
        let mut lbs = Vec::new();

        if let Some(new_lb) = LB::new() {
            lbs.push(new_lb);
        }

        Server {
            lbs: lbs,
        }
    }

    pub fn run(&mut self) {

        let xbs = self.lbs.clone();

        for lb in xbs.iter() {

            println!("listen-ip: {}", lb.listen_ip);

            let mut srv_thread = lb.clone();

            let t = std::thread::spawn(move || {
                run_server(&mut srv_thread);
            });

            t.join().unwrap();
        }
    }
}

fn run_server(lb: &mut LB) {

    println!("called run_server");

    let interface = match find_interface(lb.listen_ip) {
        Some(interface) => {
            interface
        },
        None => {
            return;
        }
    };

    let iface = interface.clone();
    
    let iface_cfg = setup_interface_cfg();
   
    let (_, mut iface_rx) = match linux::channel(&iface, iface_cfg) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!(
            "An error occurred when creating the datalink channel: {}",
            e
        ),
    };

    let proto = Layer3(IpNextHeaderProtocols::Udp);
    let (mut ipv4_tx, _) = transport_channel(4096, proto).unwrap();

    /*
    let d_addr: Ipv4Addr = "127.0.0.2".parse()
        .expect("failed to parse addr");
    */

    let d_mac: MacAddr = "ff:ff:ff:ff:ff:ff".parse()
        .expect("failed to parse mac-address");

    loop {

        match iface_rx.next() {
            Ok(frame) => {

                let recv_eth = EthernetPacket::new(frame).unwrap();

                let mut send_pkt:[u8; ETHER_SIZE] = [0u8; ETHER_SIZE];
                let mut send_eth = MutableEthernetPacket::new(&mut send_pkt).unwrap();
                send_eth.set_ethertype(recv_eth.get_ethertype());
                send_eth.set_source(recv_eth.get_source());
                send_eth.set_destination(d_mac);

                match recv_eth.get_ethertype() {
                    EtherTypes::Ipv4 => {
                        match MutableIpv4Packet::owned(send_eth.payload().to_owned()) {
                            //Some(mut ip_hdr) => {
                            Some(ip_hdr) => {

                                let dst = ip_hdr.get_destination();

                                println!("recv data - dst: {}", dst);

                                if dst == lb.listen_ip {
                                    match MutableUdpPacket::owned(ip_hdr.payload().to_owned()) {
                                        Some(mut udp_hdr) => {
                                            println!("ip_header: {:?}, udp_header: {:?}", ip_hdr, udp_hdr);

                                            //ether.set_destination(dmac);
                                            //ip_hdr.set_destination(d_addr);
                                            udp_hdr.set_destination(5555);

                                            println!("ip_header: {:?}, udp_header: {:?}", ip_hdr, udp_hdr);

                                            match ipv4_tx.send_to(
                                                ip_hdr.to_immutable(),
                                                IpAddr::V4(ip_hdr.get_destination()),
                                            ) {
                                                Ok(_) => {},
                                                Err(e) => println!("{}", e),
                                            }
                                        },
                                        None => {}
                                    }
                                }
                            },
                            None => {}
                        }
                    },
                    _ => {}
                }
            },
            Err(_) => {},
        }
    }
}

fn setup_interface_cfg() -> linux::Config {

    let fanout = Some(FanoutOption {
        group_id: rand::random::<u16>(),
        fanout_type: FanoutType::LB,
        defrag: true,
        rollover: false,
    });

    linux::Config {
        fanout,
        ..Default::default()
    }
}


fn find_interface(addr: Ipv4Addr) -> Option<NetworkInterface> {

    let ifaces = linux::interfaces();
    for iface in ifaces {
        for ip in iface.clone().ips {
            if ip.ip() == addr {
                return Some(iface);
            }
        }
    }
    None
}

fn main() {

    let mut loadbalancer = Server::new();
    loadbalancer.run();
}
