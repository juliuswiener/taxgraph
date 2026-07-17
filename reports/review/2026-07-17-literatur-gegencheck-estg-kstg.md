# Literatur-Gegencheck — Registry EStG/KStG gegen Kommentar-Darstellung (Paket 10a)

**LLM-frei, $0 Pipeline.** Gegencheck der formalisierten Registry-Regeln gegen die
**Sekundärliteratur** (Kommentare). Zweck: (a) bestätigt die h.M. die abgebildete Auslegung/Formel?
(b) nennt der Kommentar Tatbestandsmerkmale, die keine Regel führt (Coverage-Loch)? (c) offene
BFH-Streitstände/BVerfG-Vorlagen zur Norm? Ergebnis = **Prüfaufträge an den Instructor**, KEINE
Registry-Änderung (die bleibt Instructor-Ruling vorbehalten).

## ⚠ Doktrin (bindend für diesen Report)
- **Kommentar = Sekundärliteratur, NIEMALS Zitatanker.** Alle Registry-Zitatanker bleiben amtlich
  (Gesetz/BGBl/BMF). Dieser Report liefert Prüfhinweise, keine neuen Anker.
- **Keine Registry-Änderung ohne Instructor-Ruling.** Funde sind als `PRÜFAUFTRAG` formuliert;
  keine Adjudikation, kein faithful/nicht_echt, kein YAML-Edit in diesem Schritt.
- Herleitung mit Fan-out (9 sonnet-Worker, je Cluster), **jeder HOCH/MITTEL-Fund von mir per
  Voll-Zeilen-Gegenprobe im Kommentar-Volltext unabhängig nachgeprüft** (Fundstelle real? sagt sie
  das Behauptete?). Falschgrün auf Worker-Seite wird abgefangen.

## Korpus (gitignoriert, urheberrechtlich; nur lokal, NIE committen/pushen)
| Kommentar | Auflage/Stand | Deckung |
|---|---|---|
| Kirchhof/Seer, EStG-Kommentar | 22. Aufl. **2023** | 86 EStG-Regeln (Primär) |
| Rödder/Herlinghaus/Neumann, KStG-Kommentar | 2. Aufl. **2023** | 9 KStG-Regeln (Primär) |
| Herrmann/Heuer/Raupach, Jahresband 2011 | Änderungen EStG/KStG **2010** | Rand (nur alte Änderungsgesetze) |
| Wassermeyer/Baumhoff, Verrechnungspreise | **2014** | Rand (§ 1 AStG/international — für Inlandsregeln kaum einschlägig) |

### ⚠ Editions-Lag (zentrale Vorwarnung)
Beide Primär-Kommentare bilden **Rechtsstand ~2022/2023** ab. **NICHT enthalten** (jünger als die
Auflagen): Wachstumschancengesetz (27.03.2024), JStG 2024, Sekundärkreditmarktförderungsgesetz
(Zinsschranke-Reform § 4h/§ 8a ab VZ 2024), sowie VZ-2025/2026-Fortschreibungen. Wo eine Regel
**neueren Rechtsstand** abbildet als die Auflage kennt, ist eine Divergenz zum Kommentar **erwartet
(Editions-Lag), kein Fund**. Betroffen u.a.: § 6 Abs. 5 Geschenke-Freigrenze 50 € (2024),
§ 7 Abs. 2 degressive AfA (Reaktivierung 2024), § 7 Abs. 2a E-Kfz-Sonder-AfA (neu),
§ 7g Abs. 5 Sonder-AfA 40 % (2024), § 10 Abs. 1 Nr. 5 Kinderbetreuung 80 % (ab VZ 2025),
§ 23 Freigrenze 1.000 € (2024), § 35c-Sätze, § 4h/§ 8a Zinsschranke-Reform.

## Prüf-Umfang
- **103 Registry-Regeln** gesamt. **95 mit Kommentar-Deckung geprüft** (86 EStG-Kommentar +
  9 KStG-Kommentar; die 2 § 4h-Regeln laufen im EStG-Kommentar mit).
- **8 Regeln ohne Kommentar-Deckung im Korpus** (dokumentierte Lücke, s.u.): 6 GewStG + 1 SolzG +
  1 EStR. Für diese existiert im Korpus keine Kommentierung → **nicht literatur-gegenprüfbar** mit
  dem vorhandenen Material.

### Cluster-Schnitt (Fan-out)
| Cluster | §§ | Regeln |
|---|---|---|
| C1 Werbungskosten AN | § 9 (+§ 6/§ 7 AfA-Verweis) | 9 |
| C2 Sonderausgaben | § 10, § 10a, § 10b, § 83–86 | 13 |
| C3 agB + Pauschbeträge + Familie | § 33/33a/33b, § 24a/24b, § 32, § 31 | 11 |
| C4 Tarif/Steuerermäßigung | § 32b/32d/34/34c/35/35a/35c/36/101/10d/16 | 15 |
| C5 Kapital/V+V/priv. Veräußerung | § 20/21/22/23, § 7 Abs. 4 | 10 |
| C6 Gewinnermittlung | § 4/5/11 | 7 |
| C7 AfA + Bewertung | § 6/6a/7/7g | 14 |
| C8 Mitunternehmer/Verlust | § 15/15a | 5 |
| C9 KStG + Zinsschranke | § 8/8a/8b/8c/8d/9/10/23 KStG, § 4h/§ 10d EStG | 11 |

## Lücke: Regeln ohne Kommentar-Deckung (nicht gegenprüfbar)
Für folgende 8 Regeln liegt im Korpus **kein einschlägiger Kommentar** vor. Sie sind mit dem
vorhandenen Material **nicht** literatur-gegenprüfbar; ein GewStG-Kommentar (z.B. Blümich oder
Glanegger/Güroff) und ein SolzG-Kommentar wären nötig, falls der Instructor Deckung wünscht.

| Regel | norm | benötigt |
|---|---|---|
| p7_gewerbeertrag | § 7 S. 1 i.V.m. § 6 GewStG | GewStG-Kommentar |
| p8_1_hinzurechnung | § 8 Nr. 1 GewStG | GewStG-Kommentar |
| p9_1_kuerzung_grundbesitz_ez2024 | § 9 Nr. 1 S. 1 GewStG (bis EZ 2024) | GewStG-Kommentar |
| p9_kuerzungen_ez2025 | § 9 Nr. 1/2/2a GewStG (ab EZ 2025) | GewStG-Kommentar |
| p10a_gewerbeverlust | § 10a S. 1–2 GewStG | GewStG-Kommentar |
| p11_steuermessbetrag | § 11 Abs. 1/2 GewStG | GewStG-Kommentar |
| solzg_solidaritaetszuschlag | § 3, § 4 SolzG 1995 | SolzG-Kommentar |
| estr_4_6_uebergangsgewinn_verteilung | R 4.6 S. 2 EStR | Verwaltungsanweisung (Billigkeit) — Kommentar sekundär |

## ⚠ Methoden-Limitation (ehrlich vorangestellt)
Die 9 Worker erhielten ein Regel-Digest, das **nur die `geltungsbedingungen`** enthielt, **nicht das
`hinweis`-Feld**. Das hinweis-Feld trägt aber viele **Nachtrag-/Vorfrage-Deklarationen und
Input-Verträge**. Folge: **mindestens vier Worker-HOCH-Funde waren Falsch-Positive**, die erst meine
Gegenprobe am hinweis/an der amtlichen Quelle ausräumte (p9-KSt-Spendenvortrag, p23-AfA-Kürzung,
p34_3-14%-Boden, teils p10b). **Jeder „fehlt"-Fund wurde von mir gegen hinweis UND amtliche Quelle
gegengeprüft.** Für künftige Lit-Checks: Digest MUSS das hinweis-Feld enthalten. Ohne diese
Gegenprobe hätte der Report vier Scheinlücken als echt gemeldet — der Falschgrün-Fall in Reinform.

## A. Bestätigte Funde — echte Prüfaufträge an den Instructor (nach Priorität)
Jeder Fund von mir gegen die **amtliche gefrorene Quelle** (nicht nur Kommentar) UND das hinweis-Feld
verifiziert. Kein Fund ist eine Registry-Änderung — alle sind Prüfaufträge.

1. **§34c Abs. 6 DBA-Vorrang nicht materialisiert** — `p34c_1_anrechnung_hoechstbetrag`,
   `p34c_2_abzug_statt_anrechnung`. Amtl. §34c Abs. 6 S. 1: „Die Absätze 1 bis 3 sind … nicht
   anzuwenden, wenn die Einkünfte aus einem … Staat stammen, mit dem ein Abkommen … besteht." Beide
   §34c-Regeln enthalten **null DBA-Bezug**; `dba_staat` steht **nicht** in `rules.yaml`. Die
   DBA-Methoden-Kataloge (AT/US/CH/FR/LU/NL/ES/TR) docken laut Design an §34c_1/§32b, existieren aber
   nur als **Markdown-Reports, nicht als Registry-Bedingung**. → Cross-Rule-Vertrag offen; bei jedem
   DBA-Staat griffe der unilaterale §34c ohne DBA-Vorfrage. **Wichtigster struktureller Einzelfund.**
   (Verknüpft mit [[versprochene-bedingung-materialisieren]].)
2. **§15a Abs. 3 S. 3–4 Haftungsminderung fehlt komplett** — `p15a_3_einlageminderung` scopet nur
   Abs. 3 S. 1–2 (Einlageminderung). Amtl. §15a Abs. 3 S. 3 („Wird der Haftungsbetrag … gemindert
   (Haftungsminderung) … ist … der Betrag der Haftungsminderung … als Gewinn zuzurechnen") ist ein
   **eigenständiger paralleler Tatbestand** mit eigenem Auslöser + eigenem 11-Jahres-Fenster;
   `grep` über die gesamte `rules.yaml` = 0 Treffer „Haftungsminderung". Kommentar widmet ihm eine
   eigene Hauptgliederung („III. Haftungsminderung nach Abs. 3 S. 3–4", Rn. 65). → eigene Regel
   `p15a_3_haftungsminderung` erwägen.
3. **§6a-Pensionsrückstellung: Zitatanker-Fehlzuordnung** — `p6a_pension_hoechstbetrag`. Der
   zitatanker „darf höchstens mit dem Teilwert der Pensionsverpflichtung angesetzt werden" ist als
   **§ 6a Abs. 4 S. 1** deklariert (bedingung `nur_hoechstbetrags_cap_abs4_s1` + hinweis) — amtlich
   ist dieser Wortlaut aber **§ 6a Abs. 3 S. 1**. Abs. 4 S. 1 sagt etwas anderes („höchstens um den
   Unterschied … erhöht werden" = Nachholverbot). Die Formel `min(ansatz, teilwert)` bildet korrekt
   Abs. 3 S. 1 ab → nur die Quelle-Angabe ist falsch. → (a) Anker Abs. 4 S. 1 → Abs. 3 S. 1
   korrigieren; (b) Scope-Frage: echte Abs. 4-S. 1-Zuführungsgrenze (Nachholverbot) unmodelliert.
4. **§5 Abs. 5 RAP: ungeschriebenes Rspr.-Tatbestandsmerkmal fehlt** — `p5_5_aktiver_rap`,
   `p5_5_passiver_rap`. BFH (I R 19/12) verlangt „über den geschriebenen Tatbestand hinaus" einen
   **zeitraumbezogenen Gegenleistungsanspruch** (Kommentar Rn. 114/118, eigener Abschnitt „4.
   Zeitraumbezogener Gegenleistungsanspruch"). Weder in den geltungsbedingungen NOCH im hinweis der
   beiden RAP-Regeln geführt. → als geltungsbedingung/Nachtrag erwägen. [2 Regeln]
5. **§35c Eigentümer-Erfordernis nicht modelliert** — `p35c_sanierung_ermaessigung`,
   `p35c_energieberater_ermaessigung`. Amtl. §35c Abs. 1 S. 1: „zu eigenen Wohnzwecken genutzten
   **eigenen Gebäude**". Regel führt `ausschliesslich_eigene_wohnzwecke`, aber **keine Eigentümer-/
   „eigenes Gebäude"-Bedingung** — ein Nicht-Eigentümer mit Eigennutzung würde nicht gesperrt.
   Schmale, aber echte Coverage-Lücke (Kommentar Rn. 12: „nur der Eigentümer … anspruchsberechtigt").
   [2 Regeln]
6. **§33a Abs. 1 S. 4 Unterhalt: absolute Anspruchssperren ohne Signatur-Slot** — `p33a_unterhalt`.
   Amtl. S. 4 = zwei absolute Sperren (kein Anspruch auf KFB/Kindergeld für die unterhaltene Person
   UND kein/nur geringes Vermögen). Signatur `{aufwendungen, kv_pv_beitraege, andere_einkuenfte}`
   hat **keinen bool-Slot**; hinweis erwähnt sie nicht; Schwester `p33a_ausbildungsfreibetrag` HAT
   ein `hat_anspruch:bool` → Inkonsistenz. → upstream-Gating (§2-Integration) bestätigen oder Bedingung
   ergänzen; zusätzlich Opfergrenze (R 33a.1 EStR) unmodelliert.
7. **§33b Abs. 6 S. 1 Pflege-PB: absolute Anspruchsvoraussetzungen ohne Slot** — `p33b_pflege_pauschbetrag`.
   Amtl. S. 1 = drei Sperren (keine Einnahmen / persönliche Pflege / EU-EWR-Belegenheit). Signatur
   `{pflegegrad, ist_hilflos}`, hinweis erwähnt nur die ist_hilflos-Vorrang-Staffel. → upstream-Gating
   bestätigen oder dokumentieren.
8. **§36 Abs. 3 S. 2 Rundungs-Doku invertiert** — `p36_2_anrechnung`. Amtl. S. 2: „ist jeweils die
   **Summe** der Beträge einer einzelnen Abzugsteuer aufzurunden" (Summe→Runde). Der modellierte
   Ein-Dienstverhältnis-Wert ist korrekt (nur 1 Betrag), aber die beschreibung sagt für den
   Mehrfachfall „je einzeln aufzurunden und zu summieren (Abs. 3 rundet je Betrag einzeln)" —
   **invertiert** und irreführend für die Integration (Bsp. 10,40 + 10,30: korrekt 21 €, Regel-Doku 22 €).
   → beschreibung korrigieren; Erstattungsausschluss (Rn. 7b) als Bedingung erwägen.

## B. Widerlegte Worker-HOCH-Meldungen (durch amtliche/hinweis-Gegenprobe entkräftet)
Transparenz-Abschnitt — diese Meldungen kamen aus dem Fan-out, hielten meiner Gegenprobe **nicht** stand:
| Regel | Worker-Meldung | Widerlegung |
|---|---|---|
| p9_1_3_nr5/5a | „2.000-€-Auslandsgrenze erfunden/nicht auffindbar" | Steht **wörtlich** in amtl. §9-Quelle (2026-07-09); Registry ist aktueller als Kommentar 2023 → **Editions-Lag**, kein Defekt |
| p33_3_zumutbare_belastung | „evtl. alte Flatrate statt BFH-2017-Stufe" | `rules.yaml:964` zitiert wörtlich die BFH-VI-R-75/14-**Stufenberechnung** → korrekt modelliert |
| p20_6_verlustverrechnung | „Termingeschäfte-/Ausfall-Sondertöpfe (je 20k) fehlen" | Durch **JStG 2024 aufgehoben** (alle offenen Fälle); amtl. §20-Quelle kennt sie nicht mehr → 2-Töpfe-Modell aktuell korrekt |
| p9_spenden_hoechstbetrag (KSt) | „Spendenvortrag §9 Abs. 1 Nr. 2 S. 9 fehlt als Nachtrag" | hinweis deklariert ihn ausdrücklich als Nachtrag → nur Digest-Loch |
| p23_veraeusserungsgewinn | „§23 Abs. 3 S. 4 AfA-Kürzung fehlt" | hinweis: „AK/HK kommt bereits AfA-gemindert aus §2-Integration" → Input-Vertrag dokumentiert |
| p34_3 | „14%-Mindestsatz fehlt" | hinweis: `max(0,56·durchschnittssatz; 0,14)` → Boden ist im Code |
| p10b_spenden | „zweiter GdE-Deckel fehlt (HOCH)" | S. 9 als out-of-MVP deklariert → auf **MITTEL** herabgestuft (Beschreibung sollte VZ-Effekt, nicht nur Vortrag, nennen) |

## C. Risiko-Register — offene Verfassungs-/BFH-Verfahren am Regel-Kern
Keine Coverage-Lücken, aber echte Rechtsunsicherheit an der jeweiligen Regel-Mechanik. Empfehlung:
als `bfh_streit`-/Risiko-Vermerk führen (Beobachtung, keine Adjudikation):
- **§8c Abs. 1 Verlustuntergang** (`p8c_verlustuntergang`): BVerfG-Vorlage **2 BvL 19/17** (FG Hamburg) — trifft den Alles-oder-Nichts-Kern.
- **§10d/§8 Abs. 1 KStG Mindestbesteuerung** (`p10d_2_verlustvortrag_abzug`, `p10d_kst_verlustabzug`): BVerfG-Vorlage zur Definitivwirkung (BFH I. Senat).
- **§4h Zinsschranke** (`p4h_zinsschranke_kern`): BFH **I R 20/15** → BVerfG-Vorlage zur Grundsatz-Verfassungsmäßigkeit.
- **§8a Abs. 2 KStG Konzern-Escape** (`p4h_ausnahmen_freigrenze`, `p8a_massgebliches_einkommen`): BFH **I B 111/11** (ernstliche Zweifel).
- **§20 Abs. 6 Aktien-Sondertopf** (`p20_6_verlustverrechnung`): BVerfG-Vorlage **2 BvL 3/21** (BFH VIII R 11/18).
- **§22 Nr. 1 Renten-Doppelbesteuerung** (`p22_1_leibrente_besteuerungsanteil`): BVerfG **2 BvR 1457/20, 2 BvR 1140/21, 2 BvR 1143/21** (BMF-Festsetzungen laufen vorläufig).
- **§6a Abs. 3 S. 3 / §6 Abs. 1 Nr. 3a 6 %/5,5 %-Abzinsung** (`p6a_pension_hoechstbetrag`, `p6_1_3a_abzinsung`): BVerfG **2 BvL 22/17** (FG Köln, 6 %-Rechnungszinsfuß) — strahlt methodisch auf beide.
- **§32b Drittstaaten-Ausschluss** (`p32b_progressionsvorbehalt`): BFH I R 80/16 → Verfassungsbeschwerde **2 BvR 148/21**.
- **§16 Abs. 4** (`p16_4_freibetrag`): anhängige Revision **X R 10/21** (Feststellungszeitpunkt Berufsunfähigkeit).
- **§35a** (`p35a_2_3_haushaltsnahe`): 3 anhängige Revisionen (X R 11/21 Reihenfolge; VI R 14/21 Altenpflege; VI R 24/20 Handwerker/Mieter) — außerhalb der modellierten Bedingungen.

## D. MITTEL — Integrations-/Gültigkeits-Prüfaufträge (kurz)
„Bitte bestätigen, dass die §2-Integration/Phase-5-Vorfrage X abfängt" — wahrscheinlich by-design, aber
nirgends dokumentiert; oder VZ-Parametrisierung prüfen:
- **Gültigkeit:** `p8a` §8a Abs. 1 — 2024-Zinsschranke-Reform-Reichweite gegen aktuelle Fassung prüfen (Kommentar 2023 schweigt); `p10d_2` — WachstumschancenG-Satz 60 %→70 % für VZ 2024–2027 VZ-parametrisiert? (Gegenstück `p10d_kst` modelliert 70 % bereits korrekt).
- **Upstream-Gate bestätigen:** `p10_1_2` (Höchstbetrag-Verdopplung §10 Abs. 3 S. 2 + Kürzung Beamte S. 3); `p10_1_5`×2 (hälftiger Höchstbetrag bei zwei berechtigten Elternteilen, Rn. 38e); `p10a_guenstigerpruefung` (Berufseinsteiger-Bonus 200 € NICHT in Günstigerprüfung, §10a Abs. 1 S. 5); `p20_9` (Ehegatten-Umverteilung §20 Abs. 9 S. 3); `p7g_5` (AK-Kürzung §7g Abs. 2 S. 2); `p7_2` (Zwölftelung/Monatsregel §7 Abs. 1 S. 4); `p7_4` (§7 Abs. 4 S. 2 kürzere tatsächliche Nutzungsdauer); `p32b` (ao-Fünftelung §32b Abs. 2 S. 1 Nr. 2); `p34_fuenftel_ao_est` (§6b/§6c-Ausschluss Abs. 1 S. 4); `p35_1` (Mehrbetrieb, betriebsbezogene Grenze Abs. 1 S. 5); `p101` (Antragserfordernis §104); `p21`/`p21_2` (anschaffungsnahe HK, §15a-PersGes, Luxuswohnung); `p33_1_2` (Existenzgefährdungs-Rückausnahme Prozesskosten §33 Abs. 2 S. 4); `p33b_behinderten/hinterbliebenen` (Übertragung §33b Abs. 5 nicht als MVP-Ausschluss markiert; Digest-Text „Katalog Nr. 1-5" statt amtl. „Nr. 1-4"); `p86` (Rundung volle Euro §86 Abs. 1); `p10b_spenden` (S. 9 VZ-Effekt).
- **Rundung/Doku:** `p36_2` (siehe A.8).

## E. Fazit
**95 Regeln (86 EStG + 9 KStG) gegen Kirchhof/Seer 22. Aufl. 2023 + Rödder/Herlinghaus/Neumann
2. Aufl. 2023 gegengeprüft**, 8 Regeln (GewStG/SolzG/EStR) mangels Kommentar nicht prüfbar.
Nach unabhängiger Gegenprobe an der **amtlichen gefrorenen Quelle** bleiben **~10 bestätigte
Prüfaufträge** (Abschnitt A, 8 Punkte / betrifft 12 Regeln); **7 Worker-HOCH wurden widerlegt**
(davon 4 wegen der Digest-hinweis-Limitation, 3 wegen aktuellerer Rechtslage als der Kommentar).
Die Registry erwies sich in mehreren Fällen als **aktueller als die Kommentar-Auflagen 2023**
(2.000-€-Auslandsgrenze, §20-Sondertöpfe-Aufhebung JStG 2024, Renten-Kohortentempo WachstumschancenG,
§7-AfA-Kohorten) — Editions-Lag lief hier zugunsten der Registry.

**Kein Fund rechtfertigt für sich eine sofortige Registry-Änderung** — alle sind Prüfaufträge an den
Instructor (norm_teil/Anker/Coverage → Instructor-Ruling). Der wichtigste strukturelle Einzelfund ist
die **nicht materialisierte §34c-Abs.-6-DBA-Vorfrage** (Cross-Rule-Vertrag zwischen §34c_1 und den
acht DBA-Methoden-Katalogen), gefolgt vom **§6a-Anker-Fehler** und der **§15a-Haftungsminderungs-Lücke**.
Der Kommentar bestätigt im Übrigen die **große Mehrheit** der Auslegungen und Rechenformeln wörtlich.
