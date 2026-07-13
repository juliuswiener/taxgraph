# Charge-6-Zuschnitt: Kapitalvermögen (§ 20 + § 32d, Anlage KAP)

Erste NEUE Einkunftsart nach dem AN-Kern. Stufe A, $0. Freezes vom Instructor (sha verifiziert):
`estg_p20_2026-07-13.txt` (026bd349), `estg_p32d_2026-07-13.txt` (6ad58bb5). § 43a (KapESt-
Einbehalt) bewusst draussen (Erhebungsmechanik ≠ Deklaration; Anrechnung = p36).

Vorschlag: **drei Teilregeln** (Sparer-PB · Abgeltung/Günstiger · Verlustverrechnung).

## (A) p20_9_sparer_pauschbetrag — Sparer-Pauschbetrag (§ 20 Abs. 9)

Wortlaut: „Sparer-Pauschbetrag … 1 000 [Euro] … Ehegatten, die zusammen veranlagt werden, … ein
gemeinsamer Sparer-Pauschbetrag von 2 000 [Euro]"; „der Abzug der tatsächlichen Werbungskosten ist
ausgeschlossen".

Rechenkern: `einkuenfte_nach_sparer_pb = max(0, kapitalertraege − sparer_pb)`, wobei `sparer_pb =
if zusammenveranlagung then 2.000 else 1.000`. Signatur: `kapitalertraege money,
zusammenveranlagung bool -> einkuenfte_nach_sparer_pb money`.
- 1.000/2.000 = Norm-Konstanten im Wortlaut (Konstanten-Doktrin, kein Input; Werte aus § 20-auszug).
- `zusammenveranlagung` = bool-INPUT (Sachverhalt, nicht Konstante — p32_6-Muster: die Verdopplung
  hängt am Veranlagungs-Sachverhalt, den der Aufrufer legitim setzt). Entscheidung: bool-Input, weil
  es eine Sachverhalts-Verzweigung ist (anders als der feste Betrag).
- WK-Ausschluss = Anwendbarkeit/Scope (der Pauschbetrag ersetzt die tatsächlichen WK).
Seeds: kapitalertraege 3.000, einzeln → 3.000−1.000 = 2.000; kapitalertraege 3.000, zusammen →
3.000−2.000 = 1.000; kapitalertraege 800, einzeln → max(0, 800−1.000) = 0; kapitalertraege 5.000,
zusammen → 3.000.

## (B) p32d_1_abgeltung — 25-%-Abgeltung + Günstigerprüfung (§ 32d Abs. 1 + 6)

Wortlaut Abs. 1 S. 1: „Die Einkommensteuer für Einkünfte aus Kapitalvermögen … beträgt 25 Prozent."
Abs. 6: „Auf Antrag … werden … die nach § 20 ermittelten Kapitaleinkünfte den Einkünften im Sinne
des § 2 hinzugerechnet und der tariflichen Einkommensteuer unterworfen, wenn dies zu einer
niedrige[ren Steuer führt]" (Günstigerprüfung).

**ZWEI TARIF-WELTEN als § 31-Andockung** (KEIN Selbst-Tarif): die tariflichen Vergleichswerte kommen
als Inputs aus der § 2-Integration.
Rechenkern:
```
abgeltung        = 0,25 × kapitaleinkuenfte
guenstiger_delta = est_regulaer_mit_kap − est_regulaer_ohne_kap   # persönliche Steuer AUF die KapEink
kapital_steuer   = min(abgeltung, guenstiger_delta)              # Günstigerprüfung Abs. 6
```
Signatur: `kapitaleinkuenfte money, est_regulaer_mit_kap money, est_regulaer_ohne_kap money ->
kapital_steuer money`. Präzision: 0,25 in decimal, Cent-Schnitt zuletzt (praezisions_lint).

**PFLICHT-ENTSCHEID Kirchensteuer (§ 32d Abs. 1 S. 3):** bei KiSt-Pflicht ermäßigt sich der Satz
(e/(4+k)-Mechanik). Entscheidung:
- **(a) MVP-Linie [gewählt, Instructor-Default]:** glatter 25 %-Satz + Geltungsbedingung
  `keine_kirchensteuer_auf_kapitalertraege` — die Bedingung DEKLARIERT die Grenze ehrlich (der
  KiSt-Fall ist ausgeklammert, nicht still falsch gerechnet). Keine Bruch-Arithmetik, kein Klasse-5-
  Risiko.
- **(b) benannter Nachtrag:** die e/(4+k)-Formel geschlossen mitformalisieren (evtl. selbsttragend,
  aber Bruch-Arithmetik → Klasse-5-Präzisionsrisiko, eigener Zuschnitt mit Präzisions-Wächtern).
Seeds: kapitaleink. 10.000, est_mit 8.000, est_ohne 5.000 → abgeltung 2.500, delta 3.000 → min 2.500
(Abgeltung günstiger); kapitaleink. 10.000, est_mit 6.500, est_ohne 5.000 → abgeltung 2.500, delta
1.500 → min 1.500 (Günstigerprüfung greift, persönlicher Satz 15 %); kapitaleink. 0 → 0.

## (C) p20_6_verlustverrechnung — Verlust-Töpfe (§ 20 Abs. 6)

Wortlaut: „Verluste aus Kapitalvermögen dürfen nicht mit Einkünften aus anderen Einkunftsarten
ausgeglichen werden" (S. 1); „Verluste … aus der Veräußerung von Aktien … dürfen nur mit Gewinnen …
[aus Aktien] … ausgeglichen werden" (Aktien-Sondertopf).

Zwei getrennte Töpfe:
```
aktien_netto   = max(0, gewinn_aktien − verlust_aktien)          # Aktienverlust NUR gegen Aktiengewinn
sonstige_netto = max(0, gewinn_sonstige − verlust_sonstige)      # sonstiger KAP-Topf
kapitaleinkuenfte = aktien_netto + sonstige_netto
```
Signatur: `gewinn_aktien, verlust_aktien, gewinn_sonstige, verlust_sonstige (money) ->
verrechnete_kapitaleinkuenfte money`. Bedingungen: `aktienverlust_nur_gegen_aktiengewinn` (eigener
Topf), `kein_ausgleich_mit_anderen_einkunftsarten`. **Verlustvortrag** (jahresübergreifend, S. 2
„mindern jedoch die Einkünfte … in folgenden VZ") = § 10d-Backlog-Grenze (dokumentiert, NICHT
formalisiert — mehrjährig, wie § 10b-Vortrag). Konditional-Töpfe → hinweis-Kandidat (Topf-Trennung).
Seeds: gewinn_aktien 5.000/verlust_aktien 2.000/sonstige 0/0 → 3.000; verlust_aktien 8.000 >
gewinn 5.000 → aktien_netto 0 (Rest = Vortrag, außerhalb); gewinn_sonstige 4.000/verlust_sonstige
1.000 → 3.000; Aktienverlust NICHT gegen sonstige (Topf-Wächter): gewinn_sonstige 4.000,
verlust_aktien 3.000, gewinn_aktien 0 → sonstige bleibt 4.000, aktien 0.

## Scope-Grenzen (dokumentiert)
- Verlustvortrag (§ 20 Abs. 6 S. 2ff, § 10d) = mehrjährig, Backlog.
- KiSt-Ermäßigung (§ 32d Abs. 1 S. 3) = Nachtrag (b).
- Ausländische Quellensteuer-Anrechnung (§ 32d Abs. 5) = eigener Zuschnitt.
- KapESt-Einbehalt (§ 43/§ 43a) = Erhebung, nicht Deklaration.
- Teileinkünfteverfahren/§ 32d Abs. 2 (Sonderfälle unternehmerische Beteiligung) = außerhalb AN-nah.

## Nächste Schritte
1. Instructor-Review (3-Teilregel-Split, KiSt-Entscheid (a), bool-Input zusammenveranlagung,
   Verlust-Topf-Scope).
2. Nach Freigabe: Signaturen + Seeds in `rules.yaml`; Stufe B via skip-judge (deepinfra down,
   /endpoints-Status je Lauf mitloggen).
3. Landkarte: KAP von ⬜ → ✅ (andere Einkunftsart 1/4). Reihenfolge im Lauf: (A) + (C) selbsttragend
   zuerst, (B) mit Andock-Inputs + ggf. hinweis.
