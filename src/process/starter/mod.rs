mod service_starter;

use std::{collections::HashMap, fmt};

use crate::config::{DuplicateServiceNameError, ServiceConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessExit {
    Normal(i32),
    Signaled(i32),
    Unknown(libc::c_int),
}

impl fmt::Display for ProcessExit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Normal(code) => write!(formatter, "exited normally with code {code}"),
            Self::Signaled(signal) => write!(formatter, "terminated by signal {signal}"),
            Self::Unknown(status) => write!(formatter, "exited with unknown wait status {status}"),
        }
    }
}

pub(crate) fn process_exit_from_wait_status(status: libc::c_int) -> ProcessExit {
    if libc::WIFEXITED(status) {
        ProcessExit::Normal(libc::WEXITSTATUS(status))
    } else if libc::WIFSIGNALED(status) {
        ProcessExit::Signaled(libc::WTERMSIG(status))
    } else {
        ProcessExit::Unknown(status)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServiceState {
    Pending,
    Started,
    Failed,
    Exited(ProcessExit),
}

#[derive(Debug, Default)]
pub(crate) struct ServiceRegistry {
    states: HashMap<String, ServiceState>,
    pid_to_service: HashMap<libc::pid_t, String>,
}

impl ServiceRegistry {
    pub(crate) fn states(&self) -> &HashMap<String, ServiceState> {
        &self.states
    }

    pub(crate) fn associate_pid(&mut self, pid: libc::pid_t, service_name: String) {
        self.pid_to_service.insert(pid, service_name);
    }

    pub(crate) fn handle_child_exit(
        &mut self,
        pid: libc::pid_t,
        status: libc::c_int,
    ) -> Option<(String, ProcessExit)> {
        let service_name = self.pid_to_service.remove(&pid)?;
        let process_exit = process_exit_from_wait_status(status);

        if let Some(state) = self.states.get_mut(&service_name) {
            *state = ServiceState::Exited(process_exit);
        }

        Some((service_name, process_exit))
    }
}

pub(crate) fn start_services(
    services: &[ServiceConfig],
) -> Result<ServiceRegistry, DuplicateServiceNameError> {
    crate::config::validate_unique_names(services)?;

    let mut registry = ServiceRegistry::default();

    for service in services {
        registry
            .states
            .insert(service.name.clone(), ServiceState::Pending);
    }

    loop {
        let mut progress = false;

        for service in services {
            if registry.states[&service.name] != ServiceState::Pending {
                continue;
            }

            let after_ready = service.after.iter().all(|dependency| {
                registry
                    .states
                    .get(dependency)
                    .is_some_and(|state| *state != ServiceState::Pending)
            });

            let requires_ready = service
                .requires
                .iter()
                .all(|dependency| registry.states.get(dependency) == Some(&ServiceState::Started));

            let requires_failed = service.requires.iter().any(|dependency| {
                matches!(
                    registry.states.get(dependency),
                    None | Some(ServiceState::Failed) | Some(ServiceState::Exited(_))
                )
            });

            if requires_failed {
                eprintln!(
                    "Skipping {}: a required service failed or doesn't exist",
                    service.name
                );

                registry
                    .states
                    .insert(service.name.clone(), ServiceState::Failed);
                progress = true;
                continue;
            }

            if !after_ready || !requires_ready {
                println!("{} is waiting for dependencies", service.name);
                continue;
            }

            match service_starter::start_service(service) {
                Ok(pid) => {
                    println!("Started {} with PID {}", service.name, pid);
                    registry
                        .states
                        .insert(service.name.clone(), ServiceState::Started);

                    if let Ok(pid) = libc::pid_t::try_from(pid) {
                        registry.associate_pid(pid, service.name.clone());
                    } else {
                        eprintln!("Failed to track PID for {}: PID out of range", service.name);
                        registry
                            .states
                            .insert(service.name.clone(), ServiceState::Failed);
                    }
                }
                Err(error) => {
                    eprintln!("Failed to start {}: {}", service.name, error);
                    registry
                        .states
                        .insert(service.name.clone(), ServiceState::Failed);
                }
            }

            progress = true;
        }

        if !progress {
            break;
        }
    }

    for service in services {
        if registry.states[&service.name] == ServiceState::Pending {
            eprintln!(
                "Skipping {}: unresolved or circular dependencies",
                service.name
            );

            registry
                .states
                .insert(service.name.clone(), ServiceState::Failed);
        }
    }

    Ok(registry)
}

pub(crate) fn initialize_services(services: &[ServiceConfig]) -> ServiceRegistry {
    match start_services(services) {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("Failed to start services: {error}");
            ServiceRegistry::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(name: &str) -> ServiceConfig {
        ServiceConfig {
            name: name.to_string(),
            program: "/definitely/not/a/real/program".to_string(),
            args: vec![],
            after: vec![],
            requires: vec![],
        }
    }

    #[test]
    fn duplicate_names_do_not_start_processes() {
        let services = vec![service("duplicate"), service("duplicate")];

        let error = start_services(&services).unwrap_err();

        assert_eq!(error.duplicate_name, "duplicate");
    }

    #[test]
    fn initialize_with_duplicate_names_returns_empty_registry() {
        let services = vec![service("duplicate"), service("duplicate")];

        let registry = initialize_services(&services);

        assert!(registry.states().is_empty());
        assert!(registry.pid_to_service.is_empty());
    }

    #[test]
    fn pid_is_associated_with_service() {
        let mut registry = ServiceRegistry::default();
        registry
            .states
            .insert("database".to_string(), ServiceState::Started);

        registry.associate_pid(1234, "database".to_string());

        assert_eq!(
            registry.pid_to_service.get(&1234).map(String::as_str),
            Some("database")
        );
    }

    #[test]
    fn child_exit_updates_service_and_removes_pid() {
        let mut registry = ServiceRegistry::default();
        registry
            .states
            .insert("worker".to_string(), ServiceState::Started);
        registry.associate_pid(4321, "worker".to_string());

        let handled = registry.handle_child_exit(4321, 7 << 8);

        assert_eq!(
            handled,
            Some(("worker".to_string(), ProcessExit::Normal(7)))
        );
        assert_eq!(
            registry.states()["worker"],
            ServiceState::Exited(ProcessExit::Normal(7))
        );
        assert!(!registry.pid_to_service.contains_key(&4321));
    }

    #[test]
    fn unknown_pid_is_handled_without_change() {
        let mut registry = ServiceRegistry::default();
        registry
            .states
            .insert("worker".to_string(), ServiceState::Started);

        assert_eq!(registry.handle_child_exit(9999, 0), None);
        assert_eq!(registry.states()["worker"], ServiceState::Started);
    }

    #[test]
    fn normal_exit_and_signal_exit_are_distinguished() {
        assert_eq!(
            process_exit_from_wait_status(23 << 8),
            ProcessExit::Normal(23)
        );
        assert_eq!(
            process_exit_from_wait_status(libc::SIGTERM),
            ProcessExit::Signaled(libc::SIGTERM)
        );
    }
}
