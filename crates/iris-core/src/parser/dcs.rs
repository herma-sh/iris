use super::Action;

/// Parses a DCS payload into terminal actions.
///
/// Phase 1 only recognizes DCS as a bounded parser state. Unsupported payloads
/// are ignored until later protocol-specific handling is added.
#[must_use]
pub fn parse_dcs(_payload: &[u8]) -> Vec<Action> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::parse_dcs;

    #[test]
    fn unsupported_dcs_payloads_are_ignored() {
        assert!(parse_dcs(b"qignored").is_empty());
    }
}
