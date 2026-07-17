# Stufe-A-Zuschnitt — Gesamtsteuer-Ring MVP (Scheibe `an_gesamt`)

**Auftrag:** Instructor — Accessor/Golden-Zuschnitt VOR Bau, Anker/Werte handverifiziert.
**Zone:** golden/runner (Accessor+Goldens) = dev-1; NATIV_EINHEIT/Bindung = dev-2. **Kein Code.**

Drei Befunde aus der Engine ändern den Entscheid 1/2-Zuschnitt — bitte abnehmen, dann baue ich.

## Befund 1 — verfügbare Catala-Module (pkg)

Kompiliert/aufrufbar in `oracle/gettsim/_catala/pkg`: **Einkommensteuertarif** (inkl.
`festzusetzende_est_einzel`, `arbeitnehmer_pauschbetrag`), **Entfernungspauschale**, **Arbeitszimmer**.
**KEINE** Catala-Module für dHf (§9 Abs.1 Nr.5), Verpflegung (§9 Abs.4a), Arbeitsmittel (§9 Abs.1
Nr.6/7), VOR (§10). Konsequenz: die WK-Familien dHf/Verpflegung/AM lassen sich **heute nicht** als
`werbungskosten`-Summanden aus Catala ziehen — nur EP hat ein Modul. VOR ist als **Python-Andockung**
`runner._vorsorge_abzug` vorhanden (§10-Deckelung, Instructor-Semantik msg 1197).

## Befund 2 — §9a sitzt SCHON im est-Modul (Korrektur zu Entscheid 1)

`festzusetzende_est_einzel` wendet den Arbeitnehmer-Pauschbetrag (§9a Nr.1, **1230 €/VZ2025**)
**intern** an. Handverifiziert (Bruttolohn 40000):

| werbungskosten_in | festzusetzende_est | Deutung |
|---|---|---|
| 0 | 6919 € | Pauschbetrag 1230 greift |
| 1230 | 6919 € | == (WK ≤ Pauschbetrag) |
| 1500 | 6834 € | WK > Pauschbetrag → WK greift |
| 2156 | 6629 € | EP-Fall |

⇒ **`catala_werbungskosten_n` liefert die ROH-WK-Summe (kein §9a-Günstiger)** — das est-Modul macht
§9a. Das erfüllt deine Sorge („Haut baut §9a nicht nach") sogar besser: §9a bleibt im **amtlichen
Catala-Tarif**, nicht in einem WK-Aggregat. Doppelter Pauschbetrag-Abzug wäre sonst die Falle.

## Befund 3 — die MVP-Kette braucht KEINEN neuen NATIV_EINHEIT-Key

Der einzige Ring-Ausgabewert ist `festzusetzende_est` (NATIV_EINHEIT euro, **schon belegt**).
`werbungskosten`/`sonderausgaben` sind interne money-Inputs zum est-Modul, keine eigenen Ringe →
kein neuer Key nötig. dev-2-Koordination reduziert sich auf die **Bindungen** (keine Key-Zeile).

## Zuschnitt Stufe 1 (reiner Arbeitnehmerfall)

**Accessor (dev-1, golden/runner):**
- `catala_werbungskosten_n(s) -> int` (Euro, ROH): Stufe 1 = nur EP (`catala_entfernungspauschale`).
  dHf/Verpflegung/AM = 0 (kein Catala-Modul → bestätigte Null in der Haut; echte Aggregation = Stufe 1b
  nach deren Catala-Bau). Benannter Aggregat statt EP-direkt, damit die Erweiterungsstelle markiert ist
  und die Haut einen semantisch klaren „werbungskosten"-Einstieg hat.
- `sonderausgaben` via bestehendem `runner._vorsorge_abzug(s, year)` (Euro): keine neue Engine.
- Ring-Ausgabe via bestehendem `catala_est` (Dispatcher auf `bruttoarbeitslohn`) →
  `festzusetzende_est_einzel(bruttoarbeitslohn, werbungskosten, sonderausgaben, vz)`.

**Haut-bescheid_fn (dev-1, api.py, Option-A-Mechanismus unverändert):** komposite slot_fn baut
`{bruttoarbeitslohn, werbungskosten=catala_werbungskosten_n(EP-Slots), sonderausgaben=_vorsorge_abzug(VOR-Slots), veranlagung, vz}`
→ `catala_est`. Scheibe `an_gesamt`: `gesamt_ring="festzusetzende_est"`.

**Bindungen (dev-2):** `bruttoarbeitslohn` (Anker LStB Nr.3 / §2 Abs.2), `veranlagung` (§26). VOR-Felder
existieren schon (`vor_an/ag/rv_*`). Kinderzahl = Stufe 2.

**Sammelbestätigung (dev-1, Haut):** 4 Blöcke KAP / V+V / Gewinn(inkl. GWG) / sonstige = je bestätigte
Null (`wert=0, bestaetigt, laie, signal_2`). Plus dHf/Verpflegung/AM = bestätigte Null bis Stufe 1b.
Ring erst echt, wenn ALLE Inputs im bestätigten Kegel (`meet_zustand`), sonst
`grund=input_kegel_nicht_bestaetigt` — nie Teilsumme als Bescheid.

## Goldens (handverifiziert, VZ2025, golden/runner)

| Golden | Fall | Eingaben | erwartet |
|---|---|---|---|
| G-EP-roh | EP-Roh-WK | 220 Tg, 30 km, eig. Kfz | **2156 €** |
| G-AN-A | reiner AN, nur EP | brutto 40000, WK 2156, so 0 | **festzusetzende_est 6629 €** |
| G-AN-B | AN + VOR | brutto 40000, WK 2156, so 3500 (VOR: beitr 7000/AG 3500, HB 29344) | **5570 €** |
| G-AN-C | §9a-Pauschbetrag | brutto 40000, WK 0, so 0 | **6919 €** |

Kontroll-Kette G-AN-A: `einkuenfte_ns = 40000 − max(2156, 1230) = 37844`; Sonderausgaben-Pauschbetrag;
Grundtarif VZ2025 → 6629. VOR-Formel (G-AN-B): `max(0, min(beitr, 29344) − AG_steuerfrei)` (Euro).

## Offene Entscheide (zur Abnahme)

1. **§9a-Korrektur bestätigen:** `catala_werbungskosten_n` liefert ROH (kein §9a) — §9a im est-Modul. OK?
2. **Wrapper vs. EP-direkt:** benannter `catala_werbungskosten_n` (Empfehlung, erweiterbar) oder Stufe-1
   die Haut ruft `catala_entfernungspauschale` direkt als `werbungskosten`?
3. **dHf/Verpflegung/AM:** als bestätigte Null in Stufe 1 (Empfehlung) — echte Aggregation erst nach
   deren Catala-Modul-Bau (Stufe 1b, eigenes Paket). OK?
4. **Bindungs-Anker bruttoarbeitslohn/veranlagung** — dev-2 legt an, du gegen-reviewst.
5. **Golden-Ablage:** golden/cases/ neue AN-Fälle (G-AN-A/B/C) — Gate zieht mit?

Nach Abnahme: Accessor `catala_werbungskosten_n` + Goldens (dev-1), Bindungs-Micro-Zeile (dev-2 via dir),
dann Scheibe `an_gesamt` + e2e (voller bestätigter Kegel, echte festzusetzende_est).
