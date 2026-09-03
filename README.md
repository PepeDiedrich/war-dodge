# termux-poller

Ein sehr schlanker periodischer Rust-Runner für Termux. Er schläft zwischen Ausführungen vollständig, prüft vor der Aufgabe mit einem einzelnen TCP-Handshake die Erreichbarkeit und wiederholt Netz- und Befehlsfehler mit begrenztem exponentiellem Backoff. Die Release-Binary hat keine Runtime-Abhängigkeiten.

## Auf dem Handy installieren

~~~sh
pkg install rust make git
git clone <DEIN-REPOSITORY-URL> termux-poller
cd termux-poller
make install PREFIX="$PREFIX"
termux-poller init
nano ~/.config/termux-poller/config.conf
termux-poller run
~~~

Die Zeile command ist die eigentliche Aufgabe, beispielsweise:

~~~ini
command = curl -fsS https://example.org/worker
interval = 30m
retry_min = 30s
retry_max = 15m
~~~

once führt genau einen Versuch aus. retry bleibt bis zu einem erfolgreichen Durchlauf aktiv und passt damit zu einem externen Scheduler. run ist der sparsame Dauermodus: Nach Erfolg schläft der Prozess bis zum nächsten Intervall; nach Ausfall schläft er mit 30 s, 60 s, 120 s … bis maximal 15 min.

Für den automatischen Start nach Neustarts eignet sich Termux:Boot. Lege danach eine ausführbare Datei ~/.termux/boot/start-poller an:

~~~sh
#!/data/data/com.termux/files/usr/bin/sh
termux-wake-lock
exec termux-poller run >> "$HOME/.local/state/termux-poller.log" 2>&1
~~~

Deaktiviere zusätzlich die Akku-Optimierung für Termux in Android, damit Android den wartenden Prozess nicht beendet.

## Entwicklungs- und Optimierungszyklus

~~~sh
make check       # Format und lints; Warnungen sind Fehler
make test        # Logiktests
make bench       # reproduzierbare Microbenchmarks im Release-Profil
make release     # optimierte Binary
./scripts/profile-termux.sh  # auf dem Handy: Zeit + maximaler RSS
~~~

Das Release-Profil nutzt Größenoptimierung, fat LTO, eine Codegen-Unit, entfernte Symbole und panic abort. Vor jeder Optimierung speichern wir Benchmark und Geräteprofil, ändern gezielt eine Sache und messen danach mit demselben Ablauf erneut.

## Später per pkg install

pkg kann nur Pakete aus aktivierten Termux-Repositories installieren. Der erste praktische Vertriebsweg ist ein Git-Release mit vorgebautem aarch64-linux-android-Binary. Für echtes pkg install termux-poller braucht es anschließend ein Paket-Recipe und ein veröffentlichtes eigenes oder offizielles Termux-Repository.
