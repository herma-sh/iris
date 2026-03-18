use iris_core::{Parser, Terminal};

#[test]
fn parser_handles_nested_like_osc_streams_and_subsequent_title_updates() {
    let mut terminal = Terminal::new(2, 8).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(&mut terminal, b"\x1b]2;Outer\x1b]2;Inner\x07")
        .unwrap();
    assert_eq!(terminal.window_title.as_deref(), Some("Outer\x1b]2;Inner"));

    parser.advance(&mut terminal, b"\x1b]2;Final\x07").unwrap();
    assert_eq!(terminal.window_title.as_deref(), Some("Final"));
}

#[test]
fn parser_handles_chunk_split_nested_like_osc_streams() {
    let mut terminal = Terminal::new(2, 8).unwrap();
    let mut parser = Parser::new();

    parser.advance(&mut terminal, b"\x1b]2;Chunk").unwrap();
    parser.advance(&mut terminal, b"ed\x1b").unwrap();
    parser.advance(&mut terminal, b"]Tail\x07").unwrap();

    assert_eq!(terminal.window_title.as_deref(), Some("Chunked\x1b]Tail"));
}
