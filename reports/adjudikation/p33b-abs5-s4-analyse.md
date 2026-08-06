# §33b Abs.5 S.4 — Vorprüfung Stufe 2 (reines Lesen)

Stand: 2026-08-06
Quellen: sources/gesetze-im-internet/estg_p33b_2026-07-13.txt, estg_p33_2026-07-11.txt
Code: produkt/haut/api.py, api_constants.py, bindung_p33b_abs5_kind.yaml

---

## 1. Welche Aufwendungen meint S.4 genau?

**§33b Abs.1 S.1** (Wortlaut):

> Wegen der Aufwendungen für die Hilfe bei den gewöhnlichen und regelmäßig wiederkehrenden Verrichtungen des täglichen Lebens, für die Pflege sowie für einen erhöhten Wäschebedarf können Menschen mit Behinderungen unter den Voraussetzungen des Absatzes 2 anstelle einer Steuerermäßigung nach § 33 einen Pauschbetrag nach Absatz 3 geltend machen (Behinderten-Pauschbetrag).

Der Behinderten-Pauschbetrag deckt also NUR diese drei Aufwandsarten ab:

1. **Hilfe bei den gewöhnlichen und regelmäßig wiederkehrenden Verrichtungen des täglichen Lebens** (Anziehen, Waschen, Essen, Mobilität in der Wohnung, etc.)
2. **Pflege** (im Sinne von SGB XI/Pflegebedürftigkeit)
3. **Erhöhter Wäschebedarf** (durch Inkontinenz, starkes Schwitzen, etc.)

**Main's Verdacht bestätigt**: exakter Wortlaut aus §33b Abs.1 S.1.

**§33b Abs.5 S.4**:

> In diesen Fällen besteht für Aufwendungen, für die der Behinderten-Pauschbetrag gilt, kein Anspruch auf eine Steuerermäßigung nach § 33.

Bedeutung: Ist der Kind-PB übertragen, darf der Steuerpflichtige für die drei o.g. Aufwandsarten KEINE §33-Einzelabrechnung mehr machen. Krankheitskosten, Kurkosten, Beerdigungskosten, Zahnbehandlung, etc. sind NICHT betroffen — die bleiben weiterhin §33-fähig.

### Abgrenzung zu §33 Abs.2a (Fahrtkostenpauschale)

§33 Abs.2a S.8: "Sie kann auch gewährt werden, wenn ein Behinderten-Pauschbetrag nach § 33b Absatz 5 übertragen wurde."

Die Fahrtkostenpauschale ist EXPLIZIT vom S.4-Ausschluss ausgenommen (lex specialis). Ist bereits implementiert (catala_p33_2a_fahrtkostenpauschale, additiv zu agB).

---

## 2. Wie müsste das Feld aussehen?

### Problem

`agb_aufwendungen` ist ein ungetrennter CENT-Topf. Der Nutzer gibt ALLE agB zusammen ein (Krankheitskosten + behinderungsbedingte Aufwendungen + Beerdigung + ...). Der Ring kann nicht unterscheiden, welcher Teil "behinderungsbedingt" im Sinne des PB ist.

Pauschale Kürzung des gesamten Topfes wäre Over-tax (Krankheitskosten fielen mit raus). Gar keine Kürzung = Under-tax (Doppelabzug). Beides falsch.

### Muster im Repo

Das Repo hat zwei Muster für Teilmengen:

**(a) Vorsorge-Split (heute früh, 6cb2f31):** `weitere_vorsorgeaufwendungen` → 5 Kategorien, je eigenes Kz. Altes Feld durch neue ersetzt, Summe in api.py addiert. **Brechend** (neue Felder, alte Daten unsichtbar).

**(b) Fahrtkostenpauschale §33 Abs.2a:** Separater rechtlicher Tatbestand, eigenes Feld (`fahrtkosten_pausch_gdb80_oder_70g`, `fahrtkosten_pausch_ag_bl_tbl_h`), per eigenem Accessor (`catala_p33_2a_fahrtkostenpauschale`) berechnet und in api.py zu `agb_aufwendungen` addiert. **Additiv** (bestehendes Feld bleibt, neue Felder on top).

### Empfohlen: additives Teilfeld (Variante A)

**Ein neues Feld `behinderungsbedingte_aufwendungen`** (CENT, Integer, nicht negativ). Der Nutzer gibt NUR den behinderungsbedingten Teil ein (Hilfe bei täglichen Verrichtungen, Pflege, Wäschebedarf). Der Ring kürzt dann:

```python
# In _festzusetzende / _festzusetzende_r, NUR wenn Kind-PB übertragen:
if _kind_pb_uebertragen():
    agb_bereinigt = max(0, _c("agb_aufwendungen") - _c("behinderungsbedingte_aufwendungen"))
else:
    agb_bereinigt = _c("agb_aufwendungen")
```

`agb_aufwendungen` bleibt als unveränderter Rohwert erhalten. `behinderungsbedingte_aufwendungen` ist eine "davon"-Angabe: wie viel der agB sind behinderungsbedingt.

**Begründung:**
- Additiv → bricht keine Bestandsdaten (Feld kann long-term optional sein)
- Nutzer muss nur den behinderungsbedingten Teil trennen, nicht alle agB-Arten
- Fail-closed: Feld absent → 0 → kein Abzug → Under-tax bleibt (wie heute), aber kein Over-tax
- Analog zum Vorsorge-Split (ein neues Feld pro Teilmenge), aber als Abzug statt Aufspaltung
- kind_kv_pv als Präzedenz: Feld `kind_kv` (CENT) ist Teilmenge von `basis_kv`, aber separat erfasst

**Nachtrag für die Zukunft**: Ein zweites Feld `behinderungsbedingte_aufwendungen_kind` für den Fall, dass das Kind selbst (nicht der Elternteil) die Aufwendungen hat. Nicht in Stufe 2.

### Alternativ verworfen

- **Variante B (Pauschalabzug)**: agB um PB-Betrag kürzen → Over-tax wenn behinderungsbedingte Aufwendungen < PB
- **Variante C (Aufspaltung agb_aufwendungen)**: Zwei Felder statt einem → bricht alle Bestandsdaten, Migration nötig

---

## 3. Wie groß ist der Fehler heute?

### Rechenweg

**Fall**: Elternteil, einzel, keine Kinder (außer Kind-PB-Empfänger)
- GdE = 40.000 EUR (nach allen Abzügen außer agB)
- Kind-PB übertragen (GdB 100 → 2.840 EUR PB)
- 3.000 EUR behinderungsbedingte Aufwendungen unter agB
- Keine weiteren agB

**Heute (ohne S.4):**
- `ausserg` = 2.840 EUR (Kind-PB, §33b)
- agB-Deckel: `catala_p33_agb({"aussergewoehnliche_belastungen": 3.000, "gde": 40.000, ...})`
  - Zumutbare Belastung = 5% von 40.000 = 2.000 EUR (§33 Abs.3 Nr.1a, einzel, 0 Kinder)
  - Abzug = max(0, 3.000 - 2.000) = 1.000 EUR
- Gesamt agB = 2.840 + 1.000 = 3.840 EUR
- zvE = 40.000 - 3.840 = 36.160 EUR

**Mit S.4 (korrekt):**
- `ausserg` = 2.840 EUR (Kind-PB, §33b — unberührt)
- agB-Deckel: `catala_p33_agb({"aussergewoehnliche_belastungen": 0, ...})`
  - Abzug = max(0, 0 - 2.000) = 0 EUR
- Gesamt agB = 2.840 + 0 = 2.840 EUR
- zvE = 40.000 - 2.840 = 37.160 EUR

**Differenz**: 1.000 EUR mehr zvE.

### Geschätzte Steuerdifferenz

Grundtabelle 2025, einzel, zvE ~36.160 vs ~37.160:
- Grenzsteuersatz in diesem Bereich: ~28-32%
- Steuerdifferenz: **~280-320 EUR Under-tax** (ca. 30% × 1.000)

### Messaufbau für Main

Bitte fahren auf aktuellem Code (d3de52a), Scheibe `gesamt`:

```python
KEGEL = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 40000000),  # 40.000 EUR
    # Basis-Kegel (minimal)
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
    # Kind-PB-Übertragung (GdB 100)
    ("kist_konfession", "roemisch-katholisch"), ("kist_bundesland", "nordrhein_westfalen"),
    # ANNAHME: Kind-PB-Instanz braucht kind_idnr + kind_grad_der_behinderung + antrag + nicht_selbst
    # → geht über EM.instanzen("kind"), nicht über Kegel-Felder.
    # agB: 3.000 EUR behinderungsbedingt
    ("agb_aufwendungen", 300000),  # 3.000 EUR in CENT
]
```

**Erwartete Ergebnisse:**
- `zahl_cent` aktuell: ~X.XXX (mit fälschlichem 1.000-EUR-Abzug)
- `zahl_cent` korrekt: ~X.XXX + 280-320 EUR = ~Y.YYY
- Differenz im `kist_cent`/`solz_cent` entsprechend (KiSt 9% davon = ~25-29 EUR)

**Fallback bei fehlender Kind-Instanz**: Wenn der Kind-PB nicht per Instanz gesetzt werden kann, alternativ den Fall ohne Kind-PB-Übertragung (nur eigener GdB des Steuerpflichtigen): GdB 50, 1.140 EUR PB, 3.000 EUR behinderungsbedingte agB. Gleiche Mechanik, gleicher Geldbetrag (PB fix, agB 3.000).

---

## 4. Erweiterung: Abs.1 S.1+S.2 — das Wahlrecht des Steuerpflichtigen (Main 2026-08-06)

Übertragungsfall (Abs.5 S.4) ist NICHT die einzige Trennungslücke. §33b Abs.1 S.1 trägt sein **eigenes Ausschlussverhältnis**, woertlich:

> Wegen der Aufwendungen für die Hilfe bei den gewöhnlichen und regelmäßig wiederkehrenden Verrichtungen des täglichen Lebens, für die Pflege sowie für einen erhöhten Wäschebedarf können Menschen mit Behinderungen ... **anstelle einer Steuerermäßigung nach § 33** einen Pauschbetrag nach Absatz 3 geltend machen

> (2) Das Wahlrecht kann für die genannten Aufwendungen im jeweiligen Veranlagungszeitraum nur **einheitlich** ausgeübt werden.

**"Anstelle"** = entweder PB ODER §33-Einzelnachweis für die drei Abs.1-S.1-Aufwandsarten, nie beides. Unser Ring rechnet sie aber **praktisch additiv** (Mains Messung, 40.000 Lohn, einzel):

```
ohne alles                    ESt 6919,00
GdB 100, keine agB            ESt 6039,00     PB-Wirkung   880,00
kein GdB, 3.000 agB           ESt 6660,00     agB-Wirkung  259,00
GdB 100 + 3.000 agB           ESt 5788,00     zusammen   1.131,00
```

Sind die 3.000 EUR behinderungsbedingte Aufwendungen i.S.v. Abs.1 S.1, ist das **derselbe Doppelabzug** wie im Übertragungsfall — nur trifft er **JEDEN Behinderten mit agB-Eintrag**, nicht nur Übertragungsfälle. Deutlich größerer Kreis.

### Verhaltensunterschied der beiden Ausschlüsse

| | Abs.5 S.4 (Übertragung) | Abs.1 S.1 (eigener PB) |
|---|---|---|
| Art | **Automatischer Ausschluss** | **Wahlrecht** des Steuerpflichtigen |
| Wer entscheidet | Gesetz erzwingt (kein Wahlrecht des Elternteils) | Steuerpflichtiger, einheitlich (S.2) |
| Ring kann | Kürzen, ohne zu fragen | NICHT erzwingen — muss fragen |
| Was | PB ist übertragen → agB für die 3 Arten sperren | PB wählen ODER Einzelnachweis für die 3 Arten |

**Konsequenz**: Abs.5 S.4 ist ein Automatismus (darf der Ring still tun, sobald Übertragung vorliegt). Abs.1 S.1 ist eine **Nutzerentscheidung** — der Ring kann nicht wissen, ob der Behinderten-Pauschbetrag die bessere Wahl ist. Das braucht eine **Frage**, keinen Automatismus.

**Meine Einschätzung was richtig ist**: Abs.1 braucht eine Frage in der Art "Möchtest du für die behinderungsbedingten Aufwendungen den Behinderten-Pauschbetrag nutzen oder die tatsächlichen Kosten als außergewöhnliche Belastung absetzen?" — mit dem Feld `behinderungsbedingte_aufwendungen` als "davon"-Aufteilung. Der Nutzer trennt den behinderungsbedingten Teil (additiv, bricht nichts); die Wahl (PB vs Einzelnachweis) beantwortet er separat.

## 5. Fehlergröße — zweiter Fall (eigener PB)

Mains Messung: 1.131 statt korrekt 880 (falls agB behinderungsbedingt) → **~251 EUR zu wenig Steuer** im Beispielfall. Nachgeprüft: 1.131 − 880 = 251. Stimmt.

```
korrekt (PB, keine agB für die 3 Arten)   ESt 6039,00
heute (PB + agB doppelt)                   ESt 5788,00
Differenz = Under-tax                     ~251,00 EUR
```

Relevanz: **deutlich größer als Stufe 2** — betrifft jeden Behinderten mit agB-Eintrag, nicht nur Übertragungsfälle. Der Übertragungsfall (Stufe 2, ~300 EUR) ist ein Teil hiervon.

## 6. Empfehlung: ein Feld für beide Fälle

**EIN Feld `behinderungsbedingte_aufwendungen` für beide Baustellen.** Beide bräuchten dieselbe Trennung (welcher Anteil der agB ist behinderungsbedingt i.S.v. Abs.1 S.1). Zwei getrennte Baustellen hieße denselben Nutzer zweimal dieselbe Einteilung fragen — fehleranfällig, doppelt.

**Verhaltensunterschied liegt NUR in der Verarbeitung, nicht im Feld:**

```python
# Abs.5 S.4 (Übertragung): automatisch kürzen (Gesetz erzwingt)
if _kind_pb_uebertragen():
    agb_bereinigt = max(0, agb_aufwendungen - behinderungsbedingte_aufwendungen)

# Abs.1 S.1 (eigener PB): Wahlrecht — nur kürzen, wenn Nutzer den Einzelnachweis gewählt hat
elif _eigener_pb_genutzt() and _wahl_einzelnachweis():
    agb_bereinigt = max(0, agb_aufwendungen - behinderungsbedingte_aufwendungen)
```

**Begründung EIN Feld:**
- Einmalige "davon"-Trennung des Nutzers statt zwei Fragen
- Beide Absätze referenzieren dieselben 3 Aufwandsarten (Abs.1 S.1)
- Additiv, absent=0=heutiges Verhalten (Under-tax bleibt, kein Over-tax)
- Das Wahlrecht (Abs.1) ist eine eigene bool-Frage, kein zweites Betragsfeld

**Stufe 2 wird dadurch größer** — zwei Verarbeitungsstellen desselben neuen Feldes statt einer. Aber das Feld-Design ist identisch; der Mehrnahm ist eine Frage + ein Kürzungszweig, kein zweites Feld.