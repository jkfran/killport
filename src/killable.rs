use crate::container::Container;
use crate::signal::KillportSignal;
use std::fmt::Display;
use std::io::Error;
use tokio::runtime::Builder;

/// Interface for killable targets such as native processes and containers.
pub trait Killable {
    fn kill(&self, signal: KillportSignal) -> Result<bool, Error>;

    fn get_type(&self) -> KillableType;

    fn get_name(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillableType {
    Process,
    Container,
}

impl Display for KillableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            KillableType::Process => "process",
            KillableType::Container => "container",
        })
    }
}

impl Killable for Container {
    fn kill(&self, signal: KillportSignal) -> Result<bool, Error> {
        let rt = Builder::new_current_thread().enable_all().build()?;
        Self::kill(&rt, &self.name, signal)?;
        Ok(true)
    }

    fn get_type(&self) -> KillableType {
        KillableType::Container
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_killable_type_display_process() {
        assert_eq!(KillableType::Process.to_string(), "process");
    }

    #[test]
    fn test_killable_type_display_container() {
        assert_eq!(KillableType::Container.to_string(), "container");
    }

    #[test]
    fn test_killable_type_eq() {
        assert_eq!(KillableType::Process, KillableType::Process);
        assert_eq!(KillableType::Container, KillableType::Container);
    }

    #[test]
    fn test_killable_type_ne() {
        assert_ne!(KillableType::Process, KillableType::Container);
    }

    #[test]
    fn test_killable_type_clone() {
        let original = KillableType::Process;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_killable_type_debug() {
        let debug_str = format!("{:?}", KillableType::Process);
        assert_eq!(debug_str, "Process");
        let debug_str = format!("{:?}", KillableType::Container);
        assert_eq!(debug_str, "Container");
    }

    #[test]
    fn test_container_get_type() {
        let container = Container {
            name: "test".to_string(),
        };
        assert_eq!(container.get_type(), KillableType::Container);
    }

    #[test]
    fn test_container_get_name() {
        let container = Container {
            name: "my-container".to_string(),
        };
        assert_eq!(container.get_name(), "my-container");
    }

    #[test]
    fn test_container_get_name_empty() {
        let container = Container {
            name: String::new(),
        };
        assert_eq!(container.get_name(), "");
    }

    #[test]
    fn test_container_get_name_special_chars() {
        let container = Container {
            name: "my/container-name_123".to_string(),
        };
        assert_eq!(container.get_name(), "my/container-name_123");
    }
}
