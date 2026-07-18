# Repeated-Instance-Store-Modell — voller Zuschnitt (build-ready, Bau POST-UI)

**Status:** concept-first, build-ready. Erweitert den BANK-Report zum baubaren Design. Bau POST-UI
(großes Struktur-Stück). Das EINE Investment entsperrt drei deferred Fronten gemeinsam: Per-Kind-Kz,
Multi-Objekt-§21 (Person-B-V+V-Defer landet hier), Multi-Rente. LLM-frei.

## Entscheidung: Instanz-Suffix im feld_id (Option A) — Store-Kern bleibt
`feld_id#i` (i = 1..N). Der flache Store (feld_id→wert, One-Active-Event, meet_zustand) bleibt UNVERÄNDERT;
die Instanz ist reine feld_id-KONVENTION (wie `_partner` ein Suffix ist). Kein Store-Schema-Umbau, keine
Person-Dimension-Migration. Begründung: minimale Doktrin-treue Änderung, alle Invarianten (One-Active-Event
je feld_id#i, herkunft-Vektor je Instanz, meet_zustand je Instanz-Aggregat) gelten unverändert.

## Wie #i durch die Schichten threadet
### store.append_event / materialisiere / snapshot — KEINE Änderung
`bruttoarbeitslohn` und `vv_einnahmen#2` sind einfach zwei feld_ids. append_event/One-Active/meet_zustand
behandeln sie identisch. snapshot (content-adressiert) faltet sie mit — kein Instanz-Sonderpfad. **Der
Store lernt die Instanz GAR NICHT; sie ist Konvention der Bindung + est_mapping.** (Das ist die Stärke von A.)

### bindung — Instanz-Feld-GRUPPE (deklarativ, NEU)
Ein Bindungs-Konstrukt `instanz_gruppe: <name>` je wiederholbarem Feld + ein Zähl-Feld:
```
- feld_id: anzahl_objekte   (int, count-Anker; wie fam_anzahl_kinder)
- feld_id: vv_einnahmen      instanz_gruppe: vv_objekt   (die Basis-Felder tragen die Gruppe)
- feld_id: vv_gebaeude_afa   instanz_gruppe: vv_objekt
  …
```
Die konkreten Instanzen `vv_einnahmen#1..#N` entstehen zur Laufzeit (UI „+ Objekt"); die Bindung deklariert
nur die GRUPPE (welche Felder je Instanz wiederholen) + das Zähl-Feld. Schema.json: +instanz_gruppe-property.

### est_mapping — INSTANZ-Klasse (Klasse e erweitern)
Heute Klasse e = nur count (fam_anzahl_kinder→N leere Anlagen). Erweitern zu INSTANZ-MULTIPLIKATION:
je Instanz i die Kz-Gruppe der Anlage-Instanz i emittieren. Reuse der bestehenden Person-A-Kz je Instanz
(KEIN neuer Kz — die Anlage-Instanz-Dimension ist analog zur Person-B-Dimension: person_b-Bucket → instanz-i-Bucket).
Result-Struktur: `anlage_instanzen: {gruppe: [{i, felder: {kz: wert}}, …]}` neben deklaration/person_b.
Die Instanz-Achse × Person-B-Achse kombinierbar (`vv_einnahmen#2_partner`) falls je gebraucht.

### Drift-Wächter / Guards
- Kz-Eindeutigkeit ÜBER Instanzen: dieselben Kz je Instanz (wie Person-B) — kein Phantom, `_transform_quellen`
  + `_erlaubte_kz` lernen die Instanz-Gruppe (aus der Bindung ableitbar, kein Hardcode).
- feld_id-Eindeutigkeit: `#i`-Suffix hält global eindeutig; die Basis-feld_id ohne # bleibt die „Vorlage".
- fail-closed je Instanz: eine unvollständige Instanz → deren Aggregat vorlaeufig (meet_zustand je Instanz).

## Per-Front-Freischaltung (nach dem Modell)
| Front | instanz_gruppe | je-Instanz-Felder | per-Instanz-Kz (Bash-Recon vor Bau) |
|---|---|---|---|
| **Multi-Objekt §21** | vv_objekt | einnahmen + 4 WK-Details | Anlage-V je Objekt (E0700201/E0703838 je Instanz, reuse) |
| **Per-Kind** | kind | IdNr, Kindschaftsverhältnis, Zeitraum, Kindergeld-Anspruch | Anlage-Kind je Kind (E0500406/E0500807… je Instanz) |
| **Multi-Rente** | rente | jahresrente + art + beginn/alter | Anlage-R [Einz] je Rente (E1800301… je Instanz, PARTNER_VERZWEIGUNG-Muster ×Instanz) |

Je Front danach nur: bindung-Gruppe deklarieren + est_mapping-Instanz-Config + Test. Der Kern (Store/est_mapping-
Instanz-Klasse/Drift) ist EINMAL gebaut.

## Bau-Reihenfolge (POST-UI)
1. Kern: est_mapping INSTANZ-Klasse (feld_id#i-Parse + anlage_instanzen-Bucket) + schema.json instanz_gruppe +
   Drift-Wächter-Instanz-Awareness + Kern-Tests (2 Instanzen, meet je Instanz, Kz-Reuse, fail-closed).
2. Erste Front = Multi-Objekt-§21 (konkreter Person-B-V+V-Defer-Landeplatz) als Referenz-Anwendung + Goldens.
3. Dann Per-Kind (ELSTER-Form-Pflicht) + Multi-Rente inkrementell.
UI (dev-1): „+ weiteres Objekt/Kind/Rente"-Wiederholung über der Instanz-Gruppe.

## Kz-Instanz-Recon AUFGELÖST (kz_extract 2026-07-18) — alle drei = INSTANZ-REUSE
Konklusiv (Hash/Vorkommen, nicht E-Präfix): ALLE drei Anlagen tragen je Instanz DENSELBEN Kz, KEIN
distinkter Instanz-Kz (kein Objekt-2-/Kind-2-/Rente-2-Kz existiert). Multi-Instanz läuft über SEPARATE
Anlage-Sub-Dokumente (wie Person-B den person_b-Bucket), nicht über distinkte Kz.
- **Anlage V:** genau EINE Mieteinnahmen-Kz **E0700201** [Einz] (+ E0700401 „andere Räume") — keine
  E07x0201-Objekt-Serie. WK-Aggregat E0703838 je Objekt. → Instanz-Reuse.
- **Anlage R:** EINE Rentenbetrag-Kz je ART (**E1800301** gesetzl/aa, **E1801601** priv/bb, **E1803102**
  sonst) [Einz], keine Rente-2-Serie. → Instanz-Reuse × ART (PARTNER_VERZWEIGUNG-Muster × Instanz).
- **Anlage Kind:** EINE **E0500406** (IdNr) + **E0500807/808** (Art Kindschaftsverhältnis, je Elternteil) +
  E0500601/805 (Zeitraum) je Kind, keine Kind-2-Serie. → Instanz-Reuse.
→ **FOLGE: die est_mapping-INSTANZ-Klasse reused je Instanz die Person-A-Kz (kein neuer Kz, KEIN Anker-vorab
für Instanz-Kz)** — exakt das Person-B-Muster auf der Instanz-Achse. Repeated-Instance ist damit VOLL
build-ready; keine Kz-Recon mehr offen.

## Zur Abnahme
(1) Instanz-Suffix A bestätigt (vs Container B)? (2) instanz_gruppe-Bindungs-Konstrukt OK? (3) Bau-Reihenfolge
Kern→Multi-Objekt→Per-Kind/Rente OK? (4) POST-UI-Timing bestätigt? → dann Kern-Bau wenn UI durch.
