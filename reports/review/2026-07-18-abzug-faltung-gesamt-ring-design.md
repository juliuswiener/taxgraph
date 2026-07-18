# Design-Proposal: Sonder-Abzüge in den EINEN gesamt-Ring falten (Weg ii)

**Read-only Entscheidungs-Input für Julius (kein Bau).** Instructor-Auftrag 2026-07-18.
Frage: Sollen §35a/§10b/§33/§10-KiSt (+ kommende §24a/§24b/§31) als **optionale Abzugs-Fragen**
in den EINEN gesamt-Wegpunkt-Fluss gefaltet werden — statt eigener Scheiben `haushalt_gesamt`/`agb_gesamt`?

## Das Problem (warum überhaupt)

Heute sind `haushalt_gesamt` (§35a+§10b) und `agb_gesamt` (§33+§10-KiSt) **standalone §19-Basis-Ringe**.
Ein Fall wählt GENAU EINE Scheibe. Folgen:

1. **Nicht-Komposition (Korrektheitsfehler):** ein Vermieter (§21) MIT Handwerker-Rechnung (§35a) kann
   beides nicht in EINEM Bescheid erfassen — `gesamt` rechnet kein §35a, `haushalt_gesamt` kein §21.
2. **Falsche GdE-Basis:** die §10b-20%-Deckelung und die §33-zumutbar-Staffel laufen heute auf der
   §19-GdE (nur Lohn), NICHT auf dem echten Gesamtbetrag der Einkünfte (Lohn+Vermietung+Kapital). Bei
   einem Vermieter mit Nebenjob ist die §10b-Grenze/agB-zumutbar damit **zu niedrig angesetzt**.
3. **UI-Unerreichbarkeit:** die 2 Sonder-Scheiben haben gar keine Start-Kachel (Backend-live, UI-tot).

Weg (ii) löst alle drei: EIN Ring, Abzüge additiv auf JEDE Einkunfts-Kombi, GdE = echter Gesamtbetrag.

## (1) Was ändert sich am gesamt-slot_fn

Der gesamt-slot_fn rechnet heute §19+§21+§20 → `catala_gesamt(einkuenfte_*)`. Die Andock-Slots für die
Abzüge **existieren in catala_gesamt bereits** (`steuerermaessigungen`, `sonderausgaben`,
`aussergewoehnliche_belastungen`) — die Faltung fügt nur die Roh-Wert-Berechnung hinzu, KEIN Engine-Umbau:

```
# NACH den Einkunfts-Komponenten (ns, vv, kapitaleinkuenfte), VOR catala_gesamt:
GdE = summe_der_einkuenfte − §24a − §24b     # § 2 Abs. 3 — Basis für §10b/§33 (steht VOR den Abzügen fest)
§35a  = catala_p35a_haushaltsnahe(minijob, dienstleistung, handwerker)   → steuerermaessigungen
§10b  = catala_p10b_spenden(spende, GdE)                                 ┐
§10-K = catala_p10_kist(gezahlt, erstattet)                             ├→ sonderausgaben (additiv)
§33   = catala_p33_agb(agB, GdE, anzahl_kinder, splitting)               → aussergewoehnliche_belastungen
catala_gesamt(…einkuenfte…, steuerermaessigungen=§35a, sonderausgaben=§10b+§10-K, aussergewoehnliche_belastungen=§33)
```

- **Kein Zirkularitäts-Problem:** GdE (§2 Abs.3) steht VOR den Abzügen (§2 Abs.4) fest — §10b/§33 lesen GdE,
  nie umgekehrt. Der §35a-ESt-Floor (§2 Abs.6) macht p32a regel-seitig (`wirksame_ermaessigung`), unverändert.
- **GdE-Zugang:** `catala_gesamt` gibt heute nur `festzusetzende_est` zurück. Zwei Optionen: (a) GdE in der
  slot_fn aus den Komponenten ableiten (ns+vv+kap−§24a−§24b), oder (b) **empfohlen** — einen schlanken
  `catala_gesamt`-Zwilling, der `gesamtbetrag_der_einkuenfte` exponiert (EINE Wahrheit, kein Nachbau der
  §2-Abs-3-Arithmetik in der Haut). Kleiner runner-Zusatz.
- Die K2-Guards (`rechnung_unbar_offen` §35a Abs.5 S.3, `erstattungsueberhang_offen` §10 Abs.4b) wandern
  1:1 vom Sonder-Scheiben-Guard in den gesamt-Guard (verhaltensgleich).

## (2) Kegel-Auswirkung — die „bestätigte Null"-Falle

Heute sind die Sonder-Felder **Pflicht-Kegel** (jede Scheibe erzwingt sie als bestätigte Null). Gefaltet
müssen sie **optional** werden — sonst müsste JEDER gesamt-Nutzer Handwerker/Spende/agB/KiSt mit 0 bestätigen.

**Problem:** die K2-Doktrin sagt „nicht-erfasst ≠ still 0" (Fake-Grün-Schutz). Ein einfach weggelassenes
§35a-Feld dürfte NICHT still 0 werden. **Lösung (bewährtes Muster):** je Abzugs-Gruppe ein **Opt-in-Gate**
(eine Ja/Nein-Laienfrage „Hattest du Handwerker/Spenden/außergewöhnliche Belastungen/Kirchensteuer?"):

- Gate = **nein** (bestätigt) → die Detail-Felder sind bestätigte Null, Abzug 0, keine Detailfragen (analog
  den 4 `kein_*`-Flags heute).
- Gate = **ja** → die Detail-Felder werden conditional-mandatory (Kegel-Erweiterung, wie `rechnung_unbar` bei
  Dienstleistung/Handwerker>0). Unbeantwortet → `input_kegel_nicht_bestaetigt`, kein Rate-Bescheid.

Damit bleibt der Pflicht-Kegel klein (Einkunfts-Fragen + 4 Abzugs-Gates), die Details erscheinen nur bei
Opt-in. Das ist DERSELBE fail-closed-Mechanismus wie die `fremd_arten`/`kein_*`-Flags — keine neue Doktrin.

## (3) UI-Fluss (gesten-agnostisch)

Der Wegpunkt-Fluss ist heute schon scheibe-agnostisch (ein Fluss, /fragen-getrieben). Die Faltung braucht:

- **Eine Abzugs-Sektion NACH den Einkunfts-Wegpunkten** (die Traverser-Reihenfolge sortiert Einkünfte vor
  Abzügen — die Abzugs-Gates bekommen niedrigeres Frage-Gewicht/kommen später). Kein neuer Screen-Typ:
  die 4 Gate-Fragen sind normale bool-Wegpunkte, die Detail-Fragen normale cent/int-Wegpunkte.
- Optional UX-Politur: die Gates als eine „Abzüge & Ermäßigungen"-Gruppe rendern (Akkordeon), rein
  kosmetisch — der Daten-Fluss ist identisch. Gesten/ELSTER (P3) bleiben orthogonal.
- Der schrumpfende Bescheid-Ring profitiert: Abzüge senken die Spanne live, sichtbar im selben Ring.

## (4) Schicksal von haushalt_gesamt / agb_gesamt

**Empfehlung: nach der Faltung DEPRECATEN.** Sie sind dann vollständig subsumiert — ein „nur §35a"-Nutzer ist
im gefalteten Ring ein §19-Nutzer mit §21/§20=0 und §35a-Gate=ja, also ein normaler gesamt-Fall. Standalone
zu halten hieße zwei Wahrheiten pflegen (die Sonder-Scheiben rechnen die §19-GdE-Variante, gesamt die echte).

- **Kurzfristig** (während die Faltung landet): stehen lassen, bis der gesamt-Ring die Abzüge trägt + Tests grün.
- **Danach:** SCHEIBEN-Keys + Accessor-Branches + die 2 Bindungs-Scheiben-Header entfernen; die
  Accessoren (`catala_p35a/p10b/p33/p10_kist`) BLEIBEN (der gesamt-slot_fn ruft sie). Kein Kachel-Bedarf mehr
  (die Sonder-Scheiben hatten ohnehin keine Kachel — die Faltung macht sie über die EINE gesamt-Kachel erreichbar).
- **Alternative** (falls Julius Schnell-Einstiege will): als „Presets" behalten, die intern den gesamt-Ring mit
  vorbelegten Gates starten. Mehr Code, fraglicher Nutzen — nicht empfohlen.

## (5) Migrations-Aufwand-Schätzung

| Baustein | Aufwand | Anmerkung |
|---|---|---|
| gesamt-slot_fn: 4 Abzüge + GdE einhängen | klein (~40 Z.) | Accessoren + catala_gesamt-Slots existieren |
| GdE-Zwilling in runner (empfohlen) | klein (~8 Z.) | exponiert gesamtbetrag_der_einkuenfte |
| 4 Opt-in-Gate-Felder + conditional-mandatory-Kegel | mittel | Bindung: Gates existieren teils (kein_* Muster); neue Gate-Felder ggf. dev-2 |
| K2-Guards (rechnung_unbar/erstattungsueberhang) in gesamt-Guard | klein | 1:1-Umzug |
| §35a/§10b/§33/§10-Werte-Tests → gesamt-e2e (kombiniert mit §21/§20) | mittel | die Kompositions-Fälle sind NEU (der eigentliche Gewinn) |
| haushalt/agb deprecaten + Tests umziehen | klein | nach grüner Faltung |
| **Summe** | **~1–2 Bau-Sessions** | no-regret; §24a/§24b/§31 (charge30) docken danach am selben Ring an |

**§24a/§24b/§31 (charge30) fügen sich natürlich ein:** §24a/§24b senken die GdE (§2 Abs.3, eigene Slots da),
§31 ist die Kindergeld-vs-Freibetrag-Günstigerprüfung (→ `freibetraege_kinder` + `hinzurechnung_kindergeld`,
Slots da). Der gefaltete gesamt-Ring ist ihr natürliches Zuhause — Weg (ii) ist also nicht nur für die 4
heutigen Abzüge, sondern die Architektur für ALLE §2-Abs-3-bis-6-Andockungen.

## Empfehlung

**Weg (ii) falten, haushalt/agb deprecaten.** Er behebt einen echten Korrektheitsfehler (GdE-Basis +
Nicht-Komposition), nicht nur Kosmetik. Der Aufwand ist moderat (Accessoren + Engine-Slots existieren schon),
das Opt-in-Gate-Muster ist bewährt (fail-closed wie `kein_*`), und er ist die tragfähige Architektur für die
kommenden §24a/§24b/§31. Der einzige echte Neu-Aufwand ist die Kompositions-Test-Matrix — die aber genau den
Gewinn absichert. **Entscheidung liegt bei Julius (UX-Richtung).**
