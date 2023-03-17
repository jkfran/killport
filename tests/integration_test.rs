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
    let status = Command::new("rustc")
        .arg(&mock_process_path)
        .arg("--out-dir")
        .arg(&tempdir_path)
        .status()
        .expect("Failed to compile mock_process.rs");

    assert!(status.success(), "Mock process compilation failed");

    let mut mock_process = Command::new(tempdir_path.join("mock_process"))
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
}
