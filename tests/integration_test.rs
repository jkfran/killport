mod utils;
use regex::bytes::Regex;
use utils::start_listener_process;

use assert_cmd::Command;
use tempfile::tempdir;

#[cfg(unix)]
const MOCK_PROCESS_NAME: &str = "mock_process";
#[cfg(windows)]
const MOCK_PROCESS_NAME: &str = "mock_process.exe";

// test helper
fn assert_match(data: &[u8], msg: &str, port: u16) {
    let re = Regex::new(&format!(
        r"{msg} process '(\/tmp\/\.tmp\w+\/)?{MOCK_PROCESS_NAME}' listening on port {port}\n"
    ))
    .unwrap();
    assert!(re.is_match(data));
}

#[test]
fn test_basic_kill_no_process() {
    let mut cmd = Command::cargo_bin("killport").unwrap();
    cmd.args(&["8080"])
        .assert()
        .success()
        .stdout("No service found using port 8080\n");
}

/// Tests basic functionality of killing a process on a specified port without any additional options.
#[test]
fn test_basic_kill_process() {
    let tempdir = tempdir().unwrap();
    let tempdir_path = tempdir.path();
    let mut child = start_listener_process(tempdir_path, 8180);
    let mut cmd = Command::cargo_bin("killport").unwrap();
    let command = cmd.args(&["8180"]).assert().success();
    assert_match(&command.get_output().stdout, "Successfully killed", 8180);
    // Clean up
    let _ = child.kill();
    let _ = child.wait();
}

/// Tests the `--signal` option with various signals.
#[test]
fn test_signal_handling() {
    let tempdir = tempdir().unwrap();
    let tempdir_path = tempdir.path();

    for signal in ["sighup", "sigint", "sigkill"].iter() {
        let mut child = start_listener_process(tempdir_path, 8280);
        let mut cmd = Command::cargo_bin("killport").unwrap();
        let command = cmd.args(&["8280", "-s", signal]).assert().success();
        assert_match(&command.get_output().stdout, "Successfully killed", 8280);
        // Clean up
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Tests the `--mode` option for different operation modes.
#[test]
fn test_mode_option() {
    let tempdir = tempdir().unwrap();
    let tempdir_path = tempdir.path();

    for (i, mode) in ["auto", "process"].iter().enumerate() {
        let port = 8380 + i as u16;
        let mut child = start_listener_process(tempdir_path, port);
        let mut cmd = Command::cargo_bin("killport").unwrap();
        let command = cmd
            .args(&[&port.to_string(), "--mode", mode])
            .assert()
            .success();
        assert_match(&command.get_output().stdout, "Successfully killed", port);
        // Clean up
        let _ = child.kill();
        let _ = child.wait();
    }

    let mut cmd = Command::cargo_bin("killport").unwrap();
    cmd.args(&["8383", "--mode", "auto"])
        .assert()
        .success()
        .stdout(format!("No service found using port 8383\n"));

    let mut cmd = Command::cargo_bin("killport").unwrap();
    cmd.args(&["8383", "--mode", "process"])
        .assert()
        .success()
        .stdout(format!("No process found using port 8383\n"));

    let mut cmd = Command::cargo_bin("killport").unwrap();
    cmd.args(&["8383", "--mode", "container"])
        .assert()
        .success()
        .stdout(format!("No container found using port 8383\n"));
}

/// Tests the `--dry-run` option to ensure no actual killing of the process.
#[test]
fn test_dry_run_option() {
    let tempdir = tempdir().unwrap();
    let tempdir_path = tempdir.path();
    let mut child = start_listener_process(tempdir_path, 8480);

    let mut cmd = Command::cargo_bin("killport").unwrap();
    let command = cmd.args(&["8480", "--dry-run"]).assert().success();
    assert_match(&command.get_output().stdout, "Would kill", 8480);
    // Clean up
    let _ = child.kill();
    let _ = child.wait();
}
