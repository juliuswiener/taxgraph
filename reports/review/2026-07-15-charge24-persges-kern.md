# Charge 24 — PersGes-Kern: Mitunternehmer-Einkünfte + §15a Verlustausgleich (Zuschnitt, Stufe A, 2026-07-15)

W3 Personengesellschaften, Zeilen P1/P2/P3 (Landkarte `bilanz-persges-landkarte`). Erste PersGes-
Charge. Quellen: `estg_p15_2026-07-14` (§ 15 Abs. 1 Nr. 2), `estg_p15a_2026-07-14` (§ 15a Abs. 1/2).
**3 Regeln.** Kein Stufe-B ohne Cap-Wort. Alle Zitatanker VOLL-Länge via `_normalize` verifiziert
(je Anker `OK (n Zeichen)`).

**ENGER ZUSCHNITT:** § 15 ist ein Monster (Abs. 1–4, Gewerbebetriebs-Definition, Abfärbung, Prägung,
Verlustverrechnungs-Sondertöpfe). Charge 24 formalisiert NUR den Mitunternehmer-Zurechnungskern
(Abs. 1 Nr. 2 S. 1) + den §-15a-Verlustausgleichs-Grundmechanismus (Abs. 1 S. 1 / Abs. 2 S. 1). Alles
andere = benannte Nachträge.

## Sondersatz-Sweep (verbatim Freeze-Grep)

| # | Fundstelle | Konstruktion | Konsequenz |
|---|---|---|---|
| S1 | § 15 Abs. 1 Nr. 2 S. 1 | „Gewinnanteile … und die Vergütungen … für seine Tätigkeit … für die Hingabe von Darlehen … für die Überlassung von Wirtschaftsgütern" | Additive Zusammenrechnung: Gesamthand-Gewinnanteil **+ drei Sondervergütungs-Kategorien**. |
| S2 | § 15a Abs. 1 S. 1 | **„soweit ein negatives Kapitalkonto … entsteht oder sich erhöht"** | Verlustausgleich nur bis Höhe des positiven Kapitalkontos → `min`/Floor-Mechanik. Doppel-Boundary „entsteht" (KK ≥ 0 → < 0) UND „erhöht sich" (KK schon < 0). |
| S3 | § 15a Abs. 1 S. 1 | **„soweit"** (nicht „wenn") | GRADUELL, nicht binär: der ausgleichsfähige Teil ist der Betrag bis zum Kapitalkonto, der Rest verrechenbar → Klasse-2-artige Freigrenzen-/Boundary-Mechanik (kein reiner bool). |
| S4 | § 15a Abs. 2 S. 1 | „mindert er die Gewinne, die dem Kommanditisten in späteren Wirtschaftsjahren … zuzurechnen sind" | Verrechenbarer Verlust = Rest; nur mit KÜNFTIGEN Gewinnen derselben Beteiligung (Andockung/Nachtrag). |

## Regel 1 — § 15 Abs. 1 Nr. 2 S. 1: Mitunternehmer-Einkünfte (`p15_1_2_mitunternehmer_einkuenfte`)

**Wortlaut (Zitatanker `die Vergütungen, die der Gesellschafter von der Gesellschaft für seine
Tätigkeit im Dienst der Gesellschaft oder für die Hingabe von Darlehen oder für die Überlassung von
Wirtschaftsgütern bezogen hat`, 200 Zeichen voll-verifiziert; Kopf-Anker `die Gewinnanteile der
Gesellschafter einer Offenen Handelsgesellschaft, einer Kommanditgesellschaft`, 99 Zeichen):**
Die mitunternehmerischen Einkünfte = Gewinnanteil (Gesamthand) + Vergütungen für (a) Tätigkeit,
(b) Darlehen, (c) Überlassung von Wirtschaftsgütern.

- **Signatur** `MitunternehmerEinkuenfte`: `gewinnanteil: money` (Gesamthand-Anteil, kann Verlustanteil
  = negativ sein), `verguetung_taetigkeit: money`, `verguetung_darlehen: money`,
  `verguetung_ueberlassung: money` → `einkuenfte_mitunternehmer: money`.
- **Rechenkern:** `einkuenfte_mitunternehmer = gewinnanteil + verguetung_taetigkeit +
  verguetung_darlehen + verguetung_ueberlassung`. Reine additive Zusammenrechnung (Gesamthand +
  Sonderbereich), kann negativ sein (Verlustanteil überwiegt). Cent-genau.
- **Geltungsbedingungen:** `mitunternehmerstellung` (Gesellschafter als Unternehmer/Mitunternehmer
  anzusehen — Sachverhalts-Voraussetzung, Abs. 1 Nr. 2 S. 1), `sonderverguetungen_drei_kategorien`
  (die drei benannten Kategorien Tätigkeit/Darlehen/WG-Überlassung; andere Vergütungen ≠ Sonder-
  vergütung), `gewinnanteil_gesonderte_feststellung_input` (der Gesamthand-Gewinnanteil kommt aus der
  gesonderten+einheitlichen Feststellung als Input, hier nicht selbst ermittelt).
- **Seeds:** (Gewinnanteil 10000, Tät 5000, Darl 2000, Überl 3000) → 20000 · **(Verlustanteil −8000,
  Tät 5000, 0, 0) → −3000 (negativ möglich — Sondervergütung ist trotz Gesamthand-Verlust
  Gewinneinkunft)** · (10000, 0, 0, 0) → 10000 (nur Gewinnanteil) · (0, 0, 0, 0) → 0.

## Regel 2 — § 15a Abs. 1 S. 1: ausgleichsfähiger Verlust (`p15a_1_ausgleichsfaehiger_verlust`)

**Wortlaut (Zitatanker `darf weder mit anderen Einkünften aus Gewerbebetrieb noch mit Einkünften aus
anderen Einkunftsarten ausgeglichen werden, soweit ein negatives Kapitalkonto des Kommanditisten
entsteht oder sich erhöht`, 199 Zeichen voll-verifiziert):** Der Verlustanteil ist nur ausgleichsfähig,
SOWEIT kein negatives Kapitalkonto entsteht/sich erhöht — also nur bis zur Höhe des positiven
Kapitalkontos.

- **Signatur** `AusgleichsfaehigerVerlust`: `verlustanteil: money` (positiver Betrag des Verlustanteils;
  Vorzeichen setzt die Integration), `kapitalkonto: money` (steuerliches Kapitalkonto VOR Verlust,
  **signiert — kann bereits negativ sein**) → `ausgleichsfaehiger_verlust: money`.
- **Rechenkern:** `ausgleichsfaehiger_verlust = min(verlustanteil; max(kapitalkonto; 0 €))`. `max(…;0)`
  floort ein bereits negatives Kapitalkonto auf 0 (dann nichts ausgleichsfähig), `min` kappt den
  ausgleichsfähigen Verlust auf das positive Kapitalkonto.
- **⚠ Klasse-2 (Boundary, Landkarten-Warnung):** die Doppel-Bedingung „entsteht ODER sich erhöht"
  ist KEIN Split, sondern ein **Floor+Cap-Encoding** (`max(kapitalkonto;0)` deckt beide Zweige:
  „entsteht" = KK ≥ 0 wird < 0; „erhöht sich" = KK schon < 0). Encoding-hinweis ZUERST
  (`klasse2-encoding-hinweis-leiter`), Split nur Fallback. `soweit` = graduell → `min`, nicht bool.
- **Geltungsbedingungen:** `kapitalkonto_steuerlich_input` (steuerliches Kapitalkonto = gesonderte
  Feststellung, Sachverhalts-Input, signiert), `kommanditist_haftungsbeschraenkt` (§ 15a gilt für
  Kommanditisten/vergleichbar Haftungsbeschränkte; erweiterte Außenhaftung § 171 Abs. 1 HGB = Abs. 1
  S. 2/3 = Nachtrag), `boundary_entsteht_oder_erhoeht` (`max(kapitalkonto;0)`-Floor deckt beide
  Wortlaut-Zweige; „soweit" = graduell).
- **Seeds (KK=0-Wächter, beidseitig):** (Verlustanteil 6000, KK 10000) → 6000 (voll ausgleichsfähig,
  KK bleibt +4000) · (15000, 10000) → 10000 (bis KK; 5000 macht KK negativ → verrechenbar) ·
  **(5000, 0) → 0 (WÄCHTER KK=0: nichts ausgleichsfähig, ganzer Verlust verrechenbar)** ·
  **(4000, −3000) → 0 (WÄCHTER neg. KK „erhöht sich": nichts ausgleichsfähig)** ·
  **(10000, 10000) → 10000 (GRENZFALL exakt KK: alles ausgleichsfähig, KK → 0)**.

## Regel 3 — § 15a Abs. 1/2: verrechenbarer Verlust (`p15a_1_verrechenbarer_verlust`)

**Wortlaut (Zitatanker `mindert er die Gewinne, die dem Kommanditisten in späteren Wirtschaftsjahren
aus seiner Beteiligung an der Kommanditgesellschaft zuzurechnen sind`, 145 Zeichen voll-verifiziert):**
Der nicht ausgleichsfähige Rest ist verrechenbarer Verlust — nur mit künftigen Gewinnen derselben
Beteiligung (Abs. 2 S. 1).

- **Signatur** `VerrechenbarerVerlust`: `verlustanteil: money`, `ausgleichsfaehiger_verlust: money`
  (aus Regel 2, **Andockung** wie tarif/§32b — Integration verdrahtet P2 → P3) → `verrechenbarer_verlust: money`.
- **Rechenkern:** `verrechenbarer_verlust = verlustanteil − ausgleichsfaehiger_verlust` (der Rest, der
  nicht sofort ausgeglichen werden darf). Cent-genau, ≥ 0 (da ausgleichsfähig ≤ verlustanteil).
- **Geltungsbedingungen:** `verrechenbar_nur_kuenftige_gewinne_abs2` (Abs. 2 S. 1: mindert nur künftige
  Gewinne DERSELBEN Beteiligung — der Vortrag/die Verrechnung selbst = Nachtrag, hier nur die
  Betragsermittlung), `ausgleichsfaehiger_verlust_andockung_p2` (kommt aus Regel 2, keine Neuberechnung
  der min/max-Mechanik — vermeidet Doppel-Logik).
- **Seeds:** (Verlustanteil 15000, ausgleichsfähig 10000) → 5000 (verrechenbar) · (5000, 0) → 5000
  (KK=0: ganzer Verlust verrechenbar) · (6000, 6000) → 0 (voll ausgeglichen, nichts verrechenbar).

## Benannte Nachträge Charge 24

- **§ 15 Abs. 1 Nr. 2 S. 2** mittelbare Beteiligung (doppelstöckige PersGes) = eigene Regel.
- **§ 15 Abs. 1 Nr. 3** KGaA-Komplementär-Vergütungen (strukturgleich Nr. 2) = Nachtrag.
- **§ 15 Abs. 1 S. 2 / Nr. 2 S. 2** nachträgliche Einkünfte (§ 24 Nr. 2) = Nachtrag.
- **§ 15 Abs. 2** Gewerbebetriebs-Definition; **Abs. 3** Abfärbung/gewerbliche Prägung; **Abs. 4**
  Verlustverrechnungs-Sondertöpfe (Tierzucht, Termingeschäfte, Innengesellschaften) = eigene Komplexe.
- **§ 15a Abs. 1 S. 2/3** erweiterte Außenhaftung (§ 171 Abs. 1 HGB, eingetragene ./. geleistete Einlage)
  = eigene Regel (erweitert die ausgleichsfähige Grenze).
- **§ 15a Abs. 1a** nachträgliche Einlagen; **Abs. 3** Einlageminderung/Haftungsminderung (Gewinn-
  hinzurechnung); **Abs. 4** gesonderte Feststellung des verrechenbaren Verlusts; **Abs. 5** andere
  Unternehmer (stille Gesellschafter, GbR, ausländische PersGes, Mitreeder) = Nachträge.
- Der Verlustvortrag/die Verrechnung mit künftigen Gewinnen (Abs. 2 Mechanik über Jahre) = eigener
  Vortrags-Komplex; hier nur die Jahres-Betragsermittlung.

## Offene Punkte für deine Review

1. **P1** als reine additive Summe (Gewinnanteil + drei Sondervergütungen) — bestätigen. Die drei
   Sondervergütungs-Kategorien als DREI money-Inputs (Tätigkeit/Darlehen/Überlassung, wortlautnah) oder
   EIN zusammengefasster `sonderverguetung`-Input? Empfehlung: drei (die drei sind im Wortlaut einzeln
   benannt, wie die §-6-Nr3a-Ausnahme-Zweige).
2. **P2** `min(verlustanteil; max(kapitalkonto; 0))` — Encoding-hinweis (Floor+Cap) statt Split für die
   KK=0-Boundary (`klasse2-encoding-hinweis-leiter`); `max` als money-Op oder `if kapitalkonto > 0 then
   kapitalkonto else 0`? Empfehlung: Encoding, Formalisierer wählt max/if.
3. **P3** Andockung (`ausgleichsfaehiger_verlust` als Input von P2) vs. Neuberechnung aus `kapitalkonto`
   — Empfehlung: Andockung (keine Doppel-Logik, P2 ist die eine Wahrheit).
4. **`kapitalkonto` als signierter money-Input** (kann bereits negativ sein) — bestätigen.
5. Cap-Wort Stufe B: 3 Regeln (P1 2-quellig, P2 1-quellig, P3 1-quellig) → Vorschlag `--cost-cap 0.30`.
