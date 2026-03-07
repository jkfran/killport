#![allow(dead_code)]

use std::net::TcpListener;
use std::process::{Child, Command as SystemCommand};
use std::time::{Duration, Instant};
use std::{fs::File, io::Write, path::Path, thread};

/// Returns an available port by binding to port 0 and reading the assigned port.
pub fn get_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to port 0");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    // Small delay to ensure the OS releases the port
    thread::sleep(Duration::from_millis(50));
    port
}

/// Polls until a TCP connection can be established on the given port, or times out.
pub fn wait_for_tcp_port(port: u16, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Checks if a process with the given PID is still alive.
#[cfg(unix)]
pub fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(windows)]
pub fn is_process_alive(pid: u32) -> bool {
    use std::process::Command;
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid)])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}

/// Compiles a mock Rust source file and spawns it as a child process.
fn compile_and_spawn(tempdir_path: &Path, source: &str, binary_name: &str) -> Child {
    let source_path = tempdir_path.join(format!("{}.rs", binary_name));
    let mut file = File::create(&source_path).expect("Failed to create source file");
    file.write_all(source.as_bytes())
        .expect("Failed to write source code");

    let status = SystemCommand::new("rustc")
        .args([
            source_path.to_str().unwrap(),
            "--out-dir",
            tempdir_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to compile the mock process");

    assert!(status.success(), "Compilation of mock process failed");

    let binary_path = tempdir_path.join(binary_name);
    SystemCommand::new(binary_path)
        .spawn()
        .expect("Failed to start the mock process")
}

/// Generates and starts a mock Rust application that listens on a given TCP port (IPv4).
pub fn start_listener_process(tempdir_path: &Path, port: u16) -> Child {
    start_tcp_listener(tempdir_path, port)
}

/// Generates and starts a mock Rust application that listens on a given TCP port (IPv4).
pub fn start_tcp_listener(tempdir_path: &Path, port: u16) -> Child {
    let code = format!(
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

    let child = compile_and_spawn(tempdir_path, &code, "mock_process");
    assert!(
        wait_for_tcp_port(port, Duration::from_secs(5)),
        "TCP listener did not start on port {} within timeout",
        port
    );
    child
}

/// Generates and starts a mock Rust application that listens on a given TCP port (IPv6).
pub fn start_tcp6_listener(tempdir_path: &Path, port: u16) -> Child {
    let code = format!(
        r#"
        use std::net::TcpListener;
        use std::time::Duration;
        use std::thread;

        fn main() {{
            let mut listener = None;
            for _ in 0..5 {{
                match TcpListener::bind("[::1]:{}") {{
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

    let child = compile_and_spawn(tempdir_path, &code, "mock_process");
    // Wait a bit for the IPv6 listener to bind
    thread::sleep(Duration::from_secs(1));
    child
}

/// Generates and starts a mock Rust application that listens on a given UDP port (IPv4).
pub fn start_udp_listener(tempdir_path: &Path, port: u16) -> Child {
    let code = format!(
        r#"
        use std::net::UdpSocket;

        fn main() {{
            let socket = UdpSocket::bind("127.0.0.1:{}").expect("Failed to bind UDP socket");
            println!("Listening on UDP port {}");
            let mut buf = [0u8; 1024];
            loop {{ let _ = socket.recv_from(&mut buf); }}
        }}
    "#,
        port, port
    );

    let child = compile_and_spawn(tempdir_path, &code, "mock_process");
    // Wait for the UDP socket to bind
    thread::sleep(Duration::from_secs(1));
    child
}

/// Generates and starts a mock Rust application that listens on a given UDP port (IPv6).
pub fn start_udp6_listener(tempdir_path: &Path, port: u16) -> Child {
    let code = format!(
        r#"
        use std::net::UdpSocket;

        fn main() {{
            let socket = UdpSocket::bind("[::1]:{}").expect("Failed to bind UDP socket");
            println!("Listening on UDP port {}");
            let mut buf = [0u8; 1024];
            loop {{ let _ = socket.recv_from(&mut buf); }}
        }}
    "#,
        port, port
    );

    let child = compile_and_spawn(tempdir_path, &code, "mock_process");
    // Wait for the UDP socket to bind
    thread::sleep(Duration::from_secs(1));
    child
}
