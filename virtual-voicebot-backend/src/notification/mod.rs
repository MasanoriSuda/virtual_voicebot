use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use reqwest::Client;

use crate::ports::notification::{
    CallEndedNotifier, MissedCallNotifier, NotificationError, NotificationFuture, RingingNotifier,
};

#[derive(Clone, Debug, Default)]
pub struct NoopNotification;

impl NoopNotification {
    /// Creates a new no-op notification sink.
    pub fn new() -> Self {
        Self
    }
}

impl RingingNotifier for NoopNotification {
    fn notify_ringing(
        &self,
        _from: String,
        _timestamp: DateTime<FixedOffset>,
    ) -> NotificationFuture {
        Box::pin(async move { Ok(()) })
    }
}

impl MissedCallNotifier for NoopNotification {
    fn notify_missed(
        &self,
        _from: String,
        _timestamp: DateTime<FixedOffset>,
    ) -> NotificationFuture {
        Box::pin(async move { Ok(()) })
    }
}

impl CallEndedNotifier for NoopNotification {
    fn notify_ended(&self, _from: String, _duration_sec: u64) -> NotificationFuture {
        Box::pin(async move { Ok(()) })
    }
}

pub struct LineAdapter {
    client: Client,
    user_id: String,
    token: String,
}

impl LineAdapter {
    pub fn new(token: String, user_id: String) -> Result<Self, NotificationError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| NotificationError::Failed(e.to_string()))?;
        Ok(Self {
            client,
            user_id,
            token,
        })
    }

    fn push_message(&self, text: String) -> NotificationFuture {
        let client = self.client.clone();
        let token = self.token.clone();
        let user_id = self.user_id.clone();
        Box::pin(async move {
            let resp = client
                .post("https://api.line.me/v2/bot/message/push")
                .bearer_auth(token)
                .json(&serde_json::json!({
                    "to": user_id,
                    "messages": [
                        {"type": "text", "text": text}
                    ]
                }))
                .send()
                .await
                .map_err(|e| NotificationError::Failed(e.to_string()))?;

            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(NotificationError::Failed(format!(
                    "LINE push failed {}: {}",
                    status, body
                )));
            }
            Ok(())
        })
    }

    fn format_timestamp(timestamp: DateTime<FixedOffset>) -> String {
        timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

impl RingingNotifier for LineAdapter {
    fn notify_ringing(&self, from: String, timestamp: DateTime<FixedOffset>) -> NotificationFuture {
        let text = format!(
            "着信: {} ({})",
            if from.trim().is_empty() {
                "unknown"
            } else {
                from.as_str()
            },
            Self::format_timestamp(timestamp)
        );
        self.push_message(text)
    }
}

impl MissedCallNotifier for LineAdapter {
    fn notify_missed(&self, from: String, timestamp: DateTime<FixedOffset>) -> NotificationFuture {
        let text = format!(
            "不在着信: {} ({})",
            if from.trim().is_empty() {
                "unknown"
            } else {
                from.as_str()
            },
            Self::format_timestamp(timestamp)
        );
        self.push_message(text)
    }
}

impl CallEndedNotifier for LineAdapter {
    fn notify_ended(&self, from: String, duration_sec: u64) -> NotificationFuture {
        let text = format!(
            "通話終了: {} ({}秒)",
            if from.trim().is_empty() {
                "unknown"
            } else {
                from.as_str()
            },
            duration_sec
        );
        self.push_message(text)
    }
}
