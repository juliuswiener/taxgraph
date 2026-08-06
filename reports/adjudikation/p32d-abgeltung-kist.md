# §32d Abs.1 S.3-5 — Abgeltung-KiSt Analyse

Stand: 2026-08-06
Quellen: sources/gesetze-im-internet/estg_p32d_2026-07-13.txt
Code: golden/runner.py Z.796-833, produkt/haut/api.py Z.456-499 (AN), 1136-1198 (gesamt), 1620-1646 (rentner)

---

## 1. Veranlagung: Wann kommt Kapital-KiSt in die ESt-Erklärung?

§32d unterscheidet zwei Zugänge — Abs.3 (Pflicht) und Abs.4 (Antrag):

**Abs.3 — Kapitalerträge OHNE KESt-Abzug (Pflichtveranlagung):**

> (3) Steuerpflichtige Kapitalerträge, die nicht der Kapitalertragsteuer unterlegen haben, hat der Steuerpflichtige in seiner Einkommensteuererklärung anzugeben. Für diese Kapitalerträge erhöht sich die tarifliche Einkommensteuer um den nach Absatz 1 ermittelten Betrag.

Bedeutung: Kein KESt-Einbehalt (z.B. ausländische Kapitalerträge ohne inländisches Kreditinstitut, bestimmte §20 Abs.2-Sachverhalte ohne inländischen Schuldner) → **Steuerpflichtiger muss erklären**. Die §32d-Abgeltung wird auf die tarifliche ESt draufgesattelt. Das Institut hat hier nie KiSt abgeführt → die KiSt-Lücke ist eindeutig: der Ring muss die KiSt auf den Abgeltungsbetrag korrekt berechnen.

**Abs.4 — Kapitalerträge MIT KESt-Abzug (Antragsveranlagung):**

> (4) Der Steuerpflichtige kann mit der Einkommensteuererklärung für Kapitalerträge, die der Kapitalertragsteuer unterlegen haben, eine Steuerfestsetzung entsprechend Absatz 3 Satz 2 ... beantragen, ... zur Anwendung von Absatz 1 Satz 3 [KiSt-Korrektur].

Bedeutung: Selbst wenn das Institut KESt + KiSt korrekt einbehalten hat (pauschal 25% × k auf die Brutto-Kapitalerträge), kann der Steuerpflichtige die **e/(4+k)-Korrektur** beantragen. Diese Korrektur wirkt sich steuermindernd auf die ESt aus (die KiSt auf Kapital wird in die Formel eingewoben, reduziert den ESt-Satz faktisch von 25% auf 25% × 4/(4+k)).

**Relevanz für KiSt-Lücke:**
- Abs.3-Fälle: KiSt nie abgeführt → Ring MUSS sie korrekt berechnen (= heute Under-tax)
- Abs.4-Fälle: KiSt meist korrekt beim Institut abgeführt; die e/(4+k)-Korrektur ist eine **ESt-Ermäßigung** (nicht KiSt-Erhöhung). Aber: die KiSt wird separat festgesetzt — der Ring müsste sie von der abgeführten KiSt abgrenzen können

**Empfehlung:** Abs.3-Fälle priorisieren (Pflicht, klare Lücke). Abs.4-Fälle als Folgebaustein.

---

## 2. "e" — die nach §20 ermittelten Einkünfte

§32d Abs.1 S.5:

> „e" die nach den Vorschriften des § 20 ermittelten Einkünfte

Im Ring: `kapitaleinkuenfte` = `catala_sparer_pb({kapitalertraege, sparer_pb})` = max(0, kapitalertraege - pauschbetrag)

Das ist die §20-Einkünfte nach Sparer-PB (bzw. WK, die aber nach §20 Abs.9 S.2 Hs.2 durch den PB ausgeschlossen sind, wenn PB ≥ WK). Die Verlustverrechnung (§20 Abs.6) läuft separat in `catala_kapital_verrechnung` und speist das gleiche `kapitaleinkuenfte`.

**Befund:** "e" ist IDENTISCH zum Ring-Wert `kapitaleinkuenfte`. Kein neues Feld nötig. Keine Abweichung.

---

## 3. "q" — ausländische Steuer (§32d Abs.5)

§32d Abs.1 S.5:

> „q" die nach Maßgabe des Absatzes 5 anrechenbare ausländische Steuer

§32d Abs.5 — eigenständige Anrechnungsvorschrift für Kapitalerträge:

> (5) ... ist die auf ausländische Kapitalerträge festgesetzte und gezahlte ... ausländische Steuer, jedoch höchstens 25 Prozent ausländische Steuer auf den einzelnen steuerpflichtigen Kapitalertrag, auf die deutsche Steuer anzurechnen.

### Abgrenzung zu §34c/DBA-Infrastruktur

| Aspekt | §34c Abs.1 (allgemein) | §32d Abs.5 (Kapital) |
|---|---|---|
| Anwendungsbereich | Nicht-Kapital-Einkünfte | §32d-Kapitalerträge |
| Höchstbetrag | Ø-Steuersatz × ausl. Einkünfte | 25% pro Kapitalertrag |
| DBA-Vorrang | Ja (§34c Abs.6) | Ja (Abs.5 S.2) |
| Formel | min(q, HB) im Accessor `catala_p34c_1` | Formel-intern via `q/(4+k)` |

Unsere DBA-Felder (`dba_gezahlte_auslaendische_steuer`, `dba_auslaendische_einkuenfte`, `catala_p34c_1`) sind für §34c-Abs.1-Fälle, NICHT für §32d-Kapital. Der Guard `dba_kapital_offen` (api.py Z.1841-1842) stoppt korrekt, wenn Kapital + DBA gleichzeitig auftreten.

**q=0-Default vertretbar?** Ja — aus zwei Gründen:

1. **Over-tax-safe:** q=0 → e - 0/(4+k) = e → volle Abgeltung → keine KiSt-Korrektur → mehr ESt → **over-tax** (nie under-tax)
2. **Nischen-Fall:** Ausländische Kapitalerträge MIT Quellensteuer UND Antragsveranlagung nach Abs.4 sind selten und erfordern eigenes DBA-Routing für Kapital

**Default q=0 ist vertretbar. Als benannte Lücke dokumentieren** (im Code bereits über `# BENANNTE LÜCKE` in api.py Z.1186-1192 und im Guard `dba_kapital_offen`).

**Zusatz für Zukunft:** Ein Feld `p32d_auslaendische_kapitalsteuer` analog zu `dba_gezahlte_auslaendische_steuer`, aber §32d-spezifisch. Oder Wiederverwendung des DBA-Feldes mit §32d-Kapital-Marker. Nicht in MVP.

---

## 4. Korrekte Formel — Widerlegung meiner ersten Analyse

**Meine erste Analyse (gestrichen):** Behauptete ein Nullsummenspiel zwischen ESt↓ und KiSt↑.
**Main korrigiert (2026-08-06):** S.3 sagt „ermäßigt sich **um 25 Prozent** der auf die Kapitalerträge entfallenden Kirchensteuer" — NICHT um 100%. Netto-Mehrbelastung = 75% der Kapital-KiSt.

### 4.1 Gleichungssystem aus S.1 + S.3

Sei `e` = §20-Einkünfte nach Sparer-PB (kapitaleinkuenfte), `k` = KiSt-Satz (8 oder 9 %).

| Quelle | Gleichung |
|--------|-----------|
| §32d Abs.1 S.1 | Abgeltung = 25% × e |
| §32d Abs.1 S.3 | ESt_k = 25% × e − 25% × KiSt_k |
| §51a | KiSt_k = k% × ESt_k |

Auflösung:

```
ESt_k = 25% × e − 25% × k% × ESt_k
ESt_k × (1 + 25% × k%) = 25% × e
ESt_k = 25% × e / (1 + k/400)          [1]
KiSt_k = ESt_k × k%                     [2]
```

### 4.2 Beispiel (Main, bestätigt am Ring)

40.000 Lohn + 50.000 Kapital (§20-Einkünfte nach Sparer-PB), roem.-kath., NRW (k=9):

```
           heute (Under-tax)    korrekt           Differenz
ESt_kap     12.250,00           11.980,44         −269,56  (S.3-Ermäßigung)
KiSt_kap         0,00            1.078,24       +1.078,24
Summe       12.250,00           13.058,68         +808,68
```

**Heutiger Under-tax: 808,68 EUR** — 75% der KiSt_kap (1.078,24 − 269,56).

Der SolZ auf den Kapitalteil (5,5% ohne Freigrenze) folgt dem korrigierten kap_st → sinkt von 673,75 auf 658,92 EUR → 14,83 EUR weniger SolZ.

### 4.3 Für den Ring: Integer-Formel (CENT, q=0)

Aus [1] mit kap_st_heute = 25% × e × 100 (CENT):

```
e = 4 × kap_st_heute                    (denn kap_st_heute = 25% × e)
ESt_k_korrigiert_cent = kap_st_heute_cent × 400 // (400 + k)
KiSt_kap_cent = ESt_k_korrigiert_cent × k // 100
```

Probe (49.000 EUR Kapital, e=49.000, kap_st_heute=12.250):

```
kap_st_heute_cent = 1.225.000
1.225.000 × 400 // 409 = 1.198.044 cent = 11.980,44 EUR ✓
1.198.044 × 9 // 100 = 107.823 cent = 1.078,23 EUR
                     (exakt 1.078,24 — 1ct Abweichung durch CENT-Floor)
```

### 4.4 Bedingung: NUR Abs.1-Fälle (nicht Günstigerprüfung)

Die Günstigerprüfung (§32d Abs.6) ERSETZT den Abs.1 komplett (Wortlaut: "anstelle der Anwendung der Absätze 1, 3 und 4"). Wenn Abs.6 greift → Kapital im tariflichen zvE → KiSt läuft über §51a (est_mit_fb). Keine e/(4+k)-Korrektur.

Im Ring aktuell: `kap_st = min(abgeltung, delta)`. Zwei Fälle:

| Fall | Bedeutung | KiSt-Korrektur? |
|------|-----------|-----------------|
| abgeltung ≤ delta | Abgeltung greift (oder Gleichstand), Günstigerprüfung nicht beantragt/nicht besser | **JA** — Abs.1-Fall |
| delta < abgeltung | Günstigerprüfung wäre besser — ohne Antrag läuft trotzdem Abs.1 | **Offene Frage** |

**Heute kein Feld „Günstigerprüfung beantragt".** Default = Abs.1 = Korrektur ANWENDEN. Wenn der Steuerpflichtige Abs.6 beantragen würde (und es günstiger ist), wäre die Korrektur falsch — aber das ist ein Folgebaustein (neues Feld nötig).

**Sicherheitsanker:** `kap_st == abgeltung` als Bedingung (nur wenn der Ring tatsächlich den Abgeltungsbetrag verwendet, nicht den gedeckelten delta-Betrag). Wenn delta < abgeltung → kap_st = delta → Bedingung NICHT erfüllt → keine Korrektur. Das verhindert Doppelzählung.

### 4.5 SolZ-Interaktion

SolZ §3 Abs.3 S.1: Kapitaleinkünfte bleiben außer Ansatz (SolZ-Basis = est_mit_fb inkl. Kapital). §3 Abs.3 S.2: Kapital-SolZ = kap_st × 5,5% ohne Freigrenze.

```python
# Heute (api.py Z.1176-1180):
solz_container[0] = catala_solz({
    "bemessungsgrundlage": solz_info["est_mit_fb"],   # = est_raw + kap_st
    "kapital_steuer": solz_info.get("kap_st", 0),     # = kap_st
})
```

Mit Korrektur:
```python
solz_container[0] = catala_solz({
    "bemessungsgrundlage": est_raw + kap_st_korrigiert,  # korrigierte ESt
    "kapital_steuer": kap_st_korrigiert,                  # korrigiert
})
```

**Ergebnis:** Der Kapital-SolZ sinkt (von kap_st × 5,5% auf kap_st_korrigiert × 5,5%), weil die KiSt-Korrektur die ESt-Bemessungsgrundlage senkt.

### 4.6 Interaktion mit §51a-Fix (147bc24)

Keine. Der §51a-Fix betraf `est_mit_fb = solz_info["est_roh_ohne_kap"]` (statt fälschlich kap_st_total). Die Abgeltung-KiSt ist ein SEPARATER Summand in `extras["kist_cent"]`. Der §51a-Teil (auf Nicht-Kapital-ESt) bleibt bei 622,71 EUR (40.000 Lohn, KiSt 9%).

```python
# Heute: extras["kist_cent"] = §51a (est_roh_ohne_kap × k%)
# Neu:  extras["kist_cent"] = §51a + §32d-Abgeltung-KiSt
extras["kist_cent"] = kist_normal + kist_kapital   # zwei separate Summanden
```

### 4.7 Rundungsanalyse

Integer-Arithmetik mit `//` = floor:

```
ESt_k_korrigiert_cent = kap_st_heute_cent × 400 // (400 + k)
KiSt_kap_cent = ESt_k_korrigiert_cent × k // 100
```

Beispiel (e=49.000 EUR, k=9, kap_st=1.225.000 cent):

| Größe | Exakt (cent) | Floor | Abweichung | Richtung |
|-------|-------------|-------|-----------|----------|
| ESt_k_korrigiert | 1.198.044,0098 | 1.198.044 | −0,01 ct | Over-tax (ESt↓ zu wenig) |
| KiSt_kap | 107.823,96 | 107.823 | −0,96 ct | Under-tax (KiSt↑ zu wenig) |
| Netto | — | — | −0,97 ct | Under-tax, < 1 ct |

Der Netto-Floor-Fehler ist < 1 CENT pro Fall. Bei k=8: 400/408 = 0,98039... → analog. **Kein messbarer Effekt.**

Beim zweiten Floor (`KiSt_kap_cent = ESt_k_korrigiert × k // 100`) geht es um 0,xx Cent. Selbst bei 10.000 Fällen < 100 EUR jährlich.

**Vertretbar.** Fail-safe-Richtung: Under-tax (KiSt zu niedrig) → weniger Steuer für KiSt-Empfänger → kein Risiko für den Steuerpflichtigen.

---

## 5. Formel-Änderung (nur api.py, 2 Stellen)

### 5.1 Größen

| Variable | Bedeutung | Quelle |
|----------|-----------|--------|
| `kap_st` | Heutige Kapitalsteuer = min(25%×e, delta), CENT | `catala_kapital_steuer` (unchanged) |
| `k` | KiSt-Satz (8 oder 9), Prozent GANZZAHL | wie `catala_kist`: Konfession+Bundesland |
| `kap_st_k` | Korrigierte ESt auf Kapital, CENT | `kap_st × 400 // (400 + k)` |
| `kist_kapital` | KiSt auf Kapital, CENT | `kap_st_k × k // 100` |
| `abgeltung` | 25% × e (vor Günstigerprüfung), CENT | = 25 × kapitaleinkuenfte_cent ÷ 100 |

Integer-Herleitung aus 4.3:
```
ESt_k = 25% × e / (1 + k/400)
      = (25% × e) × 400 / (400 + k)
      = kap_st × 400 / (400 + k)          (für Abs.1-Fall, wo abgeltung ≤ delta)
```

### 5.2 Bedingung: NUR wenn abgeltung ≤ delta (Abs.1-Fall)

Die Günstigerprüfung ersetzt Abs.1 komplett (§32d Abs.6 S.1: "anstelle der Anwendung der Absätze 1, 3 und 4"). Im Ring ist `kap_st = min(abgeltung, delta)`. Zwei Fälle:

| Bedingung | Bedeutung | Korrektur |
|-----------|-----------|-----------|
| `abgeltung ≤ delta` | Abgeltung greift (oder gleich) | JA — e/(4+k) anwenden |
| `delta < abgeltung` | Günstigerprüfung besser | NEIN — Abs.6 → KiSt über §51a |

**Praktisch:** `kap_st == abgeltung` prüfen. Wenn true → Abs.1-Fall → korrigieren. Wenn false → delta greift → Abs.6 → keine Korrektur.

### 5.3 Code-Änderung gesamt-Ring (api.py Z.1136-1153)

```python
# Heute:
kap_st = runner.catala_kapital_steuer({...})
result = est_raw + kap_st
extras["kist_cent"] = catala_kist({"est_mit_fb": solz_info["est_roh_ohne_kap"], ...})

# Neu — NUR wenn abgeltung ≤ delta:
kap_st = runner.catala_kapital_steuer({...})
abgeltung = (kapitaleinkuenfte * 25) // 100  # 25% × e, CENT
if konfession_kiestpflichtig and abgeltung <= delta:
    k = 8 if bundesland in BAYERN_BW else 9
    kap_st_k = kap_st * 400 // (400 + k)        # ESt auf Kapital mit KiSt-Korrektur
    result = est_raw + kap_st_k
    kist_kapital = kap_st_k * k // 100           # KiSt auf Kapital
    extras["kist_cent"] = kist_normal + kist_kapital
    solz_info["kap_st"] = kap_st_k              # §32d-Kapital-SolZ auf korrigiertem Wert
else:
    result = est_raw + kap_st                   # unverändert (Abs.6 oder keine KiSt)
    extras["kist_cent"] = kist_normal           # nur §51a
    solz_info["kap_st"] = kap_st
```

### 5.4 Code-Änderung rentner-Ring (api.py Z.1620-1646)

Analog. Der rentner-Ring hat `kapital_steuer: 0` hardcodiert (Z.1634: `"kapital_steuer": 0` — MUTATION). Nachbauen: SolZ auf kap_st_korrigiert füttern.

### 5.5 Keine Änderung: AN-Ring

Der AN-Ring hat kein Kapital. `extras["kist_cent"]` dort ist pur §51a.

### 5.6 SolZ-Änderung

Bereits in 4.5 beschrieben: `kapital_steuer` an SolZ = korrigierter Wert. Der Kapital-SolZ sinkt von kap_st × 5,5% auf kap_st_k × 5,5%.

### 5.7 Kein neues Feld für q

q=0 Default. Der Guard `dba_kapital_offen` (api.py Z.1841-1842) prüft `dba_auslaendische_einkuenfte > 0` bei gleichzeitigen Kapital-Einkünften → gibt `grund = "dba_kapital_offen"` → Ring gibt "OFFEN" statt einer Zahl. Dieser Guard GREIFT HIER NICHT: er betrifft §34c-Betriebsstätten/§34c-allgemein mit Kapital, nicht §32d-Kapital. Wenn jemand ausländische Kapitalerträge mit Quellensteuer hat → q=0 → over-tax-safe (ESt zu hoch).

**Docstring-Auflage:** In der `# BENANNTE LÜCKE` api.py Z.1186-1192 vermerken: "q=0 Default → over-tax-safe bei ausländischer Quellensteuer auf Kapitalerträge."

---

## 6. Auflagen vor Bau (dev-b gibt Signal)

Main hat 5 Auflagen:

### 6.1 Test ZUERST rot

Test muss zeigen: 40.000 Lohn + 50.000 Kapital, roem.-kath. NRW → KiSt heute 622,71 (falsch), korrekt höher. Rot laufen lassen VOR der Code-Änderung.

```python
# api.py-Lesetest (in tests/test_p32d_kist_kapital.py o.ä.):
# Seed + /ergebnis → kist_cent == 62271 (622,71 EUR)
# Nach Fix: kist_cent == 62271 + 107824 = 170095 (1.700,95 EUR)
```

### 6.2 §51a-Fix bleibt unberührt

Nicht-Kapital-KiSt bei 40.000 Lohn + 50.000 Kapital = 622,71 EUR (est_roh_ohne_kap × 9%). Die Abgeltung-KiSt (1.078,24 EUR) ist SEPARATER Summand, additiv in extras["kist_cent"].

Test: `extras["kist_cent"]` = `§51a-Teil` + `§32d-Teil` (zwei Summanden, nicht verschmolzen).

### 6.3 BEIDE Seiten: ESt-Ermäßigung + KiSt-Erhöhung

Nur KiSt bauen → 1.078,24 EUR mehr KiSt, aber ESt bleibt bei 19.169,00 → Over-tax (269,56 EUR). Die ESt muss auf 11.980,44 sinken (Kapitalteil) bzw. gesamte ESt = est_raw + kap_st_k = ... . Sonst ist der Saldo falsch.

Test: `zahl_cent` sinkt um den ESt-Ermäßigungsbetrag (269,56 EUR bei 40k+50k).

### 6.4 q=0-Default mit Docstring

In der benannten Lücke dokumentieren: q=0 führt bei ausländischen Kapitalerträgen MIT Quellensteuer zu over-tax. Guard `dba_kapital_offen` ist korrekt, aber er blockiert den GESAMTEN Bescheid (grund="dba_kapital_offen"), nicht nur die KiSt-Korrektur → separater Mechanismus. Für MVP reicht die Warnung.

### 6.5 Rundung analysiert

Siehe 4.7: Floor-Fehler < 1 Cent pro Einzelfall (kap_st=1.225.000 cent → Netto-Fehler 0,97 ct Under-tax). Unbedenklich.

---

## 7. Empfehlung (aktualisiert)

**Bauen, aber erst nach dev-b Signal.** Main gibt Bescheid, wenn die _bescheid_fn-Refactor-Zweige (api.py extras) fertig sind.

Bau-Umfang unverändert: ~1h, nur api.py (2 Stellen gesamt+rentner), keine neuen Felder, keine Bindungs-Änderung.

Fehler-Größenordnung: ~809 EUR Under-tax pro KiSt-pflichtigem mit 50.000 EUR Kapital (40.000 Lohn). Vergleichbar mit den beiden heute gefixten KiSt-Bugs (147bc24, f44dac2).

### Messaufbau für Main (klar zum Fahren)

| Fall | Seeds | KiSt heute | KiSt korrekt | Diff |
|------|-------|-----------|-------------|------|
| A: 40k Lohn, kein Kapital | AN_KEGEL | 622,71 | 622,71 | 0 (Kontrolle) |
| B: 40k Lohn + 50k Kapital, KiSt | GESAMT_KEGEL + kap=5.000.000 | 622,71 | 1.700,95 | +1.078,24 KiSt, −269,56 ESt |
| C: 40k Lohn + 50k Kapital, keine KiSt | GESAMT_KEGEL + kap=5.000.000, konfession=keine | 0 | 0 | 0 |
| D: 10k Lohn + 50k Kapital (delta<abgeltung) | GESAMT_KEGEL mod. | §51a nur | §51a nur | 0 (Günstigerprüfung) |

D ist schwer zu messen weil delta<abgeltung bei niedrigem zvE → niedrige tarifliche ESt → hoher Grenzsteuersatz möglich. Main: bitte erst B fahren (klarster Fall), dann D falls gewünscht.