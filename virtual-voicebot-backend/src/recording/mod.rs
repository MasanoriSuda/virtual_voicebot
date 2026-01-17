use std::path::PathBuf;

pub const RECORDINGS_DIR: &str = "storage/recordings";

pub mod storage;

pub fn recording_dir_name(call_id: &str) -> String {
    call_id.to_string()
}

pub fn recording_dir(call_id: &str) -> PathBuf {
    PathBuf::from(RECORDINGS_DIR).join(recording_dir_name(call_id))
}

pub fn recording_url(base_url: &str, call_id: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{}/recordings/{}/mixed.wav", base, call_id)
}
