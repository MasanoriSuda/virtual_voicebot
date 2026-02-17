#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    pub uri: String,
    pub display_name: Option<String>,
}

impl Participant {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            display_name: None,
        }
    }
}
