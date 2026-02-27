#[derive(Debug)]
pub(super) struct SentenceAccumulator {
    buf: String,
    max_chars: usize,
}

impl SentenceAccumulator {
    pub(super) fn new(max_chars: usize) -> Self {
        Self {
            buf: String::new(),
            max_chars: max_chars.max(1),
        }
    }

    pub(super) fn push(&mut self, token: &str) -> Option<String> {
        self.buf.push_str(token);

        if let Some(end_idx) = last_sentence_boundary(&self.buf) {
            let tail = self.buf.split_off(end_idx);
            let completed = std::mem::replace(&mut self.buf, tail);
            return normalize(completed);
        }

        if self.buf.chars().count() >= self.max_chars {
            return self.flush();
        }

        None
    }

    pub(super) fn flush(&mut self) -> Option<String> {
        if self.buf.is_empty() {
            return None;
        }
        let text = std::mem::take(&mut self.buf);
        normalize(text)
    }
}

fn last_sentence_boundary(s: &str) -> Option<usize> {
    let mut last = None;
    for (idx, ch) in s.char_indices() {
        if matches!(ch, '。' | '！' | '？' | '!' | '?' | '…' | '\n') {
            last = Some(idx + ch.len_utf8());
            continue;
        }
        if ch == '\r' {
            if s[idx..].starts_with("\r\n") {
                last = Some(idx + 2);
            } else {
                last = Some(idx + 1);
            }
            continue;
        }
        if ch == '.' && s[idx..].starts_with("...") {
            last = Some(idx + 3);
        }
    }
    last
}

fn normalize(text: String) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::SentenceAccumulator;

    #[test]
    fn flushes_on_japanese_sentence_end() {
        let mut acc = SentenceAccumulator::new(50);
        assert_eq!(acc.push("こんにちは"), None);
        assert_eq!(acc.push("。つづき"), Some("こんにちは。".to_string()));
        assert_eq!(acc.flush(), Some("つづき".to_string()));
    }

    #[test]
    fn flushes_on_max_chars() {
        let mut acc = SentenceAccumulator::new(3);
        assert_eq!(acc.push("ab"), None);
        assert_eq!(acc.push("c"), Some("abc".to_string()));
        assert_eq!(acc.flush(), None);
    }

    #[test]
    fn test_consecutive_sentence_boundaries() {
        let mut acc = SentenceAccumulator::new(50);
        assert_eq!(acc.push("はい！？続き"), Some("はい！？".to_string()));
        assert_eq!(acc.flush(), Some("続き".to_string()));
    }

    #[test]
    fn test_empty_token_push() {
        let mut acc = SentenceAccumulator::new(10);
        assert_eq!(acc.push(""), None);
        assert_eq!(acc.flush(), None);

        assert_eq!(acc.push("次"), None);
        assert_eq!(acc.flush(), Some("次".to_string()));
    }

    #[test]
    fn test_reuse_after_flush() {
        let mut acc = SentenceAccumulator::new(3);
        assert_eq!(acc.push("ab"), None);
        assert_eq!(acc.push("c"), Some("abc".to_string()));
        assert_eq!(acc.flush(), None);

        assert_eq!(acc.push("de"), None);
        assert_eq!(acc.push("f"), Some("def".to_string()));
        assert_eq!(acc.flush(), None);
    }

    #[test]
    fn flushes_on_unicode_ellipsis() {
        let mut acc = SentenceAccumulator::new(50);
        assert_eq!(acc.push("待って…続き"), Some("待って…".to_string()));
        assert_eq!(acc.flush(), Some("続き".to_string()));
    }

    #[test]
    fn flushes_on_ascii_three_dots() {
        let mut acc = SentenceAccumulator::new(50);
        assert_eq!(acc.push("wait...next"), Some("wait...".to_string()));
        assert_eq!(acc.flush(), Some("next".to_string()));
    }

    #[test]
    fn flushes_on_newline_and_crlf() {
        let mut acc = SentenceAccumulator::new(50);
        assert_eq!(acc.push("1行目\n2行目"), Some("1行目".to_string()));
        assert_eq!(acc.flush(), Some("2行目".to_string()));

        assert_eq!(acc.push("A\r\nB"), Some("A".to_string()));
        assert_eq!(acc.flush(), Some("B".to_string()));
    }
}
