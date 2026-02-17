use std::sync::Arc;

use anyhow::Context;
use virtual_voicebot_backend::interface::db::PostgresAdapter;
use virtual_voicebot_backend::interface::sync::{FrontendPullWorker, OutboxWorker};
use virtual_voicebot_backend::shared::{config::SyncConfig, logging};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init();

    let sync_config = SyncConfig::from_env()?;
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

    let postgres_adapter = Arc::new(PostgresAdapter::new(database_url.clone()).await?);
    let outbox_worker = OutboxWorker::new(postgres_adapter, sync_config.clone())?;
    let frontend_pull_worker = FrontendPullWorker::new(database_url, sync_config.clone()).await?;

    log::info!(
        "[serversync] started (outbox_poll_interval={}s, frontend_pull_poll_interval={}s, batch_size={})",
        sync_config.poll_interval_sec,
        sync_config.frontend_poll_interval_sec,
        sync_config.batch_size
    );

    let outbox_task = {
        let worker = outbox_worker.clone();
        tokio::spawn(async move {
            worker.run().await;
        })
    };
    let frontend_pull_task = {
        let worker = frontend_pull_worker.clone();
        tokio::spawn(async move {
            worker.run().await;
        })
    };

    tokio::signal::ctrl_c().await?;
    log::info!("[serversync] shutdown signal received");
    outbox_task.abort();
    frontend_pull_task.abort();

    Ok(())
}
