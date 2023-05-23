use std::{net::SocketAddr, time::Duration,iter::repeat_with, io::Error,path::Path, fs};

use log::error;
use tokio_native_tls::native_tls::{TlsConnector, Certificate};
use openssl::ssl::{SslMethod, SslContext};
use statistics::stats_task;
use sender::{sender_task_udp, sender_task_dtls, sender_task_tcp};
use tokio::{net::{UdpSocket, TcpSocket, TcpStream}, time::sleep, task::JoinSet, io::AsyncWrite};
use derive_new::new;
use tokio_dtls_stream_sink::{Session, Client};
// use tokio_rustls::{rustls::{ClientConfig, RootCertStore, OwnedTrustAnchor, ServerName}, TlsConnector};
// use webpki::TrustAnchor;

mod statistics;
mod sender;


pub async fn manager(params: Parameters) {
    let (udp, (use_tls, ca_file)) = params.connection_type;
    if use_tls && ca_file.is_none() {
        error!("DTLS requires CA file to verify server credentials");
        return ;
    }

    let stats_tx = stats_task(params.connections);

    let mut tasks = JoinSet::new();
    let mut start_port = params.start_port; 

    for id in 0..params.connections {
        let payload = generate_payloads(params.len);
        let stats_tx_cloned = stats_tx.clone();
        let ca_file= ca_file.clone();
        if use_tls {
            if udp {
                let session = setup_dtls_session(start_port, params.server_addr, ca_file.unwrap()).await;
                tasks.spawn(async move {
                    sender_task_dtls(id, session, payload, params.rate, stats_tx_cloned).await
                });
            } else {
                let stream = setup_tls_stream(start_port, params.server_addr, ca_file.unwrap()).await;
                tasks.spawn(async move {
                    sender_task_tcp(id, stream, payload, params.rate, stats_tx_cloned).await;

                });

            }

        } else if udp {
            let socket = setup_udp_socket(params.server_addr, start_port).await;
            tasks.spawn(async move {
                sender_task_udp(id, socket, payload, params.rate, stats_tx_cloned).await
            });
        } else {
            let stream = setup_tcp_stream(params.server_addr, start_port).await;
            tasks.spawn(async move {
                sender_task_tcp(id, Box::new(stream), payload, params.rate, stats_tx_cloned).await;
            });
        }
            
        
        start_port+=1;
        sleep(Duration::from_millis(params.sleep)).await;
    }
    while (tasks.join_next().await).is_some() {

    }
}

async fn setup_udp_socket(addr: SocketAddr,port: usize) -> UdpSocket{
    let socket = UdpSocket::bind("0.0.0.0:".to_owned() + &port.to_string()).await.unwrap();
    socket.connect(addr).await.unwrap();
    socket
}

async fn setup_tcp_stream(addr: SocketAddr,port: usize) -> TcpStream {
    let local_addr = ("0.0.0.0:".to_owned() + &port.to_string()).parse().unwrap();
    let socket = TcpSocket::new_v4().unwrap();
    socket.bind(local_addr).unwrap();
    socket.connect(addr).await.unwrap()
}

async fn setup_dtls_session(port: usize, addr: SocketAddr, ca_file: String) -> DtlsSession {
    let mut ctx = SslContext::builder(SslMethod::dtls()).unwrap();
    ctx.set_ca_file(ca_file).unwrap();
    let socket = UdpSocket::bind("0.0.0.0:".to_owned() + &port.to_string()).await.unwrap();
    let client = Client::new(socket);
    let session = client.connect(addr, Some(ctx.build())).await.unwrap();
    DtlsSession::new(client,session)
}

async fn setup_tls_stream(port: usize, addr: SocketAddr, ca_file: String) -> Box<dyn AsyncWrite + Unpin + Send> {
    
    let pem = fs::read(Path::new(&ca_file)).unwrap();
    let cert = Certificate::from_pem(&pem).unwrap();
    let connector = TlsConnector::builder()
        .add_root_certificate(cert)
        .danger_accept_invalid_hostnames(true)
        .build()
        .unwrap();
    let connector = tokio_native_tls::TlsConnector::from(connector);
    let tcp_stream = setup_tcp_stream(addr, port).await;
    Box::new(connector.connect(addr.ip().to_string().as_str(), tcp_stream).await.unwrap())
    
}

fn generate_payloads(len: usize) -> Vec<u8>{
    repeat_with(|| fastrand::u8(..)).take(len).collect()
}

#[derive(new)]
pub struct Parameters {
    server_addr: SocketAddr,
    rate: usize,
    connections: usize,
    len: usize,
    start_port: usize,
    sleep: u64,
    connection_type: (bool,(bool, Option<String>))
}

#[derive(new)]
pub struct DtlsSession {
    _client: Client,
    session: Session
}

impl DtlsSession {
    pub async fn write(&mut self, buf: &[u8]) ->Result<(), Error> {
        self.session.write(buf).await
    }
}
