# ELSTER-Versand — Anleitung für Julius

Für den Moment, in dem du das zum ersten Mal machst (auch um 23 Uhr). Lies das komplett,
bevor du irgendetwas mit `--echtversand` ausführst.

Zwei Programme tragen den Weg, beide in `rust/`:

- `taxgraph-xml` schreibt die Erklärung eines gespeicherten Falls als XML-Datei. Es sendet nichts,
  braucht kein Zertifikat und ändert den Fall nicht.
- `taxgraph-versand` (Crate `rust/versand`) prüft diese Datei und sendet sie. Es ruft
  `EricBearbeiteVorgang` mit `ERIC_VALIDIERE | ERIC_SENDE` auf. Das ist der einzige Aufruf im ganzen
  Repo, der wirklich ins Netz geht und beim Finanzamt ankommt. Alles andere (`POST /einreichen`,
  `taxgraph-xml`) bleibt lokal.

Der Weg hat vier Schritte. Jeder steht unten mit seinem Befehl:

1. `taxgraph-xml`: Fall → XML-Datei (Abschnitt 2).
2. `taxgraph-versand --dry-run`: zeigt, was passieren würde, und sendet nichts (Abschnitt 3).
3. `taxgraph-versand --testversand`: geht ans Netz, aber als Test (Abschnitt 4).
4. `taxgraph-versand --echtversand`: geht wirklich ans Finanzamt (Abschnitt 5). Heute nicht möglich.

Beide Programme laufen aus dem Ordner `rust/`:

```bash
cd rust
```

Der Weg ist bisher nur gegen eine Attrappe von `libericapi.so` geprüft, nie mit einem echten
Zertifikat. Die Datei von `taxgraph-xml` besteht für einen Basisfall (Angestellte ohne Anlagen)
die lokale Prüfung `checkESt` mit der echten Hersteller-ID (`rc=0`); gesendet wurde sie nie.
Mach den ersten Lauf mit `--dry-run`, dann mit `--testversand`.

## 1. Was du brauchst

- **Dein ELSTER-Zertifikat** (`.pfx`- oder `.p12`-Datei), das du von ELSTER bekommst/hast.
- **Die PIN** dazu.
- **Die Hersteller-ID** (`ELSTER_HERSTELLER_ID`). Sie steht schon in `.env`.

Alles ist geheim, nichts davon darf ins Repo. Am einfachsten in `.env` eintragen (die Datei ist schon
in `.gitignore`, wird nie committet):

```bash
# .env, ergänzen:
ELSTER_ZERTIFIKAT_PFAD=/pfad/zu/deinem/zertifikat.pfx
ELSTER_ZERTIFIKAT_PIN=deine-pin
```

`taxgraph-xml` liest `.env` selbst. `taxgraph-versand` liest keine `.env`; lade sie vor jedem Lauf von
`taxgraph-versand`:

```bash
set -a; . ../.env; set +a
```

Leg die `.pfx`-Datei NICHT in den Repo-Ordner, wenn du es vermeiden kannst. Falls doch
(z.B. weil es gerade praktischer ist): `*.pfx` und `*.p12` stehen in `.gitignore`, git wird
sie ignorieren — trotzdem lieber außerhalb halten, z.B. `~/.elster/`.

`taxgraph-versand` findet ERiC über `$ERIC_DIR` (sonst `~/02_Software/eric`).

## 2. Die XML-Datei schreiben

Die Fall-ID ist der Dateiname der Akte unter `~/.local/share/taxgraph/faelle/` (ohne `.json`):

```bash
ls ~/.local/share/taxgraph/faelle/
cargo run -p api --bin taxgraph-xml -- --fall MEIN_FALL --ausgabe ~/elster/mein_fall.xml
```

Das Programm baut dieselbe Erklärung, die der Knopf „Einreichen“ im Browser an ERiC zur Prüfung gibt.
Sie trägt den Testmerker `700000004`. Es gilt:

- **Keine Datei, wenn etwas fehlt.** Sperrt die Abgabe (fehlende Angabe, Sperrgrund, Writer), steht der
  Grund auf dem Bildschirm, Exit-Code 2, und es entsteht keine Datei. Beheben, dann neu aufrufen.
- **Nie überschreiben.** Existiert die Ausgabedatei schon, bricht das Programm ab (Exit-Code 1). Lösch
  die alte Datei selbst oder nimm einen neuen Namen. So geht nie ein altes XML still weiter.
- **Nur du liest die Datei.** Sie hat die Rechte `600`: sie enthält Name, Anschrift und Steuernummer.
  Leg sie nicht ins Repo.
- Ohne Hersteller-ID (`ELSTER_HERSTELLER_ID` fehlt) baut der Writer nichts: Exit-Code 2, Grund
  „keine Hersteller-ID“.
- Mit `--land` wählst du das Land des Finanzamts (Vorgabe `BY`).
- Der erste Aufruf baut das Programm. Das dauert einige Minuten.

Für den ersten Testversand nimm einen Fall mit erfundenen Daten.

## 3. Erst anschauen, was passieren würde

```bash
set -a; . ../.env; set +a
cargo run -p versand --bin taxgraph-versand -- --xml ~/elster/mein_fall.xml --datenart ESt_2025 --dry-run
```

Das sendet nichts. Es zeigt `datenart_version`, `modus`, den Merker im XML (`merker_im_xml`),
`merker_konsistent`, die Größe und ob dein Zertifikat gefunden wird (`zertifikat_vorhanden`, nur
true/false, nie Pfad oder Inhalt). Steht bei `merker_konsistent` ein `false` oder eine Warnung: nicht
weitermachen, sondern die Datei neu mit `taxgraph-xml` schreiben.

## 4. Testversand — IMMER zuerst

Testversand geht technisch genauso ins Netz wie ein echter, trägt aber den amtlichen
Testmerker `700000004`. Der Aufbau bei der Clearingstelle: **die Fälle werden dort
aussortiert und verworfen, es findet keine Verarbeitung im Finanzamt statt.**

Es gibt keinen Standardmodus. Du nennst immer genau einen von `--dry-run`, `--testversand`,
`--echtversand`; ohne einen bricht das Programm ab (Exit-Code 2).

```bash
cargo run -p versand --bin taxgraph-versand -- --xml ~/elster/mein_fall.xml --datenart ESt_2025 --testversand
```

Ohne Zertifikat bricht das Programm vor jedem Netzaufruf ab: „Kein Zertifikatspfad gesetzt“.

**Ohne eigenes Zertifikat geht der Testversand mit dem Demo-Zertifikat von ERiC** (gemessen am
2026-10-11, ein Lauf: `rc=0`, Telenummer `KN6`, ERiC-Log „Daten wurden erfolgreich angenommen“). Setze
dafür nur für diesen Lauf `ELSTER_ZERTIFIKAT_PFAD` auf
`$ERIC_DIR/extracted/ERiC-44.2.4.0/Linux-x86_64/Beispiel/ericdemo-python/test-softidnr-pse.pfx` und
`ELSTER_ZERTIFIKAT_PIN` auf die öffentliche Demo-PIN (`123456`, steht als Vorgabe in
`Beispiel/ericdemo-cpp/ericdemo/system.cpp`). Trag beides nicht in `.env` ein. Das gilt nur für `--testversand`; ein Echtversand braucht ein echtes Zertifikat.

## 5. Echtversand — geht wirklich ans Finanzamt

**Heute nicht möglich.** `taxgraph-xml` schreibt nur Dateien mit Testmerker, und `taxgraph-versand`
bricht ab, wenn der Merker im XML nicht zum Modus passt. Ein XML ohne Testmerker baut heute kein
Programm. Das entscheidet Julius, bevor der erste Echtversand läuft. Die Hürden unten gelten dann
unverändert.

Erst wenn der Testversand sauber durchlief. Zwei Dinge sind hier zwingend, beide unabhängig
voneinander, beide müssen exakt stimmen:

1. `--freigabe` mit der wörtlichen Phrase:

   ```bash
   cargo run -p versand --bin taxgraph-versand -- --xml PFAD.xml --datenart ESt_2025 \
       --echtversand --freigabe "JA ICH SENDE ECHT AN DAS FINANZAMT"
   ```

2. Danach fragt das Programm interaktiv noch einmal nach genau dieser Phrase — die musst du
   selbst eintippen (nicht aus der Shell-History kopieren). Dafür braucht es ein **Terminal**: eine
   Phrase aus einer Pipe (`echo … |`) lehnt das Programm ab. Erst dann geht der Aufruf raus.

Wenn du eine der beiden Hürden nicht nimmst (Phrase falsch/vergessen, Prompt falsch
beantwortet, kein Terminal für die Eingabe verfügbar): kein Versand, klare Fehlermeldung,
Programm bricht ab. Das ist Absicht — niemand soll aus Versehen real senden können.

Zusätzlich prüft `taxgraph-versand` selbst das XML, bevor es überhaupt an ERiC geht: behauptest
du Echtversand, aber im XML steht ein `<Testmerker>`-Element (oder umgekehrt: Testversand
behauptet, aber der Merker fehlt oder ist falsch) — Abbruch, kein ERiC-Aufruf. Es liest den Merker
aus dem XML-Baum: ein `<Testmerker>` in einem Kommentar oder außerhalb von `TransferHeader` zählt
nicht; zwei Merker oder ein unlesbares XML brechen ab. Das Programm vertraut nicht darauf, dass du
(oder der XML-Writer) das richtig gebaut hast.

## 6. Woran du Erfolg erkennst

Nach einem erfolgreichen Versand (Test oder echt) druckt `taxgraph-versand`:

```
[versand] rc=0
[versand] Telenummer: N55...
```

Die **Telenummer** ist der amtliche Nachweis, dass ELSTER die Übermittlung angenommen hat
(bei Testversand: von der Clearingstelle angenommen und dort verworfen — trotzdem eine echte
Telenummer als Bestätigung, dass der technische Weg funktioniert). Kein `rc=0` **oder** keine
Telenummer heißt: kein Erfolg, auch wenn `rc=0` allein schon gut aussieht.

## 7. Wenn etwas schiefgeht

**Nicht blind wiederholen.** Ein zweiter Versand desselben Falls ist keine harmlose Wiederholung
— beim Finanzamt kann daraus eine doppelte Abgabe werden. Das gilt vor allem bei
`--echtversand`.

Stattdessen:

1. Fehlermeldung genau lesen — `taxgraph-versand` sagt, an welcher Stufe es hakt (Freigabe,
   XML/Merker-Mismatch, Zertifikat, oder ERiC-rc mit Rückgabetext).
2. Bei ERiC-Fehlern (`rc != 0`): den Rückgabetext ansehen (wird bei `rc != 0` mit ausgegeben) — der
   nennt den genauen ERiC-Fehlercode. Das Programm nennt außerdem den Ort von `eric.log`, aber nie
   dessen Inhalt: er enthält Auszüge der Erklärung.
3. Bei Unsicherheit, ob eine vorherige Übermittlung schon angekommen ist: NICHT einfach
   nochmal senden. Erst klären (z.B. über das ELSTER-Portal/eine vorherige Telenummer),
   ob der Fall schon drin ist.
4. Zertifikatsfehler: Pfad/PIN in `.env` prüfen. Die Fehlermeldung nennt
   nie den Pfad oder die PIN selbst — nur, dass/warum es nicht ging.

## 8. Was die Programme NIE tun

- Nie automatisch senden — jeder Aufruf ist eine bewusste Handlung von dir am Terminal.
- Nie eine Freigabe erraten oder einen Standard für Echtversand annehmen.
- Nie Zertifikatspfad, PIN, Steuernummer oder IBAN in einer Log-/Fehlermeldung ausgeben.
- Nie senden, wenn das XML nicht exakt zum behaupteten Modus (Test/Echt) passt.
- `taxgraph-xml` schreibt nie eine vorhandene Datei neu und nie eine Datei, wenn die Abgabe sperrt.
