mod test_config;

pub(crate) struct ServiceConfig {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    pub after: Vec<String>,
    pub requires: Vec<String>,
}

pub(crate) use test_config::ordered_processes;
