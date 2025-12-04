#![allow(dead_code)]
// session.rs
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::session::types::*;

#[derive(Clone)]
pub struct SessionHandle {
    pub tx_in: UnboundedSender<SessionIn>,
}

pub struct Session {
    state: SessState,
    call_id: String,
    peer_sdp: Option<Sdp>,
    local_sdp: Option<Sdp>,
    tx_up: UnboundedSender<SessionOut>,
    media_cfg: MediaConfig,
    // RTP送出用
    rtp_seq: u16,
    rtp_ts: u32,
    // バッファ/タイマ
    speaking: bool,
}

impl Session {
    pub fn new(
        call_id: String,
        tx_up: UnboundedSender<SessionOut>,
        media_cfg: MediaConfig,
    ) -> SessionHandle {
        let (tx_in, rx_in) = tokio::sync::mpsc::unbounded_channel();
        let mut s = Self {
            state: SessState::Idle,
            call_id,
            peer_sdp: None,
            local_sdp: None,
            tx_up,
            media_cfg,
            rtp_seq: 0,
            rtp_ts: 0,
            speaking: false,
        };
        tokio::spawn(async move { s.run(rx_in).await; });
        SessionHandle { tx_in }
    }

    async fn run(&mut self, mut rx: UnboundedReceiver<SessionIn>) {
        while let Some(ev) = rx.recv().await {
            match (self.state, ev) {
                (SessState::Idle, SessionIn::Invite { offer, .. }) => {
                    self.peer_sdp = Some(offer);
                    let answer = self.build_answer_pcmu8k();
                    self.local_sdp = Some(answer.clone());
                    let _ = self.tx_up.send(SessionOut::SendSip180);
                    let _ = self.tx_up.send(SessionOut::SendSip200 { answer });
                    self.state = SessState::Early;
                }
                (SessState::Early, SessionIn::Ack) => {
                    // 相手SDPからRTP宛先を確定して送信開始
                    let (ip, port) = self.peer_rtp_dst();
                    let _ = self.tx_up.send(SessionOut::StartRtpTx { dst_ip: ip, dst_port: port, pt: 0 }); // PCMU
                    self.state = SessState::Established;
                    // 例: 最初の発話をキック（固定文でOK）
                    let _ = self.tx_up.send(SessionOut::BotSynthesize { text: "はじめまして、ずんだもんです。".into() });
                }
                (SessState::Established, SessionIn::RtpIn { payload, .. }) => {
                    // 受信音声→VADで検出→Bot合成の割り込み制御など
                    // （MVPはログだけ）
                    let _ = self.tx_up.send(SessionOut::Metrics { name: "rtp_in", value: payload.len() as i64 });
                }
                (SessState::Established, SessionIn::BotAudio { pcm48k: _ }) => {
                    // 48k→8k→μ-law→RTPパケット化は下位メディア層に委譲してOK
                    // ここではTS/Seqの進行のみ示唆
                    self.rtp_ts = self.rtp_ts.wrapping_add(160);
                    // 実際の送出はメディア層スレッドへ
                }
                (_, SessionIn::Bye) => {
                    let _ = self.tx_up.send(SessionOut::StopRtpTx);
                    let _ = self.tx_up.send(SessionOut::SendSipBye200);
                    self.state = SessState::Terminated;
                }
                (_, SessionIn::Abort(e)) => {
                    eprintln!("call {} abort: {e:?}", self.call_id);
                    let _ = self.tx_up.send(SessionOut::StopRtpTx);
                    self.state = SessState::Terminated;
                }
                _ => { /* それ以外は無視 or ログ */ }
            }
        }
    }

    fn build_answer_pcmu8k(&self) -> Sdp {
        // PCMU/8000 でローカル SDP を組み立て
        Sdp::pcmu(self.media_cfg.local_ip.clone(), self.media_cfg.local_port)
    }

    fn peer_rtp_dst(&self) -> (String, u16) {
        if let Some(sdp) = &self.peer_sdp {
            (sdp.ip.clone(), sdp.port)
        } else {
            ("0.0.0.0".to_string(), 0)
        }
    }
}
