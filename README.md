# War Dodger

War Dodger is a lightweight Termux monitor for U.S. Department of State Travel
Advisory levels. It checks the official RSS feed every hour for the country
where the device is located and every directly bordering country.

It is a personal alerting tool, not an emergency service. Always read the full
country-specific advisory and follow local authorities.

## Alert rules

- **Current country:** notify for every level change.
- **Current country at Level 3:** send an additional reminder after each
  successful hourly check until the level changes.
- **Directly bordering countries:** notify only for escalations from Level 2 to
  3 or Level 3 to 4.

The official levels are: 1 — Exercise normal precautions; 2 — Exercise
increased caution; 3 — Reconsider travel; 4 — Do not travel.

## Data and privacy

Travel Advisory levels are read only from the official State Department RSS
feed. Country names and land borders are geographic metadata, cached locally
for 30 days; they do not contain security ratings.

By default, the app asks Termux:API for the Android network location, then
turns the coordinates into a country. If that is unavailable, it falls back to
an IP-based country lookup. IP location can be wrong when using a VPN, a mobile
carrier gateway, or a corporate network.

## Install on Termux

Install the **Termux:API** Android app from the same source as Termux, grant it
location permission, then run:

```sh
pkg install rust make git termux-api
git clone https://github.com/PepeDiedrich/war-dodge.git
cd war-dodge
make install PREFIX="$PREFIX"
war-dodger init
war-dodger once
```

The generated configuration is at `~/.config/war-dodger/config.conf`.
`location_provider = termux` is the default; set it to `ip` to only use IP
country lookup.

```sh
war-dodger once     # Run one check.
war-dodger status   # Show saved country levels.
war-dodger run      # Run continuously and sleep between checks.
```

For automatic startup after reboot, install Termux:Boot and create an
executable `~/.termux/boot/start-war-dodger`:

```sh
#!/data/data/com.termux/files/usr/bin/sh
termux-wake-lock
exec war-dodger run >> "$HOME/.local/state/war-dodger.log" 2>&1
```

Disable Android battery optimization for Termux so Android does not stop the
waiting process.

## Termux package submission

The recipe is in `packaging/termux/war-dodger/`. Before submitting it to
`termux/termux-packages`, create a versioned GitHub release and replace
`SKIP_CHECKSUM` in the recipe with the release archive SHA-256. See
`packaging/termux/README.md` for the complete checklist.
