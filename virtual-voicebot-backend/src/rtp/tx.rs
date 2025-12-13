use std::net::SocketAddr;

use tokio::net::UdpSocket;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub enum RtpTxCommand {
    Start { dst: SocketAddr },
    Stop,
    Send { payload: Vec<u8> },
}

#[derive(Clone)]
pub struct RtpTxHandle {
    tx: UnboundedSender<RtpTxCommand>,
}

impl RtpTxHandle {
    pub fn new() -> Self {
        let (tx, rx) = unbounded_channel();
        tokio::spawn(async move { run_tx(rx).await });
        Self { tx }
    }

    pub fn start(&self, dst: SocketAddr) {
        let _ = self.tx.send(RtpTxCommand::Start { dst });
    }

    pub fn stop(&self) {
        let _ = self.tx.send(RtpTxCommand::Stop);
    }

    pub fn send(&self, payload: Vec<u8>) {
        let _ = self.tx.send(RtpTxCommand::Send { payload });
    }
}

async fn run_tx(mut rx: UnboundedReceiver<RtpTxCommand>) {
    let mut sock: Option<UdpSocket> = None;
    let mut dst: Option<SocketAddr> = None;

    while let Some(cmd) = rx.recv().await {
        match cmd {
            RtpTxCommand::Start { dst: new_dst } => {
                dst = Some(new_dst);
                if sock.is_none() {
                    match UdpSocket::bind("0.0.0.0:0").await {
                        Ok(s) => sock = Some(s),
                        Err(e) => {
                            log::warn!("[rtp tx] failed to bind RTP socket: {e:?}");
                            dst = None;
                        }
                    }
                }
            }
            RtpTxCommand::Stop => {
                dst = None;
                sock = None;
            }
            RtpTxCommand::Send { payload } => {
                if let (Some(s), Some(target)) = (sock.as_ref(), dst) {
                    let _ = s.send_to(&payload, target).await.ok();
                } else {
                    log::warn!("[rtp tx] send requested but RTP not started");
                }
            }
        }
    }
}
