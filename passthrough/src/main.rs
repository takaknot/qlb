#![feature(maybe_uninit_uninit_array, maybe_uninit_slice)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
//use std::collections::HashMap;
use std::str::FromStr;

use pnet::datalink::{linux, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{FanoutOption, FanoutType};

use pnet::packet::Packet;
use pnet::packet::udp;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::MutableIpv4Packet;
use pnet::packet::ipv4;
use pnet::packet::udp::MutableUdpPacket;

use pnet::transport::transport_channel;
use pnet::transport::TransportChannelType::Layer3;

use pnet::packet::ethernet::MutableEthernetPacket;
use pnet::util::MacAddr;

pub const ETHER_SIZE: usize = 128; //42;

#[derive(Clone)]
pub struct LB {
    pub listen_ip: Ipv4Addr,
    pub listen_port: u16,
    pub iface: NetworkInterface,
}

impl LB {

    pub fn new() -> Option<LB> {

        let listen_addr: SocketAddr = FromStr::from_str("172.16.0.1:4433")
            .ok()
            .expect("Failed to parse listen host:port string");

        /*
        let mut backend_servers = HashMap::new();

        let addr1: SocketAddr = FromStr::from_str("10.25.96.4:4433")
            .ok()
            .expect("");

        backend_servers.insert(addr1, 100);
        */

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

    let d_addr: Ipv4Addr = "172.16.0.2".parse()
        .expect("failed to parse addr");

    /*
    let d_mac: MacAddr = "ff:ff:ff:ff:ff:ff".parse()
        .expect("failed to parse mac-address");

    let mut buf_eth:[u8; ETHER_SIZE] = [0u8; ETHER_SIZE];
    */

    loop {
        match iface_rx.next() {
            Ok(frame) => {
                let recv_pkt = EthernetPacket::new(frame).unwrap();

                match recv_pkt.get_ethertype() {
                    EtherTypes::Ipv4 => {

                        /*
                        let mut send_pkt = MutableEthernetPacket::new(&mut buf_eth).unwrap();
                        
                        send_pkt.set_ethertype(recv_pkt.get_ethertype());
                        send_pkt.set_source(recv_pkt.get_source());
                        send_pkt.set_destination(d_mac);
                        send_pkt.set_payload(recv_pkt.payload());
                        
                        println!("---- recv_pkt: {:?}", recv_pkt);
                        println!("---- send_pkt: {:?}", send_pkt);
                        */

                        match MutableIpv4Packet::owned(recv_pkt.payload().to_owned()) {
                            Some(mut ip_hdr) => {

                                if ip_hdr.get_destination() == lb.listen_ip {

                                    ip_hdr.set_destination(d_addr);
                                    ip_hdr.set_checksum(ipv4::checksum(&ip_hdr.to_immutable()));

                                    match MutableUdpPacket::owned(ip_hdr.payload().to_owned()) {
                                        Some(mut udp_hdr) => {

                                            println!("ether: {:?}", recv_pkt);
                                            println!("ip: {:?}", ip_hdr);
                                            println!("udp {:?}", udp_hdr);

                                            udp_hdr.set_checksum(udp::ipv4_checksum(
                                                    &udp_hdr.to_immutable(),
                                                    &ip_hdr.get_source(),
                                                    &ip_hdr.get_destination(),));

                                            match ipv4_tx.send_to(
                                                ip_hdr.to_immutable(), // udp_hdr.to_immutable(),
                                                IpAddr::V4(ip_hdr.get_destination()),
                                                ) {
                                                Ok(_) => {
                                                    println!("sent data {:?}", udp_hdr);
                                                },
                                                Err(e) => println!("{}", e),
                                            }
                                        },
                                        None => {
                                            println!("-- failed to parse UDP Packet");
                                        },
                                    }
                                }
                            },
                            None => {
                                println!("failed to parse IPv4 Packet");
                            },
                        }
                    },
                    _ => {}
                }
            },
            Err(e) => {
                println!("recv error: {:?}", e);
            },
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
                //println!("found iface {:?}", iface);
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
