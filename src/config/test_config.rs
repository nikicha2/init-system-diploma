use super::ServiceConfig;

pub(crate) fn ordered_processes() -> Vec<ServiceConfig> {
    vec![
        ServiceConfig {
            name: "test2".to_string(),
            program: "/bin/busybox".to_string(),
            args: vec!["sleep".to_string(), "5".to_string()],
            after: vec!["test1".to_string()],
            requires: vec![],
        },
        ServiceConfig {
            name: "test1".to_string(),
            program: "/bin/busybox".to_string(),
            args: vec!["sleep".to_string(), "5".to_string()],
            after: vec![],
            requires: vec![],
        },
    ]
}
