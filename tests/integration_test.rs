use assert_cmd::Command;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_killport() {
    // Create a temporary directory for testing.
    let tempdir = tempdir().expect("Failed to create temporary directory");
    let tempdir_path = tempdir.path();

    // Create a mock process that listens on a port.
    let mock_process = format!(
        r#"
        use std::net::TcpListener;
        fn main() {{
            let _listener = TcpListener::bind("127.0.0.1:8080").unwrap();
            loop {{}}
        }}
    "#
    );

    let mock_process_path = tempdir_path.join("mock_process.rs");
    let mut file = File::create(&mock_process_path).expect("Failed to create mock_process.rs file");
    file.write_all(mock_process.as_bytes())
        .expect("Failed to write mock_process.rs content");

    // Compile and run the mock process in the background.
    let status = std::process::Command::new("rustc")
        .arg(&mock_process_path)
        .arg("--out-dir")
        .arg(&tempdir_path)
        .status()
        .expect("Failed to compile mock_process.rs");

    assert!(status.success(), "Mock process compilation failed");

    // Test killport execution without options
    let mut mock_process = std::process::Command::new(tempdir_path.join("mock_process"))
        .spawn()
        .expect("Failed to run the mock process");

    // Test killport command
    let mut cmd = Command::cargo_bin("killport").expect("Failed to find killport binary");
    cmd.arg("8080")
        .assert()
        .success()
        .stdout("Successfully killed process listening on port 8080\n");

    // Cleanup: Terminate the mock process (if still running).
    let _ = mock_process.kill();

    // Test killport execution with -s option
    let mut mock_process = std::process::Command::new(tempdir_path.join("mock_process"))
        .spawn()
        .expect("Failed to run the mock process");

    // Test killport command with specifying a signal name
    let mut cmd = Command::cargo_bin("killport").expect("Failed to find killport binary");
    cmd.arg("8080")
        .arg("-s")
        .arg("sigterm")
        .assert()
        .success()
        .stdout("Successfully killed process listening on port 8080\n");

    // Cleanup: Terminate the mock process (if still running).
    let _ = mock_process.kill();
}


#[test]
fn test_killport_for_docker() {
    // Run a mock container in the background
    let mut mock_process = std::process::Command::new("docker")
        .args(["run", "-d", "--name", "test", "--rm", "-p", "8081:80", "nginx:latest"])
        .spawn()
        .expect("Failed to run the mock container process");

    // Test killport command with specifying a port is bound for docker container
    let mut cmd = Command::cargo_bin("killport").expect("Failed to find killport binary");
    cmd.arg("8081")
        .assert()
        .success()
        .stdout("Successfully killed process listening on port 8081\n");

    // Cleanup: Terminate the mock process (if still running).
    let _ = mock_process.kill();
}
