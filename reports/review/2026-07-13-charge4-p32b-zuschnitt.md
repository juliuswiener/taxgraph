# Charge-4-Zuschnitt: § 32b Progressionsvorbehalt

Erster Vollabdeckungs-Zuschnitt (Coverage-Landkarte, Charge 4). Stufe A, $0. Instructor-GO
2026-07-13. Ziel: der Lohnersatz-Progressionsvorbehalt — sehr häufig im AN-Fall, tarif-nah,
klarer Wortlaut.

## Rechtsinhalt (§ 32b Abs. 1 + 2 EStG)

Bezieht ein Steuerpflichtiger bestimmte steuerfreie Lohnersatz-/Einkommensersatzleistungen
(ALG I, Eltern-, Kranken-, Mutterschafts-, Insolvenz-, Kurzarbeitergeld u. a.), so sind diese
selbst steuerfrei, erhöhen aber den **Steuersatz** auf das übrige zu versteuernde Einkommen
(Progressionsvorbehalt).

Rechenkern (§ 32b Abs. 2 S. 1 Nr. 1): auf das zu versteuernde Einkommen wird ein **besonderer
Steuersatz** angewendet:

```
besonderer_steuersatz = ESt(zvE + Progressionseinkünfte) / (zvE + Progressionseinkünfte)
festzusetzende_ESt    = besonderer_steuersatz × zvE
```

d. h. der Durchschnittssteuersatz, der sich bei Hinzurechnung der Lohnersatzleistungen ergäbe,
wird auf das tatsächliche zvE (ohne die steuerfreien Leistungen) angewandt.

## Andockung — Integrations-Muster wie § 31 (KEIN Selbst-Tarif)

§ 32b ruft den § 32a-Tarif NICHT selbst. Wie `p31_familienleistungsausgleich` (dessen
Geltungsbedingung `tarifwerte_sind_p32a_ergebnis`: „est_ohne/est_mit sind Tarifergebnisse (p32a)")
bekommt die Regel die **Tarif-Ergebnisse als Inputs** aus der § 2-Integration und macht nur die
§ 32b-spezifische Arithmetik. Das hält die Regel deterministisch prüfbar und vermeidet eine
zweite Tarif-Implementierung (Rechtsprädikat-Muster).

Die § 2-Integrationsschicht (arbeitnehmerfall/p32a) berechnet `est_auf_erhoehte_bmg` =
Tarif(zvE + Progressionseinkünfte) und reicht sie zusammen mit zvE und der Summe der
Progressionseinkünfte hinein.

## Signatur-Vorschlag

```
inputs:
  zu_versteuerndes_einkommen        money   # zvE OHNE die steuerfreien Leistungen (§ 2-Ergebnis)
  progressionseinkuenfte            money   # Summe der Lohnersatzleistungen (Kz s. u.)
  est_auf_erhoehte_bemessung        money   # Tarif(zvE + Progressionseinkuenfte), aus p32a (Andockung)
output:
  festzusetzende_est_mit_progression  money
internal:
  erhoehte_bemessung  = zvE + progressionseinkuenfte
  besonderer_steuersatz (decimal) = est_auf_erhoehte_bemessung / erhoehte_bemessung
  # festzusetzende = besonderer_steuersatz × zvE
```

Präzisions-Hinweis (Klasse 5): der besondere Steuersatz ist ein `decimal`-Quotient; die
Multiplikation mit zvE erfolgt in `decimal`, erst am Ende Cent-Schnitt — money×decimal-Rundung
darf NICHT vor dem finalen Schnitt greifen (praezisions_lint wird das automatisch prüfen). Der
Steuersatz wird amtlich auf mehrere Nachkommastellen bestimmt (§ 32b: „auf vier Dezimalstellen"
— beim Freeze verifizieren, Stufe B).

Randfall: `erhoehte_bemessung <= 0` → besonderer Steuersatz 0 (kein Progressionsvorbehalt auf
negativer/null Bemessung); Geltungsbedingung.

## Lohnersatz-Kz — liegen bereits in den eingefrorenen Vordrucken (Instructor-Hunch bestätigt)

Kein neuer Vordruck-Freeze nötig, die Kennzahlen sind schon da:

| Quelle (eingefroren) | Zeile | Kz | Inhalt |
|---|---|---|---|
| `est1a_2025.txt:150` | Mantel Z. 35 | **120 / 121** | Einkommensersatzleistungen mit Progressionsvorbehalt (ALG, Eltern-, Insolvenz-, Kranken-, Mutterschaftsgeld, Verdienstausfallentsch.) — Person A / B |
| `est1a_2025.txt:154` | Mantel Z. 36 | **136 / 137** | vergleichbare Leistungen aus EU-/EWR-Staat / Schweiz |
| `anlage_n_2025.txt:79` | Anlage N Z. 20 | **119** | Kurzarbeitergeld, Zuschuss Mutterschaftsgeld, Verdienstausfallentsch., Aufstockungsbeträge Altersteilzeit, Qualifizierungsgeld (LSt-Besch. Nr. 15) |

`progressionseinkuenfte` = Summe(120/121 + 136/137 + 119). Der Mantel-Kz-120-Hinweis „ohne
Beträge laut Zeile 20 der Anlage N" bestätigt die Additivität ohne Doppelzählung.

## Quellen-Freeze — der EINE gated Schritt (Download-Boundary)

`§ 32b` ist noch nicht lokal eingefroren. Der Freeze läuft über `scripts/freeze_source.py`, das
per `urllib` von gesetze-im-internet **fetcht** — ein Netz-Download. Nach meiner Boundary
(ausgehende/Download-Aktionen nur auf Julius' direktes Wort, nicht auf Kanal-Ansage) hole ich
dafür Julius' Freigabe.

- **Quelle:** `https://www.gesetze-im-internet.de/estg/__32b.html` (amtliches Gesetzesportal)
- **Was:** § 32b EStG, ganzer Paragraph (Abs. 1–4), Text ~1 Anlage-N-Seite
- **Womit:** bestehendes `scripts/freeze_source.py --url … --start … --ende …` (dasselbe Werkzeug
  wie für die ~40 bereits eingefrorenen §§)
- **Zitatanker (nach Freeze zu verifizieren):** „besonderer Steuersatz"; „ein Zwölftel" trifft
  hier nicht — für Abs. 2 Nr. 1 der Wortlaut zur Durchschnittssatz-Bildung + die Vier-Dezimal-
  stellen-Regel.

Ohne den Freeze bleibt der `quellen`-Block (verbatim zitatanker/auszug) offen — alles andere
(Signatur, Andockung, Kz, Rechenlogik, Seeds) steht.

## Wächter-Seeds (Vorschlag, Rechenweg belegt — Erwartungswerte nach Freeze final)

Erwartungswerte gegen den p32a-Tarif (VZ 2026) zu rechnen, sobald die Signatur steht. Skizze:

| Fall | zvE | Progr.-Eink. | Rechenweg | erwartet |
|---|---|---|---|---|
| Standard | 30.000 | 10.000 | Satz = ESt(40.000)/40.000; fest = Satz × 30.000 | Satz×30k |
| kein Lohnersatz | 30.000 | 0 | Satz = ESt(30.000)/30.000 → = normaler Durchschnittssatz; fest = normale ESt | = ESt(30.000) |
| Bemessung 0 | 0 | 5.000 | erhoehte_bemessung > 0, aber zvE 0 → fest 0 | 0,00 |

Der „kein Lohnersatz"-Fall ist der Wächter, dass § 32b bei 0 Progressionseinkünften exakt die
normale ESt reproduziert (keine Verzerrung). Konkrete Euro-Werte trägt der clerk-Test gegen den
p32a-Tarif nach dem Freeze.

## Schätzung Stufe B (Freigabe über Instructor)

Ein Standard-Doppelformalisierungs-Lauf (A+B + Gates + ggf. Judge) liegt erfahrungsgemäß bei
~$0,03–0,08 (skip-judge) bzw. ~$0,1–0,5 mit Judge, plus evtl. ein Nachlauf bei Run-Varianz.
**Schätzung: ~$0,15–0,40** für den vollen Zuschnitt inkl. Judge, deutlich unter dem Charge-Muster
2–5 USD. Kleiner Zuschnitt (3 Inputs, 1 Output, klare Arithmetik).

## Nächste Schritte
1. **Julius-Freigabe für den § 32b-Freeze** (Netz-Fetch, s. o.) — der einzige gated Schritt.
2. Nach Freeze: `quellen`-Block mit verbatim Zitatanker + Vier-Dezimalstellen-Regel; Signatur in
   `rules.yaml`; Seeds gegen p32a-Tarif final rechnen.
3. Stufe B: Doppelformalisierung + Gates (Instructor-Freigabe je Lauf).
4. Coverage-Landkarte aktualisieren: § 32b von ⬜ auf ✅, %-Zahl in den Chargen-Report.

## Addendum — Freeze verifiziert + Wortlaut-Befunde (2026-07-13)

Der § 32b-Freeze wurde vom Instructor gefahren (Julius-Delegation dessen Session; ich habe
nichts heruntergeladen, meine Download-Boundary bleibt unberührt). Selbst verifiziert
(falsches-gruen): `sources/gesetze-im-internet/estg_p32b_2026-07-13.txt`, 8970 B, **sha256
1bf7e924…** (Claim bestätigt), enthält den echten § 32b-Wortlaut (Progressionsvorbehalt,
besonderer Steuersatz, Lohnersatz-Katalog). `make sources-check` grün.

### Befund 1 — Abs. 2 Nr. 1: AN-Pauschbetrag mindert die Progressionseinkünfte
Wortlaut Abs. 2 S. 1: „Der besondere Steuersatz … ist der Steuersatz, der sich ergibt, wenn …
das nach § 32a Absatz 1 zu versteuernde Einkommen vermehrt oder vermindert wird um 1. … die
**Summe der Leistungen nach Abzug des Arbeitnehmer-Pauschbetrags (§ 9a Satz 1 Nummer 1), soweit
er nicht bei der Ermittlung der Einkünfte aus nichtselbständiger Arbeit abziehbar ist**".

→ Die augmentierte Bemessung ist NICHT zvE + Roh-Leistungssumme, sondern zvE + (Summe −
Rest-AN-Pauschbetrag). Der § 9a-Pauschbetrag (2026: 1.230 €) mindert die Progressionseinkünfte,
soweit er nicht schon gegen den regulären Arbeitslohn verbraucht ist. Konsequenz für die
Signatur (Andockung wie § 31): die § 2-Integration reicht die **bereits netto gerechneten**
`progressionseinkuenfte` (Summe − unverbrauchter Pauschbetrag) herein; die Regel trägt den
Wortlaut-Anker „nach Abzug des Arbeitnehmer-Pauschbetrags" als Geltungsbedingung
(`progressionseinkuenfte_sind_netto_nach_an_pauschbetrag`).

### Befund 2 — Vier-Dezimalstellen-Regel ist KEIN Gesetzestext → Rundungs-Entscheid nötig
`grep "Dezimalstell" §32b-Freeze` = 0; `grep "dezimalstell|steuersatz…rund"` über die
eingefrorenen Anleitungen = 0. Die 4-Dezimalstellen-Rundung des besonderen Steuersatzes ist eine
**Verwaltungs-/Berechnungskonvention** (amtliche Programmablaufpläne PAP zur maschinellen
Lohn-/Einkommensteuer, R 32b EStR), NICHT § 32b-Wortlaut. Sie darf daher nicht als gesetzlich
deklarierte Rundung ins Manifest. Zwei saubere Wege — **Instructor/Julius-Entscheid**:

- **(b1) EMPFEHLUNG — reine Wortlaut-Formalisierung, keine Satz-Rundung.** Der besondere
  Steuersatz bleibt voller `decimal`-Quotient, `× zvE`, Cent-Schnitt zuletzt. Deckt sich exakt
  mit dem § 32b-Wortlaut (der KEINE Satz-Rundung anordnet). Grenzfall-Notiz dokumentiert: die
  amtliche PAP-4-Dezimal-Rundung wird bewusst NICHT repliziert → mögliche Cent-Divergenz zur
  ELSTER/checkESt-Berechnung; Wiedervorlage, wenn der PAP als `typ:verwaltung` eingefroren ist.
  Vorteil: rein Wortlaut-gedeckt, kein Raten, `rundungs_lint` sauber (keine undeklarierte Rundung,
  weil gar nicht gerundet).
- **(b2) Alternative — 4-Dezimal-Rundung als deklarierte Konvention.** Rundung ins Manifest mit
  Begründung „amtliche Berechnungspraxis (PAP § 32b), verwaltung-Quelle folgt" + Grenzfall-Notiz.
  Matcht die amtlichen Euro-Werte, aber die deklarierte Rundung hat (noch) keinen eingefrorenen
  Quell-Anker — ein „Quelle folgt"-Zustand, den wir bisher vermieden haben.

Der Unterschied ist ein Cent-Effekt am Endbetrag. **Empfehlung (b1):** erst das reine
Wortlaut-§32b grün, die PAP-Rundung als eigener, sauber quellenbelegter Nachtrag (eigener kleiner
Backlog: PAP freezen → Rundung deklarieren → Seeds auf amtliche Werte nachziehen). So bleibt jede
Zahl im Manifest quellenbelegt.

### Verbatim-Zitatanker (aus dem Freeze, für den quellen-Block)
- Abs. 1 S. 1: „ein besonderer Steuersatz anzuwenden" (Progressionsvorbehalt-Grundsatz).
- Abs. 1 Nr. 1 a: „Arbeitslosengeld, Teilarbeitslosengeld … Kurzarbeitergeld, Insolvenzgeld" +
  Nr. 1 j „Elterngeld nach dem Bundeselterngeld- und Elternzeitgesetz" (Lohnersatz-Katalog).
- Abs. 2 S. 1 Nr. 1: „die Summe der Leistungen nach Abzug des Arbeitnehmer-Pauschbetrags"
  (Bemessungs-Ermittlung + Pauschbetrag-Abzug).

### Blockade-Status
Der Rundungs-Entscheid (b1/b2) ist die offene Frage VOR den Seeds — die Euro-Erwartungswerte
hängen daran (Instructor-Auflage). Signatur, Andockung, Kz, Quellen-Anker stehen. Nach dem
Entscheid: Seeds mit exaktem Rechenweg (inkl. gewählter Satz-Behandlung) → Stufe B.

## Stufe B — Läufe (2026-07-13)

Rundungs-Entscheid: **(b1)** (Instructor) — reines Wortlaut, keine Satz-Rundung, `rundung: []`.
Seeds gegen p32a VZ2026 validiert (Formel gegen bekannte Testwerte 28734→3862, 58734→13747
geprüft): A 5.406,75 · B(PE=0) 4.217,00 · C(Anhebung) 290,00.

- **Lauf 1** ($0,0538, Judge lief sauber): `equivalence=FAIL`, `clerk=FAIL`. catala_a zog den
  AN-Pauschbetrag (1.230) INNERHALB der Regel ab (Doppelzählung — unser Input ist bereits netto),
  catala_b korrekt. Kontext-Hunger wie GWG: der auszug „nach Abzug des Arbeitnehmer-Pauschbetrags"
  erreicht den Formalisierer, die netto-Geltungsbedingung nur den Judge. Dreifach geflaggt
  (clerk + equivalence + Judge-Discovery).
- **Fix:** `hinweis` pinnt die netto-Semantik an den Formalisierer (Instructor-Freigabe).
- **Lauf 2** ($0,0427): **alle Gates grün** — `equivalence=PASS`, `clerk=PASS` (3/3),
  Judge-Gates PASS. catala_a rechnet jetzt `erhoehte = zvE + progressionseinkuenfte` (kein
  Pauschbetrag-Abzug), `satz = est/erhoehte`, `festzus = satz × zvE`. A ≡ B.
- **Zustand:** `discovery_triage` (backlog +4). Die Judge-Discoveries betreffen unsere eigenen
  dokumentierten Design-Entscheidungen (netto-Semantik, p32a-Andockung, b1-Rundung) + § 32b-
  norm_teil-Referenzen — Standard-Triage, kein Gate-Defekt. verified_bedingt zieht nach der
  Julius-Triage.

Landkarte: § 32b von ⬜ auf 🟠 (deterministisch grün, Triage offen) — auf ✅, sobald die Triage
durch ist und die Ratsche schließt.
