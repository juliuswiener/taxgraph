# Konzept — Gesamtsteuer-Ring (Roh-Slot → catala_gesamt-§2-Integration)

**Auftrag:** Instructor (vorab, „Konzept wenn A steht"). Der ehrliche Multi-Regel-Bescheid-Ring:
die **festzusetzende ESt** statt Zwischensummen. **Analyse/Design, KEIN Code — größeres Paket mit
eigenem Zuschnitt.** Zone: Engine/Accessoren/Bindung = dev-2; Erfassung/Verdrahtung = Haut.

Gegliedert nach den vier Instructor-Kernfragen.

## Warum überhaupt

Ein Teil-Accessor (`werbungskosten_gesamt`) als „Bescheid" wäre K2-Verletzung (Zahl, die kein
Bescheid ist). Der einzig ehrliche Multi-Regel-Ring ist `festzusetzende_est` über die volle §2-Kette.
Zwei Engine-Andockstellen existieren schon:
- `catala_est` mit `bruttoarbeitslohn` → `festzusetzende_est_einzel(bruttoarbeitslohn, werbungskosten,
  sonderausgaben, vz)` — **reiner AN-Fall**, §9a-Günstiger + §2 + Tarif intern.
- `catala_gesamt` → `festzusetzende_est_gesamt[_zusammen]` — **voller §2-Fall**, ~18 aggregierte Inputs.
`NATIV_EINHEIT["festzusetzende_est"]="euro"` ist belegt → `quantitaet="festzusetzende_est"` ohne Engine-Neubau.

## Frage 1 — catala_gesamt-Inputs und ihre Roh-Slot-Speisung

| catala_gesamt-Input | speist sich aus (Scheibe → Roh-Slot, Aggregation) | Status |
|---|---|---|
| `einkuenfte_nichtselbststaendig` | Bruttolohn − Werbungskosten(EP+dHf+Verpflegung+AM); §9a-Günstiger im Modul | **bruttoarbeitslohn NEU**; WK-Accessor fehlt |
| `sonderausgaben` | VOR-Altersvorsorge-Abzug (p10, gedeckelt) + weitere §10 | VOR-Accessor fehlt; Rest Lücke |
| `freibetraege_kinder` / `hinzurechnung_kindergeld` | Kinderzahl + `kinder_ganzjaehrig` (§31/§32, params-Kindergeld) | Bindung fehlt |
| `veranlagung` (scope einzel/zusammen) | Personen-Parameter (Grundtarif vs Splitting) | Bindung fehlt |
| `einkuenfte_kapitalvermoegen` / `steuer_kapital_gesondert` | KAP-Scheibe (§20/§32d) | nicht erfasst (Lücke) |
| `einkuenfte_vermietung` | V+V-Scheibe (§21) | nicht erfasst (Lücke) |
| `einkuenfte_gewinn` | Gewinn/EÜR-Scheibe (§§13/15/18) — **hier GWG** | nicht erfasst (Lücke) |
| `einkuenfte_sonstige` | §22 (Renten etc.) | nicht erfasst (Lücke) |
| `altersentlastungsbetrag` / `entlastungsbetrag_alleinerziehende` | §24a / §24b (Alter/Alleinerz.) | nicht erfasst (Lücke) |
| `aussergewoehnliche_belastungen` | §33/§33a/§33b-Scheibe | nicht erfasst (Lücke) |
| `sonstige_abzuege_vom_einkommen` / `steuerermaessigungen` | z.B. §35a/§35c | teils (§35c-Accessor da), nicht verdrahtet |
| `anzurechnende_auslaendische_steuern` | §34c/DBA | nicht erfasst (Lücke) |
| `hinzurechnung_zulage`, `tarif_modifiziert`, `tarifliche_est_modifiziert` | §10a-Zulage / §32b Progression / §34-Fünftel | nicht erfasst (Lücke) |
| `veranlagungszeitraum` | Fall-Stammdatum (schon im Store) | ✓ vorhanden |

## Frage 2 — Integrations-Lücken (Roh-Slots ohne Pfad, ehrlich benannt)

- **WK-Familien** dHf/Verpflegung/Arbeitsmittel: haben Roh-Slots + Regeln, aber **keinen exponierten
  Accessor** → speisen `werbungskosten` heute nicht. (EP ist der einzige mit Accessor.)
- **VOR**: kein exponierter Abzugs-Accessor → speist `sonderausgaben` heute nicht.
- **bruttoarbeitslohn / veranlagung / Kinderzahl**: keine Bindung → gar nicht erfassbar.
- **Alle Nicht-N-Einkunftsarten** (KAP/V+V/Gewinn inkl. GWG/sonstige): keine Scheibe → kein Pfad.
- **GWG-Einkunftsart-Grenze**: GWG ist §4/EÜR (Gewinn-Einkünfte), NICHT Anlage-N-Werbungskosten —
  gehört in `einkuenfte_gewinn`, nicht in den WK-Summanden. Heute in `n_vor_gwg` nur deklariert.

## Frage 3 — fehlende VZ-/Personen-Parameter

**Params-geankert (intern gezogen, KEIN Erfassungsbedarf):** Grundtarif/Splitting-Tabelle,
Grundfreibetrag, Kindergeld-Satz (§66, `params/<vz>/kindergeld_p66.yaml`), Vorsorge-Höchstbetrag,
EP-Sätze — catala_gesamt liest sie über `VZ_ENUM[year]` + `params/<vz>/`. Diese sind **keine**
Interview-Felder.

**Personen-Parameter (Erfassung nötig, neue Bindung):**
- `veranlagung` einzel/zusammen → wählt Grundtarif vs. Splitting-scope (harte Bescheids-Weiche).
- Kinderzahl + Ganzjährigkeit → `freibetraege_kinder`/Kindergeld-Hinzurechnung (§31-Günstiger).
- ggf. Alleinerziehend-Status (§24b), Alter (§24a) — je nach Ziel-Tiefe.

## Frage 4 — wann der Ring ECHT wird (bestätigte-Null-Doktrin, fail-closed)

Der Ring ist **erst dann** ein Bescheid, wenn JEDER erreichbare Zielsteuer-Input im **bestätigten
Input-Kegel** liegt (`meet_zustand == bestaetigt`). Ein nicht-erfasster Input darf **nicht still 0**
sein — das wäre Fake-Grün („ich habe deine Kapitaleinkünfte ungefragt auf 0 gesetzt").

**Mechanik (nutzt die bestehende Store-/Meet-Algebra, kein Neubau):**
- Jeder catala_gesamt-Input ist entweder (a) aus einer Scheibe verdrahtet ODER (b) eine **bestätigte
  Null**: ein Event `wert=0, zustand=bestaetigt, herkunft=laie, signal_2` — der Nutzer bestätigt
  bewusst „diese Einkunftsart habe ich nicht".
- Ökonomisch: eine **Abwesenheits-Sammelbestätigung** je Block („Außer Arbeitslohn noch andere
  Einkünfte? Nein" → setzt KAP/V+V/Gewinn/sonstige = bestätigte 0 in einem Zwei-Signal-Akt), statt 18
  Einzel-Nullen. Jede Sammelbestätigung ist ein bewusster append_event.
- **Bis alle Inputs bestätigt sind:** `gesamt_ring`-`_bescheid_fn` liefert None → `/ergebnis`
  `zahl_cent=null`, `grund=input_kegel_nicht_bestaetigt`, `/stand` `engine=unavailable`. **Nie eine
  Teilsumme als Bescheid.** Das ist exakt der Option-A-Mechanismus, nur mit vollem Input-Kegel.

## Zuschnitt (Zonen + Stufen)

**dev-2 (Engine/Bindung):**
1. Accessoren für die WK-Familien — Empfehlung EIN `catala_werbungskosten_n`-Aggregat (EP+dHf+
   Verpflegung+AM → `werbungskosten`, §9a-Günstiger im Modul) + EIN VOR-Sonderausgaben-Accessor;
   je `NATIV_EINHEIT`-Key.
2. Bindungen: `bruttoarbeitslohn` (LStB Nr. 3), `veranlagung`, Kinderzahl — Anker §2/§26/§31.

**Haut (mein Teil, danach):**
3. Scheibe `an_gesamt` mit `gesamt_ring="festzusetzende_est"`; `_bescheid_fn` baut den sachverhalt
   (`bruttoarbeitslohn`, `werbungskosten`=N-Aggregat, `sonderausgaben`=VOR, `veranlagung`) → `catala_est`.
   Trägt der bestehende `SCHEIBEN`/`gesamt_ring`-Mechanismus **unverändert** (nur ein Eintrag + ein
   `_bescheid_fn`-Zweig).
4. Abwesenheits-Sammelbestätigungen für die Nicht-N-Einkunftsarten (bestätigte Null).

**Stufe 1 (MVP-Ring):** reiner AN-Fall über `festzusetzende_est_einzel` (bruttoarbeitslohn + N-WK +
VOR), Nicht-N-Einkünfte als bestätigte Null. Ehrliche Beschriftung: **„ESt — reiner Arbeitnehmerfall"**.
**Stufe 2:** KAP/V+V/Gewinn(inkl. GWG)/agB als eigene Scheiben → `catala_gesamt` voll; der AN-Ring
wird ein Summand.

## Offene Entscheide (Instructor/dev-2)

1. WK: EIN `catala_werbungskosten_n`-Aggregat (Empfehlung) oder vier Familien-Accessoren?
2. MVP-Engine: `festzusetzende_est_einzel` (wenige Inputs, schneller Ring) ODER gleich `catala_gesamt`
   (voll, mehr bestätigte Nullen)?
3. `bruttoarbeitslohn`/`veranlagung`/Kinderzahl-Bindung: dev-2 legt sie an (Anker LStB/§2/§26/§31)?
4. Abwesenheits-Sammelbestätigung: Granularität (ein „andere Einkünfte? Nein" vs. je Einkunftsart)?
5. GWG: bleibt in `n_vor_gwg` (Deklaration) bis der EÜR-Gewinn-Ring kommt — bestätigt?

**Kein Code in dieser Stufe.** Nach eurer Accessor-/Bindungs-Entscheidung baue ich `an_gesamt`
(kleiner additiver Eintrag auf dem Option-A-Mechanismus) + e2e mit vollem bestätigten Input-Kegel.
