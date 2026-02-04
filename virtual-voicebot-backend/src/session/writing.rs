#![allow(dead_code)]
// wiring.rs（上位＝SIP/メディア/Botとの結線）
//
// 責務: Session 内部の状態機械と、外部モジュール (sip/rtp/app) への出口を結線する。
// ここでは経路だけ定義し、実際の送信/受信はまだスタブのまま（挙動は従来どおり）。
use std::sync::Arc;

use crate::ports::app::AppEventTx;
use crate::ports::ingest::IngestPort;
use crate::ports::storage::StoragePort;
use crate::rtp::tx::RtpTxHandle;
use crate::session::types::*;
use crate::session::{Session, SessionHandle};
use crate::config::SessionRuntimeConfig;

/// セッションを生成し、SessionOut を上位レイヤに配線する（挙動は従来と同じ）。
pub fn spawn_call(
    call_id: CallId,
    from_uri: String,
    to_uri: String,
    media_cfg: MediaConfig,
    session_out_tx: tokio::sync::mpsc::Sender<(CallId, SessionOut)>,
    app_tx: AppEventTx,
    rtp_tx: RtpTxHandle,
    ingest_url: Option<String>,
    recording_base_url: Option<String>,
    ingest_port: Arc<dyn IngestPort>,
    storage_port: Arc<dyn StoragePort>,
    runtime_cfg: Arc<SessionRuntimeConfig>,
) -> SessionHandle {
    let handle = Session::spawn(
        call_id.clone(),
        from_uri,
        to_uri,
        session_out_tx,
        app_tx,
        media_cfg,
        rtp_tx,
        ingest_url,
        recording_base_url,
        ingest_port,
        storage_port,
        runtime_cfg,
    );

    handle
}

pub async fn spawn_session(
    call_id: CallId,
    from_uri: String,
    to_uri: String,
    registry: SessionRegistry,
    media_cfg: MediaConfig,
    session_out_tx: tokio::sync::mpsc::Sender<(CallId, SessionOut)>,
    app_tx: AppEventTx,
    rtp_tx: RtpTxHandle,
    ingest_url: Option<String>,
    recording_base_url: Option<String>,
    ingest_port: Arc<dyn IngestPort>,
    storage_port: Arc<dyn StoragePort>,
    runtime_cfg: Arc<SessionRuntimeConfig>,
) -> tokio::sync::mpsc::Sender<SessionIn> {
    let handle = spawn_call(
        call_id.clone(),
        from_uri,
        to_uri,
        media_cfg,
        session_out_tx,
        app_tx,
        rtp_tx,
        ingest_url,
        recording_base_url,
        ingest_port,
        storage_port,
        runtime_cfg,
    );
    // Session manager の薄いラッパ経由で登録
    registry.insert(call_id, handle.tx_in.clone()).await;
    handle.tx_in
}
