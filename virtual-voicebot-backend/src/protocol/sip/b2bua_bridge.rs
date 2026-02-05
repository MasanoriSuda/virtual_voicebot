use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use tokio::sync::mpsc;

use crate::protocol::sip::message::{SipMessage, SipResponse};
use crate::protocol::sip::tx::{SipTransportRequest, SipTransportTx};
use crate::protocol::transport::TransportPeer;

#[derive(Debug, Clone)]
pub struct B2buaSipMessage {
    pub peer: TransportPeer,
    pub message: SipMessage,
}

#[derive(Debug)]
pub struct B2buaRegistration {
    call_id: String,
}

impl Drop for B2buaRegistration {
    fn drop(&mut self) {
        unregister(self.call_id.as_str());
    }
}

struct BridgeState {
    transport_tx: Option<SipTransportTx>,
    sip_port: u16,
    sessions: HashMap<String, mpsc::Sender<B2buaSipMessage>>,
}

static BRIDGE: OnceLock<Mutex<BridgeState>> = OnceLock::new();

fn state() -> &'static Mutex<BridgeState> {
    BRIDGE.get_or_init(|| {
        Mutex::new(BridgeState {
            transport_tx: None,
            sip_port: 0,
            sessions: HashMap::new(),
        })
    })
}

pub fn init(transport_tx: SipTransportTx, sip_port: u16) {
    let mut bridge = state().lock().unwrap();
    bridge.transport_tx = Some(transport_tx);
    bridge.sip_port = sip_port;
}

const B2BUA_CHANNEL_CAPACITY: usize = 32;

pub fn register(call_id: String) -> (B2buaRegistration, mpsc::Receiver<B2buaSipMessage>) {
    let (tx, rx) = mpsc::channel(B2BUA_CHANNEL_CAPACITY);
    let mut bridge = state().lock().unwrap();
    bridge.sessions.insert(call_id.clone(), tx);
    (B2buaRegistration { call_id }, rx)
}

fn unregister(call_id: &str) {
    let mut bridge = state().lock().unwrap();
    bridge.sessions.remove(call_id);
}

pub fn send(peer: TransportPeer, payload: Vec<u8>) -> bool {
    let (tx, sip_port) = {
        let bridge = state().lock().unwrap();
        (bridge.transport_tx.clone(), bridge.sip_port)
    };
    let Some(tx) = tx else {
        return false;
    };
    tx.try_send(SipTransportRequest {
        peer,
        src_port: sip_port,
        payload,
    })
    .is_ok()
}

pub fn dispatch_message(peer: TransportPeer, message: &SipMessage) -> bool {
    let Some(call_id) = message_call_id(message) else {
        return false;
    };
    let sender = {
        let bridge = state().lock().unwrap();
        bridge.sessions.get(call_id).cloned()
    };
    let Some(sender) = sender else {
        return false;
    };
    if let Err(err) = sender.try_send(B2buaSipMessage {
        peer,
        message: message.clone(),
    }) {
        log::warn!(
            "[sip] b2bua message dropped (channel full/closed) call_id={} err={:?}",
            call_id,
            err
        );
    }
    true
}

fn message_call_id(message: &SipMessage) -> Option<&str> {
    match message {
        SipMessage::Request(req) => req.header_value("Call-ID"),
        SipMessage::Response(resp) => response_header_value(resp, "Call-ID"),
    }
}

fn response_header_value<'a>(resp: &'a SipResponse, name: &str) -> Option<&'a str> {
    resp.headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}
