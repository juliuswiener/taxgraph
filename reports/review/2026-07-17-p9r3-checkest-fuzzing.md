# P9 Runde 3 — checkESt-Fuzzing (AUFTRAG 2, Scout vor Vollausbau)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor
**Harness:** `elster/fuzz/checkest_fuzz.py` (additiv, neu) · **Rohdaten:** `scratchpad/fuzz.json`
Golden-Sachverhalt → est_mapping (Deklarations-Kz) → ELSTER-XML → `EricBearbeiteVorgang(ERIC_VALIDIERE)`,
offline, KEIN Versand. Hersteller-ID nur aus `$ELSTER_HERSTELLER_ID` (nie in Datei/Katalog).

## Kernaussage

- **Kein echter Engine-Fund.** Alle 6 injizierten Goldens (EP/VOR/Arbeitnehmer) validieren `rc=0` —
  unser Mapping erzeugt für diese Felder eine plausible Deklaration.
- **Ein struktureller Infrastruktur-Fund, hohe Priorität:** checkESt **kappt die Fehlerliste bei 20**
  (36 eingebaute Fehler → 20 gemeldet). Stilles Falsch-Grün-Risiko bei >20 Fehlern. Härtung des
  ERiC-Gates nötig.
- **Ein Mapping-Präzisierungs-Kandidat** (VOR-Vorsorge-Split, keine Bug).
- **Grounded Struktur-Regel-Katalog** (10 Operatoren → 11 Regeln) als Nebenprodukt.

## P1 — Golden-Injektion (5 Gruppen quer)

est_mapping-Brücke, nur geerdete Kz (Label via `kz_extract` aus E10-2025.html bestätigt):
EP `entfernung_km_roh→E0203504`, `arbeitstage→E0203503`; VOR `ag_anteil→E2000801`,
`gesamt−ag→E2000401`; N `bruttoarbeitslohn→E0200201`.

| Gruppe | Golden-Fall | injiziert | rc | Ergebnis |
|---|---|---|---|---|
| EP | ep_2024_staffel_30km | E0203503, E0203504 | 0 | CLEAN |
| EP | ep_2024_beispiel1_oepnv | E0203503, E0203504 | 0 | CLEAN (ÖPNV-Feld s.u.) |
| EP | ep_2026_flach_30km | E0203503, E0203504 | 0 | CLEAN |
| VOR | gesamt_2024_vorsorge_capped | E2000801, E2000401 | 0 | CLEAN (Unschärfe s.u.) |
| VOR | gesamt_2026_vorsorge_capped | E2000801, E2000401 | 0 | CLEAN (Unschärfe s.u.) |
| N | arbeitnehmer_2026_einzel_60000 | E0200201 | 0 | CLEAN |

**Triage der Notizen:**
- `mapping-fix` (Präzisierung, kein Bug) — **VOR-Vorsorge-Split:** Golden führt EINE Zahl
  `vorsorge_gesamtbeitraege_inkl_ag`; ELSTER trennt LStB-zeilenscharf: E2000401 (AN-Anteil Nr. 23 a/b),
  E2000801 (AG-Anteil Nr. 22 a/b), E2000601 (ges. RV außerhalb LStB). Best-effort-Split `AN=gesamt−AG`
  ist plausibel (rc=0), aber die 1:1-Herkunft je LStB-Zeile ist nicht abgebildet. Für die ECHTE
  Einreichung braucht das Mapping die LStB-Zeilenaufteilung als Input (heute nicht im Golden-Schema).
- `erwartbar` — **ÖPNV:** `oepnv_kosten_jahr → E0203611` (Aufwand ÖPNV) ist nicht im Minimalfixture;
  Injektion bräuchte Insertion in `Erste_Taetig`. Nicht injiziert.

**Gruppen ohne ESt1A-Deklarationsprojektion (erwartbar, kein Golden zu injizieren):**
- **GWG:** `p6_2`-Sofortabzug ist Anlage-EÜR-Feld E6002301 (eigene Datenart E77), nicht im ESt1A-Kern.
- **§10d:** Verlustvortrag ist gesondert FESTGESTELLTE Bescheid-Größe (analog tarifliche Steuer); der
  vorhandene `verlustvortrag_bestand` liegt im KStG-Nenner-B-Kontext, nicht in der ESt-Deklaration.
- **KAP:** Steuerabzugsbeträge (E1904701 ff.) sind im Fixture bereits populiert + rc=0-plausibel; kein
  ESt-seitiger Golden-KAP-Fall (zinsertrag/-aufwand sind KStG-Nenner-B).

> Befund-Charakter: checkESt validiert die **Deklaration**, unsere Goldens prüfen die **Berechnung**.
> Nur input-seitige Goldens (EP/VOR/N) haben überhaupt eine Deklarationsprojektion — das ist die
> Grenze der Golden×checkESt-Kreuzung, kein Mangel.

## P2 — Struktur-Regel-Katalog (Mutations-Operatoren)

checkESt prüft **Format/Struktur, nicht Magnitude** (999 Arbeitstage / 9999 km → rc=0). Findings
entstehen aus Struktur-Regeln. Katalog (Feldidentifikator → FachlicheFehlerId | RegelName):

| Operator (Mutation) | rc | FachlicheFehlerId | Regel |
|---|---|---|---|
| drop E0203003 (EP Ziel) | 610001002 | 100200126 | Regel_N_2023_100200126 |
| drop E0203501 (EP PLZ/Ort/Str.) | 610001002 | 100200126 | Regel_N_2023_100200126 |
| drop E0203503 (EP Tage) | 610001002 | 111301 **+** 111101 | Werbungskosten_111301 / _111101 |
| drop E0203504 (EP km) | 610001002 | 111301 | Werbungskosten_111301 |
| empty BV (E0102002) | 610001002 | kontextLeer **+** 1016 | formalePruefung / Bankverbindungen_1016 |
| Zeitraum ≠ VZ | 610001002 | 109000005 | Vorsatzdaten_Zeitraum_VZ |
| drop NutzdatenTicket | **610301200** | — | I/O-Gate, short-circuit VOR Plausibilität |
| drop E0200201 (Bruttolohn) | 610001002 | 310030 | Lohnsteuerbescheinigungen_8 |
| drop E1900601 (KAP KiSt) | 610001002 | kontextLeer **+** 192016 | formalePruefung / Steuerabzugsbetraege_192016 |
| magnitude 999 AT / 9999 km | **0** | — | keine Regel (Magnitude ungeprüft) |

Zwei rc-Klassen unterscheiden: `610001002` = Plausibilität (liefert FehlerRegelpruefung);
`610301200` = I/O-Gate (fehlendes Ticket), kurzschließt VOR der Plausibilität → **0** Fehler zurück.
Diese Unterscheidung ist selbst falsch-grün-relevant (s. P3).

## P3 — Trunkierung: HAUPTFUND (Falsch-Grün-Sperre)

- (a) 8 unabhängige Struktur-Drops gestapelt → **alle 8** gemeldet (keine Kappung bei niedriger Zahl).
- (b) **36** numerische Kz mit ungültigem Wert korrumpiert → **nur 20** FehlerRegelpruefung gemeldet.

> **TRUNKIERUNG BESTÄTIGT: checkESt kappt die Fehlerliste bei 20.** Eine Erklärung mit >20 Fehlern
> verliert Fehler 21+ **still**. Wer „Liste nicht leer, aber überschaubar" als „das sind alle Fehler"
> liest, läuft ins Falsch-Grün.

**Mitigation (setting-unabhängig, sofort umsetzbar):** Fixpunkt-Revalidierung — gemeldete Fehler
beheben, RE-validieren, wiederholen bis `rc==0`; eine nicht-leere Fehlerliste NIE als vollständig
behandeln. **Offen:** Cap-Anhebung via `EricEinstellungSetzen` — der Einstellungsname steht im
ERiC-Entwicklerhandbuch ("Bedeutung der ERiC-Einstellungen"); das PDF liegt nicht im Extract unter
`~/02_Software/eric` (nur Vordrucke). Auch `drop NutzdatenTicket → 610301200 mit 0 Fehlern` gehört zur
Sperre: rc-Klasse IMMER prüfen, nie „0 Fehler" allein als grün werten.

## P4 — Groß-XML + Nebenläufigkeit (A1-Nachlauf)

- warm-Latenz auf befülltem Fixture (3788 B): **median ~74ms** — bestätigt A1 (Payload-Größe irrelevant).
- Nebenläufigkeit: EIN ERiC-Prozess = EIN ctypes-Aufruf zur Zeit (serialisiert). Bei M gleichzeitigen
  UI-Validierungen ≈ M×74ms seriell → **Worker-Pool** nötig; das ~70ms-Budget aus A1 gilt pro Slot,
  nicht global. (Echt großes reales XML weiterhin nicht verfügbar — Fixkosten-dominiert, s. A1-Grenzen.)

## Empfehlung für den Vollausbau

1. **ERiC-Gate härten (P3):** Fixpunkt-Revalidierungs-Schleife in den Fuzz-/Gate-Pfad; rc-Klasse
   (Plausibilität vs. I/O-Gate) explizit klassifizieren; Cap-Anhebungs-Setting aus dem
   Entwicklerhandbuch nachtragen. **Höchste Priorität** — betrifft jede künftige checkESt-Nutzung.
2. **VOR-Mapping präzisieren:** Golden-Schema um LStB-Zeilenaufteilung (Nr. 22/23) erweitern, damit
   E2000401/E2000801/E2000601 herkunftsecht befüllt werden.
3. **Vollausbau der Injektion:** ÖPNV-Insertion (E0203611), Anlage-EÜR-Datenart (E77) für GWG/§7g,
   restliche EP-Goldens; erst nach Gate-Härtung, sonst zählt man gegen einen gekappten Fehlerstrom.

## Nachtrag: Prio 1 + 2 umgesetzt (2026-07-17, nach Instructor-Ruling)

**Prio 1 — ERiC-Gate-Härtung** (Details: `reports/review/2026-07-17-eric-gate-haertung.md`): Cap auf
1000 angehoben (`validieren.fehler_max/hinweise_max`, Bereich 1–1000; 10000 wäre WERT_UNGUELTIG),
`klassifiziere_rc`, `eric_gate` Stufe C red-fähiger Trunkierungs-Guard. Verifiziert.

**Prio 2 — Injektions-Vollausbau** (Harness P1 erweitert + neue P5):
- **P1 jetzt 9 Goldens CLEAN** (statt 6): ÖPNV `oepnv_kosten_jahr → E0203611` per XSD-sequenz-korrekter
  Insertion (rc=0), + ep_2024_rz12, + arbeitnehmer 2024/2025. Alle rc=0.
- **P5 — EÜR-E77 (Datenart `EUER_2025`, `libcheckEUER_2025.so`): feasibility BESTÄTIGT** — amtliches
  `EUER_2025_ok.xml` validiert rc=0. **Scope-Fund:** anders als der ESt1A-Kern prüft checkESt bei EÜR
  **Arithmetik-Konsistenz** — Injektion in die Summe der Betriebseinnahmen (E6005501, Übertrag) ohne
  Anpassung der Detailfelder → Regel **604030** „Summe der Betriebseinnahmen fehlerhaft übertragen".
  Das **verfeinert die Kernaussage**: „Struktur/Format, nicht Magnitude" gilt für ESt1A-Deklarations-
  Inputs; **EÜR fügt eine Rechen-/Übertrag-Prüfung hinzu**.
  - Mapping-Erkenntnisse: (1) EÜR-Geldbeträge brauchen Format `N,NN` (2 Nachkommastellen, Komma) —
    Integer wie im ESt1A gibt `zahlOhneDezimalTrenner`. (2) Summen/Übertrag nur konsistent mit den
    Detailfeldern injizieren.
  - **Golden-seitige Lücke (benannt):** kein EÜR-Input-Golden vorhanden (weder GWG/§6(2) noch
    Gewinnermittlung); GWG-Kz E6002301 fehlt im Beispiel → Insertion an korrekter EÜR-XSD-Sequenz-
    position ist der nächste Schritt (nicht fragil geraten).

## Reproduktion

```bash
ERIC_DIR=~/02_Software/eric ELSTER_HERSTELLER_ID=<id> \
    python3 elster/fuzz/checkest_fuzz.py --json out.json
```
