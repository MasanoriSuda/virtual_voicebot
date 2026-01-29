use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{anyhow, Result};
use chrono::{DateTime, FixedOffset};
use reqwest::Client;

pub type NotificationFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

pub trait NotificationPort: Send + Sync {
    fn notify_ringing(&self, from: String, timestamp: DateTime<FixedOffset>) -> NotificationFuture;
    fn notify_missed(&self, from: String, timestamp: DateTime<FixedOffset>) -> NotificationFuture;
    fn notify_ended(&self, from: String, duration_sec: u64) -> NotificationFuture;
}

#[derive(Clone, Debug, Default)]
pub struct NoopNotification;

impl NoopNotification {
    pub fn new() -> Self {
        Self
    }
}

impl NotificationPort for NoopNotification {
    fn notify_ringing(
        &self,
        _from: String,
        _timestamp: DateTime<FixedOffset>,
    ) -> NotificationFuture {
        Box::pin(async move { Ok(()) })
    }

    fn notify_missed(
        &self,
        _from: String,
        _timestamp: DateTime<FixedOffset>,
    ) -> NotificationFuture {
        Box::pin(async move { Ok(()) })
    }

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
    pub fn new(token: String, user_id: String) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
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
                .await?;

            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(anyhow!("LINE push failed {}: {}", status, body));
            }
            Ok(())
        })
    }

    fn format_timestamp(timestamp: DateTime<FixedOffset>) -> String {
        timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

impl NotificationPort for LineAdapter {
    fn notify_ringing(&self, from: String, timestamp: DateTime<FixedOffset>) -> NotificationFuture {
        let from = if from.trim().is_empty() {
            "unknown".to_string()
        } else {
            from
        };
        let text = format!(
            "着信\n発信者: {}\n時刻: {}",
            from,
            Self::format_timestamp(timestamp)
        );
        self.push_message(text)
    }

    fn notify_missed(&self, from: String, timestamp: DateTime<FixedOffset>) -> NotificationFuture {
        let from = if from.trim().is_empty() {
            "unknown".to_string()
        } else {
            from
        };
        let text = format!(
            "不在着信\n発信者: {}\n時刻: {}",
            from,
            Self::format_timestamp(timestamp)
        );
        self.push_message(text)
    }

    fn notify_ended(&self, from: String, duration_sec: u64) -> NotificationFuture {
        let from = if from.trim().is_empty() {
            "unknown".to_string()
        } else {
            from
        };
        let minutes = duration_sec / 60;
        let seconds = duration_sec % 60;
        let text = format!(
            "通話終了\n発信者: {}\n通話時間: {:02}:{:02}",
            from, minutes, seconds
        );
        self.push_message(text)
    }
}
