use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use war_dodger::{Config, Phase, load_config, next_backoff, write_example};

const APP: &str = "war-dodger";
const ADVISORIES_URL: &str = "https://travel.state.gov/_res/rss/TAsTWs.xml";
const COUNTRIES_URL: &str =
    "https://raw.githubusercontent.com/mledoze/countries/master/countries.json";
const IP_LOCATION_URL: &str = "https://ipapi.co/json/";

#[derive(Deserialize)]
struct DeviceLocation {
    latitude: f64,
    longitude: f64,
}
#[derive(Deserialize)]
struct ReverseLocation {
    address: ReverseAddress,
}
#[derive(Deserialize)]
struct ReverseAddress {
    country_code: String,
}
#[derive(Deserialize)]
struct IpLocation {
    country_code: String,
}
#[derive(Deserialize)]
struct CountryName {
    common: String,
    official: String,
}
#[derive(Deserialize)]
struct Country {
    cca2: String,
    cca3: String,
    name: CountryName,
    #[serde(default)]
    borders: Vec<String>,
    #[serde(default, rename = "altSpellings")]
    alt_spellings: Vec<String>,
}
struct Snapshot {
    current: String,
    levels: BTreeMap<String, Phase>,
}

fn config_path() -> PathBuf {
    env::var_os("WAR_DODGER_CONFIG")
        .map(Into::into)
        .unwrap_or_else(|| {
            PathBuf::from(env::var_os("HOME").unwrap_or_else(|| ".".into()))
                .join(".config")
                .join(APP)
                .join("config.conf")
        })
}
fn state_path() -> PathBuf {
    env::var_os("WAR_DODGER_STATE")
        .map(Into::into)
        .unwrap_or_else(|| {
            PathBuf::from(env::var_os("HOME").unwrap_or_else(|| ".".into()))
                .join(".local")
                .join("state")
                .join(APP)
                .join("levels")
        })
}
fn cache_path() -> PathBuf {
    PathBuf::from(env::var_os("XDG_CACHE_HOME").unwrap_or_else(|| {
        PathBuf::from(env::var_os("HOME").unwrap_or_else(|| ".".into()))
            .join(".cache")
            .into_os_string()
    }))
    .join(APP)
    .join("countries.json")
}
fn usage() {
    eprintln!("Usage: {APP} [--config PATH] <init|once|retry|run|status>");
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("war-dodger/0.1 (personal safety monitor)")
        .build()
        .map_err(|e| e.to_string())
}
fn get_text(client: &reqwest::blocking::Client, url: &str) -> Result<String, String> {
    client
        .get(url)
        .send()
        .map_err(|e| format!("request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("request failed: {e}"))?
        .text()
        .map_err(|e| e.to_string())
}
fn location_from_termux() -> Result<(f64, f64), String> {
    let output = Command::new("termux-location")
        .args(["-p", "network"])
        .output()
        .map_err(|e| format!("Termux location unavailable: {e}"))?;
    if !output.status.success() {
        return Err("Termux location command failed".into());
    }
    let location: DeviceLocation = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("invalid Termux location: {e}"))?;
    Ok((location.latitude, location.longitude))
}
fn country_from_ip(client: &reqwest::blocking::Client) -> Result<String, String> {
    let response: IpLocation = client
        .get(IP_LOCATION_URL)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;
    Ok(response.country_code.to_uppercase())
}
fn current_country(client: &reqwest::blocking::Client, provider: &str) -> Result<String, String> {
    if provider == "termux" {
        if let Ok((lat, lon)) = location_from_termux() {
            let url = format!(
                "https://nominatim.openstreetmap.org/reverse?format=jsonv2&lat={lat}&lon={lon}"
            );
            if let Ok(value) = client
                .get(url)
                .send()
                .map_err(|e| e.to_string())
                .and_then(|response| response.error_for_status().map_err(|e| e.to_string()))
                .and_then(|response| {
                    response
                        .json::<ReverseLocation>()
                        .map_err(|e| e.to_string())
                })
            {
                return Ok(value.address.country_code.to_uppercase());
            }
        }
    }
    country_from_ip(client)
}
fn countries(client: &reqwest::blocking::Client) -> Result<Vec<Country>, String> {
    let path = cache_path();
    let fresh = fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|m| SystemTime::now().duration_since(m).ok())
        .is_some_and(|age| age < Duration::from_secs(30 * 24 * 3600));
    let text = if fresh {
        fs::read_to_string(&path).map_err(|e| e.to_string())?
    } else {
        let text = get_text(client, COUNTRIES_URL)?;
        fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))
            .map_err(|e| e.to_string())?;
        fs::write(&path, &text).map_err(|e| e.to_string())?;
        text
    };
    serde_json::from_str(&text).map_err(|e| format!("invalid country metadata: {e}"))
}
fn advisory_levels(feed: &str, countries: &[&Country]) -> Result<BTreeMap<String, Phase>, String> {
    let mut levels = BTreeMap::new();
    for country in countries {
        let mut names = vec![country.name.common.as_str(), country.name.official.as_str()];
        names.extend(country.alt_spellings.iter().map(String::as_str));
        let phase = names
            .iter()
            .find_map(|name| {
                let marker = format!("<title>{name} - Level ");
                feed.find(&marker)
                    .and_then(|index| feed.get(index + marker.len()..))
                    .and_then(|rest| rest.chars().next())
                    .and_then(|ch| Phase::parse(&ch.to_string()).ok())
            })
            .ok_or_else(|| format!("no RSS advisory level for {}", country.cca2))?;
        levels.insert(country.cca2.clone(), phase);
    }
    Ok(levels)
}
fn read_snapshot(config: &Config) -> Result<Snapshot, String> {
    let client = client()?;
    let current = current_country(&client, &config.location_provider)?;
    let data = countries(&client)?;
    let current_country = data
        .iter()
        .find(|c| c.cca2 == current)
        .ok_or("current country not in metadata")?;
    let watched: Vec<&Country> = std::iter::once(current_country)
        .chain(
            current_country
                .borders
                .iter()
                .filter_map(|border| data.iter().find(|c| &c.cca3 == border)),
        )
        .collect();
    let feed = get_text(&client, ADVISORIES_URL)?;
    Ok(Snapshot {
        current,
        levels: advisory_levels(&feed, &watched)?,
    })
}
fn parse_levels(text: &str) -> Result<BTreeMap<String, Phase>, String> {
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (country, phase) = line
                .split_once('=')
                .ok_or_else(|| format!("invalid state line: {line:?}"))?;
            Ok((
                country.to_owned(),
                Phase::parse(phase).map_err(|e| e.to_string())?,
            ))
        })
        .collect()
}
fn load_levels(path: &Path) -> Result<BTreeMap<String, Phase>, String> {
    match fs::read_to_string(path) {
        Ok(text) => parse_levels(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(e.to_string()),
    }
}
fn save_levels(path: &Path, levels: &BTreeMap<String, Phase>) -> Result<(), String> {
    fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|e| e.to_string())?;
    fs::write(
        path,
        levels
            .iter()
            .map(|(c, p)| format!("{c}={p}\n"))
            .collect::<String>(),
    )
    .map_err(|e| e.to_string())
}
fn notify(config: &Config, country: &str, from: Option<Phase>, to: Phase) -> Result<(), String> {
    let message = match from {
        Some(p) if p == to => format!(
            "{country}: Level {} ({}) remains active. Hourly reminder.",
            to.number(),
            to.label()
        ),
        Some(p) => format!(
            "{country}: Travel Advisory Level {} ({}) → Level {} ({}).",
            p.number(),
            p.label(),
            to.number(),
            to.label()
        ),
        None => format!(
            "{country}: Travel Advisory Level {} ({}).",
            to.number(),
            to.label()
        ),
    };
    let status = Command::new("sh")
        .arg("-c")
        .arg(&config.notify_command)
        .env("WAR_DODGER_TITLE", format!("War Dodger: {country}"))
        .env("WAR_DODGER_MESSAGE", message)
        .stdin(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("notification command exited with {status}"))
}
fn check_once(config: &Config, state: &Path) -> Result<(), String> {
    let snapshot = read_snapshot(config)?;
    let previous = load_levels(state)?;
    let level = snapshot.levels[&snapshot.current];
    let old = previous.get(&snapshot.current).copied();
    if old != Some(level) && (old.is_some() || config.notify_on_initial) {
        notify(config, &snapshot.current, old, level)?;
    }
    if old == Some(Phase::Three) && level == Phase::Three {
        notify(config, &snapshot.current, old, level)?;
    }
    for (country, phase) in &snapshot.levels {
        if country != &snapshot.current
            && matches!(
                (previous.get(country), phase),
                (Some(Phase::Two), Phase::Three) | (Some(Phase::Three), Phase::Four)
            )
        {
            notify(config, country, previous.get(country).copied(), *phase)?;
        }
    }
    save_levels(state, &snapshot.levels)?;
    println!("{APP}: {} is at phase {}", snapshot.current, level.number());
    Ok(())
}
fn retry_until_success(config: &Config, state: &Path) {
    let mut delay = None;
    loop {
        if let Some(delay) = delay {
            thread::sleep(delay);
        }
        match check_once(config, state) {
            Ok(()) => return,
            Err(error) => {
                eprintln!("{APP}: {error}; retrying");
                delay = Some(next_backoff(delay, config.retry_min, config.retry_max));
            }
        }
    }
}
fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let mut path = config_path();
    let action = match args.next() {
        Some(arg) if arg == "--config" => args.next().and_then(|value| {
            path = value.into();
            args.next()
        }),
        value => value,
    };
    if args.next().is_some() || action.is_none() {
        usage();
        return ExitCode::from(2);
    }
    let action = action.expect("checked");
    if action == "init" {
        return write_example(&path)
            .map(|_| {
                println!("Created {}", path.display());
                ExitCode::SUCCESS
            })
            .unwrap_or_else(|e| {
                eprintln!("{APP}: {e}");
                ExitCode::FAILURE
            });
    }
    let config = match load_config(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{APP}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let state = state_path();
    match action.to_string_lossy().as_ref() {
        "once" => check_once(&config, &state)
            .map(|_| ExitCode::SUCCESS)
            .unwrap_or_else(|e| {
                eprintln!("{APP}: {e}");
                ExitCode::FAILURE
            }),
        "retry" => {
            retry_until_success(&config, &state);
            ExitCode::SUCCESS
        }
        "run" => loop {
            retry_until_success(&config, &state);
            thread::sleep(config.interval);
        },
        "status" => match load_levels(&state) {
            Ok(levels) => {
                for (c, p) in levels {
                    println!("{c}: phase {}", p.number());
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{APP}: {e}");
                ExitCode::FAILURE
            }
        },
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}
