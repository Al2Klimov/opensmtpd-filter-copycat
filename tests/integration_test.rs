use std::io::Write;
use std::process::{Command, Stdio};

fn run_filter(input: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_opensmtpd-filter-copycat"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn filter binary");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(input)
        .expect("failed to write to filter stdin");

    child.wait_with_output().expect("failed to wait on filter")
}

/// config|ready must trigger all required registration lines.
#[test]
fn test_config_ready_registers_filters() {
    let output = run_filter(b"config|ready\n");
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("register|report|smtp-in|tx-begin"));
    assert!(stdout.contains("register|report|smtp-in|tx-rcpt"));
    assert!(stdout.contains("register|filter|smtp-in|data-line"));
    assert!(stdout.contains("register|filter|smtp-in|commit"));
    assert!(stdout.contains("register|report|smtp-in|link-disconnect"));
    assert!(stdout.contains("register|ready"));
}

/// data-line input must be echoed back as a filter-dataline response.
#[test]
fn test_data_line_is_echoed() {
    let input = b"config|ready\nfilter|1|0|smtp-in|data-line|sess1|tok1|Hello World\n";
    let output = run_filter(input);
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("filter-dataline|sess1|tok1|Hello World"));
}

/// commit with no active session must produce a proceed verdict.
#[test]
fn test_commit_no_session_proceeds() {
    let input = b"config|ready\nfilter|1|0|smtp-in|commit|sess1|tok1\n";
    let output = run_filter(input);
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("filter-result|sess1|tok1|proceed"));
}

/// commit must proceed when the sender display name does not contain the
/// recipient domain.
#[test]
fn test_commit_proceeds_when_sender_name_lacks_recipient_domain() {
    let input = b"config|ready\n\
        report|1|0|smtp-in|tx-begin|sess1\n\
        report|1|0|smtp-in|tx-rcpt|sess1|m1|ok|user@example.com\n\
        filter|1|0|smtp-in|data-line|sess1|tok1|From: Alice <alice@other.com>\n\
        filter|1|0|smtp-in|data-line|sess1|tok2|.\n\
        filter|1|0|smtp-in|commit|sess1|tok3\n";
    let output = run_filter(input);
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("filter-result|sess1|tok3|proceed"));
}

/// commit must reject when the sender display name contains the recipient
/// domain.
#[test]
fn test_commit_rejects_when_sender_name_contains_recipient_domain() {
    let input = b"config|ready\n\
        report|1|0|smtp-in|tx-begin|sess1\n\
        report|1|0|smtp-in|tx-rcpt|sess1|m1|ok|user@evil.com\n\
        filter|1|0|smtp-in|data-line|sess1|tok1|From: User from evil.com <alice@other.com>\n\
        filter|1|0|smtp-in|data-line|sess1|tok2|.\n\
        filter|1|0|smtp-in|commit|sess1|tok3\n";
    let output = run_filter(input);
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout
            .contains("filter-result|sess1|tok3|reject|550 Sender name contains recipient domain")
    );
}

/// link-disconnect must remove the session so a subsequent commit proceeds.
#[test]
fn test_link_disconnect_removes_session() {
    let input = b"config|ready\n\
        report|1|0|smtp-in|tx-begin|sess1\n\
        report|1|0|smtp-in|tx-rcpt|sess1|m1|ok|user@evil.com\n\
        report|1|0|smtp-in|link-disconnect|sess1\n\
        filter|1|0|smtp-in|commit|sess1|tok1\n";
    let output = run_filter(input);
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Session was removed, so no recipient domain check → proceed.
    assert!(stdout.contains("filter-result|sess1|tok1|proceed"));
}
