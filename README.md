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

See the [safety guide](SAFETY_GUIDE.md) for a conservative, practical checklist
for each level.

## Data and privacy

Travel Advisory levels are read only from the official State Department RSS
feed. Country names and land borders are geographic metadata, cached locally
for 30 days; they do not contain security ratings.

By default, the app asks Termux:API for the Android network location, then
turns the coordinates into a country. If that is unavailable, it falls back to
an IP-based country lookup. IP location can be wrong when using a VPN, a mobile
carrier gateway, or a corporate network.

## Install from source

Use this method until the official Termux package is accepted. Install the
**Termux:API** Android app from the same source as Termux, then run:

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

## Install with `pkg`

The package has been submitted to the official Termux repository but is not
available through `pkg` until the maintainers accept and publish it. Once it is
published, installation will be:

```sh
pkg update
pkg install war-dodger
war-dodger init
war-dodger run
```

Until then, use the source-build instructions above.

## Required Android settings

Before starting the monitor, complete every item below:

- Install the **Termux:API** Android app from the same distribution source as
  Termux, then install the `termux-api` package in Termux.
- Grant **location permission** to Termux:API. The default location method uses
  it; the app falls back to less accurate IP location if it is unavailable.
- Allow **notifications** for Termux:API so `termux-notification` can display
  alerts.
- Disable Android **battery optimization** / enable unrestricted battery use
  for both Termux and Termux:API. Otherwise Android can stop the hourly
  monitor while it is waiting in the background.
- Optionally install Termux:Boot and add the startup script below so monitoring
  resumes after a device reboot.

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

## Termux package submission

The package recipe is in `packaging/termux/war-dodger/` and the current
submission is tracked in the [Termux package pull request](https://github.com/termux/termux-packages/pull/31476).
