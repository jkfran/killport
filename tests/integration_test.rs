use assert_cmd::Command;
use std::{fs::File, path};
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_killport() {
    // Create a temporary directory for testing.
    let tempdir = tempdir().expect("Failed to create temporary directory");
    let tempdir_path = tempdir.path();
    generate_temp_process(tempdir_path);

    // Test killport execution without options
    test_killport_noargs(tempdir_path);

    // Test killport execution with -s option
    // Hangup
    test_killport_signal_arg(tempdir_path, "sighup");
    // Interrupt
    test_killport_signal_arg(tempdir_path, "sigint");
    // Quit
    test_killport_signal_arg(tempdir_path, "sigquit");
    // Illegal instruction (not reset when caught)
    test_killport_signal_arg(tempdir_path, "sigill");
    // Trace trap (not reset when caught)
    test_killport_signal_arg(tempdir_path, "sigtrap");
    // Abort
    test_killport_signal_arg(tempdir_path, "sigabrt");
    // Bus error
    test_killport_signal_arg(tempdir_path, "sigbus");
    // Floating point exception
    test_killport_signal_arg(tempdir_path, "sigfpe");
    // Kill (cannot be caught or ignored)
    test_killport_signal_arg(tempdir_path, "sigkill");
    // User defined signal 1
    test_killport_signal_arg(tempdir_path, "sigusr1");
    // Segmentation violation
    test_killport_signal_arg(tempdir_path, "sigsegv");
    // User defined signal 2
    test_killport_signal_arg(tempdir_path, "sigusr2");
    // Write on a pipe with no one to read it
    test_killport_signal_arg(tempdir_path, "sigpipe");
    // Alarm clock
    test_killport_signal_arg(tempdir_path, "sigalrm");
    // Software termination signal from kill
    test_killport_signal_arg(tempdir_path, "sigterm");
    // Stack fault (obsolete)
    #[cfg(all(any(target_os = "android", target_os = "emscripten",
                  target_os = "fuchsia", target_os = "linux"),
              not(any(target_arch = "mips", target_arch = "mips64",
                      target_arch = "sparc64"))))]
    test_killport_signal_arg(tempdir_path, "sigstkflt");
    // To parent on child stop or exit
    test_killport_signal_arg(tempdir_path, "sigchld");
    // Continue a stopped process
    test_killport_signal_arg(tempdir_path, "sigcont");
    // Sendable stop signal not from tty
    test_killport_signal_arg(tempdir_path, "sigstop");
    // Stop signal from tty
    test_killport_signal_arg(tempdir_path, "sigtstp");
    // To readers pgrp upon background tty read
    test_killport_signal_arg(tempdir_path, "sigttin");
    // Like TTIN if (tp->t_local&LTOSTOP)
    test_killport_signal_arg(tempdir_path, "sigttou");
    // Urgent condition on IO channel
    test_killport_signal_arg(tempdir_path, "sigurg");
    // Exceeded CPU time limit
    test_killport_signal_arg(tempdir_path, "sigxcpu");
    // Exceeded file size limit
    test_killport_signal_arg(tempdir_path, "sigxfsz");
    // Virtual time alarm
    test_killport_signal_arg(tempdir_path, "sigvtalrm");
    // Profiling time alarm
    test_killport_signal_arg(tempdir_path, "sigprof");
    // Window size changes
    test_killport_signal_arg(tempdir_path, "sigwinch");
    // Input/output possible signal
    #[cfg(not(target_os = "haiku"))]
    #[cfg_attr(docsrs, doc(cfg(all())))]
    test_killport_signal_arg(tempdir_path, "sigio");
    #[cfg(any(target_os = "android", target_os = "emscripten",
              target_os = "fuchsia", target_os = "linux"))]
    #[cfg_attr(docsrs, doc(cfg(all())))]
    // Power failure imminent.
    test_killport_signal_arg(tempdir_path, "sigpwr");
    // Bad system call
    test_killport_signal_arg(tempdir_path, "sigsys");
    #[cfg(not(any(target_os = "android", target_os = "emscripten",
                  target_os = "fuchsia", target_os = "linux",
                  target_os = "redox", target_os = "haiku")))]
    #[cfg_attr(docsrs, doc(cfg(all())))]
    // Emulator trap
    test_killport_signal_arg(tempdir_path, "sigemt");
    #[cfg(not(any(target_os = "android", target_os = "emscripten",
                  target_os = "fuchsia", target_os = "linux",
                  target_os = "redox", target_os = "haiku")))]
    #[cfg_attr(docsrs, doc(cfg(all())))]
    // Information request
    test_killport_signal_arg(tempdir_path, "siginfo");
}

fn test_killport_signal_arg(tempdir_path: &path::Path, signal: &str) {
    let mut mock_process = std::process::Command::new(tempdir_path.join("mock_process"))
        .spawn()
        .expect("Failed to run the mock process");

    // Test killport command with specifying a signal name
    let mut cmd = Command::cargo_bin("killport").expect("Failed to find killport binary");
    cmd.arg("8080")
        .arg("-s")
        .arg(signal)
        .assert()
        .success()
        .stdout("Successfully killed process listening on port 8080\n");

    // Cleanup: Terminate the mock process (if still running).
    let _ = mock_process.kill();
}

fn test_killport_noargs(tempdir_path: &path::Path) {
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
}

fn generate_temp_process(tempdir_path: &path::Path) {
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
}
