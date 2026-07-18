# Repeated-Instance-Store-Modell — konzeptionelle Bank-Note (dev-2)

**Status:** concept-first, KEIN Bau (read-only während Bash-Outage). Bankt die GEMEINSAME Struktur-
Investition, die DREI deferred Fronten zugleich unlockt. Per-Front-Kz-Enumeration = Bash-pending
(kz_extract). LLM-frei.

## Die drei Fronten teilen EIN Modell
Der est_mapping-MULTIPLIKATION-Kommentar (Klasse e) benennt es schon: Per-Kind, Multi-Objekt-§21 und
Multi-Rente brauchen alle „ein Repeated-Instance-Store-Modell (variable 1..N, keine flach-gecappte
X_1..N-Krücke)". Statt drei Einzel-Krücken = EINE Struktur:

| Front | Instanz-Einheit | pro Instanz | Tarif-Effekt |
|---|---|---|---|
| **Multi-Objekt §21** (V+V-partner-Defer landet hier) | Rental-Objekt (Anlage V je Objekt) | einnahmen + 4 WK-Details | im §2 summiert (per-Objekt egal, aber ELSTER braucht je-Objekt-Zeilen) |
| **Per-Kind** (Nachtrag B) | Kind (Anlage Kind je Kind) | IdNr, Kindschaftsverhältnis, Zeitraum, Kindergeld-Anspruch | count×Betrag tarif-korrekt schon (MVP), Per-Kind-Kz = ELSTER-Form |
| **Multi-Rente** | Rente (Anlage R [Einz] je Rente) | jahresrente + art + beginn/alter | §2 summiert je Person; art je Rente |

## Aktuelle Krücke vs. Ziel-Modell
- **Heute:** flach, EIN Feld je Konzept (kap_kapitalertraege, vv_einnahmen, rentner_jahresrente) +
  Person-B-Zwilling (_partner). Deckt 1 Objekt/1 Rente/count-only-Kinder. Mehr-als-eins = GAP.
- **Ziel:** der Store trägt INSTANZ-gescopte Felder. Zwei saubere Optionen:
  - **(A) Instanz-Suffix im feld_id:** `vv_einnahmen#1`, `vv_einnahmen#2` … — der bestehende flache
    feld_id→wert-Store bleibt, das Suffix ist die Instanz. est_mapping-Multiplikation iteriert #i →
    Anlage-Instanz i. Minimal-invasiv (kein Store-Kern-Umbau), aber feld_id-Parsing.
  - **(B) Instanz-Gruppe im Store:** ein `instanzen: {gruppe: [{feld:wert}, …]}`-Container neben den
    flachen Feldern. Sauberer semantisch, aber Store-Schema-Erweiterung + materialisiere/meet_zustand
    müssen die Gruppen mit-falten (Meet je Instanz).
  → EMPFEHLUNG: (A) Instanz-Suffix — der Store-Kern (feld_id→wert, One-Active-Event, meet_zustand) bleibt
    unverändert, die Instanz ist reine feld_id-Konvention; est_mapping + Bindung + Traverser lernen das
    Suffix. Das ist die kleinste Doktrin-treue Erweiterung (wie _partner ein feld_id-Suffix ist).

## Was jede Front dann NOCH braucht (nach dem Modell)
- **est_mapping:** eine INSTANZ-Multiplikation (Klasse e heute = nur count; erweitern zu „je Instanz i die
  Kz-Gruppe emittieren", Anlage-Instanz i). Reuse der bestehenden Kz je Instanz (kein neuer Kz — wie
  Person-B-Reuse; die Anlage-Instanz-Dimension ersetzt die Person-B-Dimension analog).
- **Bindung:** die Instanz-Feld-Gruppe (welche Felder je Instanz wiederholen) — deklarativ.
- **Drift-Wächter:** Kz-Eindeutigkeit über Instanzen (dieselben Kz je Instanz, wie Person-B — kein Phantom).
- **UI:** „+ weiteres Objekt/Kind/Rente"-Wiederholung (dev-1-Haut).
- **Per-Front-Kz:** die je-Instanz-Kz je Anlage (V/Kind/R) — Bash-pending (kz_extract Hash/Vordruck-Beleg).

## K2 / Doktrin-Passung
- Instanz-Suffix bricht KEINE Invariante: One-Active-Event je feld_id#i, meet_zustand je Instanz, herkunft-
  Vektor je Instanz. Fail-closed bleibt (eine unvollständige Instanz = deren Aggregat vorlaeufig).
- Konsistent mit dem Person-B-Muster (feld_id-Suffix _partner = eine „Instanz"-Achse); Instanz-# ist die
  zweite Achse. Person-B × Multi-Instanz kombinierbar (vv_einnahmen#2_partner) falls je gebraucht.

## Zur Abnahme (wenn Julius den Front priorisiert)
(1) Instanz-Suffix (A) vs Instanz-Gruppe (B)? (Empfehlung A.) (2) Welche Front zuerst (Multi-Objekt-§21 =
der konkrete V+V-partner-Defer-Landeplatz, oder Per-Kind = ELSTER-Form-Pflicht)? (3) Dann per-Front-Kz-
Recon (Bash) + Bau. Bis dahin: gebankt, kein Bau. Kein externer Dienst → kein Cap.
