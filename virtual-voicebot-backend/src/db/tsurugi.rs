use std::time::Duration;

use anyhow::anyhow;
use log::warn;
use tsubakuro_rust_core::prelude::{
    CommitOption, ConnectionOption, Session, SqlClient, SqlQueryResultFetch, TransactionOption,
};

use crate::ports::phone_lookup::{
    PhoneLookupError, PhoneLookupFuture, PhoneLookupPort, PhoneLookupResult,
};

const LOOKUP_TIMEOUT: Duration = Duration::from_secs(5);

pub struct TsurugiAdapter {
    endpoint: String,
}

impl TsurugiAdapter {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    async fn lookup_phone_inner(
        &self,
        phone_number: &str,
    ) -> anyhow::Result<Option<PhoneLookupResult>> {
        let mut connection_option = ConnectionOption::new();
        connection_option
            .set_endpoint_url(self.endpoint.as_str())
            .map_err(|e| anyhow!("tsurugi endpoint: {}", e))?;
        connection_option.set_application_name("virtual-voicebot");
        connection_option.set_session_label("phone_lookup");
        connection_option.set_default_timeout(LOOKUP_TIMEOUT);

        let session = Session::connect(&connection_option)
            .await
            .map_err(|e| anyhow!("tsurugi connect: {}", e))?;
        let client: SqlClient = session.make_client();
        let transaction = client
            .start_transaction(&TransactionOption::default())
            .await
            .map_err(|e| anyhow!("tsurugi start transaction: {}", e))?;

        let sql = format!(
            "SELECT ivr_enabled FROM phone_entries WHERE phone_number = '{}'",
            escape_sql_literal(phone_number)
        );
        let mut query_result = client
            .query(&transaction, sql.as_str())
            .await
            .map_err(|e| anyhow!("tsurugi query: {}", e))?;

        let mut ivr_enabled = None;
        if query_result
            .next_row()
            .await
            .map_err(|e| anyhow!("tsurugi next_row: {}", e))?
        {
            if query_result
                .next_column()
                .await
                .map_err(|e| anyhow!("tsurugi next_column: {}", e))?
            {
                let value: Option<i32> = query_result
                    .fetch()
                    .await
                    .map_err(|e| anyhow!("tsurugi fetch: {}", e))?;
                ivr_enabled = Some(value.unwrap_or(0) != 0);
            }
        }

        if let Err(err) = query_result.close().await {
            warn!("[tsurugi] query_result close error: {}", err);
        }
        if let Err(err) = client.commit(&transaction, &CommitOption::default()).await {
            warn!("[tsurugi] commit error: {}", err);
        }
        if let Err(err) = transaction.close().await {
            warn!("[tsurugi] transaction close error: {}", err);
        }
        if let Err(err) = session.close().await {
            warn!("[tsurugi] session close error: {}", err);
        }

        Ok(ivr_enabled.map(|ivr_enabled| PhoneLookupResult {
            phone_number: phone_number.to_string(),
            ivr_enabled,
        }))
    }
}

impl PhoneLookupPort for TsurugiAdapter {
    fn lookup_phone(&self, phone_number: String) -> PhoneLookupFuture {
        let endpoint = self.endpoint.clone();
        Box::pin(async move {
            let handle = tokio::task::spawn_blocking(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| anyhow!("tsurugi runtime: {}", e))?;
                runtime.block_on(async move {
                    let adapter = TsurugiAdapter { endpoint };
                    adapter.lookup_phone_inner(&phone_number).await
                })
            });

            let result = match tokio::time::timeout(LOOKUP_TIMEOUT, handle).await {
                Ok(join_result) => match join_result {
                    Ok(result) => result,
                    Err(err) => Err(anyhow!("tsurugi task: {}", err)),
                },
                Err(_) => Err(anyhow!("tsurugi lookup timed out")),
            };
            result.map_err(|e| PhoneLookupError::LookupFailed(e.to_string()))
        })
    }
}

fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}
