# Morgen-Paket — Julius-Entscheidungen (Nacht 2026-07-11/12)

Konsolidiert für den Morgen-Review. Alles unten ist entweder entschieden (mit
Widerrufsvorbehalt) oder wartet auf deine Entscheidung. Nacht-Summe **$0,50 / 10 USD**.
Alle Gate-Verdikte frisch, doppelt gerechnet (dev + instructor), kein Falschgrün.

---

## 1. Formalisierer-Besetzung — RE-EVALUATIONS-TRIGGER AUSGELÖST (deine Entscheidung)

Drei Formalisierer-Schwäche-Fälle in Folge (models.yaml-Trigger: rollende Eskalation /
Human-Review-Miss):
- **p33** Rundung (A rundete ab, B nicht) — in Charge 1 gelöst durch A-Neulauf.
- **nr6_7** Letztjahr-AfA-Rest (A modelliert ihn nicht) + year0-Zwölftelung-Regress
  im redo_a (A-Lauf-Varianz).
- **nr5a** 48-Monats-Gate fehlt — **A UND B** kappen unbedingt (nicht nur A). Erstes
  Mal, dass BEIDE Formalisierer denselben Norm-Mechanismus verfehlen.

Besetzung ist Julius-Entscheid (nicht dev/instructor). Optionen + Kostenrahmen:
- **(A) Modellwechsel A und/oder B**: anderes Modell für Formalisierer A (aktuell
  anthropic/claude-sonnet-4.6) und/oder B (z-ai/glm-5.2). Erfordert einen kleinen
  Bake-off (G2-Muster, ~14 Tasks × Paarung) zur begründeten Neubesetzung — Größen-
  ordnung ~2-4 USD einmalig. Ändert den models.yaml-Hash.
- **(B) Provider-Variante**: nur Provider-Pin (wie beim Judge together→deepinfra
  heute Nacht). $0 Config + Re-Runs. Hilft NUR bei Infra/Quantisierung, NICHT bei
  echten Modell-Fähigkeitslücken (die drei Fälle sind Fähigkeit, nicht Infra) →
  vermutlich unzureichend allein.
- **(C) Zuschnitts-Feedback-Schleife**: die failenden Seeds/Gaps strukturiert in die
  Kaskade zurückspielen (Prompt-Anreicherung). ACHTUNG: Prompt-Änderung → braucht
  vorregistrierten Messplan (Dekret), nicht heute Nacht umsetzbar. Mittelfristig das
  sauberste, aber methodisch teuerste.
Empfehlung dev/instructor: (A) für B (glm-5.2 fiel bei nr5a UND lieferte bei nr6_7
erst gar kein Catala) prüfen; A (sonnet) hält sich besser (year0 war im 5/6-Lauf
korrekt). Aber das ist deine Entscheidung.

---

## 2. Widerrufsvorbehalt-Bestätigungen (Nacht-Delegation → dein OK)

Vom Instructor unter Nacht-Delegation bestätigt, mit deinem Widerrufsvorbehalt:
- **7× verified_bedingt (Charge 1)**: p10_1_7, p9_6, p9_1_3_nr5, p33 (Schritt 2) +
  p24b, p9_4a, p35a (nach Abweichungs-Triage). Alle Gates frisch grün, doppelt gerechnet.
- **K1-Konvention** `stunden_je_kalendertag` (0..24) in signatur_konventionen.yaml.
- **K2-Konvention** `nichtnegative_betraege` um Zählgrößen erweitert.
- **cap4000-Seeds** (§35a Abs2, synthetisch, Sichtprüfung bracket 4.000,00).
- **Auto-Apply-Whitelist** {bedingung_neu, nicht_material} (feat/triage-ui, gemergt).
- **dHf/p9_4a-Triage-Mappings** (Abweichungs-Triage der 12 Items).

---

## 3. Netto/Brutto-GWG-Grenzfall (§ 6 Abs. 2 → § 9b Abs. 1) — Recherche nötig

nr6_7: `anschaffungskosten` = maßgebliche AK i.S.d. § 6 Abs. 2 S. 1. § 6 Abs. 2
verweist auf § 9b Abs. 1 (Vorsteuer nur raus, SOWEIT bei der USt abziehbar — beim
Arbeitnehmer i.d.R. NICHT). Also für den AN: Brutto-AK (inkl. USt) ist die maßgebliche
Grenze für die 800-Euro-Prüfung. Nicht still festgelegt; als Bedingung
`anschaffungskosten_sind_massgebliche_ak` deklariert. Braucht BMF/LStR-Bestätigung,
dann Kommentar/Seed. Materiell für die 800/801-Grenze (Brutto vs Netto verschiebt sie).

---

## 4. Zwei dokumentierte Selbstkorrekturen (Kultur: sichtbar, nicht geglättet)

- **nr5a-Boundary-Fehlurteil (Instructor)**: Instructor entschied zunächst (b) "kein
  defekt, nur Seed-Korrektur" unter der Prämisse "A kappt ab 48 korrekt". Die dev-
  Per-Seed-Probe **falsifizierte** das (A kappt unbedingt ab 0). Instructor zog die
  Konsequenz → (a) defekt. Prämisse offen falsch, Korrektur sichtbar.
- **Seed-Spec-Selbstkorrektur (Instructor)**: die 48/49-Boundary-Spec war unter Index-
  Semantik gedacht; die tatsächliche Norm (Ablauf-Semantik) + un-gepinnte Input-
  Semantik von monate_bisher_am_ort → Neupinnung (Wert 48 = Kappung). Dokumentiert.

---

## 5. Stufe-B-Go Teil 2 (nach Besetzungsentscheid #1)

Teil 2 Stufe A ist komplett reviewed + freigegeben: 8 Zuschnitte (SolzG, § 36 Abs2,
§ 33 Abs1/2, § 32 Abs6, § 31, § 10 v1/v2/v4; v3 Backlog). Manifest-Einträge werden mit
den Auflagen finalisiert ($0). **Stufe B (Kaskaden-Läufe) wartet bewusst auf deinen
Besetzungsentscheid** (#1) — mit einer geschwächten Formalisierer-Paarung zu laufen
verbrennt Budget. Kostenrahmen Stufe B Teil 2: ~0,5-0,8 USD für 8 Regeln. PLUS die
zwei defekt-Neuläufe (nr6_7, nr5a) nach Besetzungsfix.

---

## Kosten-Schlusszeile

**Nacht gesamt: $0,50 / 10 USD.** Aufschlüsselung: Charge-2-Stufe-B Kaskaden + Redos
+ Infra-Schleife (Judge-Hang) ~$0,50; alles andere (Charge-1-Seeding, Restore, Regates,
Triagen, Probes, 8 Teil-2-Zuschnitte, 8 Quellen-Freezes, Tooling-Fix) **$0**
(deterministisch/lokal).
