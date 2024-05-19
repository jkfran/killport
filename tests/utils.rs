use std::process::{Child, Command as SystemCommand};
use std::{fs::File, io::Write, path::Path, thread, time::Duration};

/// Generates and starts a mock Rust application that listens on a given port.
pub fn start_listener_process(tempdir_path: &Path, port: u16) -> Child {
    let mock_process_code = format!(
        r#"
        use std::net::TcpListener;
        use std::time::Duration;
        use std::thread;

        fn main() {{
            let mut listener = None;
            for _ in 0..5 {{
                match TcpListener::bind("127.0.0.1:{}") {{
                    Ok(l) => {{
                        listener = Some(l);
                        break;
                    }},
                    Err(_) => thread::sleep(Duration::from_millis(500)),
                }}
            }}
            let listener = listener.expect("Failed to bind to port after several attempts");
            println!("Listening on port {}");
            loop {{ let _ = listener.accept(); }}
        }}
    "#,
        port, port
    );

    let mock_process_path = tempdir_path.join("mock_process.rs");
    let mut file = File::create(&mock_process_path).expect("Failed to create mock_process.rs file");
    file.write_all(mock_process_code.as_bytes())
        .expect("Failed to write mock process code");

    let status = SystemCommand::new("rustc")
        .args(&[
            mock_process_path.to_str().unwrap(),
            "--out-dir",
            tempdir_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to compile the mock process");

    assert!(status.success(), "Compilation of mock process failed");

    let mock_binary_path = tempdir_path.join("mock_process");
    let child = SystemCommand::new(mock_binary_path)
        .spawn()
        .expect("Failed to start the mock process");

    thread::sleep(Duration::from_secs(1));

    child
}
