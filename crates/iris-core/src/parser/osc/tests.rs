
use super::parse_osc;
use crate::parser::Action;

#[test]
fn osc_parses_window_titles() {
    assert_eq!(
        parse_osc(b"2;Iris"),
        vec![Action::SetWindowTitle("Iris".to_string())]
    );
}

#[test]
fn osc_parses_hyperlinks() {
    assert_eq!(
        parse_osc(b"8;id=prompt-1;https://example.com"),
        vec![Action::SetHyperlink {
            id: Some("prompt-1".to_string()),
            uri: "https://example.com".to_string(),
        }]
    );
}
