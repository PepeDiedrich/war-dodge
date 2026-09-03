use std::fmt;
use std::fs;
use std::io;
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub command: String,
    pub interval: Duration,
    pub retry_min: Duration,
    pub retry_max: Duration,
    pub health_host: SocketAddr,
    pub connect_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            command: "echo 'set command in config.conf'".into(),
            interval: Duration::from_secs(30 * 60),
            retry_min: Duration::from_secs(30),
            retry_max: Duration::from_secs(15 * 60),
            // TCP is cheaper than an HTTP request and needs no TLS dependency.
            health_host: "1.1.1.1:53".parse().expect("static socket address"),
            connect_timeout: Duration::from_secs(8),
        }
    }
}

#[derive(Debug)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ConfigError {}

pub fn parse_duration(input: &str) -> Result<Duration, ConfigError> {
    let input = input.trim();
    let (number, unit) = input
        .chars()
        .position(|ch| !ch.is_ascii_digit())
        .map(|index| input.split_at(index))
        .ok_or_else(|| ConfigError(format!("duration needs a unit: {input:?}")))?;
    let amount: u64 = number
        .parse()
        .map_err(|_| ConfigError(format!("invalid duration: {input:?}")))?;
    if amount == 0 {
        return Err(ConfigError("duration must be greater than zero".into()));
    }
    let seconds = match unit {
        "s" => Some(amount),
        "m" => amount.checked_mul(60),
        "h" => amount.checked_mul(60 * 60),
        _ => {
            return Err(ConfigError(format!(
                "duration unit must be s, m, or h: {input:?}"
            )));
        }
    }
    .ok_or_else(|| ConfigError(format!("duration is too large: {input:?}")))?;
    Ok(Duration::from_secs(seconds))
}

pub fn load_config(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let mut config = Config::default();
    for (line_number, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| {
            ConfigError(format!("line {}: expected key = value", line_number + 1))
        })?;
        match key.trim() {
            "command" => config.command = value.trim().to_owned(),
            "interval" => config.interval = parse_duration(value)?,
            "retry_min" => config.retry_min = parse_duration(value)?,
            "retry_max" => config.retry_max = parse_duration(value)?,
            "health_host" => {
                config.health_host = value.trim().parse().map_err(|_| {
                    ConfigError(format!("line {}: invalid host:port", line_number + 1))
                })?
            }
            "connect_timeout" => config.connect_timeout = parse_duration(value)?,
            key => {
                return Err(Box::new(ConfigError(format!(
                    "line {}: unknown setting {key:?}",
                    line_number + 1
                ))));
            }
        }
    }
    if config.command.is_empty() {
        return Err(Box::new(ConfigError("command must not be empty".into())));
    }
    if config.retry_max < config.retry_min {
        return Err(Box::new(ConfigError(
            "retry_max must be at least retry_min".into(),
        )));
    }
    Ok(config)
}

pub fn write_example(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))?;
    fs::write(path, include_str!("../config.example.conf"))
}

pub fn network_available(config: &Config) -> io::Result<()> {
    TcpStream::connect_timeout(&config.health_host, config.connect_timeout).map(|_| ())
}

pub fn next_backoff(previous: Option<Duration>, min: Duration, max: Duration) -> Duration {
    previous
        .map(|duration| duration.saturating_mul(2).min(max))
        .unwrap_or(min)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durations_are_strict() {
        assert_eq!(parse_duration("15m").unwrap(), Duration::from_secs(900));
        for invalid in ["", "0s", "5", "-1s", "3d"] {
            assert!(parse_duration(invalid).is_err(), "{invalid} should fail");
        }
    }
    #[test]
    fn backoff_is_capped() {
        let min = Duration::from_secs(2);
        let max = Duration::from_secs(5);
        assert_eq!(next_backoff(None, min, max), min);
        assert_eq!(next_backoff(Some(min), min, max), Duration::from_secs(4));
        assert_eq!(next_backoff(Some(Duration::from_secs(4)), min, max), max);
    }
}
