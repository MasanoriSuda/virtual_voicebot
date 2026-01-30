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
    /// Creates a new no-op notification sink.
    ///
    /// This returns a default `NoopNotification` instance that performs no actions when notification
    /// methods are invoked.
    ///
    /// # Examples
    ///
    /// ```
    /// let n = NoopNotification::new();
    /// let m = NoopNotification::default();
    /// assert_eq!(format!("{:?}", n), format!("{:?}", m));
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl NotificationPort for NoopNotification {
    /// Handle a ringing event without performing any action.
    ///
    /// This no-op implementation accepts a sender and timestamp but does not send any notification
    /// and always resolves successfully.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::executor;
    /// use chrono::{Utc, FixedOffset};
    /// let notifier = crate::NoopNotification::new();
    /// let ts = Utc::now().with_timezone(&FixedOffset::east(0));
    /// executor::block_on(notifier.notify_ringing("alice".to_string(), ts)).unwrap();
    /// ```
    /* no outer attributes */
    fn notify_ringing(
        &self,
        _from: String,
        _timestamp: DateTime<FixedOffset>,
    ) -> NotificationFuture {
        Box::pin(async move { Ok(()) })
    }

    /// No-op handler for missed-call notifications that ignores the event.
    ///
    /// # Returns
    ///
    /// The returned future resolves to `Ok(())`.
    ///
    /// # Examples
    ///
    /// ```
    /// let adapter = crate::NoopNotification::new();
    /// let ts = chrono::FixedOffset::east(0).ymd(2020, 1, 1).and_hms(0, 0, 0);
    /// let res = futures::executor::block_on(adapter.notify_missed("alice".into(), ts));
    /// assert!(res.is_ok());
    /// ```
    fn notify_missed(
        &self,
        _from: String,
        _timestamp: DateTime<FixedOffset>,
    ) -> NotificationFuture {
        Box::pin(async move { Ok(()) })
    }

    /// Notify that a call has ended for a given sender and duration.
    ///
    /// Returns a future that resolves to `Ok(())` on success.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::executor::block_on;
    ///
    /// let n = NoopNotification::new();
    /// let res = block_on(n.notify_ended("alice".to_string(), 90));
    /// assert!(res.is_ok());
    /// ```
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
    /// Creates a new LineAdapter configured with the given channel token and target user ID.
    ///
    /// The adapter is initialized with an HTTP client that has a 5-second timeout. Returns an
    /// error if building the underlying HTTP client fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let adapter = LineAdapter::new("token".to_string(), "user_id".to_string()).unwrap();
    /// let _ = adapter;
    /// ```
    pub fn new(token: String, user_id: String) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        Ok(Self {
            client,
            user_id,
            token,
        })
    }

    /// Send a plain-text message to the configured LINE user using the Messaging API.
    ///
    /// The returned future resolves to `Ok(())` when the HTTP request succeeds (2xx). It
    /// resolves to an `Err` if the request fails or the API returns a non-success HTTP status;
    /// the error includes the status and response body when available.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use notification::{LineAdapter};
    /// # async fn run() -> anyhow::Result<()> {
    /// let adapter = LineAdapter::new("TOKEN".into(), "USER_ID".into())?;
    /// adapter.push_message("Hello from LineAdapter".into()).await?;
    /// # Ok(()) }
    /// ```
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

    /// Formats a timestamp as "YYYY-MM-DD HH:MM:SS".
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{DateTime, FixedOffset};
    /// let ts = DateTime::parse_from_rfc3339("2023-01-02T15:04:05+09:00").unwrap();
    /// assert_eq!(format_timestamp(ts), "2023-01-02 15:04:05");
    /// ```
    fn format_timestamp(timestamp: DateTime<FixedOffset>) -> String {
        timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

impl NotificationPort for LineAdapter {
    /// Send a LINE push notification for an incoming call.
    ///
    /// The `from` value is normalized to `"unknown"` when it is empty or only whitespace.
    /// The message includes the sender and the formatted timestamp.
    ///
    /// # Parameters
    ///
    /// - `from`: Sender identifier; empty or whitespace-only values become `"unknown"`.
    /// - `timestamp`: Time when the incoming call was observed.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` on success, or an error containing the HTTP status and response body on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{DateTime, FixedOffset};
    /// # use futures::executor::block_on;
    /// # // Create adapter (token and user_id are placeholders)
    /// let adapter = LineAdapter::new("token".to_string(), "user_id".to_string()).unwrap();
    /// let timestamp: DateTime<FixedOffset> = "2025-01-01T12:00:00+00:00".parse().unwrap();
    /// let fut = adapter.notify_ringing("Alice".to_string(), timestamp);
    /// // run the future in a synchronous test harness
    /// let _ = block_on(fut);
    /// ```
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

    /// Send a missed-call notification for the given sender at the specified timestamp.
    ///
    /// Empty or whitespace-only `from` values are normalized to `"unknown"`. The notification
    /// message includes the sender and a formatted timestamp.
    ///
    /// # Parameters
    ///
    /// - `from`: the caller identifier to include in the notification; may be empty.
    /// - `timestamp`: the time of the missed call; formatted as `YYYY-MM-DD HH:MM:SS`.
    ///
    /// # Returns
    ///
    /// A `NotificationFuture` that resolves to `Ok(())` when the push completes successfully,
    /// or an `Err` if sending the notification fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chrono::FixedOffset;
    /// use your_crate::NoopNotification;
    ///
    /// let notifier = NoopNotification::new();
    /// let ts = chrono::Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
    /// let fut = notifier.notify_missed("Alice".to_string(), ts);
    /// futures::executor::block_on(fut).unwrap();
    /// ```
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

    /// Sends a "call ended" notification containing the caller identifier and the call duration.
    ///
    /// # Parameters
    ///
    /// - `from`: Caller identifier; when empty or whitespace, it is treated as `"unknown"`.
    /// - `duration_sec`: Total call duration in seconds; formatted as `MM:SS` in the message.
    ///
    /// # Returns
    ///
    /// A future resolving to a `Result<(), anyhow::Error>` that indicates whether the notification was sent successfully.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use futures::executor::block_on;
    /// let adapter = LineAdapter::new("token".to_string(), "user".to_string()).unwrap();
    /// let fut = adapter.notify_ended("Alice".to_string(), 125);
    /// block_on(fut).unwrap();
    /// ```
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