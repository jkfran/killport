#![cfg(unix)]

use killport::cli::Mode;
use killport::killport::{Killable, KillableType};
use killport::signal::KillportSignal;
use killport::unix::UnixProcess;
use mockall::*;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::io::Error;

// Setup Mocks
mock! {
    DockerContainer {}

    impl Killable for DockerContainer {
        fn kill(&self, signal: KillportSignal) -> Result<bool, Error>;
        fn get_type(&self) -> KillableType;
        fn get_name(&self) -> String;
    }
}
mock! {
    UnixProcess {}

    impl Killable for UnixProcess {
        fn kill(&self, signal: KillportSignal) -> Result<bool, Error>;
        fn get_type(&self) -> KillableType;
        fn get_name(&self) -> String;
    }
}
mock! {
    KillportOperations {
        fn find_target_killables(&self, port: u16, mode: Mode) -> Result<Vec<Box<dyn Killable>>, Error>;
        fn kill_service_by_port(&self, port: u16, signal: KillportSignal, mode: Mode, dry_run: bool) -> Result<Vec<(KillableType, String)>, Error>;
    }
}

#[test]
fn native_process_kill_succeeds() {
    let mut mock_process = MockUnixProcess::new();
    // Setup the expectation for the mock
    mock_process
        .expect_kill()
        .with(mockall::predicate::eq(KillportSignal(Signal::SIGKILL)))
        .times(1) // Ensure the kill method is called exactly once
        .returning(|_| Ok(true)); // Simulate successful kill

    assert_eq!(
        mock_process.kill(KillportSignal(Signal::SIGKILL)).unwrap(),
        true
    );
}

#[test]
fn docker_container_kill_succeeds() {
    let mut mock_container = MockDockerContainer::new();
    mock_container
        .expect_kill()
        .with(mockall::predicate::eq(KillportSignal(Signal::SIGKILL)))
        .times(1)
        .returning(|_| Ok(true));

    assert_eq!(
        mock_container
            .kill(KillportSignal(Signal::SIGKILL))
            .unwrap(),
        true
    );
}

#[test]
fn find_killables_processes_only() {
    let mut mock_killport = MockKillportOperations::new();

    mock_killport
        .expect_find_target_killables()
        .withf(|&port, &mode| port == 8080 && mode == Mode::Process)
        .returning(|_, _| {
            let mut mock_process = MockUnixProcess::new();
            mock_process
                .expect_get_type()
                .return_const(KillableType::Process);
            mock_process
                .expect_get_name()
                .return_const("mock_process".to_string());
            Ok(vec![Box::new(mock_process)])
        });

    let port = 8080;
    let mode = Mode::Process;
    let found_killables = mock_killport.find_target_killables(port, mode).unwrap();
    assert!(found_killables
        .iter()
        .all(|k| k.get_type() == KillableType::Process));
}

#[test]
fn kill_service_by_port_dry_run() {
    let mut mock_killport = MockKillportOperations::new();
    let mut mock_process = MockUnixProcess::new();

    mock_process.expect_kill().never();
    mock_process
        .expect_get_type()
        .return_const(KillableType::Process);
    mock_process
        .expect_get_name()
        .return_const("mock_process".to_string());

    mock_killport
        .expect_kill_service_by_port()
        .returning(|_, _, _, _| Ok(vec![(KillableType::Process, "mock_process".to_string())]));

    let port = 8080;
    let mode = Mode::Process;
    let dry_run = true;
    let signal = KillportSignal(Signal::SIGKILL);

    let results = mock_killport
        .kill_service_by_port(port, signal, mode, dry_run)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, KillableType::Process);
    assert_eq!(results[0].1, "mock_process");
}

#[test]
fn check_process_type_and_name() {
    let process = UnixProcess::new(Pid::from_raw(1234), "unique_process".to_string());

    assert_eq!(process.get_type(), KillableType::Process);
    assert_eq!(process.get_name(), "unique_process");
}

#[test]
fn check_docker_container_type_and_name() {
    let mut mock_container = MockDockerContainer::new();
    mock_container
        .expect_get_type()
        .times(1)
        .returning(|| KillableType::Container);
    mock_container
        .expect_get_name()
        .times(1)
        .returning(|| "docker_container".to_string());

    assert_eq!(mock_container.get_type(), KillableType::Container);
    assert_eq!(mock_container.get_name(), "docker_container");
}
