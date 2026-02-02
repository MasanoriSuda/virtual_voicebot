use std::fmt;

/// SIP Call-ID に対応
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallId(String);

impl CallId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CallId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 内部セッション識別子
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(uuid::Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

/// 録音識別子
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordingId(uuid::Uuid);

impl RecordingId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}
