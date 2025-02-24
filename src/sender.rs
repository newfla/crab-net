use coarsetime::{Duration, Instant};
use kanal::AsyncSender;
use log::debug;
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    net::UdpSocket,
    time::sleep,
};

use crate::{DtlsSession, statistics::StatPacket};

pub async fn sender_task_udp(
    id: usize,
    socket: UdpSocket,
    payload: Vec<u8>,
    rate: usize,
    stats_tx: AsyncSender<StatPacket>,
) {
    debug!("client {id} spawned");
    let one_sec = Duration::new(1, 0);

    loop {
        let start_time = Instant::now();
        let mut packets_error = 0;

        for _ in 0..rate {
            if socket.send(&payload).await.is_err() {
                packets_error += 1;
            }
        }

        send_stats(rate, payload.len(), packets_error, &stats_tx).await;
        maybe_sleep(start_time, one_sec).await;
    }
}

pub async fn sender_task_dtls(
    id: usize,
    mut session: DtlsSession,
    payload: Vec<u8>,
    rate: usize,
    stats_tx: AsyncSender<StatPacket>,
) {
    debug!("client {id} spawned");
    let one_sec = Duration::new(1, 0);

    loop {
        let start_time = Instant::now();
        let mut packets_error = 0;

        for _ in 0..rate {
            if session.write(&payload).await.is_err() {
                packets_error += 1;
            }
        }

        send_stats(rate, payload.len(), packets_error, &stats_tx).await;
        maybe_sleep(start_time, one_sec).await;
    }
}

pub async fn sender_task_tcp(
    id: usize,
    mut stream: Box<dyn AsyncWrite + Unpin + Send>,
    payload: Vec<u8>,
    rate: usize,
    stats_tx: AsyncSender<StatPacket>,
) {
    debug!("client {id} spawned");
    let one_sec = Duration::new(1, 0);

    loop {
        let start_time = Instant::now();
        let mut packets_error = 0;

        for _ in 0..rate {
            if stream.write_all(&payload).await.is_err() {
                packets_error += 1;
            }
        }

        send_stats(rate, payload.len(), packets_error, &stats_tx).await;
        maybe_sleep(start_time, one_sec).await;
    }
}

async fn send_stats(
    rate: usize,
    payload_len: usize,
    packets_error: usize,
    stats_tx: &AsyncSender<StatPacket>,
) {
    let packets_sent = rate - packets_error;
    let _ = stats_tx
        .send((packets_sent * payload_len, packets_sent))
        .await;
}

async fn maybe_sleep(start_time: Instant, duration: Duration) {
    let time_elapsed = Instant::now() - start_time;

    if time_elapsed < duration {
        let time_to_sleep = duration - time_elapsed;
        sleep(time_to_sleep.into()).await;
    }
}
