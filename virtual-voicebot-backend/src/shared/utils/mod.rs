use std::path::Path;

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

pub fn is_safe_announcement_url_path(url_path: &str) -> bool {
    let Some(rest) = url_path.strip_prefix("/audio/announcements/") else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    !rest.contains('/')
        && rest != "."
        && rest != ".."
        && !rest.contains('%')
        && !rest.contains('\\')
}

pub fn map_audio_file_url_to_cache_path(audio_dir: &str, audio_file_url: &str) -> Option<String> {
    let url_path = extract_url_path(audio_file_url);
    if !is_safe_announcement_url_path(&url_path) {
        return None;
    }
    let filename = url_path
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())?;
    let path = Path::new(audio_dir).join(filename);
    Some(path.to_string_lossy().to_string())
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
