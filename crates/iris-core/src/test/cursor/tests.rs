use super::Cursor;

#[test]
fn cursor_movement_respects_bounds() {
    let mut cursor = Cursor::new();
    cursor.move_right(100, 80);
    cursor.move_down(100, 24);
    assert_eq!(cursor.position.col, 79);
    assert_eq!(cursor.position.row, 23);

    cursor.move_left(200);
    cursor.move_up(200);
    assert_eq!(cursor.position.col, 0);
    assert_eq!(cursor.position.row, 0);
}

#[test]
fn cursor_save_and_restore_round_trips() {
    let mut cursor = Cursor::new();
    cursor.move_to(4, 9);
    let saved = cursor.save();
    cursor.move_to(0, 0);
    cursor.restore(saved);
    assert_eq!(cursor.position.row, 4);
    assert_eq!(cursor.position.col, 9);
}
