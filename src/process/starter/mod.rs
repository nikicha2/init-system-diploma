mod service_starter;

use std::collections::HashMap;

use crate::config::ServiceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServiceState {
    Pending,
    Started,
    Failed,
}

pub(crate) fn start_services(services: &[ServiceConfig]) -> HashMap<String, ServiceState> {
    let mut states = HashMap::new();

    for service in services {
        states.insert(service.name.clone(), ServiceState::Pending);
    }

    loop {
        let mut progress = false;

        for service in services {
            if states[&service.name] != ServiceState::Pending {
                continue;
            }

            let after_ready = service.after.iter().all(|dependency| {
                states
                    .get(dependency)
                    .is_some_and(|state| *state != ServiceState::Pending)
            });

            let requires_ready = service
                .requires
                .iter()
                .all(|dependency| states.get(dependency) == Some(&ServiceState::Started));

            let requires_failed = service.requires.iter().any(|dependency| {
                matches!(states.get(dependency), None | Some(ServiceState::Failed))
            });

            if requires_failed {
                eprintln!(
                    "Skipping {}: a required service failed or doesn't exist",
                    service.name
                );

                states.insert(service.name.clone(), ServiceState::Failed);
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
                    states.insert(service.name.clone(), ServiceState::Started);
                }
                Err(error) => {
                    eprintln!("Failed to start {}: {}", service.name, error);
                    states.insert(service.name.clone(), ServiceState::Failed);
                }
            }

            progress = true;
        }

        if !progress {
            break;
        }
    }

    for service in services {
        if states[&service.name] == ServiceState::Pending {
            eprintln!(
                "Skipping {}: unresolved or circular dependencies",
                service.name
            );

            states.insert(service.name.clone(), ServiceState::Failed);
        }
    }

    states
}
