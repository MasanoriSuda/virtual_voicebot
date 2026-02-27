pub fn extract_url_path(audio_file_url: &str) -> String {
    let trimmed = audio_file_url.trim();
    let without_fragment = trimmed.split('#').next().unwrap_or(trimmed);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    if let Some(scheme_sep) = without_query.find("://") {
        let after_scheme = &without_query[scheme_sep + 3..];
        if let Some(path_pos) = after_scheme.find('/') {
            return after_scheme[path_pos..].to_string();
        }
        return "/".to_string();
    }
    without_query.to_string()
}

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
