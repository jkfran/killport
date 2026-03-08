mod utils;
use regex::bytes::Regex;
#[cfg(target_os = "linux")]
use utils::start_udp_listener;
use utils::{get_available_port, is_process_alive, start_tcp_listener};

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
    assert!(
        re.is_match(data),
        "Output did not match expected pattern. Got: {}",
        String::from_utf8_lossy(data)
    );
}

// ─── CLI Basics ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_no_args() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.assert().failure();
}

#[test]
fn test_cli_help_flag() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args(["--help"]).assert().success();
}

#[test]
fn test_cli_version_flag() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args(["--version"]).assert().success();
}

#[test]
fn test_cli_invalid_port_string() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args(["abc"]).assert().failure();
}

#[test]
fn test_cli_invalid_port_overflow() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args(["99999"]).assert().failure();
}

#[test]
fn test_cli_invalid_port_negative() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args(["-1"]).assert().failure();
}

#[test]
fn test_cli_invalid_mode() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args(["8080", "--mode", "foobar"]).assert().failure();
}

#[cfg(unix)]
#[test]
fn test_cli_invalid_signal() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args(["8080", "-s", "NOTASIGNAL"]).assert().failure();
}

// ─── Basic Kill Behavior ─────────────────────────────────────────────────────

#[test]
fn test_basic_kill_no_process() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string()])
        .assert()
        .code(2)
        .stdout(format!("No service found using port {}\n", port));
}

#[test]
fn test_basic_kill_process() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd.args([&port.to_string()]).assert().success();
    assert_match(&command.get_output().stdout, "Successfully killed", port);
    let _ = child.kill();
    let _ = child.wait();
}

// ─── TCP Killing ─────────────────────────────────────────────────────────────

#[test]
fn test_kill_tcp_ipv4_process() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd.args([&port.to_string()]).assert().success();
    assert_match(&command.get_output().stdout, "Successfully killed", port);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_kill_tcp_process_already_dead() {
    let port = get_available_port();
    // No listener started, port is free
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string()])
        .assert()
        .code(2)
        .stdout(format!("No service found using port {}\n", port));
}

// ─── UDP Killing ─────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
#[test]
fn test_kill_udp_process() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_udp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd.args([&port.to_string()]).assert().success();
    assert_match(&command.get_output().stdout, "Successfully killed", port);
    let _ = child.kill();
    let _ = child.wait();
}

// ─── Multiple Ports ──────────────────────────────────────────────────────────

#[test]
fn test_kill_multiple_ports_all_succeed() {
    let port1 = get_available_port();
    let port2 = get_available_port();
    let tempdir1 = tempdir().unwrap();
    let mut child1 = start_tcp_listener(tempdir1.path(), port1);
    let tempdir2 = tempdir().unwrap();
    let mut child2 = start_tcp_listener(tempdir2.path(), port2);

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port1.to_string(), &port2.to_string()])
        .assert()
        .success();
    let stdout = &command.get_output().stdout;
    let stdout_str = String::from_utf8_lossy(stdout);
    assert!(
        stdout_str.contains(&format!("listening on port {}", port1)),
        "Expected port {} in output: {}",
        port1,
        stdout_str
    );
    assert!(
        stdout_str.contains(&format!("listening on port {}", port2)),
        "Expected port {} in output: {}",
        port2,
        stdout_str
    );

    let _ = child1.kill();
    let _ = child1.wait();
    let _ = child2.kill();
    let _ = child2.wait();
}

#[test]
fn test_kill_multiple_ports_some_empty() {
    let port1 = get_available_port();
    let port2 = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port1);
    // port2 has no listener

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port1.to_string(), &port2.to_string()])
        .assert()
        .code(2);
    let stdout_str = String::from_utf8_lossy(&command.get_output().stdout);
    assert!(stdout_str.contains(&format!("listening on port {}", port1)));
    assert!(stdout_str.contains(&format!("No service found using port {}", port2)));

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_kill_multiple_ports_all_empty() {
    let port1 = get_available_port();
    let port2 = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port1.to_string(), &port2.to_string()])
        .assert()
        .code(2)
        .stdout(format!(
            "No service found using port {}\nNo service found using port {}\n",
            port1, port2
        ));
}

// ─── Signal Handling ─────────────────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn test_signal_sigkill() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port.to_string(), "-s", "sigkill"])
        .assert()
        .success();
    assert_match(&command.get_output().stdout, "Successfully killed", port);
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
#[test]
fn test_signal_sigterm() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port.to_string(), "-s", "sigterm"])
        .assert()
        .success();
    assert_match(&command.get_output().stdout, "Successfully killed", port);
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
#[test]
fn test_signal_sighup() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port.to_string(), "-s", "sighup"])
        .assert()
        .success();
    assert_match(&command.get_output().stdout, "Successfully killed", port);
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
#[test]
fn test_signal_sigint() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port.to_string(), "-s", "sigint"])
        .assert()
        .success();
    assert_match(&command.get_output().stdout, "Successfully killed", port);
    let _ = child.kill();
    let _ = child.wait();
}

// ─── Mode Options ────────────────────────────────────────────────────────────

#[test]
fn test_mode_auto_finds_process() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port.to_string(), "--mode", "auto"])
        .assert()
        .success();
    assert_match(&command.get_output().stdout, "Successfully killed", port);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_mode_process_finds_process() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port.to_string(), "--mode", "process"])
        .assert()
        .success();
    assert_match(&command.get_output().stdout, "Successfully killed", port);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_mode_container_does_not_find_native_process() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let output = cmd
        .args([&port.to_string(), "--mode", "container"])
        .assert()
        .code(2);
    let stdout_str = String::from_utf8_lossy(&output.get_output().stdout);
    // In container mode, native processes should not be killed
    assert!(
        stdout_str.contains("No container found") || stdout_str.contains("No service found"),
        "Expected no container found message, got: {}",
        stdout_str
    );
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_mode_auto_no_service_message() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string(), "--mode", "auto"])
        .assert()
        .code(2)
        .stdout(format!("No service found using port {}\n", port));
}

#[test]
fn test_mode_process_no_service_message() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string(), "--mode", "process"])
        .assert()
        .code(2)
        .stdout(format!("No process found using port {}\n", port));
}

#[test]
fn test_mode_container_no_service_message() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string(), "--mode", "container"])
        .assert()
        .code(2)
        .stdout(format!("No container found using port {}\n", port));
}

#[test]
fn test_mode_short_flag() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port.to_string(), "-m", "process"])
        .assert()
        .success();
    assert_match(&command.get_output().stdout, "Successfully killed", port);
    let _ = child.kill();
    let _ = child.wait();
}

// ─── Dry Run ─────────────────────────────────────────────────────────────────

#[test]
fn test_dry_run_does_not_kill() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let pid = child.id();

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port.to_string(), "--dry-run"])
        .assert()
        .success();
    assert_match(&command.get_output().stdout, "Would kill", port);

    // Verify the process is still alive after dry run
    assert!(
        is_process_alive(pid),
        "Process should still be alive after dry run"
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_dry_run_output_format() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port.to_string(), "--dry-run"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&command.get_output().stdout);
    assert!(
        stdout.contains("Would kill"),
        "Dry run should say 'Would kill', got: {}",
        stdout
    );
    assert!(
        !stdout.contains("Successfully killed"),
        "Dry run should not say 'Successfully killed'"
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_dry_run_no_process() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string(), "--dry-run"])
        .assert()
        .code(2)
        .stdout(format!("No service found using port {}\n", port));
}

#[cfg(unix)]
#[test]
fn test_dry_run_with_signal() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port.to_string(), "--dry-run", "-s", "sigterm"])
        .assert()
        .success();
    assert_match(&command.get_output().stdout, "Would kill", port);

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_dry_run_multiple_ports() {
    let port1 = get_available_port();
    let port2 = get_available_port();
    let tempdir1 = tempdir().unwrap();
    let tempdir2 = tempdir().unwrap();
    let mut child1 = start_tcp_listener(tempdir1.path(), port1);
    let mut child2 = start_tcp_listener(tempdir2.path(), port2);

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([&port1.to_string(), &port2.to_string(), "--dry-run"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&command.get_output().stdout);
    assert!(stdout.contains("Would kill"));
    assert!(stdout.contains(&format!("port {}", port1)));
    assert!(stdout.contains(&format!("port {}", port2)));

    let _ = child1.kill();
    let _ = child1.wait();
    let _ = child2.kill();
    let _ = child2.wait();
}

// ─── Exit Codes ──────────────────────────────────────────────────────────────

#[test]
fn test_exit_code_success_on_kill() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string()]).assert().success();
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_exit_code_not_found() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string()]).assert().code(2);
}

#[test]
fn test_exit_code_not_found_with_no_fail() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string(), "--no-fail"])
        .assert()
        .success();
}

#[test]
fn test_exit_code_error_invalid_args() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.assert().failure();
}

// ─── Edge Cases ──────────────────────────────────────────────────────────────

#[test]
fn test_port_zero() {
    // Port 0 may match system processes on some OSes, so just verify it doesn't crash
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args(["0", "--no-fail"]).assert().success();
}

#[test]
fn test_port_65535() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args(["65535"])
        .assert()
        .code(2)
        .stdout("No service found using port 65535\n");
}

#[test]
fn test_kill_same_port_twice_rapidly() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);

    // First kill should succeed
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string()]).assert().success();

    // Wait briefly for process to fully die
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Second kill should find nothing
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string()])
        .assert()
        .code(2)
        .stdout(format!("No service found using port {}\n", port));

    let _ = child.kill();
    let _ = child.wait();
}

// ─── Output Format Verification ──────────────────────────────────────────────

#[test]
fn test_output_no_service_format_auto() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string()])
        .assert()
        .code(2)
        .stdout(format!("No service found using port {}\n", port));
}

#[test]
fn test_output_no_process_format() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string(), "--mode", "process"])
        .assert()
        .code(2)
        .stdout(format!("No process found using port {}\n", port));
}

#[test]
fn test_output_no_container_format() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string(), "--mode", "container"])
        .assert()
        .code(2)
        .stdout(format!("No container found using port {}\n", port));
}

#[test]
fn test_output_kill_message_contains_process_name() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd.args([&port.to_string()]).assert().success();
    let stdout = String::from_utf8_lossy(&command.get_output().stdout);
    assert!(stdout.contains("Successfully killed"));
    assert!(stdout.contains("mock_process"));
    assert!(stdout.contains(&format!("port {}", port)));
    let _ = child.kill();
    let _ = child.wait();
}

// ─── Combined Flags ──────────────────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn test_combined_flags() {
    let port = get_available_port();
    let tempdir = tempdir().unwrap();
    let mut child = start_tcp_listener(tempdir.path(), port);
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let command = cmd
        .args([
            &port.to_string(),
            "-m",
            "process",
            "-s",
            "sigterm",
            "--dry-run",
        ])
        .assert()
        .success();
    assert_match(&command.get_output().stdout, "Would kill", port);
    let _ = child.kill();
    let _ = child.wait();
}

// ─── Verbosity ──────────────────────────────────────────────────────────────

#[test]
fn test_verbose_output_succeeds() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string(), "-vvv", "--no-fail"])
        .assert()
        .success();
}

#[test]
fn test_quiet_suppresses_stderr() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let output = cmd
        .args([&port.to_string(), "-q", "--no-fail"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "Quiet mode should not produce stderr output, got: {}",
        stderr
    );
}

#[test]
fn test_very_quiet_suppresses_stderr() {
    let port = get_available_port();
    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let output = cmd
        .args([&port.to_string(), "-qq", "--no-fail"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "Very quiet mode should produce no stderr, got: {}",
        stderr
    );
}
