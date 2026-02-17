mod evaluator;
mod executor;

pub use evaluator::{ActionConfig, RoutingError, RuleEvaluator};
pub use executor::ActionExecutor;

pub fn normalize_phone_number_e164(phone_number: &str) -> Result<String, RoutingError> {
    let cleaned: String = phone_number
        .chars()
        .filter(|ch| !matches!(ch, '-' | ' ' | '\t' | '\n' | '\r' | '(' | ')' | '（' | '）'))
        .collect();

    let normalized = if cleaned.starts_with('+') {
        cleaned
    } else if let Some(domestic) = cleaned.strip_prefix('0') {
        if domestic.is_empty() || !domestic.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(RoutingError::InvalidPhoneNumber(phone_number.to_string()));
        }
        format!("+81{}", domestic)
    } else {
        return Err(RoutingError::InvalidPhoneNumber(phone_number.to_string()));
    };

    if !normalized.starts_with('+') {
        return Err(RoutingError::InvalidPhoneNumber(phone_number.to_string()));
    }

    let digits = &normalized[1..];
    if digits.len() < 2 || digits.len() > 15 {
        return Err(RoutingError::InvalidPhoneNumber(phone_number.to_string()));
    }

    let mut chars = digits.chars();
    let Some(first) = chars.next() else {
        return Err(RoutingError::InvalidPhoneNumber(phone_number.to_string()));
    };
    if !('1'..='9').contains(&first) {
        return Err(RoutingError::InvalidPhoneNumber(phone_number.to_string()));
    }
    if !chars.all(|ch| ch.is_ascii_digit()) {
        return Err(RoutingError::InvalidPhoneNumber(phone_number.to_string()));
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::normalize_phone_number_e164;

    #[test]
    fn normalize_phone_number_e164_accepts_domestic_090() {
        let normalized = normalize_phone_number_e164("09028894539").expect("should normalize");
        assert_eq!(normalized, "+819028894539");
    }

    #[test]
    fn normalize_phone_number_e164_accepts_hyphenated_domestic() {
        let normalized =
            normalize_phone_number_e164("090-2889-4539").expect("should normalize with hyphen");
        assert_eq!(normalized, "+819028894539");
    }

    #[test]
    fn normalize_phone_number_e164_accepts_existing_e164() {
        let normalized =
            normalize_phone_number_e164("+819028894539").expect("should keep e164 unchanged");
        assert_eq!(normalized, "+819028894539");
    }

    #[test]
    fn normalize_phone_number_e164_rejects_invalid_text() {
        let err = normalize_phone_number_e164("abc").expect_err("invalid text should fail");
        assert!(err.to_string().contains("invalid phone number"));
    }
}
