use std::time::Duration;

use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use tokio::time::{interval, MissedTickBehavior};

use crate::interface::sync::converters::{
    apply_frontend_snapshot, default_anonymous_action, default_default_action, CallActionsPayload,
    CallerGroup, ConverterError, IvrFlowDefinition, StoredAction,
};
use crate::shared::config::SyncConfig;

const FRONTEND_PULL_MAX_CONNECTIONS: u32 = 5;

#[derive(Clone)]
pub struct FrontendPullWorker {
    pool: PgPool,
    http_client: reqwest::Client,
    frontend_base_url: String,
    poll_interval_sec: u64,
}

#[derive(Debug, Error)]
pub enum FrontendPullError {
    #[error("database failed: {0}")]
    DatabaseFailed(#[from] sqlx::Error),
    #[error("http failed: {0}")]
    HttpFailed(#[from] reqwest::Error),
    #[error("frontend response invalid: {0}")]
    InvalidResponse(String),
    #[error("converters failed: {0}")]
    ConverterFailed(#[from] ConverterError),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NumberGroupsResponse {
    ok: bool,
    #[serde(default)]
    caller_groups: Vec<CallerGroup>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CallActionsResponse {
    ok: bool,
    #[serde(default)]
    rules: Vec<crate::interface::sync::converters::IncomingRule>,
    anonymous_action: Option<StoredAction>,
    default_action: Option<StoredAction>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IvrFlowsResponse {
    ok: bool,
    #[serde(default)]
    flows: Vec<IvrFlowDefinition>,
    error: Option<String>,
}

impl FrontendPullWorker {
    pub async fn new(database_url: String, config: SyncConfig) -> Result<Self, FrontendPullError> {
        let timeout = Duration::from_secs(config.timeout_sec);
        let http_client = reqwest::Client::builder().timeout(timeout).build()?;
        let pool = PgPoolOptions::new()
            .max_connections(FRONTEND_PULL_MAX_CONNECTIONS)
            .connect(&database_url)
            .await?;
        Ok(Self {
            pool,
            http_client,
            frontend_base_url: config.frontend_base_url.trim_end_matches('/').to_string(),
            poll_interval_sec: config.frontend_poll_interval_sec,
        })
    }

    pub async fn run(&self) {
        log::info!(
            "[serversync] frontend pull worker started (poll_interval={}s)",
            self.poll_interval_sec
        );
        let mut ticker = interval(Duration::from_secs(self.poll_interval_sec));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            if let Err(error) = self.process_once().await {
                log::warn!("[serversync] frontend pull failed: {}", error);
            }
        }
    }

    pub async fn process_once(&self) -> Result<(), FrontendPullError> {
        log::info!("[serversync] frontend pull started");
        let groups = self.fetch_number_groups().await?;
        log::info!(
            "[serversync] GET /api/number-groups: success groups={}",
            groups.len()
        );
        let actions = self.fetch_call_actions().await?;
        log::info!(
            "[serversync] GET /api/call-actions: success rules={}",
            actions.rules.len()
        );
        let flows = self.fetch_ivr_flows_export().await?;
        log::info!(
            "[serversync] GET /api/ivr-flows/export: success flows={}",
            flows.len()
        );

        self.save_snapshot(&groups, &actions, &flows).await?;
        log::info!("[serversync] frontend pull saved");
        Ok(())
    }

    async fn fetch_number_groups(&self) -> Result<Vec<CallerGroup>, FrontendPullError> {
        let url = format!("{}/api/number-groups", self.frontend_base_url);
        let response = self.http_client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/number-groups returned status {}",
                status
            )));
        }
        let body: NumberGroupsResponse = response.json().await?;
        if !body.ok {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/number-groups returned ok=false{}",
                body.error
                    .as_ref()
                    .map(|value| format!(" ({value})"))
                    .unwrap_or_default()
            )));
        }
        Ok(body.caller_groups)
    }

    async fn fetch_call_actions(&self) -> Result<CallActionsPayload, FrontendPullError> {
        let url = format!("{}/api/call-actions", self.frontend_base_url);
        let response = self.http_client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/call-actions returned status {}",
                status
            )));
        }
        let body: CallActionsResponse = response.json().await?;
        if !body.ok {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/call-actions returned ok=false{}",
                body.error
                    .as_ref()
                    .map(|value| format!(" ({value})"))
                    .unwrap_or_default()
            )));
        }
        Ok(CallActionsPayload {
            rules: body.rules,
            anonymous_action: body
                .anonymous_action
                .unwrap_or_else(default_anonymous_action),
            default_action: body.default_action.unwrap_or_else(default_default_action),
        })
    }

    async fn fetch_ivr_flows_export(&self) -> Result<Vec<IvrFlowDefinition>, FrontendPullError> {
        let url = format!("{}/api/ivr-flows/export", self.frontend_base_url);
        let response = self.http_client.get(url).send().await?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Err(FrontendPullError::InvalidResponse(
                "GET /api/ivr-flows/export returned 404".to_string(),
            ));
        }
        if !status.is_success() {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/ivr-flows/export returned status {}",
                status
            )));
        }
        let body: IvrFlowsResponse = response.json().await?;
        if !body.ok {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/ivr-flows/export returned ok=false{}",
                body.error
                    .as_ref()
                    .map(|value| format!(" ({value})"))
                    .unwrap_or_default()
            )));
        }
        Ok(body.flows)
    }

    async fn save_snapshot(
        &self,
        groups: &[CallerGroup],
        actions: &CallActionsPayload,
        flows: &[IvrFlowDefinition],
    ) -> Result<(), FrontendPullError> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        if let Err(error) = apply_frontend_snapshot(&mut tx, groups, actions, flows).await {
            tx.rollback().await?;
            return Err(FrontendPullError::ConverterFailed(error));
        }
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CallActionsResponse, NumberGroupsResponse};

    #[test]
    fn number_groups_response_accepts_camel_case_fields() {
        let raw = r#"{"ok":true,"callerGroups":[{"id":"11111111-1111-4111-8111-111111111111","name":"grp","phoneNumbers":["+819012345678"]}]}"#;
        let parsed: NumberGroupsResponse = serde_json::from_str(raw).expect("valid response");
        assert!(parsed.ok);
        assert_eq!(parsed.caller_groups.len(), 1);
    }

    #[test]
    fn call_actions_response_can_omit_fallback_actions() {
        let raw = r#"{"ok":true,"rules":[]}"#;
        let parsed: CallActionsResponse = serde_json::from_str(raw).expect("valid response");
        assert!(parsed.ok);
        assert!(parsed.anonymous_action.is_none());
        assert!(parsed.default_action.is_none());
    }
}
