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
#[path = "../test/parser/dcs/tests.rs"]
mod tests;
