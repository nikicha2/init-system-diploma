mod test_config;

use std::{collections::HashSet, error::Error, fmt};

pub(crate) struct ServiceConfig {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    pub after: Vec<String>,
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DuplicateServiceNameError {
    pub duplicate_name: String,
}

impl fmt::Display for DuplicateServiceNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "duplicate service name: {}", self.duplicate_name)
    }
}

impl Error for DuplicateServiceNameError {}

pub(crate) fn validate_unique_names(
    services: &[ServiceConfig],
) -> Result<(), DuplicateServiceNameError> {
    let mut names = HashSet::new();

    for service in services {
        if !names.insert(service.name.as_str()) {
            return Err(DuplicateServiceNameError {
                duplicate_name: service.name.clone(),
            });
        }
    }

    Ok(())
}

pub(crate) use test_config::ordered_processes;

#[cfg(test)]
mod tests {
    use super::*;

    fn service(name: &str) -> ServiceConfig {
        ServiceConfig {
            name: name.to_string(),
            program: "/bin/true".to_string(),
            args: vec![],
            after: vec![],
            requires: vec![],
        }
    }

    #[test]
    fn unique_service_names_are_accepted() {
        let services = vec![service("one"), service("two")];

        assert_eq!(validate_unique_names(&services), Ok(()));
    }

    #[test]
    fn duplicate_service_names_are_rejected() {
        let services = vec![service("duplicate"), service("duplicate")];

        let error = validate_unique_names(&services).unwrap_err();

        assert_eq!(error.duplicate_name, "duplicate");
    }
}
