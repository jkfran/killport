use crate::docker::DockerContainer;
use crate::killable::{Killable, KillableType};
#[cfg(target_os = "linux")]
use crate::linux::find_target_processes;
#[cfg(target_os = "macos")]
use crate::macos::find_target_processes;
#[cfg(target_os = "windows")]
use crate::windows::find_target_processes;
use crate::{cli::Mode, signal::KillportSignal};
use std::io::Error;
use tokio::runtime::{Builder, Runtime};

/// Trait for finding native processes on a port (enables mocking in tests).
pub trait ProcessFinder {
    fn find_target_processes(&self, port: u16) -> Result<Vec<Box<dyn Killable>>, Error>;
}

/// Trait for Docker operations (enables mocking in tests).
pub trait DockerOps {
    fn is_docker_present(&self) -> Result<bool, Error>;
    fn find_target_containers(&self, port: u16) -> Result<Vec<DockerContainer>, Error>;
}

/// Real implementation of ProcessFinder that calls the platform-specific functions.
pub struct NativeProcessFinder;

impl ProcessFinder for NativeProcessFinder {
    fn find_target_processes(&self, port: u16) -> Result<Vec<Box<dyn Killable>>, Error> {
        let processes = find_target_processes(port)?;
        Ok(processes
            .into_iter()
            .map(|p| Box::new(p) as Box<dyn Killable>)
            .collect())
    }
}

/// Real implementation of DockerOps that calls the Docker API.
pub struct RealDockerOps {
    rt: Runtime,
}

impl RealDockerOps {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            rt: Builder::new_current_thread().enable_all().build()?,
        })
    }
}

impl DockerOps for RealDockerOps {
    fn is_docker_present(&self) -> Result<bool, Error> {
        DockerContainer::is_docker_present(&self.rt)
    }

    fn find_target_containers(&self, port: u16) -> Result<Vec<DockerContainer>, Error> {
        DockerContainer::find_target_containers(&self.rt, port)
    }
}

/// Killport implementation with injectable dependencies for testability.
pub struct Killport<P: ProcessFinder, D: DockerOps> {
    process_finder: P,
    docker_ops: D,
}

impl Killport<NativeProcessFinder, RealDockerOps> {
    /// Creates a Killport with real (production) dependencies.
    pub fn with_real_deps() -> Result<Self, Error> {
        Ok(Self::new(NativeProcessFinder, RealDockerOps::new()?))
    }
}

impl<P: ProcessFinder, D: DockerOps> Killport<P, D> {
    pub fn new(process_finder: P, docker_ops: D) -> Self {
        Self {
            process_finder,
            docker_ops,
        }
    }

    pub fn find_target_killables(
        &self,
        port: u16,
        mode: Mode,
    ) -> Result<Vec<Box<dyn Killable>>, Error> {
        let mut target_killables: Vec<Box<dyn Killable>> = vec![];
        let docker_present = mode != Mode::Process && self.docker_ops.is_docker_present()?;

        if mode != Mode::Container {
            let target_processes = self.process_finder.find_target_processes(port)?;

            for process in target_processes {
                // Check if the process name contains 'docker' and skip if in docker mode
                if docker_present && process.get_name().to_lowercase().contains("docker") {
                    continue;
                }

                target_killables.push(process);
            }
        }

        // Add containers if Docker is present and mode is not set to only process
        if docker_present && mode != Mode::Process {
            let target_containers = self.docker_ops.find_target_containers(port)?;

            for container in target_containers {
                target_killables.push(Box::new(container));
            }
        }

        Ok(target_killables)
    }

    pub fn kill_service_by_port(
        &self,
        port: u16,
        signal: KillportSignal,
        mode: Mode,
        dry_run: bool,
    ) -> Result<Vec<(KillableType, String)>, Error> {
        let mut results = Vec::new();
        let target_killables = self.find_target_killables(port, mode)?;

        for killable in target_killables {
            let killed = dry_run || killable.kill(signal.clone())?;
            if killed {
                results.push((killable.get_type(), killable.get_name()));
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use nix::sys::signal::Signal;
    use std::cell::RefCell;

    // ─── Mock implementations for testing orchestration logic ────────────

    struct MockKillable {
        kill_result: Result<bool, Error>,
        killable_type: KillableType,
        name: String,
        kill_called: RefCell<bool>,
    }

    impl MockKillable {
        fn process(name: &str) -> Self {
            Self {
                kill_result: Ok(true),
                killable_type: KillableType::Process,
                name: name.to_string(),
                kill_called: RefCell::new(false),
            }
        }

        fn with_kill_result(mut self, result: Result<bool, Error>) -> Self {
            self.kill_result = result;
            self
        }
    }

    impl Killable for MockKillable {
        fn kill(&self, _signal: KillportSignal) -> Result<bool, Error> {
            *self.kill_called.borrow_mut() = true;
            match &self.kill_result {
                Ok(v) => Ok(*v),
                Err(e) => Err(Error::new(e.kind(), e.to_string())),
            }
        }

        fn get_type(&self) -> KillableType {
            self.killable_type.clone()
        }

        fn get_name(&self) -> String {
            self.name.clone()
        }
    }

    struct FnProcessFinder<F: Fn(u16) -> Result<Vec<Box<dyn Killable>>, Error>> {
        finder: F,
    }

    impl<F: Fn(u16) -> Result<Vec<Box<dyn Killable>>, Error>> ProcessFinder for FnProcessFinder<F> {
        fn find_target_processes(&self, port: u16) -> Result<Vec<Box<dyn Killable>>, Error> {
            (self.finder)(port)
        }
    }

    struct FnDockerOps<
        P: Fn() -> Result<bool, Error>,
        C: Fn(u16) -> Result<Vec<DockerContainer>, Error>,
    > {
        is_present: P,
        find_containers: C,
    }

    impl<P: Fn() -> Result<bool, Error>, C: Fn(u16) -> Result<Vec<DockerContainer>, Error>>
        DockerOps for FnDockerOps<P, C>
    {
        fn is_docker_present(&self) -> Result<bool, Error> {
            (self.is_present)()
        }

        fn find_target_containers(&self, port: u16) -> Result<Vec<DockerContainer>, Error> {
            (self.find_containers)(port)
        }
    }

    #[allow(clippy::type_complexity)]
    fn no_docker() -> FnDockerOps<
        impl Fn() -> Result<bool, Error>,
        impl Fn(u16) -> Result<Vec<DockerContainer>, Error>,
    > {
        FnDockerOps {
            is_present: || Ok(false),
            find_containers: |_| Ok(vec![]),
        }
    }

    #[allow(clippy::type_complexity)]
    fn docker_with_containers(
        containers: Vec<String>,
    ) -> FnDockerOps<
        impl Fn() -> Result<bool, Error>,
        impl Fn(u16) -> Result<Vec<DockerContainer>, Error>,
    > {
        FnDockerOps {
            is_present: || Ok(true),
            find_containers: move |_| {
                Ok(containers
                    .iter()
                    .map(|n| DockerContainer { name: n.clone() })
                    .collect())
            },
        }
    }

    fn signal() -> KillportSignal {
        #[cfg(unix)]
        {
            KillportSignal(Signal::SIGKILL)
        }
        #[cfg(not(unix))]
        {
            KillportSignal("SIGKILL".to_string())
        }
    }

    // ─── Orchestration Tests: find_target_killables ──────────────────────

    #[test]
    fn test_find_killables_mode_auto_no_docker() {
        let finder = FnProcessFinder {
            finder: |_| {
                let p: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p])
            },
        };
        let kp = Killport::new(finder, no_docker());
        let results = kp.find_target_killables(8080, Mode::Auto).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get_type(), KillableType::Process);
        assert_eq!(results[0].get_name(), "my_app");
    }

    #[test]
    fn test_find_killables_mode_auto_with_docker_containers() {
        let finder = FnProcessFinder {
            finder: |_| {
                let p: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p])
            },
        };
        let docker = docker_with_containers(vec!["nginx".to_string()]);
        let kp = Killport::new(finder, docker);
        let results = kp.find_target_killables(8080, Mode::Auto).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].get_type(), KillableType::Process);
        assert_eq!(results[1].get_type(), KillableType::Container);
        assert_eq!(results[1].get_name(), "nginx");
    }

    #[test]
    fn test_find_killables_filters_docker_proxy_process() {
        let finder = FnProcessFinder {
            finder: |_| {
                let p1: Box<dyn Killable> = Box::new(MockKillable::process("docker-proxy"));
                let p2: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p1, p2])
            },
        };
        let docker = docker_with_containers(vec!["nginx".to_string()]);
        let kp = Killport::new(finder, docker);
        let results = kp.find_target_killables(8080, Mode::Auto).unwrap();
        // docker-proxy should be filtered out, my_app and nginx container remain
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].get_name(), "my_app");
        assert_eq!(results[1].get_name(), "nginx");
    }

    #[test]
    fn test_find_killables_filters_dockerd_daemon() {
        // The Docker daemon process itself (dockerd) must be filtered out
        // This was a real bug: killing dockerd instead of the container
        let finder = FnProcessFinder {
            finder: |_| {
                let p1: Box<dyn Killable> = Box::new(MockKillable::process("dockerd"));
                let p2: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p1, p2])
            },
        };
        let docker = docker_with_containers(vec!["nginx".to_string()]);
        let kp = Killport::new(finder, docker);
        let results = kp.find_target_killables(8080, Mode::Auto).unwrap();
        // dockerd must be filtered out -- only my_app and the nginx container remain
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].get_name(), "my_app");
        assert_eq!(results[1].get_name(), "nginx");
        // Critically: dockerd must NOT be in the results
        assert!(
            !results.iter().any(|r| r.get_name() == "dockerd"),
            "dockerd daemon must never be killed"
        );
    }

    #[test]
    fn test_find_killables_filters_docker_case_insensitive() {
        // Docker filtering should be case-insensitive (Docker, DOCKER, docker)
        let finder = FnProcessFinder {
            finder: |_| {
                let p1: Box<dyn Killable> = Box::new(MockKillable::process("Docker Desktop"));
                let p2: Box<dyn Killable> = Box::new(MockKillable::process("com.docker.backend"));
                let p3: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p1, p2, p3])
            },
        };
        let docker = docker_with_containers(vec!["nginx".to_string()]);
        let kp = Killport::new(finder, docker);
        let results = kp.find_target_killables(8080, Mode::Auto).unwrap();
        // Both Docker Desktop and com.docker.backend should be filtered
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].get_name(), "my_app");
        assert_eq!(results[1].get_name(), "nginx");
    }

    #[test]
    fn test_find_killables_no_docker_filter_when_docker_absent() {
        // When Docker is NOT present, processes with "docker" in name should NOT be filtered
        // (they're just regular processes in this context)
        let finder = FnProcessFinder {
            finder: |_| {
                let p1: Box<dyn Killable> = Box::new(MockKillable::process("docker-proxy"));
                let p2: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p1, p2])
            },
        };
        let kp = Killport::new(finder, no_docker());
        let results = kp.find_target_killables(8080, Mode::Auto).unwrap();
        // Both should be returned since Docker is not present
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].get_name(), "docker-proxy");
        assert_eq!(results[1].get_name(), "my_app");
    }

    #[test]
    fn test_find_killables_no_docker_filter_in_process_mode() {
        // In Process mode, docker filtering should NOT apply even if the name contains "docker"
        // because Docker is not checked at all in Process mode
        let finder = FnProcessFinder {
            finder: |_| {
                let p1: Box<dyn Killable> = Box::new(MockKillable::process("docker-proxy"));
                let p2: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p1, p2])
            },
        };
        // Docker should never be checked in Process mode
        let docker = FnDockerOps {
            is_present: || panic!("Docker should not be checked in Process mode"),
            find_containers: |_| panic!("Docker should not be checked in Process mode"),
        };
        let kp = Killport::new(finder, docker);
        let results = kp.find_target_killables(8080, Mode::Process).unwrap();
        // Both processes should be returned (no docker filtering in process mode)
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].get_name(), "docker-proxy");
        assert_eq!(results[1].get_name(), "my_app");
    }

    #[test]
    fn test_find_killables_docker_present_but_no_containers_on_port() {
        // Docker is running but no containers on this port
        // docker-proxy still gets filtered (correct: it's a docker process)
        let finder = FnProcessFinder {
            finder: |_| {
                let p1: Box<dyn Killable> = Box::new(MockKillable::process("docker-proxy"));
                let p2: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p1, p2])
            },
        };
        let docker = docker_with_containers(vec![]); // docker present, no containers
        let kp = Killport::new(finder, docker);
        let results = kp.find_target_killables(8080, Mode::Auto).unwrap();
        // docker-proxy is filtered (docker is present), only my_app remains
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get_name(), "my_app");
    }

    #[test]
    fn test_find_killables_only_docker_processes_all_filtered() {
        // Edge case: all found processes are docker-related, and a container is the real target
        let finder = FnProcessFinder {
            finder: |_| {
                let p1: Box<dyn Killable> = Box::new(MockKillable::process("docker-proxy"));
                let p2: Box<dyn Killable> = Box::new(MockKillable::process("dockerd"));
                Ok(vec![p1, p2])
            },
        };
        let docker = docker_with_containers(vec!["nginx".to_string()]);
        let kp = Killport::new(finder, docker);
        let results = kp.find_target_killables(8080, Mode::Auto).unwrap();
        // All native processes filtered, only the container remains
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get_type(), KillableType::Container);
        assert_eq!(results[0].get_name(), "nginx");
    }

    #[test]
    fn test_find_killables_mode_process_only() {
        let finder = FnProcessFinder {
            finder: |_| {
                let p: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p])
            },
        };
        // Docker should never be checked in Process mode
        let docker = FnDockerOps {
            is_present: || panic!("Docker should not be checked in Process mode"),
            find_containers: |_| panic!("Docker should not be checked in Process mode"),
        };
        let kp = Killport::new(finder, docker);
        let results = kp.find_target_killables(8080, Mode::Process).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get_type(), KillableType::Process);
    }

    #[test]
    fn test_find_killables_mode_container_only() {
        let finder = FnProcessFinder {
            finder: |_| panic!("Process finder should not be called in Container mode"),
        };
        let docker = docker_with_containers(vec!["redis".to_string()]);
        let kp = Killport::new(finder, docker);
        let results = kp.find_target_killables(8080, Mode::Container).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get_type(), KillableType::Container);
        assert_eq!(results[0].get_name(), "redis");
    }

    #[test]
    fn test_find_killables_empty_results() {
        let finder = FnProcessFinder {
            finder: |_| Ok(vec![]),
        };
        let kp = Killport::new(finder, no_docker());
        let results = kp.find_target_killables(8080, Mode::Auto).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_killables_process_finder_error() {
        let finder = FnProcessFinder {
            finder: |_| {
                Err(Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "access denied",
                ))
            },
        };
        let kp = Killport::new(finder, no_docker());
        let result = kp.find_target_killables(8080, Mode::Auto);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn test_find_killables_docker_check_error() {
        let finder = FnProcessFinder {
            finder: |_| Ok(vec![]),
        };
        let docker = FnDockerOps {
            is_present: || Err(Error::other("docker error")),
            find_containers: |_| Ok(vec![]),
        };
        let kp = Killport::new(finder, docker);
        let result = kp.find_target_killables(8080, Mode::Auto);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_killables_container_find_error() {
        let finder = FnProcessFinder {
            finder: |_| Ok(vec![]),
        };
        let docker = FnDockerOps {
            is_present: || Ok(true),
            find_containers: |_| Err(Error::other("container error")),
        };
        let kp = Killport::new(finder, docker);
        let result = kp.find_target_killables(8080, Mode::Auto);
        assert!(result.is_err());
    }

    // ─── Orchestration Tests: kill_service_by_port ───────────────────────

    #[test]
    fn test_kill_service_actual_kill_succeeds() {
        let finder = FnProcessFinder {
            finder: |_| {
                let p: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p])
            },
        };
        let kp = Killport::new(finder, no_docker());
        let results = kp
            .kill_service_by_port(8080, signal(), Mode::Auto, false)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, KillableType::Process);
        assert_eq!(results[0].1, "my_app");
    }

    #[test]
    fn test_kill_service_kill_returns_false() {
        let finder = FnProcessFinder {
            finder: |_| {
                let p: Box<dyn Killable> =
                    Box::new(MockKillable::process("my_app").with_kill_result(Ok(false)));
                Ok(vec![p])
            },
        };
        let kp = Killport::new(finder, no_docker());
        let results = kp
            .kill_service_by_port(8080, signal(), Mode::Auto, false)
            .unwrap();
        assert!(
            results.is_empty(),
            "Process that returned false should not be in results"
        );
    }

    #[test]
    fn test_kill_service_kill_error_propagates() {
        let finder =
            FnProcessFinder {
                finder: |_| {
                    let p: Box<dyn Killable> =
                        Box::new(MockKillable::process("my_app").with_kill_result(Err(
                            Error::new(std::io::ErrorKind::PermissionDenied, "EPERM"),
                        )));
                    Ok(vec![p])
                },
            };
        let kp = Killport::new(finder, no_docker());
        let result = kp.kill_service_by_port(8080, signal(), Mode::Auto, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_kill_service_dry_run_collects_without_killing() {
        // We can't directly check kill_called on the mock since it's moved,
        // but we can verify the results are collected and the names match
        let finder = FnProcessFinder {
            finder: |_| {
                let p: Box<dyn Killable> = Box::new(MockKillable::process("my_app"));
                Ok(vec![p])
            },
        };
        let kp = Killport::new(finder, no_docker());
        let results = kp
            .kill_service_by_port(8080, signal(), Mode::Auto, true)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, KillableType::Process);
        assert_eq!(results[0].1, "my_app");
    }

    #[test]
    fn test_kill_service_dry_run_empty() {
        let finder = FnProcessFinder {
            finder: |_| Ok(vec![]),
        };
        let kp = Killport::new(finder, no_docker());
        let results = kp
            .kill_service_by_port(8080, signal(), Mode::Auto, true)
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_kill_service_multiple_targets() {
        let finder = FnProcessFinder {
            finder: |_| {
                let p1: Box<dyn Killable> = Box::new(MockKillable::process("app1"));
                let p2: Box<dyn Killable> = Box::new(MockKillable::process("app2"));
                Ok(vec![p1, p2])
            },
        };
        let docker = docker_with_containers(vec!["nginx".to_string()]);
        let kp = Killport::new(finder, docker);
        let results = kp
            .kill_service_by_port(8080, signal(), Mode::Auto, true)
            .unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].1, "app1");
        assert_eq!(results[1].1, "app2");
        assert_eq!(results[2].1, "nginx");
    }
}
