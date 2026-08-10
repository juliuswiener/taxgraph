# ELSTER-Versand — Anleitung fuer Julius

Fuer den Moment, in dem du das zum ersten Mal machst (auch um 23 Uhr). Lies das komplett,
bevor du irgendetwas mit `--echtversand` ausfuehrst.

Der Code dazu: `elster/versand.py`. Der ruft `EricBearbeiteVorgang` mit `ERIC_VALIDIERE | ERIC_SENDE`
auf — das ist der einzige Aufruf im ganzen Repo, der wirklich ins Netz geht und beim Finanzamt
ankommt. Alles andere (`checkest_gate.py`, `/einreichen`) validiert nur lokal.

## 1. Was du brauchst

- **Dein ELSTER-Zertifikat** (`.pfx`- oder `.p12`-Datei), das du von ELSTER bekommst/hast.
- **Die PIN** dazu.

Beides ist geheim, keins von beiden darf ins Repo. Genau wie `.env`:

```bash
export ELSTER_ZERTIFIKAT_PFAD="/pfad/zu/deinem/zertifikat.pfx"
export ELSTER_ZERTIFIKAT_PIN="deine-pin"
```

Am einfachsten in `.env` eintragen (die Datei ist schon in `.gitignore`, wird nie committet):

```bash
# .env, ergaenzen:
ELSTER_ZERTIFIKAT_PFAD=/pfad/zu/deinem/zertifikat.pfx
ELSTER_ZERTIFIKAT_PIN=deine-pin
```

und vor jedem Lauf laden (gleiches Muster wie beim `eric-gate`-Target im Makefile):

```bash
set -a; . ./.env; set +a
```

Leg die `.pfx`-Datei NICHT in den Repo-Ordner, wenn du es vermeiden kannst. Falls doch
(z.B. weil es gerade praktischer ist): `*.pfx` und `*.p12` stehen in `.gitignore`, git wird
sie ignorieren — trotzdem lieber ausserhalb halten, z.B. `~/.elster/`.

## 2. Testversand — IMMER zuerst

Testversand geht technisch genauso ins Netz wie ein echter, traegt aber den amtlichen
Testmerker `700000004`. Der Aufbau bei der Clearingstelle: **die Faelle werden dort
aussortiert und verworfen, es findet keine Verarbeitung im Finanzamt statt.** Das ist der
Default in `versand.py` — du musst nichts extra angeben, um ihn zu bekommen.

Erst anschauen, was passieren wuerde (sendet nichts):

```bash
set -a; . ./.env; set +a
python3 elster/versand.py --xml pfad/zur/fall.xml --datenart ESt_2025 --dry-run
```

Zeigt: Modus, ob der Testmerker im XML steht, ob dein Zertifikat gefunden wird (nur ja/nein,
nie der Pfad oder Inhalt). Wenn hier eine Warnung zu XML/Modus-Inkonsistenz steht: nicht
weitermachen, sondern das XML pruefen (`elster_xml.erzeuge_xml(..., testmerker=...)`).

Dann der eigentliche Testversand:

```bash
python3 elster/versand.py --xml pfad/zur/fall.xml --datenart ESt_2025 --testversand
```

## 3. Echtversand — geht wirklich ans Finanzamt

Erst wenn der Testversand sauber durchlief. Zwei Dinge sind hier zwingend, beide unabhaengig
voneinander, beide muessen exakt stimmen:

1. `--freigabe` mit der woertlichen Phrase (steht in `elster/versand.py`, `ECHTVERSAND_FREIGABE`):

   ```bash
   python3 elster/versand.py --xml pfad/zur/fall.xml --datenart ESt_2025 \
       --echtversand --freigabe "JA ICH SENDE ECHT AN DAS FINANZAMT"
   ```

2. Danach fragt das Programm interaktiv noch einmal nach genau dieser Phrase — die musst du
   selbst eintippen (nicht aus der Shell-History kopieren). Erst dann geht der Aufruf raus.

Wenn du eine der beiden Huerden nicht nimmst (Phrase falsch/vergessen, Prompt falsch
beantwortet, kein Terminal fuer die Eingabe verfuegbar): kein Versand, klare Fehlermeldung,
Programm bricht ab. Das ist Absicht — niemand soll aus Versehen real senden koennen.

Zusaetzlich prueft `versand.py` selbst das XML, bevor es ueberhaupt an ERiC geht: behauptest
du Echtversand, aber im XML steht noch ein `<Testmerker>`-Element (oder umgekehrt: Testversand
behauptet, aber der Merker fehlt oder ist falsch) — Abbruch, kein ERiC-Aufruf. Das Programm
vertraut nicht darauf, dass du (oder der XML-Writer) das richtig gebaut habt.

## 4. Woran du Erfolg erkennst

Nach einem erfolgreichen Versand (Test oder echt) druckt das Programm:

```
[versand] rc=0
[versand] Telenummer: N55...
```

Die **Telenummer** ist der amtliche Nachweis, dass ELSTER die Uebermittlung angenommen hat
(bei Testversand: von der Clearingstelle angenommen und dort verworfen — trotzdem eine echte
Telenummer als Bestaetigung, dass der technische Weg funktioniert). Kein `rc=0` **oder** keine
Telenummer heisst: kein Erfolg, auch wenn `rc=0` allein schon gut aussieht.

## 5. Wenn etwas schiefgeht

**Nicht blind wiederholen.** Ein zweiter Versand desselben Falls ist keine harmlose Wiederholung
— beim Finanzamt kann daraus eine doppelte Abgabe werden. Das gilt vor allem bei
`--echtversand`.

Stattdessen:

1. Fehlermeldung genau lesen — `versand.py` sagt, an welcher Stufe es hakt (Freigabe,
   XML/Merker-Mismatch, Zertifikat, oder ERiC-rc mit Rueckgabetext).
2. Bei ERiC-Fehlern (`rc != 0`): den `rueckgabe_xml`-Text ansehen (wird bei `rc != 0`
   mit ausgegeben) — der nennt den genauen ERiC-Fehlercode.
3. Bei Unsicherheit, ob eine vorherige Uebermittlung schon angekommen ist: NICHT einfach
   nochmal senden. Erst klaeren (z.B. ueber das ELSTER-Portal/eine vorherige Telenummer),
   ob der Fall schon drin ist.
4. Zertifikatsfehler (`ZertifikatFehlt`): Pfad/PIN in `.env` pruefen. Die Fehlermeldung nennt
   nie den Pfad oder die PIN selbst — nur, dass/warum es nicht ging.

## 6. Was `versand.py` NIE tut

- Nie automatisch senden — jeder Aufruf ist eine bewusste CLI-Aktion von dir.
- Nie eine Freigabe erraten oder einen Default fuer Echtversand annehmen.
- Nie Zertifikatspfad, PIN, Steuernummer oder IBAN in einer Log-/Fehlermeldung ausgeben.
- Nie senden, wenn das XML nicht exakt zum behaupteten Modus (Test/Echt) passt.
