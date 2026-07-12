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
