# War Dodger

War Dodger is a lightweight, location-aware monitor for U.S. Department of
State Travel Advisory levels. It is designed to run continuously in Termux on
Android while spending almost all of its time asleep.

It monitors the country where the phone currently appears to be located and
all countries that share a land border with it. When a relevant advisory
changes, War Dodger sends an Android notification through Termux:API.

War Dodger is a personal alerting aid, not an emergency service. Network,
location, upstream data, and Android background execution can all fail. Always
read the complete country advisory and follow local authorities.

## How it works

One successful check consists of the following steps:

1. Open a short TCP connection to the configured health host. This detects
   airplane mode and common offline conditions before starting slower HTTPS
   requests.
2. Determine the current country. The default provider first asks
   Termux:API for the Android network location and reverse-geocodes the
   coordinates. If that fails, it falls back to IP geolocation.
3. Load country names and land-border metadata. This data is cached locally
   for 30 days to reduce downloads, CPU work, and battery use.
4. Download the official U.S. Department of State Travel Advisory RSS feed.
5. Extract the advisory level for the current and directly bordering
   countries.
6. Compare those levels with the previous snapshot stored on the phone.
7. Send notifications for matching changes, then save the new snapshot.

In continuous mode, the process blocks in an operating-system sleep between
checks. It does not busy-poll, so idle CPU use should be effectively zero.

## Alert rules

- Current country: notify whenever its advisory level changes.
- Current country at Level 3: send a reminder after every successful check
  while Level 3 remains active.
- Directly bordering countries: notify only for escalation from Level 2 to
  Level 3 or from Level 3 to Level 4.
- Initial run: establish a silent baseline by default. Set
  **notify_on_initial = true** if the first observed levels should also create
  notifications.

The official advisory levels are:

| Level | Meaning |
| --- | --- |
| 1 | Exercise normal precautions |
| 2 | Exercise increased caution |
| 3 | Reconsider travel |
| 4 | Do not travel |

See [SAFETY_GUIDE.md](SAFETY_GUIDE.md) for a conservative checklist for each
level.

## Requirements

Install Termux and all Termux add-ons from the same distribution source. Their
signatures must match. The recommended source is F-Droid.

You need:

- Termux
- the Termux:API Android app
- the termux-api package inside Termux
- location and notification permissions for Termux:API
- Termux:Boot for automatic restart after reboot
- unrestricted battery use for Termux and Termux:API

After installing Termux:Boot, open its launcher icon once. Android does not
allow the app to receive boot events until this initial launch has happened.

No application can run while the phone is physically powered off. War Dodger
resumes after Android starts again.

## Install from source

Use this method until the package is accepted into the official Termux
repository.

1. Install the Termux:API and Termux:Boot Android apps from the same source as
   Termux.
2. Open Termux:Boot once.
3. In Termux, run:

~~~sh
pkg update
pkg install rust make git termux-api
git clone https://github.com/PepeDiedrich/war-dodge.git
cd war-dodge
make verify
make install PREFIX="$PREFIX"
~~~

Create the initial configuration:

~~~sh
war-dodger init
~~~

This writes:

    ~/.config/war-dodger/config.conf

Test Android notifications before starting the monitor:

~~~sh
war-dodger notify-test
~~~

If no notification appears, see Troubleshooting below.

Run one complete data check:

~~~sh
war-dodger once
~~~

Finally start continuous monitoring:

~~~sh
war-dodger run
~~~

The run command automatically creates or updates the executable Termux:Boot
entry:

    ~/.termux/boot/start-war-dodger

It then begins monitoring in the current terminal. The generated boot entry
takes a wake lock, starts the same installed executable after Android boots,
and writes output to:

    ~/.local/state/war-dodger/run.log

You do not need to create the boot script manually.

## Install with pkg

The package recipe has been submitted to the official Termux repository. It is
not available through pkg until the Termux maintainers accept and publish it.
After publication, setup will be:

~~~sh
pkg update
pkg install war-dodger
war-dodger init
war-dodger notify-test
war-dodger run
~~~

Until then, use the source installation above.

## Android setup checklist

Before relying on background alerts, verify every item:

- Termux:API is installed from the same source as Termux.
- The termux-api package is installed inside Termux.
- Android location permission is granted to Termux:API.
- Android notification permission is granted to Termux:API.
- Battery optimization is disabled, or unrestricted battery use is enabled,
  for both Termux and Termux:API.
- Termux:Boot is installed and has been opened once.
- **war-dodger notify-test** displays a notification.
- **war-dodger once** completes successfully.
- **war-dodger run** reports the generated boot-script location.

Some Android vendors add their own background-process restrictions. If alerts
stop after several hours, also allow autostart and background activity in the
vendor-specific battery settings.

## Commands

| Command | Purpose |
| --- | --- |
| **war-dodger init** | Create the default configuration |
| **war-dodger notify-test** | Send a harmless test notification |
| **war-dodger once** | Perform one check and return success or failure |
| **war-dodger retry** | Retry with backoff until one check succeeds |
| **war-dodger run** | Configure boot startup and monitor continuously |
| **war-dodger status** | Print the last stored advisory levels |

Use an alternative configuration file by placing the option before the
command:

~~~sh
war-dodger --config /path/to/config.conf once
~~~

## Configuration

The generated file is a simple key-value configuration:

~~~ini
location_provider = termux
interval = 1h
retry_min = 30s
retry_max = 15m
health_host = 1.1.1.1:443
connect_timeout = 8s
notify_command = termux-notification --title "$WAR_DODGER_TITLE" --content "$WAR_DODGER_MESSAGE"
notify_on_initial = false
~~~

| Setting | Description |
| --- | --- |
| **location_provider** | termux uses Android location with IP fallback; ip always uses IP geolocation |
| **interval** | Delay after a successful check; supported units are s, m, and h |
| **retry_min** | Delay before the first retry |
| **retry_max** | Maximum exponential retry delay |
| **health_host** | Numeric IP address and TCP port used for the fail-fast connectivity check |
| **connect_timeout** | Maximum duration of the connectivity check |
| **notify_command** | Shell command used to display an alert |
| **notify_on_initial** | Whether the first baseline should produce notifications |

The notification command receives the environment variables
**WAR_DODGER_TITLE** and **WAR_DODGER_MESSAGE**. Keeping notification delivery
as a command makes it easy to test or replace.

## Offline and retry behavior

The **once** command makes one attempt and exits nonzero on failure.

The **retry** and **run** commands retry network, upstream-data, location, and
notification failures using exponential backoff:

    30 seconds, 60 seconds, 120 seconds, ... up to 15 minutes

After a successful run, the retry delay resets. Continuous mode then sleeps for
the configured regular interval before checking again.

## Files and privacy

| Path | Contents |
| --- | --- |
| ~/.config/war-dodger/config.conf | User configuration |
| ~/.local/state/war-dodger/levels | Last advisory snapshot |
| ~/.local/state/war-dodger/run.log | Output from automatic boot runs |
| ~/.cache/war-dodger/countries.json | Country and border metadata cache |
| ~/.termux/boot/start-war-dodger | Automatically generated boot entry |

Travel Advisory levels come from the official State Department feed. Country
names and borders are geographic metadata and do not contain risk ratings.

Location coordinates are sent to the configured reverse-geocoding service when
Android location succeeds. If it fails, the IP geolocation service sees the
public IP address. VPNs, carrier gateways, and corporate networks can make IP
location inaccurate.

## Development, tests, and optimization

The complete local verification cycle is:

~~~sh
make verify
~~~

It runs formatting checks, strict Clippy lints, unit tests, dependency-free
microbenchmarks, and an optimized release build.

Individual targets are also available:

~~~sh
make check
make test
make bench
make release
~~~

To measure elapsed time and maximum resident memory on the actual phone:

~~~sh
pkg install time
./scripts/profile-termux.sh
~~~

Device measurements are more useful than desktop measurements for battery and
memory optimization. The release profile uses size optimization, fat LTO, one
code-generation unit, stripped symbols, and abort-on-panic.

## Troubleshooting

### notify-test reports command not found

Install the package inside Termux:

~~~sh
pkg install termux-api
~~~

Also install the Termux:API Android app from the same source as Termux.

### notify-test succeeds but nothing appears

Allow notifications for Termux:API in Android settings. On newer Android
versions, confirm that notifications are not disabled for its notification
channel.

### Location fails

Grant location permission to Termux:API and enable Android location services.
As a less accurate fallback, edit the configuration:

~~~ini
location_provider = ip
~~~

### run works until the screen is off

Set battery usage for Termux and Termux:API to unrestricted. Check
manufacturer-specific autostart and background restrictions as well.

### Monitoring does not resume after reboot

Confirm that Termux:Boot is installed from the same source as Termux and that
you opened its icon once. Then run **war-dodger run** again and verify:

~~~sh
ls -l ~/.termux/boot/start-war-dodger
tail -n 100 ~/.local/state/war-dodger/run.log
~~~

### Network errors continue while internet works

Some networks may block the default health endpoint. Change **health_host** to
another reliable numeric IP and TCP port reachable from that network.

## Termux package submission

The package recipe is in
[packaging/termux/war-dodger](packaging/termux/war-dodger). The current
submission is tracked in
[termux/termux-packages#31476](https://github.com/termux/termux-packages/pull/31476).
