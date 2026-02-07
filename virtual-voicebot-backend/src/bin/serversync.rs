use std::sync::Arc;

use anyhow::Context;
use virtual_voicebot_backend::interface::db::PostgresAdapter;
use virtual_voicebot_backend::interface::sync::OutboxWorker;
use virtual_voicebot_backend::shared::{config::SyncConfig, logging};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init();

    let sync_config = SyncConfig::from_env()?;
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

    let postgres_adapter = Arc::new(PostgresAdapter::new(database_url).await?);
    let outbox_worker = OutboxWorker::new(postgres_adapter, sync_config.clone())?;

    log::info!(
        "[serversync] started (poll_interval={}s, batch_size={})",
        sync_config.poll_interval_sec,
        sync_config.batch_size
    );

    outbox_worker.run().await;
    Ok(())
}
