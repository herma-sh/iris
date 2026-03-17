use super::{Parser, ParserConfig, ParserState};
use crate::cell::{CellFlags, Color};
use crate::parser::{Action, GraphicsRendition};

#[test]
fn parser_starts_in_ground_state() {
    assert_eq!(Parser::new().state(), ParserState::Ground);
}

#[test]
fn parser_parses_printable_characters_and_escape_transitions() {
    let mut parser = Parser::new();
    assert_eq!(parser.parse(b"A"), vec![Action::Print('A')]);
    assert!(parser.parse(b"\x1b").is_empty());
    assert_eq!(parser.state(), ParserState::Escape);
}

#[test]
fn parser_collects_csi_parameters_and_defaults() {
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse(b"\x1b[12;24H"),
        vec![Action::CursorPosition { row: 12, col: 24 }]
    );
    assert_eq!(parser.state(), ParserState::Ground);

    let mut parser = Parser::new();
    assert_eq!(parser.parse(b"\x1b[A"), vec![Action::CursorUp(1)]);
}

#[test]
fn parser_handles_private_modes_and_sgr() {
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse(b"\x1b[?25l"),
        vec![Action::ResetModes {
            private: true,
            modes: vec![25],
        }]
    );
    assert_eq!(
        parser.parse(b"\x1b[1;31;48;5;240m"),
        vec![Action::SetGraphicsRendition(vec![
            GraphicsRendition::Bold(true),
            GraphicsRendition::Foreground(Color::Ansi(1)),
            GraphicsRendition::Background(Color::Indexed(240)),
        ])]
    );
}

#[test]
fn parser_handles_escape_index_sequences() {
    let mut parser = Parser::new();
    assert_eq!(parser.parse(b"\x1bD"), vec![Action::Index]);
    assert_eq!(parser.parse(b"\x1bE"), vec![Action::NextLine]);
    assert_eq!(parser.parse(b"\x1bM"), vec![Action::ReverseIndex]);
}

#[test]
fn parser_decodes_utf8_printable_characters() {
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse("\u{00e9}\u{4e2d}".as_bytes()),
        vec![Action::Print('\u{00e9}'), Action::Print('\u{4e2d}')]
    );
}

#[test]
fn parser_preserves_utf8_state_across_chunks() {
    let mut parser = Parser::new();
    assert!(parser.parse(&[0xe2, 0x82]).is_empty());
    assert_eq!(parser.parse(&[0xac]), vec![Action::Print('\u{20ac}')]);
}

#[test]
fn parser_recovers_from_malformed_utf8_sequences() {
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse(&[0xe2, b'A']),
        vec![
            Action::Print(char::REPLACEMENT_CHARACTER),
            Action::Print('A'),
        ]
    );
}

#[test]
fn parser_handles_malformed_sequences_gracefully() {
    let mut parser = Parser::new();
    assert!(parser.parse(b"\x1b[12$").is_empty());
    assert_eq!(parser.state(), ParserState::Ground);
    assert_eq!(parser.parse(b"B"), vec![Action::Print('B')]);
}

#[test]
fn parser_applies_actions_to_terminal() {
    let mut parser = Parser::new();
    let mut terminal = crate::terminal::Terminal::new(2, 8).unwrap();

    parser
        .advance(&mut terminal, b"\x1b[1;31mA\x1b[0m")
        .unwrap();

    let cell = terminal.grid.cell(0, 0).copied().unwrap();
    assert_eq!(cell.character, 'A');
    assert_eq!(cell.attrs.fg, Color::Ansi(1));
    assert!(cell.attrs.flags.contains(CellFlags::BOLD));
    assert_eq!(terminal.attrs.fg, Color::Default);
    assert!(!terminal.attrs.flags.contains(CellFlags::BOLD));
}

#[test]
fn parser_parses_osc_window_title_with_bel_terminator() {
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse(b"\x1b]2;Iris\x07"),
        vec![Action::SetWindowTitle("Iris".to_string())]
    );
}

#[test]
fn parser_parses_osc_hyperlink_with_st_terminator() {
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse(b"\x1b]8;id=prompt-1;https://example.com\x1b\\"),
        vec![Action::SetHyperlink {
            id: Some("prompt-1".to_string()),
            uri: "https://example.com".to_string(),
        }]
    );
}

#[test]
fn parser_handles_dcs_with_st_terminator() {
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse(b"\x1bPqignored\x1b\\A"),
        vec![Action::Print('A')]
    );
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn parser_handles_partial_dcs_across_chunks() {
    let mut parser = Parser::new();
    assert!(parser.parse(b"\x1bPqhello").is_empty());
    assert_eq!(parser.state(), ParserState::DcsString);
    assert_eq!(parser.parse(b"\x1b\\Z"), vec![Action::Print('Z')]);
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn parser_ignores_sos_pm_and_apc_strings() {
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse(b"\x1bXone\x1b\\A\x1b^two\x1b\\B\x1b_three\x1b\\C"),
        vec![Action::Print('A'), Action::Print('B'), Action::Print('C')]
    );
}

#[test]
fn parser_applies_charset_designation_to_active_g0() {
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse(b"\x1b(A#\x1b(B#"),
        vec![Action::Print('\u{00a3}'), Action::Print('#')]
    );
}

#[test]
fn parser_switches_between_g0_and_g1_charsets() {
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse(b"\x1b)0\x0eqx\x0fq"),
        vec![
            Action::Print('\u{2500}'),
            Action::Print('\u{2502}'),
            Action::Print('q'),
        ]
    );
}

#[test]
fn parser_limits_osc_payload_growth() {
    let mut parser = Parser::with_config(ParserConfig {
        max_params: 16,
        max_osc_bytes: 4,
        max_dcs_bytes: 4,
        max_ignored_string_bytes: 4,
    });

    assert!(parser.parse(b"\x1b]2;h").is_empty());
    assert_eq!(
        parser.parse(b"ello"),
        vec![Action::Print('l'), Action::Print('l'), Action::Print('o')]
    );
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn parser_limits_dcs_payload_growth() {
    let mut parser = Parser::with_config(ParserConfig {
        max_params: 16,
        max_osc_bytes: 4,
        max_dcs_bytes: 4,
        max_ignored_string_bytes: 4,
    });

    assert!(parser.parse(b"\x1bPabcd").is_empty());
    assert_eq!(
        parser.parse(b"ef"),
        vec![Action::Print('e'), Action::Print('f')]
    );
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn parser_limits_ignored_string_growth() {
    let mut parser = Parser::with_config(ParserConfig {
        max_params: 16,
        max_osc_bytes: 4,
        max_dcs_bytes: 4,
        max_ignored_string_bytes: 4,
    });

    assert!(parser.parse(b"\x1bXabcd").is_empty());
    assert_eq!(
        parser.parse(b"ef"),
        vec![Action::Print('e'), Action::Print('f')]
    );
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn finishing_dcs_does_not_clear_ignored_string_state() {
    let mut parser = Parser::new();
    parser.state = ParserState::DcsString;
    parser.dcs_buffer.extend_from_slice(b"qignored");
    parser.ignored_string_len = 3;

    assert!(parser.finish_dcs().is_empty());
    assert_eq!(parser.ignored_string_len, 3);
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn finishing_ignored_string_does_not_clear_other_buffers() {
    let mut parser = Parser::new();
    parser.state = ParserState::IgnoreString;
    parser.osc_buffer.extend_from_slice(b"title");
    parser.dcs_buffer.extend_from_slice(b"qdata");
    parser.ignored_string_len = 4;

    parser.finish_ignored_string();

    assert_eq!(parser.osc_buffer, b"title");
    assert_eq!(parser.dcs_buffer, b"qdata");
    assert_eq!(parser.ignored_string_len, 0);
    assert_eq!(parser.state(), ParserState::Ground);
}
