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

## Ablage / Provenance

- ERiC unter `~/02_Software/eric/` (ausserhalb Repo, `$ERIC_DIR`; kein absoluter Pfad im Code).
- `elster/smoke_test.py` liest `ERIC_DIR`, Default `~/02_Software/eric/`.
- `elster/README.md` dokumentiert Pfad + Aufruf + Verdikt.
