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
    let mut child = start_listener_process(tempdir_path, 8081);
    let mut cmd = Command::cargo_bin("killport").unwrap();
    let command = cmd.args(&["8081"]).assert().success();
    assert_match(&command.get_output().stdout, "Successfully killed", 8081);
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
        let mut child = start_listener_process(tempdir_path, 8082);
        let mut cmd = Command::cargo_bin("killport").unwrap();
        let command = cmd.args(&["8082", "-s", signal]).assert().success();
        assert_match(&command.get_output().stdout, "Successfully killed", 8082);
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

    for mode in ["auto", "process"].iter() {
        let mut child = start_listener_process(tempdir_path, 8083);
        let mut cmd = Command::cargo_bin("killport").unwrap();
        let command = cmd.args(&["8083", "--mode", mode]).assert().success();
        assert_match(&command.get_output().stdout, "Successfully killed", 8083);
        // Clean up
        let _ = child.kill();
        let _ = child.wait();
    }

    let mut cmd = Command::cargo_bin("killport").unwrap();
    cmd.args(&["8083", "--mode", "auto"])
        .assert()
        .success()
        .stdout(format!("No service found using port 8083\n"));

    let mut cmd = Command::cargo_bin("killport").unwrap();
    cmd.args(&["8083", "--mode", "process"])
        .assert()
        .success()
        .stdout(format!("No process found using port 8083\n"));

    let mut cmd = Command::cargo_bin("killport").unwrap();
    cmd.args(&["8083", "--mode", "container"])
        .assert()
        .success()
        .stdout(format!("No container found using port 8083\n"));
}

/// Tests the `--dry-run` option to ensure no actual killing of the process.
#[test]
fn test_dry_run_option() {
    let tempdir = tempdir().unwrap();
    let tempdir_path = tempdir.path();
    let mut child = start_listener_process(tempdir_path, 8084);

    let mut cmd = Command::cargo_bin("killport").unwrap();
    let command = cmd.args(&["8084", "--dry-run"]).assert().success();
    assert_match(&command.get_output().stdout, "Would kill", 8084);
    // Clean up
    let _ = child.kill();
    let _ = child.wait();
}
