# War Dodger

Ein schlanker, lokaler Termux-Dienst für einen später zu definierenden
vierstufigen War-Dodger. Das Projekt enthält bewusst noch keine Bewertung
geopolitischer Daten und keine Regeln für Eskalationsstufen. Es stellt die
Ausführung, Speicherung und Erinnerung bei einem Phasenübergang bereit.

## Ablauf

Der Standort wird stündlich ermittelt. `scope_command` liefert das aktuelle
Land plus direkte Landnachbarn als ISO-Codes. Rust lädt den offiziellen
Travel-State-RSS-Feed und bestimmt für jedes dieser Länder dessen
Reisehinweisstufe `1` bis `4`.

Die Stufen aller beobachteten Länder werden lokal gespeichert. Jede Änderung
im Aufenthaltsland löst eine Benachrichtigung aus. Bei direkten Nachbarländern
werden nur die Eskalationen `2 → 3` und `3 → 4` gemeldet. Solange im
Aufenthaltsland Stufe 3 aktiv ist, folgt nach jeder erfolgreichen
Stundenprüfung zusätzlich eine Erinnerung. Der erste Lauf speichert nur den
Startwert.

`phase_command` ist austauschbar, falls später eine zusätzliche Datenquelle
oder eine andere Bewertungsregel ergänzt werden soll.

## Termux installieren

```sh
pkg install rust make git termux-api
git clone <DEIN-REPOSITORY-URL> war-dodger
cd war-dodger
make install PREFIX="$PREFIX"
war-dodger init
nano ~/.config/war-dodger/config.conf
war-dodger once
```

Zusätzlich muss die Android-App **Termux:API** installiert sein; erteile ihr
die Standortberechtigung und deaktiviere keine Benachrichtigungen. Rust nutzt
die Standortkoordinaten nur, um Aufenthaltsland und Landnachbarn zu bestimmen.
Falls Termux:API nicht verfügbar ist, wird nur das Land über die öffentliche
IP ermittelt; das kann bei VPN oder Mobilfunk-Routing ungenau sein.
`termux-notification` erhält Titel und Text über `WAR_DODGER_TITLE` bzw.
`WAR_DODGER_MESSAGE`, sodass später auch eine andere Push-Lösung verwendbar ist.

## Paketvorbereitung

Ein Termux-Paketrezept liegt unter `packaging/termux/war-dodger/`. Nach einem
versionierten GitHub-Release muss dessen SHA-256 im Rezept eingetragen werden,
bevor es als Pull Request an `termux-packages` eingereicht werden kann.

```sh
war-dodger once     # eine Prüfung
war-dodger status   # zuletzt gespeicherte Stufe
war-dodger run      # Dauerbetrieb; schläft zwischen Prüfungen
```

Für den Autostart nach Neustarts eignet sich Termux:Boot:

```sh
#!/data/data/com.termux/files/usr/bin/sh
termux-wake-lock
exec war-dodger run >> "$HOME/.local/state/war-dodger.log" 2>&1
```

Deaktiviere für Termux die Akku-Optimierung, damit Android den wartenden Prozess
nicht beendet.
