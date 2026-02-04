pub fn mask_pii(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    let len = trimmed.chars().count();
    format!("<redacted len={}>", len)
}

pub fn mask_phone(value: &str) -> String {
    mask_pii(value)
}
