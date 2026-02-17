use crate::protocol::transport::TransportPeer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteTxState {
    Proceeding,
    Completed,
    Confirmed,
    Terminated,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum InviteTxAction {
    Retransmit(Vec<u8>),
    Timeout,
}

/// INVITE サーバトランザクションの簡易実装（UDP前提）。
/// - 2xx 送信時は即 Terminated（2xx 再送は SipCore で ACK 到着まで管理）
/// - 3xx–6xx の再送や Timer G/H/I の詳細は後続で拡張予定
pub struct InviteServerTransaction {
    pub state: InviteTxState,
    pub peer: TransportPeer,
    last_provisional: Option<Vec<u8>>,
    last_final: Option<Vec<u8>>,
    pub invite_req: Option<crate::protocol::sip::SipRequest>,
}

impl InviteServerTransaction {
    pub fn new(peer: TransportPeer) -> Self {
        Self {
            state: InviteTxState::Proceeding,
            peer,
            last_provisional: None,
            last_final: None,
            invite_req: None,
        }
    }

    pub fn remember_provisional(&mut self, resp: Vec<u8>) {
        self.last_provisional = Some(resp);
    }

    pub fn on_final_sent(&mut self, resp: Vec<u8>, status: u16) {
        self.last_final = Some(resp);
        if status >= 300 {
            self.state = InviteTxState::Completed;
        } else {
            self.state = InviteTxState::Terminated;
        }
    }

    pub fn on_retransmit(&self) -> Option<InviteTxAction> {
        match self.state {
            InviteTxState::Proceeding => self
                .last_provisional
                .as_ref()
                .cloned()
                .map(InviteTxAction::Retransmit),
            InviteTxState::Completed => self
                .last_final
                .as_ref()
                .cloned()
                .map(InviteTxAction::Retransmit),
            _ => None,
        }
    }

    pub fn on_ack(&mut self) -> Option<InviteTxAction> {
        if self.state == InviteTxState::Completed {
            self.state = InviteTxState::Confirmed;
            // Timer I → Terminated を予定。ここでは即時に Terminated にしておく。
            self.state = InviteTxState::Terminated;
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonInviteTxState {
    Trying,
    Completed,
    Terminated,
}

/// 非 INVITE サーバトランザクション（簡易版）。最終応答の再送と Timer J 相当の期限管理のみ。
pub struct NonInviteServerTransaction {
    pub state: NonInviteTxState,
    #[allow(dead_code)]
    #[allow(dead_code)]
    pub peer: TransportPeer,
    pub last_request: Option<crate::protocol::sip::SipRequest>,
    pub last_final: Option<Vec<u8>>,
    pub expires_at: std::time::Instant,
}

impl NonInviteServerTransaction {
    pub fn new(peer: TransportPeer, req: crate::protocol::sip::SipRequest) -> Self {
        Self {
            state: NonInviteTxState::Trying,
            peer,
            last_request: Some(req),
            last_final: None,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(32),
        }
    }

    pub fn on_final_sent(&mut self, resp: Vec<u8>) {
        self.last_final = Some(resp);
        self.state = NonInviteTxState::Completed;
        self.expires_at = std::time::Instant::now() + std::time::Duration::from_secs(32);
    }

    pub fn on_retransmit(&self) -> Option<Vec<u8>> {
        match self.state {
            NonInviteTxState::Completed => self.last_final.clone(),
            _ => None,
        }
    }
}
