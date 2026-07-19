# K2-Sweep Ring/Haut — Funde & Defer-Backlog (dev-1, 2026-07-19)

**Auftrag** (Julius #1, via Instructor): die §33/p10d_2-Klasse systematisch jagen — eine Einkunftsart,
die aus einer Basis (GdE-/zvE-Summe, Höchstbetrag, Schwelle, Ratio) **vergessen** wurde, als eine neue
Art (§§13-18-Gewinn, §16-vg, §22-Renten) dazukam ohne nachgezogen zu werden → latenter Under/Over-Tax.

**Zone**: `produkt/haut/api.py` + `golden/runner.py` (READ-ONLY-Sweep, Fixes danach adjudiziert).

## Ergebnis: KEIN neuer Under-Tax
§33 war der einzige Under-Tax der Klasse (schon gefixt 684f902). Alle Rest-Funde **over-tax/neutral**.
Geprüft & sauber: Kapital via Günstiger-Addend (est_mit−est_ohne, `catala_kapital_steuer`) korrekt besteuert
(nicht aus dem steuerbaren GdE vergessen); §35-Nenner (ns+vv+sonstige+gewinn) / §35-Zähler (nur Gewerbe)
vollständig; §31-Familienleistung est-basiert (voll income); §32b-Progressionsvorbehalt nicht implementiert
(Feature absent); §10-Vorsorge-Höchstbetrag = fixe params-yaml (nicht income-abhängig); Haupt-Engine
`catala_gesamt` erhält alle Einkunftsarten (keine Art entkommt der Besteuerung).

## Gefixt (Sofort-Bundle, dieser Commit)
- **#1 §24a Altersentlastungsbetrag-Bemessung** (`api.py` gesamt-slot_fn): `positive_andere_einkuenfte`
  nutzte nur `max(0, vv)` → **einkuenfte_gewinn (§§13-18) vergessen**. §24a S.1: Bemessung = Arbeitslohn +
  positive Summe der Nicht-§19-Einkünfte; S.2-Ausnahmen = nur Versorgungsbezüge/Leibrenten/§22-Nr.4-5 →
  §§13-18-Gewinn MUSS rein. Over-tax (Bemessung↓ → Altersentlastung↓ → GdE↑ → est↑), gedeckelt auf
  Kohorten-Höchstbetrag. Empirisch: geb1958 gewinn30000 kein-Lohn → est 4293→4105 (188€ over-tax).
  Golden `test_gesamt_gewinn_24a_bemessung`.
- **#3 §10d gde_p10d** (`api.py` gesamt-slot_fn): `einkuenfte_sonstige` ergänzt (war ns+vv+gewinn).
  Neutral heute (sonstige≡0 im gesamt-Ring), Konsistenz-Fix = schließt das §22-Loch-Vorsorge → kein
  künftiger §10d-Under-tax. Kein Golden (0-Impact).

## DEFER (over-tax, nicht dringend — Feature-Reste/Backlog)
- **#2 rentner-Ring §24a fehlt ganz**: Rentner (64+, §24a S.3 erfüllt) mit §§13-18-Gewinn kriegt KEINEN
  Altersentlastungsbetrag auf den Gewinn (Leibrente korrekt raus §24a S.2). Over-tax bis
  ~Höchstbetrag×marginal. Braucht §24a-Accessor-Aufruf im rentner-slot_fn + `geburtsjahr`-Feld im
  rentner-Kegel/Bindung. Eigener kleiner Baustein.
- **Person-B §24a** (`api.py` Z.586, `positive_andere_einkuenfte=0`): vergisst B's Nicht-Lohn-Einkünfte
  (vv/gewinn). Dokumentiert-konservativ (over-tax-safe) — braucht **Owner-Trennung im Ring**
  (pre-existing strukturell, nicht 1-Zeile).
- **§16-vg ohne §34-Fünftelregelung** (adjacent, KEIN forgotten-art): Veräußerungsgewinn (netto nach
  §16 Abs.4-Freibetrag) fließt voll progressiv in `einkuenfte_gewinn` statt §34-geglättet. Over-tax,
  **evtl. signifikant** bei großem vg. Eigenes §34-Feature → Feature-Reste-Kandidat (Priorität prüfen,
  over-tax kann groß sein — anders als die gedeckelten §24a-Funde).
