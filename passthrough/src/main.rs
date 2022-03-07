
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::collections::HashMap;
use pnet::datalink::{linux, NetworkInterface};
use std::thread;
use std::str::FromStr;

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

            let mut srv_thread = lb.clone();
            let _t = thread::spawn(move || {
                run_server(&mut srv_thread);
            });
        }
    }
}

pub fn run_server(lb: &mut LB) {

    let interface = match find_interface(lb.listen_ip) {
        Some(interface) => {
            interface
        },
        None => {
            return;
        }
    };

    let _iface = interface.clone();

    //process_packets(iface);
}

pub fn find_interface(addr: Ipv4Addr) -> Option<NetworkInterface> {

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
