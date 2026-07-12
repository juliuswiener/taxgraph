# ERiC-Smoke-Befund + Phase-4-Machbarkeit (2026-07-12)

Julius hat ERiC manuell heruntergeladen; Auftrag: nach `~/02_Software/eric/` verschieben, entpacken,
Smoke-Test, Befund. Alles $0, rein lokal, kein Netz, keine Credentials, kein Versand.

## Setup (erledigt)

- 3 Dateien aus dem fremden Projektverzeichnis `~/00_projects/bundestags-brief/` (Julius'
  Download-cwd — untracked Streu-Dateien, unrelated zu jenem Web-App-Projekt) nach
  `~/02_Software/eric/` **verschoben** (mv, Julius-Konvention as-is-Software). bundestags-brief
  bereinigt, dessen git unberuehrt (Dateien waren nie getrackt).
- `ERiC-44.2.4.0-Linux-x86_64.jar` (jar=zip) entpackt nach
  `~/02_Software/eric/extracted/ERiC-44.2.4.0/Linux-x86_64/` (lib/, include/, plugins/, Beispiel/).

## Smoke-Test-Verdikt: READY

`ERIC_DIR=~/02_Software/eric python elster/smoke_test.py`:
- `libericapi.so` gefunden + `ctypes.CDLL` geladen.
- `EricInitialisiere(pluginPfad=.../lib)` -> **ERIC_OK**.
- `EricVersion()` -> **ERIC_OK**, Versions-XML gelesen (alle Plugins Produktversion 44,2,4,0).
- `EricBeende()` sauber.

## Antworten auf die vier Befund-Fragen

1. **Version:** ERiC **44.2.4.0** (API 44.2.4; libericapi Produktversion 44,2,4,0, Dateiversion
   2026,44,2,4). Auslieferung vom 2026-07-03.
2. **Ladbar:** **ja.** Native Lib laedt unter Linux-x86_64, Init/Version/Beende alle ERIC_OK. Keine
   fehlenden Hilfs-Libs, keine glibc-Probleme.
3. **ESt-2026-Support:** **NEIN, noch nicht.** Offline-`checkESt`-Plugins: VZ **2015-2025**
   (`libcheckESt_2015.so` … `libcheckESt_2025.so`). Kein `libcheckESt_2026.so`. Erwartbar: das
   VZ-2026-Modul kommt mit einer spaeteren ERiC-Auslieferung (VZ-2026-Veranlagung oeffnet erst
   2027). **VZ 2025 ist jetzt schon offline validierbar** — das reicht als CI-Anker, bis 2026 folgt.
4. **CI-Gate deterministisch offline:** **ja, machbar.** `EricBearbeiteVorgang(datenpuffer,
   datenartVersion, bearbeitungsFlags, …)` mit `bearbeitungsFlags = ERIC_VALIDIERE` (`1<<1`) und
   OHNE `ERIC_SENDE` (`1<<2`) fuehrt die Plausibilitaetspruefung lokal im Plugin-`.so` aus — kein
   Netz, keine Zertifikate, kein Server. Rueckgabecode + Prüf-XML kommen deterministisch zurueck.
   `ERIC_VALIDIERE_OHNE_FREIGABEDATUM` (`1<<8`) erlaubt Validierung ausserhalb des Freigabefensters
   (nuetzlich fuer Randdaten / kuenftige Jahre im Test). Genau das ehemalige „checkESt".

## Phase-4-Schnitt-Vorschlag (konkretisiert)

Reihenfolge nach Risiko, Versand ganz am Ende:

- **(i) Feldmapping ESt1A ↔ Signatur-Outputs** — deterministisch, kein Modell. `elster/feldmodell/
  <vz>.yaml` aus der ERiC-Schemadokumentation (Schemadok-Zip liegt bereit) parsen; `elster/
  feldmapping.stub.yaml` von Platzhalter-IDs auf amtliche ELSTER-Feld-IDs heben. Reviewbares Artefakt.
- **(ii) checkESt-CI-Gate (offline, EMPFEHLUNG als erster Schritt)** — ein deterministisches Gate:
  aus den Regel-Outputs eine ESt-XML bauen (Feldmapping), `EricBearbeiteVorgang(xml, "ESt_2025",
  ERIC_VALIDIERE)` aufrufen, rc==0 == plausibel. **Kein Versand-Risiko, kein Login.** Erster
  CI-Seed: der **solzg-0,11-Fall** (20351 -> 0,11) — der amtliche Gegencheck zum Klasse-5-Fix.
- **(iii) Versand (spaeter, Julius-Territorium)** — `ERIC_SENDE` braucht Zertifikat/Softwarezertifikat
  + Portal-Credentials + echte Steuernummer. Das ist ausdruecklich NICHT Teil des CI-Gates und
  bleibt manueller Julius-Schritt; ich gebe keine Credentials in Login-/Zertifikatsfelder ein.

**Empfehlung: erst (ii) das Offline-checkESt-CI-Gate verdrahten** (validiert die formalisierten Werte
gegen ELSTERs amtliche Pruefung, null Versand-Risiko), dann (i) das Feldmapping fuellen, das (ii)
speist. (iii) Versand separat, wenn Julius es will.

## checkESt-CI-Gate-Harness (Phase 4 ii) — gebaut, Mechanismus bewiesen, Vollbeweis PENDING Hersteller-ID

`elster/checkest_gate.py` ruft `EricBearbeiteVorgang(xml, "ESt_2020", ERIC_VALIDIERE, NULL, NULL,
puffer, NULL)` — offline, kein Versand. `--prove` fuehrt den amtlichen ESt_2020-Beispieldatensatz
(plausibel) + eine verfaelschte Kopie (implausibel) durch.

**Ergebnis: MECHANISMUS BEWIESEN, VOLLBEWEIS PENDING HERSTELLER-ID.** Beide Faelle liefern
deterministisch `rc=610301202 = ERIC_IO_TESTHERSTELLERID_GESPERRT` (korrekt dekodiert). Grund: der
Beispieldatensatz traegt die Alt-Test-Hersteller-ID 74931, die **seit ERiC 39.4.x gesperrt** ist —
laut Entwicklerhandbuch 10.1.1 ist **die eigene registrierte Hersteller-ID Pflicht, auch fuer
Validierung; eine Dummy-/Test-ID gibt es nicht mehr.**

Das ist eine **Registrierungs-Voraussetzung = Julius-Territorium** (analog zum Versand-Zertifikat),
KEIN Harness-Fehler. Der Harness ruft die API korrekt auf und bekommt einen deterministischen,
richtig gedeuteten Returncode. Er liest die ID aus `$ELSTER_HERSTELLER_ID` (z.B. aus Julius'
`.env.elster`): sobald gesetzt, vervollstaendigt sich der Beweis (plausibel `rc==0`, implausibel
`rc!=0`) **ohne Code-Aenderung**.

**Konsequenz fuer den Plan:** der rc==0/rc!=0-Differenzbeweis braucht Julius' Hersteller-ID. Bis dahin
ist die Harness-Verdrahtung + die deterministische API-Antwort der belegbare Stand. Kein Falsch-Gruen:
ich melde NICHT „plausibel rc==0", weil ERiC am Hersteller-ID-Gate blockt.

## Zusatzauftrag: unverbindliche Steuerberechnung offline? — NEIN (in 44.2.4)

Instructor-Hoffnung: ein Bearbeitungsflag (ERIC_BERECHNE o.ae.) fuer eine amtliche Offline-
Probeberechnung der ESt = amtliches Rechen-Oracle. **Ergebnis: existiert in ERiC 44.2.4 NICHT.**
Belege:
- Bearbeitungsflags (`eric_types.h`) sind vollstaendig: `ERIC_VALIDIERE` (1<<1), `ERIC_SENDE`
  (1<<2), `ERIC_DRUCKE` (1<<5), `ERIC_PRUEFE_HINWEISE` (1<<7), `ERIC_VALIDIERE_OHNE_FREIGABEDATUM`
  (1<<8). **Kein Berechnungs-Flag.**
- Keine `Eric*`-API-Funktion mit `berechn`/`rechn`/`calc`. Kein Compute-Plugin (nur `libcheck*.so`
  + `libcommonData.so`). Keine Compute-Datenart (ESt1/ESt2/ESt6/EStA = Deklaration + Bescheid-
  abholung). Weder im Entwicklerhandbuch noch in den Releasenotes eine „Steuerberechnung".
- Der „Bescheid" (Kap. 7.4) ist Server-Rueckuebermittlung — online, Zertifikat, nach Abgabe.

**Was checkESt (ERIC_VALIDIERE) LEISTET:** die Plausibilitaetspruefung enthaelt eine Formelsprache
(`Summe(...)`, `SummeVonProdukten(...)`, Vergleiche) und prueft deklarierte Summen- und abhaengige
Felder auf interne Konsistenz. Das ist ein PARTIELLES Konsistenz-Oracle fuer deklarierte
Rechenfelder — KEINE vollstaendige, unabhaengige Steuerberechnung.

**Konsequenz fuer das zweistufige Gate:** Stufe 2 (Vergleich gegen unsere festzusetzende Steuer)
laesst sich NICHT ueber ERiC offline abbilden. Das Projekt hat den Offline-Rechen-Oracle aber
bereits: das **GETTSIM-Differential** (`Makefile` Ziel `s02`, Catala vs GETTSIM). Damit steht die
Arbeitsteilung: **GETTSIM = Offline-Rechen-Oracle (vorhanden), ERiC checkESt = amtliche
Deklarations-Plausibilitaet (neu, komplementaer).** Amtliche Steuerberechnung nur online (Bescheid)
= Julius/spaeter.

## Feldmapping (Richtung a) — Groundwork

Kz-Katalog liegt vor: `E10-2025.html` (ESt-Schemadok, **3948 Kz**, parsebar). Stub auf Richtung (a)
korrigiert (berechnete-Steuer-Zeilen entfernt). Kz je Zeile folgen als Instructor-Review-Tabelle
(Kz-Kandidaten je Regel-Input; Labels mappen nicht 1:1 — z.B. Kirchensteuer hat Sonderausgabe-/
KapESt-/Lohnsteuer-Varianten).

## Ablage / Provenance

- ERiC unter `~/02_Software/eric/` (ausserhalb Repo, `$ERIC_DIR`; kein absoluter Pfad im Code).
- `elster/smoke_test.py` liest `ERIC_DIR`, Default `~/02_Software/eric/`.
- `elster/README.md` dokumentiert Pfad + Aufruf + Verdikt.
