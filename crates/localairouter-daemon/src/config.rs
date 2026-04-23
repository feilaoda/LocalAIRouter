use std::env;
use std::net::{IpAddr, Ipv4Addr};

use localairouter_core::{AppPaths, DEFAULT_MONITOR_BUFFER_LIMIT, load_app_settings};

pub const DAEMON_BINARY_NAME: &str = "localairouter-daemon";
pub const DAEMON_PORT_ENV: &str = "LOCALAIROUTER_PORT";
pub const LEGACY_DAEMON_PORT_ENV: &str = "LOCALOPENROUTER_PORT";
pub const OLDER_DAEMON_PORT_ENV: &str = "LOCALROUTER_DAEMON_PORT";
pub const OLDEST_DAEMON_PORT_ENV: &str = "LOCALROUTER_PORT";
pub const DAEMON_ALLOW_LAN_ENV: &str = "LOCALAIROUTER_ALLOW_LAN";
pub const LEGACY_DAEMON_ALLOW_LAN_ENV: &str = "LOCALOPENROUTER_ALLOW_LAN";
pub const OLDER_DAEMON_ALLOW_LAN_ENV: &str = "LOCALROUTER_ALLOW_LAN";
pub const DAEMON_PARENT_PID_ENV: &str = "LOCALAIROUTER_PARENT_PID";
pub const LEGACY_DAEMON_PARENT_PID_ENV: &str = "LOCALOPENROUTER_PARENT_PID";
pub const OLDER_DAEMON_PARENT_PID_ENV: &str = "LOCALROUTER_PARENT_PID";
pub const DEFAULT_TRACING_FILTER: &str = "localairouter=info,localairouter_daemon=info";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonConfig {
    pub port: u16,
    pub allow_lan_access: bool,
    pub monitor_buffer_limit: usize,
    pub parent_pid: Option<u32>,
    pub tracing_filter: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            port: localairouter_core::onboarding::DEFAULT_PORT,
            allow_lan_access: false,
            monitor_buffer_limit: DEFAULT_MONITOR_BUFFER_LIMIT as usize,
            parent_pid: None,
            tracing_filter: DEFAULT_TRACING_FILTER.into(),
        }
    }
}

impl DaemonConfig {
    pub fn from_env() -> Self {
        let primary_port = env::var(DAEMON_PORT_ENV).ok();
        let legacy_port = env::var(LEGACY_DAEMON_PORT_ENV).ok();
        let older_port = env::var(OLDER_DAEMON_PORT_ENV).ok();
        let oldest_port = env::var(OLDEST_DAEMON_PORT_ENV).ok();
        let primary_allow_lan = env::var(DAEMON_ALLOW_LAN_ENV).ok();
        let legacy_allow_lan = env::var(LEGACY_DAEMON_ALLOW_LAN_ENV).ok();
        let older_allow_lan = env::var(OLDER_DAEMON_ALLOW_LAN_ENV).ok();
        let primary_parent_pid = env::var(DAEMON_PARENT_PID_ENV).ok();
        let legacy_parent_pid = env::var(LEGACY_DAEMON_PARENT_PID_ENV).ok();
        let older_parent_pid = env::var(OLDER_DAEMON_PARENT_PID_ENV).ok();
        let mut config = Self::from_sources(
            primary_port.as_deref(),
            legacy_port.as_deref(),
            older_port.as_deref(),
            oldest_port.as_deref(),
            primary_allow_lan.as_deref(),
            legacy_allow_lan.as_deref(),
            older_allow_lan.as_deref(),
            primary_parent_pid.as_deref(),
            legacy_parent_pid.as_deref(),
            older_parent_pid.as_deref(),
        );
        let stored_settings = load_stored_settings();
        if let Some(settings) = stored_settings.as_ref() {
            config.monitor_buffer_limit = settings.monitor_buffer_limit as usize;
        }
        if select_env_value(&[
            primary_port.as_deref(),
            legacy_port.as_deref(),
            older_port.as_deref(),
            oldest_port.as_deref(),
        ])
        .is_none()
        {
            config.port = stored_settings
                .as_ref()
                .map(|settings| settings.daemon_port)
                .unwrap_or(localairouter_core::onboarding::DEFAULT_PORT);
        }
        if select_env_value(&[
            primary_allow_lan.as_deref(),
            legacy_allow_lan.as_deref(),
            older_allow_lan.as_deref(),
        ])
        .is_none()
        {
            config.allow_lan_access = stored_settings
                .as_ref()
                .map(|settings| settings.allow_lan_access)
                .unwrap_or(false);
        }
        config
    }

    fn from_sources(
        primary_port: Option<&str>,
        legacy_port: Option<&str>,
        older_port: Option<&str>,
        oldest_port: Option<&str>,
        primary_allow_lan: Option<&str>,
        legacy_allow_lan: Option<&str>,
        older_allow_lan: Option<&str>,
        primary_parent_pid: Option<&str>,
        legacy_parent_pid: Option<&str>,
        older_parent_pid: Option<&str>,
    ) -> Self {
        let mut config = Self::default();
        config.port = select_env_value(&[primary_port, legacy_port, older_port, oldest_port])
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(config.port);
        config.allow_lan_access =
            select_env_value(&[primary_allow_lan, legacy_allow_lan, older_allow_lan])
                .map(parse_bool_env)
                .unwrap_or(config.allow_lan_access);
        config.parent_pid =
            select_env_value(&[primary_parent_pid, legacy_parent_pid, older_parent_pid])
                .and_then(|value| value.parse::<u32>().ok())
                .filter(|pid| *pid > 1);
        config
    }

    pub fn bind_ip(&self) -> IpAddr {
        if self.allow_lan_access {
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        } else {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        }
    }
}

fn select_env_value<'a>(values: &[Option<&'a str>]) -> Option<&'a str> {
    values.iter().copied().flatten().next()
}

fn parse_bool_env(value: &str) -> bool {
    matches!(
        value.trim(),
        "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
    )
}

fn load_stored_settings() -> Option<localairouter_core::AppSettings> {
    AppPaths::discover()
        .ok()
        .and_then(|paths| load_app_settings(&paths).ok())
}

#[cfg(test)]
mod tests {
    use super::{
        DAEMON_ALLOW_LAN_ENV, DAEMON_PARENT_PID_ENV, DAEMON_PORT_ENV, DEFAULT_TRACING_FILTER,
        DaemonConfig, LEGACY_DAEMON_ALLOW_LAN_ENV, LEGACY_DAEMON_PARENT_PID_ENV,
        LEGACY_DAEMON_PORT_ENV, OLDER_DAEMON_ALLOW_LAN_ENV, OLDER_DAEMON_PARENT_PID_ENV,
        OLDER_DAEMON_PORT_ENV, OLDEST_DAEMON_PORT_ENV,
    };
    use localairouter_core::DEFAULT_MONITOR_BUFFER_LIMIT;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn daemon_config_defaults_when_env_is_missing() {
        let config =
            DaemonConfig::from_sources(None, None, None, None, None, None, None, None, None, None);
        assert_eq!(config.port, localairouter_core::onboarding::DEFAULT_PORT);
        assert!(!config.allow_lan_access);
        assert_eq!(config.bind_ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(
            config.monitor_buffer_limit,
            DEFAULT_MONITOR_BUFFER_LIMIT as usize
        );
        assert_eq!(config.parent_pid, None);
        assert_eq!(config.tracing_filter, DEFAULT_TRACING_FILTER);
    }

    #[test]
    fn daemon_config_prefers_primary_env_values() {
        let config = DaemonConfig::from_sources(
            Some("7440"),
            Some("7331"),
            Some("7222"),
            Some("7111"),
            Some("1"),
            Some("0"),
            Some("0"),
            Some("222"),
            Some("111"),
            Some("99"),
        );
        assert_eq!(config.port, 7440);
        assert!(config.allow_lan_access);
        assert_eq!(config.parent_pid, Some(222));
    }

    #[test]
    fn daemon_config_uses_legacy_values_when_primary_is_missing() {
        let config = DaemonConfig::from_sources(
            None,
            Some("7441"),
            Some("7338"),
            Some("7337"),
            None,
            Some("true"),
            Some("0"),
            None,
            Some("333"),
            Some("222"),
        );
        assert_eq!(config.port, 7441);
        assert!(config.allow_lan_access);
        assert_eq!(config.parent_pid, Some(333));
    }

    #[test]
    fn daemon_config_uses_older_values_when_newer_names_are_missing() {
        let config = DaemonConfig::from_sources(
            None,
            None,
            Some("7442"),
            Some("7337"),
            None,
            None,
            Some("yes"),
            None,
            None,
            Some("334"),
        );
        assert_eq!(config.port, 7442);
        assert!(config.allow_lan_access);
        assert_eq!(config.parent_pid, Some(334));
    }

    #[test]
    fn daemon_config_keeps_defaults_for_invalid_values() {
        let config = DaemonConfig::from_sources(
            Some("not-a-port"),
            Some("7441"),
            Some("7442"),
            Some("7337"),
            Some("not-a-bool"),
            Some("1"),
            Some("1"),
            Some("0"),
            Some("333"),
            Some("444"),
        );
        assert_eq!(config.port, localairouter_core::onboarding::DEFAULT_PORT);
        assert!(!config.allow_lan_access);
        assert_eq!(config.parent_pid, None);
    }

    #[test]
    fn daemon_config_env_names_are_stable() {
        assert_eq!(DAEMON_PORT_ENV, "LOCALAIROUTER_PORT");
        assert_eq!(LEGACY_DAEMON_PORT_ENV, "LOCALOPENROUTER_PORT");
        assert_eq!(OLDER_DAEMON_PORT_ENV, "LOCALROUTER_DAEMON_PORT");
        assert_eq!(OLDEST_DAEMON_PORT_ENV, "LOCALROUTER_PORT");
        assert_eq!(DAEMON_ALLOW_LAN_ENV, "LOCALAIROUTER_ALLOW_LAN");
        assert_eq!(LEGACY_DAEMON_ALLOW_LAN_ENV, "LOCALOPENROUTER_ALLOW_LAN");
        assert_eq!(OLDER_DAEMON_ALLOW_LAN_ENV, "LOCALROUTER_ALLOW_LAN");
        assert_eq!(DAEMON_PARENT_PID_ENV, "LOCALAIROUTER_PARENT_PID");
        assert_eq!(LEGACY_DAEMON_PARENT_PID_ENV, "LOCALOPENROUTER_PARENT_PID");
        assert_eq!(OLDER_DAEMON_PARENT_PID_ENV, "LOCALROUTER_PARENT_PID");
    }
}
