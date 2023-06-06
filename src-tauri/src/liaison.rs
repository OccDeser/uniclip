use crate::clipboard::Clipboard;
use crate::{debug, error, info, warn};

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

use crc::{Crc, CRC_16_IBM_SDLC};
use serde::{Deserialize, Serialize};

// use tokio::net::UdpSocket;
use tokio::sync::{Mutex, MutexGuard};
use tokio::task;
use tokio::time::sleep;

const OP_PING: u8 = 0x00;
const OP_PONG: u8 = 0x01;
const OP_DATA: u8 = 0x02;

#[derive(Serialize, Deserialize, Clone)]
struct PacketHeader {
    pub opcode: u8,
    pub length: u16,
}

#[derive(Serialize, Deserialize, Clone)]
struct PayloadMessage {
    pub data: Vec<u8>,
    pub crc: u16,
}

#[derive(Serialize, Deserialize, Clone)]
struct PeerV4 {
    pub host: u32,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Clone)]
struct PayloadPong {
    pub peers: Vec<PeerV4>,
}

pub struct Liaison {
    ip: u32,
    port: u16,
    socket: Arc<Mutex<UdpSocket>>,
    peers: Vec<PeerV4>,
    clipboard: Arc<Mutex<Clipboard>>,
}

impl Liaison {
    pub fn new(
        ip: &str,
        port: u16,
        clipboard: Arc<Mutex<Clipboard>>,
    ) -> Result<Arc<Mutex<Self>>, std::io::Error> {
        let host: Ipv4Addr = ip.parse::<Ipv4Addr>().unwrap();
        let addr: SocketAddr = SocketAddr::new(host.into(), port);
        let socket: UdpSocket = UdpSocket::bind(addr).unwrap();
        socket.set_write_timeout(Some(Duration::from_secs(1)))?;
        socket.set_read_timeout(Some(Duration::from_millis(50)))?;
        let socket: Arc<Mutex<UdpSocket>> = Arc::new(Mutex::new(socket));

        info!("Liaison started on {}:{}", ip, port);

        Ok(Arc::new(Mutex::new(Self {
            ip: u32::from(host),
            port,
            socket,
            peers: Vec::new(),
            clipboard,
        })))
    }

    async fn handle_ping(&mut self, src: SocketAddr) {
        info!("Ping from {}", src);
        let mut peers = Vec::new();
        for peer in self.peers.iter() {
            peers.push(peer.clone());
        }
        peers.push(PeerV4 {
            host: self.ip,
            port: self.port,
        });

        let pong: Vec<u8> = bincode::serialize(&PayloadPong { peers }).unwrap();
        let header: Vec<u8> = bincode::serialize(&PacketHeader {
            opcode: OP_PONG,
            length: pong.len() as u16,
        })
        .unwrap();
        let pkt: Vec<u8> = [header, pong].concat();

        let sock_lock: MutexGuard<UdpSocket> = self.socket.lock().await;
        sock_lock.send_to(pkt.as_slice(), src).unwrap();
        let ip = match src.ip() {
            IpAddr::V4(ip) => u32::from(ip),
            _ => panic!("IPv6 address not supported"),
        };
        drop(sock_lock);

        if self.ip == ip {
            return;
        }

        for peer in self.peers.iter() {
            if peer.host == ip {
                return;
            }
        }

        self.peers.push(PeerV4 {
            host: ip,
            port: self.port,
        });
    }

    fn handle_pong(&mut self, src: SocketAddr, payload: &[u8]) {
        info!("Pong from {}", src);
        let peers: PayloadPong = bincode::deserialize(&payload).unwrap();
        let mut new_peers = Vec::new();
        for remote_peer in peers.peers.iter() {
            if remote_peer.host == self.ip {
                continue;
            }
            
            let mut existed = false;
            for local_peer in self.peers.iter() {
                if remote_peer.host == local_peer.host && remote_peer.port == local_peer.port {
                    existed = true;
                    break;
                }
            }

            if existed {
                continue;
            }

            info!("Peer {}:{}", Ipv4Addr::from(remote_peer.host), remote_peer.port);
            new_peers.push(remote_peer.clone());
        }

        for peer in new_peers.iter() {
            self.peers.push(peer.clone());
        }
    }

    fn handle_data(&self, src: SocketAddr, payload: &[u8]) -> Vec<u8> {
        info!("Data from {}", src);
        let message: PayloadMessage = bincode::deserialize(&payload).unwrap();

        // check CRC
        const X25: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_SDLC);
        assert_eq!(X25.checksum(message.data.as_slice()), message.crc);

        message.data
    }

    async fn handle(this: Arc<Mutex<Self>>, data: Vec<u8>, src: SocketAddr) -> Option<Vec<u8>> {
        // read a packet
        let packet = data.as_slice();

        // deserialize the packet header
        let header = &packet[..3];
        let header: PacketHeader = bincode::deserialize(&header).unwrap();

        // get payload
        let payload: &[u8] = &packet[3..];
        assert_eq!(header.length, payload.len() as u16);

        let mut liaison = this.lock().await;
        let res = match header.opcode {
            OP_PING => {
                liaison.handle_ping(src).await;
                None
            }
            OP_PONG => {
                liaison.handle_pong(src, payload);
                None
            }
            OP_DATA => Some(liaison.handle_data(src, payload)),
            _ => {
                warn!("Unknown opcode {}", header.opcode);
                None
            }
        };
        drop(liaison);

        res
    }

    async fn send(
        this: Arc<Mutex<Self>>,
        data: &[u8],
        dest: SocketAddr,
    ) -> Result<usize, std::io::Error> {
        let liaison: MutexGuard<Liaison> = this.lock().await;
        debug!("fn send: lock Liaison");
        let socket: Arc<Mutex<UdpSocket>> = liaison.socket.clone();
        drop(liaison);
        debug!("fn send: free Liaison");

        let sock_lock: MutexGuard<UdpSocket> = socket.lock().await;
        debug!("fn send: lock UdpSocket");
        let res: Result<usize, std::io::Error> = sock_lock.send_to(data, dest);
        drop(sock_lock);
        debug!("fn send: free UdpSocket");

        res
    }

    async fn ping_all(this: Arc<Mutex<Self>>) {
        let ping: Vec<u8> = bincode::serialize(&PacketHeader {
            opcode: OP_PING,
            length: 0,
        })
        .unwrap();

        // 获取当前网段
        let liasion: MutexGuard<Liaison> = this.lock().await;
        debug!("fn ping_all: lock Liaison");
        let port = liasion.port;
        let ip_seg = liasion.ip & 0xffffff00;
        let ip_self = liasion.ip & 0xff;
        drop(liasion);
        debug!("fn ping_all: free Liaison");

        info!(
            "Ping all peers on {}:{}:{}:{}",
            ip_seg >> 24,
            (ip_seg >> 16) & 0xff,
            (ip_seg >> 8) & 0xff,
            ip_seg & 0xff
        );

        for i in 1..255 {
            if i == ip_self {
                continue;
            }
            let host: u32 = ip_seg | i;
            let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::from(host)), port);
            debug!("Ping {}", addr);
            let _ = Self::send(this.clone(), ping.as_slice(), addr).await;
        }
    }

    pub async fn broadcast(this: Arc<Mutex<Self>>, data: &[u8]) {
        const X25: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_SDLC);
        let crc: u16 = X25.checksum(data);
        let message: Vec<u8> = bincode::serialize(&PayloadMessage {
            crc,
            data: data.to_vec(),
        })
        .unwrap();
        let header = bincode::serialize(&PacketHeader {
            opcode: OP_DATA,
            length: message.len() as u16,
        })
        .unwrap();
        let message = [header, message].concat();
        let message_bytes = message.as_slice();

        let liasion: MutexGuard<Liaison> = this.lock().await;
        debug!("fn broadcast: lock Liaison");
        let peers: Vec<PeerV4> = liasion.peers.clone();
        drop(liasion);
        debug!("fn broadcast: free Liaison");

        for peer in peers.iter() {
            let addr: SocketAddr =
                SocketAddr::new(IpAddr::V4(Ipv4Addr::from(peer.host).into()), peer.port);
            match Self::send(this.clone(), message_bytes, addr).await {
                Ok(_) => (),
                Err(e) => {
                    error!("Broadcast error: {}", e);
                    info!("Remove peer {}", addr);
                    let mut liasion: MutexGuard<Liaison> = this.lock().await;
                    debug!("fn broadcast: lock Liaison");
                    liasion.peers.retain(|p| p.host != peer.host);
                    drop(liasion);
                    debug!("fn broadcast: free Liaison");
                }
            };
        }
    }

    pub async fn start(this: Arc<Mutex<Self>>) {
        let liaison: MutexGuard<Liaison> = this.lock().await;
        debug!("fn start: lock Liaison");
        let socket: Arc<Mutex<UdpSocket>> = liaison.socket.clone();
        drop(liaison);
        debug!("fn start: free Liaison");

        // 创建接收消息的任务
        let this_clone = Arc::clone(&this);
        let receiver_sock: Arc<Mutex<UdpSocket>> = Arc::clone(&socket);
        task::spawn(async move {
            let mut buf = [0u8; 65540];
            loop {
                // let mut recv_len: usize;
                // let mut src: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);
                let zero_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);

                let sock_lock: MutexGuard<UdpSocket> = receiver_sock.lock().await;
                debug!("fn start: lock UdpSocket");

                let (recv_len, src) = match sock_lock.recv_from(&mut buf) {
                    Ok((n, s)) => (n, s),
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (0, zero_addr),
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => (0, zero_addr),
                    Err(e) => {
                        error!("recv_from error: {}", e);
                        (0, zero_addr)
                    }
                };

                drop(sock_lock);
                debug!("fn start: free UdpSocket");

                if recv_len <= 0 {
                    sleep(Duration::from_secs(1)).await;
                } else {
                    let data: Vec<u8> = buf[..recv_len].to_vec();
                    let res: Option<Vec<u8>> =
                        Liaison::handle(Arc::clone(&this_clone), data, src).await;
                    if let Some(data) = res {
                        let liaison: MutexGuard<Liaison> = this_clone.lock().await;
                        debug!("fn start: lock Liaison");
                        let clipboard: Arc<Mutex<Clipboard>> = liaison.clipboard.clone();
                        drop(liaison);
                        debug!("fn start: free Liaison");

                        let mut clipboard_lock: MutexGuard<Clipboard> = clipboard.lock().await;
                        debug!("fn start: lock Clipboard");
                        clipboard_lock.append(data);
                        drop(clipboard_lock);
                        debug!("fn start: free Clipboard");
                    }
                }
            }
        });
        info!("Start listening...");

        Liaison::ping_all(Arc::clone(&this)).await;
    }

    // pub async fn stop(this: Arc<Mutex<Self>>) {}
}
