use std::fmt;
use std::fs;
use std::io;
use std::net::{SocketAddr, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub location_provider: String,
    pub interval: Duration,
    pub retry_min: Duration,
    pub retry_max: Duration,
    pub health_host: SocketAddr,
    pub connect_timeout: Duration,
    pub notify_command: String,
    pub notify_on_initial: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            location_provider: "termux".into(),
            interval: Duration::from_secs(30 * 60),
            retry_min: Duration::from_secs(30),
            retry_max: Duration::from_secs(15 * 60),
            // TCP is cheaper than an HTTP request and needs no TLS dependency.
            health_host: "1.1.1.1:443".parse().expect("static socket address"),
            connect_timeout: Duration::from_secs(8),
            notify_command: "termux-notification --title \"$WAR_DODGER_TITLE\" --content \"$WAR_DODGER_MESSAGE\"".into(),
            notify_on_initial: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
}

impl Phase {
    pub fn number(self) -> u8 {
        self as u8
    }
    pub fn parse(input: &str) -> Result<Self, ConfigError> {
        match input.trim() {
            "1" => Ok(Self::One),
            "2" => Ok(Self::Two),
            "3" => Ok(Self::Three),
            "4" => Ok(Self::Four),
            value => Err(ConfigError(format!(
                "phase must be 1, 2, 3, or 4, got {value:?}"
            ))),
        }
    }
    pub fn title(self) -> String {
        format!("War Dodger: Level {}", self.number())
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::One => "Exercise normal precautions",
            Self::Two => "Exercise increased caution",
            Self::Three => "Reconsider travel",
            Self::Four => "Do not travel",
        }
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.number())
    }
}
pub fn parse_phase_output(output: &str) -> Result<Phase, ConfigError> {
    Phase::parse(output)
}

pub fn load_phase(path: &Path) -> io::Result<Option<Phase>> {
    match fs::read_to_string(path) {
        Ok(value) => Phase::parse(&value)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
pub fn save_phase(path: &Path, phase: Phase) -> io::Result<()> {
    fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))?;
    fs::write(path, format!("{phase}\n"))
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
            "location_provider" => config.location_provider = value.trim().to_owned(),
            "interval" => config.interval = parse_duration(value)?,
            "retry_min" => config.retry_min = parse_duration(value)?,
            "retry_max" => config.retry_max = parse_duration(value)?,
            "health_host" => {
                config.health_host = value.trim().parse().map_err(|_| {
                    ConfigError(format!("line {}: invalid host:port", line_number + 1))
                })?
            }
            "connect_timeout" => config.connect_timeout = parse_duration(value)?,
            "notify_command" => config.notify_command = value.trim().to_owned(),
            "notify_on_initial" => {
                config.notify_on_initial = match value.trim() {
                    "true" => true,
                    "false" => false,
                    value => {
                        return Err(Box::new(ConfigError(format!(
                            "line {}: expected true or false, got {value:?}",
                            line_number + 1
                        ))));
                    }
                }
            }
            key => {
                return Err(Box::new(ConfigError(format!(
                    "line {}: unknown setting {key:?}",
                    line_number + 1
                ))));
            }
        }
    }
    if !matches!(config.location_provider.as_str(), "termux" | "ip") {
        return Err(Box::new(ConfigError(
            "location_provider must be termux or ip".into(),
        )));
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

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

/// Writes the executable Termux:Boot entry atomically and marks it executable.
pub fn write_boot_script(boot_dir: &Path, executable: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(boot_dir)?;
    let destination = boot_dir.join("start-war-dodger");
    let temporary = boot_dir.join(format!(".start-war-dodger.{}.tmp", std::process::id()));
    let executable = shell_quote(&executable.to_string_lossy());
    let script = format!(
        "#!/data/data/com.termux/files/usr/bin/sh\n\
         mkdir -p \"$HOME/.local/state/war-dodger\"\n\
         termux-wake-lock\n\
         exec {executable} run >> \"$HOME/.local/state/war-dodger/run.log\" 2>&1\n"
    );
    fs::write(&temporary, script)?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o700))?;
    fs::rename(&temporary, &destination)?;
    Ok(destination)
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
    #[test]
    fn phase_output_is_restricted_to_four_phases() {
        assert_eq!(parse_phase_output(" 3\n").unwrap(), Phase::Three);
        assert!(parse_phase_output("0").is_err());
        assert!(parse_phase_output("critical").is_err());
    }

    #[test]
    fn boot_script_is_executable_and_shell_quotes_the_binary() {
        let directory =
            std::env::temp_dir().join(format!("war-dodger-boot-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        let path = write_boot_script(&directory, Path::new("/tmp/war dodger's/bin")).unwrap();
        let script = fs::read_to_string(&path).unwrap();
        assert!(script.contains("exec '/tmp/war dodger'\"'\"'s/bin' run"));
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
