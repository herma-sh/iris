use super::parse_dcs;

#[test]
fn unsupported_dcs_payloads_are_ignored() {
    assert!(parse_dcs(b"qignored").is_empty());
}
