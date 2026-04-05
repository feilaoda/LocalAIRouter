use std::env;

pub const DAEMON_BINARY_NAME: &str = "localopenrouter-daemon";
pub const DAEMON_PORT_ENV: &str = "LOCALOPENROUTER_PORT";
pub const LEGACY_DAEMON_PORT_ENV: &str = "LOCALROUTER_DAEMON_PORT";
pub const DAEMON_PARENT_PID_ENV: &str = "LOCALOPENROUTER_PARENT_PID";
pub const LEGACY_DAEMON_PARENT_PID_ENV: &str = "LOCALROUTER_PARENT_PID";
pub const DEFAULT_TRACING_FILTER: &str = "localopenrouter=info,localopenrouter_daemon=info";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonConfig {
    pub port: u16,
    pub parent_pid: Option<u32>,
    pub tracing_filter: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            port: localopenrouter_core::onboarding::DEFAULT_PORT,
            parent_pid: None,
            tracing_filter: DEFAULT_TRACING_FILTER.into(),
        }
    }
}

impl DaemonConfig {
    pub fn from_env() -> Self {
        let primary_port = env::var(DAEMON_PORT_ENV).ok();
        let legacy_port = env::var(LEGACY_DAEMON_PORT_ENV).ok();
        let primary_parent_pid = env::var(DAEMON_PARENT_PID_ENV).ok();
        let legacy_parent_pid = env::var(LEGACY_DAEMON_PARENT_PID_ENV).ok();
        Self::from_sources(
            primary_port.as_deref(),
            legacy_port.as_deref(),
            primary_parent_pid.as_deref(),
            legacy_parent_pid.as_deref(),
        )
    }

    fn from_sources(
        primary_port: Option<&str>,
        legacy_port: Option<&str>,
        primary_parent_pid: Option<&str>,
        legacy_parent_pid: Option<&str>,
    ) -> Self {
        let mut config = Self::default();
        config.port = select_env_value(primary_port, legacy_port)
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(config.port);
        config.parent_pid = select_env_value(primary_parent_pid, legacy_parent_pid)
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|pid| *pid > 1);
        config
    }
}

fn select_env_value<'a>(primary: Option<&'a str>, legacy: Option<&'a str>) -> Option<&'a str> {
    primary.or(legacy)
}

#[cfg(test)]
mod tests {
    use super::{
        DAEMON_PARENT_PID_ENV, DAEMON_PORT_ENV, DEFAULT_TRACING_FILTER, DaemonConfig,
        LEGACY_DAEMON_PARENT_PID_ENV, LEGACY_DAEMON_PORT_ENV,
    };

    #[test]
    fn daemon_config_defaults_when_env_is_missing() {
        let config = DaemonConfig::from_sources(None, None, None, None);
        assert_eq!(config.port, localopenrouter_core::onboarding::DEFAULT_PORT);
        assert_eq!(config.parent_pid, None);
        assert_eq!(config.tracing_filter, DEFAULT_TRACING_FILTER);
    }

    #[test]
    fn daemon_config_prefers_primary_env_values() {
        let config =
            DaemonConfig::from_sources(Some("7440"), Some("7331"), Some("222"), Some("111"));
        assert_eq!(config.port, 7440);
        assert_eq!(config.parent_pid, Some(222));
    }

    #[test]
    fn daemon_config_uses_legacy_values_when_primary_is_missing() {
        let config = DaemonConfig::from_sources(None, Some("7441"), None, Some("333"));
        assert_eq!(config.port, 7441);
        assert_eq!(config.parent_pid, Some(333));
    }

    #[test]
    fn daemon_config_keeps_defaults_for_invalid_values() {
        let config =
            DaemonConfig::from_sources(Some("not-a-port"), Some("7441"), Some("0"), Some("333"));
        assert_eq!(config.port, localopenrouter_core::onboarding::DEFAULT_PORT);
        assert_eq!(config.parent_pid, None);
    }

    #[test]
    fn daemon_config_env_names_are_stable() {
        assert_eq!(DAEMON_PORT_ENV, "LOCALOPENROUTER_PORT");
        assert_eq!(LEGACY_DAEMON_PORT_ENV, "LOCALROUTER_DAEMON_PORT");
        assert_eq!(DAEMON_PARENT_PID_ENV, "LOCALOPENROUTER_PARENT_PID");
        assert_eq!(LEGACY_DAEMON_PARENT_PID_ENV, "LOCALROUTER_PARENT_PID");
    }
}
