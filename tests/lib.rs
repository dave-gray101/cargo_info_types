use cargo_info_types::strip_ansi_escapes;

#[test]
fn strip_ansi_escapes_removes_vt100_sequences() {
    let colored = "\x1b[1;32m+default\x1b[0m      = [std]";
    assert_eq!(strip_ansi_escapes(colored), "+default      = [std]");
}

#[test]
fn strip_ansi_escapes_leaves_plain_text_intact() {
    let plain = "no escapes here";
    assert_eq!(strip_ansi_escapes(plain), plain);
}
