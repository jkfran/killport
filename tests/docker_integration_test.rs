mod utils;

use std::process::Command as SystemCommand;
use std::time::{Duration, Instant};
use utils::get_available_port;

/// Helper: run a Docker container that listens on the given port.
/// Returns the container ID.
fn start_docker_container(port: u16) -> String {
    let output = SystemCommand::new("docker")
        .args([
            "run",
            "-d",
            "--rm",
            "-p",
            &format!("{}:80", port),
            "nginx:alpine",
        ])
        .output()
        .expect("Failed to start Docker container");

    assert!(
        output.status.success(),
        "docker run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("Invalid container ID")
        .trim()
        .to_string()
}

/// Helper: check if a Docker container is running.
fn is_container_running(container_id: &str) -> bool {
    let output = SystemCommand::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", container_id])
        .output();

    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim() == "true",
        Err(_) => false,
    }
}

/// Helper: wait for a Docker container to be healthy/running and its port to be reachable.
fn wait_for_container(container_id: &str, port: u16, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if is_container_running(container_id)
            && std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// Helper: force-remove a container (cleanup).
fn remove_container(container_id: &str) {
    let _ = SystemCommand::new("docker")
        .args(["rm", "-f", container_id])
        .output();
}

// ─── Docker Integration Tests ────────────────────────────────────────────────
// All tests are #[ignore] so they only run when explicitly requested
// via `cargo test --test docker_integration_test -- --ignored`

#[test]
#[ignore]
fn test_docker_is_present() {
    // Verify Docker is available in this environment
    let output = SystemCommand::new("docker")
        .args(["version"])
        .output()
        .expect("Docker not found");
    assert!(
        output.status.success(),
        "Docker is not running: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore]
fn test_kill_container_mode_container() {
    let port = get_available_port();
    let container_id = start_docker_container(port);

    assert!(
        wait_for_container(&container_id, port, Duration::from_secs(15)),
        "Container did not become ready on port {}",
        port
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let output = cmd
        .args([&port.to_string(), "--mode", "container"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("Successfully killed"),
        "Expected kill message, got: {}",
        stdout
    );
    assert!(
        stdout.contains("container"),
        "Expected 'container' in output, got: {}",
        stdout
    );

    // Wait briefly for container to stop
    std::thread::sleep(Duration::from_secs(1));
    assert!(
        !is_container_running(&container_id),
        "Container should be stopped after kill"
    );

    remove_container(&container_id);
}

#[test]
#[ignore]
fn test_kill_container_mode_auto() {
    let port = get_available_port();
    let container_id = start_docker_container(port);

    assert!(
        wait_for_container(&container_id, port, Duration::from_secs(15)),
        "Container did not become ready on port {}",
        port
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let output = cmd.args([&port.to_string()]).assert().success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("Successfully killed"),
        "Expected kill message, got: {}",
        stdout
    );

    std::thread::sleep(Duration::from_secs(1));
    assert!(
        !is_container_running(&container_id),
        "Container should be stopped after kill"
    );

    remove_container(&container_id);
}

#[test]
#[ignore]
fn test_dry_run_container_still_alive() {
    let port = get_available_port();
    let container_id = start_docker_container(port);

    assert!(
        wait_for_container(&container_id, port, Duration::from_secs(15)),
        "Container did not become ready on port {}",
        port
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let output = cmd
        .args([&port.to_string(), "--mode", "container", "--dry-run"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("Would kill"),
        "Dry run should say 'Would kill', got: {}",
        stdout
    );

    // Container should still be running after dry run
    assert!(
        is_container_running(&container_id),
        "Container should still be alive after dry run"
    );

    remove_container(&container_id);
}

#[test]
#[ignore]
fn test_no_container_on_port() {
    let port = get_available_port();
    // No container started on this port

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    cmd.args([&port.to_string(), "--mode", "container"])
        .assert()
        .success()
        .stdout(format!("No container found using port {}\n", port));
}

#[test]
#[ignore]
fn test_container_mode_ignores_native_process() {
    // Start a native TCP listener, but use --mode container
    // The native process should NOT be killed
    let port = get_available_port();
    let tempdir = tempfile::tempdir().unwrap();
    let mut child = utils::start_tcp_listener(tempdir.path(), port);
    let pid = child.id();

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let output = cmd
        .args([&port.to_string(), "--mode", "container"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("No container found"),
        "Expected 'No container found', got: {}",
        stdout
    );

    // Native process should still be alive
    assert!(
        utils::is_process_alive(pid),
        "Native process should not be killed in container mode"
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
#[test]
#[ignore]
fn test_kill_container_with_signal() {
    let port = get_available_port();
    let container_id = start_docker_container(port);

    assert!(
        wait_for_container(&container_id, port, Duration::from_secs(15)),
        "Container did not become ready on port {}",
        port
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let output = cmd
        .args([&port.to_string(), "--mode", "container", "-s", "sigkill"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("Successfully killed"),
        "Expected kill message, got: {}",
        stdout
    );

    std::thread::sleep(Duration::from_secs(1));
    assert!(
        !is_container_running(&container_id),
        "Container should be stopped after SIGKILL"
    );

    remove_container(&container_id);
}

#[test]
#[ignore]
fn test_auto_mode_docker_proxy_not_in_output() {
    // In auto mode with a container, docker-proxy processes should be filtered
    // and the container should be the one killed
    let port = get_available_port();
    let container_id = start_docker_container(port);

    assert!(
        wait_for_container(&container_id, port, Duration::from_secs(15)),
        "Container did not become ready on port {}",
        port
    );

    let mut cmd = assert_cmd::cargo_bin_cmd!("killport");
    let output = cmd.args([&port.to_string()]).assert().success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    // docker-proxy should not appear as a killed process
    assert!(
        !stdout.contains("docker-proxy"),
        "docker-proxy should be filtered out, got: {}",
        stdout
    );

    remove_container(&container_id);
}
