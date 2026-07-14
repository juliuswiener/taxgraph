# Coverage-Landkarte — Messbasis Vollabdeckung (2026-07-13)

Stufe A, $0. Macht den FMS-227-Katalog von Archiv zu Roadmap: jede ESt-relevante Anlage /
jeder Regelungsbereich → Status + unsere Regel-IDs. Ist-Stand als ehrliche Startmarke, dann
Priorisierungs-Vorschlag Charge 4 ff.

## Was „100 %" heißt (Nenner-Definition — zuerst, sonst ist jede Zahl Willkür)

Der 227er-Katalog ist KEIN sinnvoller Nenner: 177 der 227 sind Formular/Sonstiges (Zoll,
Energiesteuer, Verfahrensrecht, BZSt-Portale, DBA-Bescheinigungen) — für die Einkommensteuer
natürlicher Personen **Nicht-Gegenstand**. Der ehrliche Nenner ist die
**Einkommensteuer-Erklärung einer natürlichen Person, arbeitnehmer-nah** — der Satz an Anlagen
und EStG-Regelungsbereichen, den ein Angestellter / Rentner / Vermieter tatsächlich ausfüllt.

Wir messen auf **Regelungsbereich-Ebene** (die Granularität unserer Regeln), nicht pro Anlage
(eine Anlage trägt mehrere §§). Zwei Zahlen, beide ehrlich:

- **AN-Kern** (Lohnsteuerfall: § 19-Lohn + Standard-Werbungskosten/Sonderausgaben, Familie,
  agB, Haushaltsnahe): **~90 % — nahezu vollständig.** Das ist das bisherige MVP-Ziel.
- **Voller natürliche-Person-Umfang** (alle sieben Einkunftsarten + übergreifende Mechanismen
  + Spezial-Anlagen): **~40 %.** Das ist das neue 100-%-Ziel.

## Ist-Stand: 21 Pipeline-Regeln + 5 Handschrift-Module

Pipeline: **17 aktiv**, 3 `zuschnitt_ersetzt` (Monolithen, durch Teilregeln abgelöst), 1
`handgeschrieben` (solzg). Handschrift-Schicht: p32a-Tarif, solzg, p04-Arbeitszimmer,
p09-Entfernungspauschale, p07-AfA-Überhangsjahr. Plus die § 2-Integration (arbeitnehmerfall:
§ 9a-Pauschbetrag, § 10c, Stufenfolge).

Hinweis Reifegrad: „formalisiert" = deterministische Gates grün. Von den 17 aktiven trägt genau
`p9_1_3_nr6_7_afa_laufend_nb` gerade `strukturgeprueft_judge_offen` (Judge-Nachzug offen); der
Rest ist verified_bedingt/verified. Coverage-Landkarte zählt Existenz+Struktur, nicht das
Judge-Siegel — Reifegrad ist eine eigene Spalte.

## Coverage-Matrix (AN-naher Nenner)

Legende: ✅ formalisiert · 🟡 teilformalisiert (Kern da, Rest offen) · 🟠 geplant (Charge N) ·
⬜ Lücke · 🚫 Nicht-Gegenstand.

### Mantelbogen ESt 1 A / Tarif — ✅ vollständig (5/5)
| Regelungsbereich | § | Status | Regel |
|---|---|---|---|
| Einkommensteuertarif | § 32a | ✅ | p32a Einkommensteuertarif (Handschrift) |
| Solidaritätszuschlag | § 3/4 SolzG | ✅ | solzg (Handschrift) |
| § 2-Stufenfolge, Pauschbeträge | § 2, § 9a, § 10c | ✅ | arbeitnehmerfall (Handschrift) |
| Steueranrechnung | § 36 Abs. 2 | ✅ | p36_2_anrechnung |
| Entlastungsbetrag Alleinerziehende | § 24b | ✅ | p24b_entlastungsbetrag |

### Anlage N — nichtselbständige Arbeit / Werbungskosten — ✅ Kern vollständig (7/7 WK-Blöcke)
| Regelungsbereich | § | Status | Regel |
|---|---|---|---|
| Entfernungspauschale | § 9 Abs. 1 S. 3 Nr. 4 | ✅ | p09 (Handschrift) |
| Arbeitsmittel / GWG / lineare AfA | § 9 Nr. 6/7, § 6 Abs. 2, § 7 | ✅ | p9_1_3_nr6_7_afa_laufend_nb (+ p07 Überhangsjahr) |
| Verpflegungsmehraufwand | § 9 Abs. 4a | ✅ | p9_4a_verpflegungsmehraufwand |
| Übernachtung/Reisekosten (≤/>48 Mon) | § 9 Nr. 5a | ✅ | nr5a_vor_48, nr5a_nach_48 |
| Arbeitszimmer/Homeoffice | § 4 Abs. 5 Nr. 6b, § 9 | ✅ | p04 (Handschrift) |
| Erstausbildung-Abgrenzung | § 9 Abs. 6 | ✅ | p9_6_erstausbildung_abgrenzung |
| **Bruttolohn/AG-Leistungen (Deklaration)** | § 19 | 🟡 | Erfassung via Submission-XML, keine Rechenregel nötig |

### Anlage N-DHF — doppelte Haushaltsführung — ✅ (1/1)
| doppelte Haushaltsführung (Monats-Cap 1000 €) | § 9 Nr. 5 | ✅ | p9_1_3_nr5_doppelte_haushaltsfuehrung |

### Anlage Vorsorgeaufwand — ✅ Kern (2/2)
| Altersvorsorge (Basis-Rente) | § 10 Abs. 1 Nr. 2, Abs. 3 | ✅ | p10_1_2_altersvorsorge |
| KV/PV-Beiträge | § 10 Abs. 1 Nr. 3/3a, Abs. 4 | ✅ | p10_1_3_3a_kv_pv |

### Anlage Haushaltsnahe Aufwendungen — ✅ (1/1)
| Haushaltsnahe Beschäftigung/Dienstleistung/Handwerker | § 35a | ✅ | p35a_2_3_haushaltsnahe (12 Bedingungen) |

### Anlage Kind — ✅ (4/4)
| Kinderfreibeträge | § 32 Abs. 6 | ✅ | p32_6_kinderfreibetraege |
| Familienleistungsausgleich (Günstigerprüfung) | § 31 | ✅ | p31_familienleistungsausgleich |
| Ausbildungsfreibetrag | § 33a Abs. 2 | ✅ | p33a_ausbildungsfreibetrag (Charge 5, verified_bedingt) |
| Kinderbetreuungskosten | § 10 Abs. 1 Nr. 5 | ✅ | p10_1_5_kinderbetreuung (Charge 5, verified_bedingt) |

### Anlage Sonderausgaben — ✅ (4/4 ohne Vorsorge)
| Berufsausbildung (eigene) | § 10 Abs. 1 Nr. 7 | ✅ | p10_1_7_berufsausbildung |
| Kirchensteuer | § 10 Abs. 1 Nr. 4 | ✅ | p10_1_4_kirchensteuer |
| Spenden/Mitgliedsbeiträge | § 10b | ✅ | p10b_spenden (Charge 5, verified_bedingt) |
| Unterhalt-Realsplitting (Anlage U) | § 10 Abs. 1a | ✅ | p10_1a_realsplitting (Charge 5, verified_bedingt) |

### Anlage Außergewöhnliche Belastungen — ✅ (4/4)
| agB allgemein — Abzug | § 33 Abs. 1, 2 | ✅ | p33_1_2_agb_abzug |
| zumutbare Belastung | § 33 Abs. 3 | ✅ | p33_3_zumutbare_belastung |
| Unterhalt an bedürftige Personen (Anlage Unterhalt) | § 33a Abs. 1 | ✅ | p33a_unterhalt (Charge 5, verified_bedingt) |
| Pauschbeträge Behinderte/Hinterbliebene/Pflege | § 33b | ✅ | p33b_behinderten/pflege/hinterbliebenen_pauschbetrag (Charge 5, verified_bedingt) |

### Andere Einkunftsarten — ✅ (4/4 Anlagen)
| Kapitalvermögen (Abgeltungsteuer, Sparer-PB) | § 20, § 32d, § 20 Abs. 9 | ✅ | p20_6/p20_9/p32d_1 (Charge 6, verified_bedingt) |
| Renten / sonstige wiederkehrende Bezüge | § 22 Nr. 1 | ✅ | p22_1_leibrente_besteuerungsanteil (Charge 7, Kohorten-params, verified_bedingt) |
| Vermietung und Verpachtung | § 21 | ✅ | p21_vermietung_einkuenfte + p7_4_gebaeude_afa (Charge 8) + p21_2_verbilligte_vermietung_wk (Charge 10, Abs2 50/66-Schwellen; Korridor 50–66 = Prognose-Nachtrag) — verified_bedingt |
| Sonstige Einkünfte / private Veräußerung | § 22 Nr. 3, § 23 | ✅ | p23_veraeusserungsgewinn + p23_freigrenze (1.000-€-Freigrenze) + p22_3_sonstige_leistungen (256-€-Freigrenze) — Charge 12, verified_bedingt (§ 22 Nr. 2 Spekulation via § 23 abgedeckt) |

### Übergreifende Tarif-Mechanismen — ✅ (4/4)
| Progressionsvorbehalt (Lohnersatz ALG/Elterngeld) | § 32b | ✅ | p32b_progressionsvorbehalt (Charge 4, verified_bedingt) |
| Außerordentliche Einkünfte / Fünftelregelung | § 34 | ✅ | p34_fuenftel_ao_est (Charge 9, Tarif-Andockung, verified_bedingt; Abs. 3 ermäßigter Satz = benannter Nachtrag) |
| Verlustabzug (Vortrag) | § 10d Abs. 2 | ✅ | p10d_2_verlustvortrag_abzug (Charge 9, verified_bedingt; Abs. 1 Rücktrag = Nicht-Gegenstand) |
| Altersentlastungsbetrag | § 24a | ✅ | p24a_altersentlastungsbetrag (Charge 5, Kohorten-params, verified_bedingt) |

### Spezial-Anlagen / Förderung — ✅ (4/4: 3 formalisiert + 1 begründeter Nicht-Gegenstand)
| Riester / Altersvorsorgezulage (Anlage AV) | § 10a, § 79 ff. | ✅ | p8x_zulage_anspruch + p86_mindesteigenbeitrag + p86_zulage_kuerzung + p10a_sonderausgabenabzug + p10a_guenstigerpruefung — Charge 11, verified_bedingt (Zulage §§83-86 + §10a SA/Günstigerprüfung) |
| Energetische Maßnahmen (Anlage Energ.) | § 35c | ✅ | p35c_sanierung_ermaessigung (7/6 %-Staffel) + p35c_energieberater_ermaessigung (50 %-Sondersatz) — Charge 10, verified_bedingt |
| Mobilitätsprämie | §§ 101 ff. | ✅ | p101_mobilitaetspraemie (14 % × min(EP ab 21 km, GFB-Unterschreitung)) — Charge 12, verified_bedingt |
| Wohneigentumsförderung (Anlage FW) | § 10e/10f/10g | 🚫 | Nicht-Gegenstand mit Grund: § 10e Altfall „vor dem 1. Januar 1995" (Freeze-Anker), Förderzeitraum ~2002 ausgelaufen; § 10f/g Nischen. Keine AN-Relevanz (Charge 12) |

## Nicht-Gegenstand (bewusst außerhalb, mit Grund)
- **Zoll/Energiesteuer/Verbrauchsteuer** (010xxx, 033xxx, 1102–2735 usw., ~150 PDFs) — nicht ESt.
- **Verfahrensrecht/AO** (Fristen, Vollmachten, Steuernummer-Anträge, BZSt-Portale) — kein
  materielles Steuerrecht.
- **Beschränkte Steuerpflicht / Auslandsbezug** (ESt1C, Anlage AUS/N-AUS/R-AUS, § 34c/34d DBA) —
  eigener Komplex; für AN-nahen MVP zunächst außen vor (nach Kern-Vollabdeckung wiedervorlegen).
- **Gewinneinkünfte / Selbständige** (Anlage G/S/EÜR, § 4/§ 13/§ 15/§ 18) — eigener großer
  Komplex, außerhalb AN-nah; Ausland/DBA + EÜR bleiben laut Julius-Grobreihenfolge zuletzt.
- **§ 2-Terminal-Arithmetik** (AfA-Überhangsjahr) — Integrations-Arithmetik, kein Wortlaut →
  bereits Handschrift-Schicht (p07), keine Formalisierungsregel.
- **§ 10d Abs. 1 Verlustrücktrag** — ändert den Steuerbescheid des vorangegangenen VZ (Abs. 1
  S. 4–5) → Verfahrens-/Mehrjahres-State-Territorium (§ 2-Integration + AO-Bescheidänderung);
  isolierte Betrags-Kappung ohne Ziel-VZ-Mechanik wäre Scheinabdeckung. Vortrag (Abs. 2) deckt
  den AN-nahen Kern. Fortschreibung (Abs. 4) ebenfalls § 2-/State-Territorium.
- **§ 34 Abs. 3 (ermäßigter 56-%-Durchschnittssatz)** — antragsabhängiger Sonder-Tarifpfad ab
  55. LJ / Berufsunfähigkeit, nur Betriebsveräußerung, einmal im Leben; AN-fern → benannter
  Nachtrag, kein Charge-9-Gegenstand.
- **§ 35c 40 000-€-Objekt-Höchstbetrag (Abs. 1 S. 5)** — Lebensdauer-Deckel über alle Maßnahmen +
  3 VZ, kumulativ = § 2-/State-Territorium (§ 10d-Abs4-Präzedenz); als Geltungsbedingung
  `objekt_hoechstbetrag_40k_nicht_ausgeschoepft` deklariert (Charge 10), aber pro-VZ nicht gerechnet.
- **§ 35c Energieberater-Deckel-Interaktion** — ✅ **erledigt Charge 14 (BMF Rn 56, verwaltung):**
  die 50 % zählen gegen den gemeinsamen 14k/12k-Jahresdeckel (Kombination `P35cJahresdeckel`,
  `rules/estg/p35c/energetische_massnahmen.catala_en`, 5 clerk-Seeds) UND den 40k-Objektdeckel; VZ =
  Abschlussjahr, nicht gedrittelt. 40k-Objekt bleibt Mehrjahres-State (BMF-gedeckt, s. o.).
- **§ 21 Abs. 2 Korridor 50–< 66 % (Totalüberschussprognose)** — die Aufteilungsentscheidung im
  Zwischenband hängt an der BMF-Prognose, kein Norm-Wortlaut → benannter Nachtrag; Regel deckt
  quote < 50 (anteilig) und quote ≥ 66 (voll).
- **Riester § 10a Abs. 2 Hinzurechnung voll vs. bereinigt** — ✅ **erledigt Charge 14 (BMF Rn 106,
  verwaltung):** die Hinzurechnung nimmt den **bereinigten** Zulageanspruch (ohne § 84-S 2-Erhöhung
  200 €); bestätigt das bereinigte Design (`p10a_guenstigerpruefung`), Festlegung an der § 2-Integration
  (Abs 6 `hinzurechnung_zulage`). Keine neue Regel.
- **Riester Berufseinsteiger-Einmaligkeit** (§ 84 S 2/3) = Einmal-im-Leben-State (40k-Präzedenz),
  bool-Input; **Ehegatten-Übertragung / mittelbar Zulageberechtigte** (§ 10a Abs 3, § 79 S 2, § 86
  Abs 1 S 2 Hs 2, § 85 Abs 2), **ausländische Alterssicherung** (§ 10a Abs 6, § 86 Abs 5),
  **Landwirte** (§ 86 Abs 3), **§ 86 Abs 2 Entgelt-Sonderfälle** = Nicht-Gegenstand/Nachträge.
- **§ 23 Abs. 3 S. 7 Verlustverrechnung (privater Veräußerungs-Verlusttopf)** — Verluste nur mit
  § 23-Gewinnen DESSELBEN Jahres = **benannter Nachtrag, FORMALISIERBAR** (p20_6-Topf-Trennungs-
  Präzedenz); S. 8 (Vor-/Rücktrag nach § 10d-Maßgabe) = Nicht-Gegenstand/Mehrjahres-State.
- **§ 23 Tatbestands-Fiktionen** (Abs 1 S 2 Entnahme/Betriebsaufgabe, S 4 PersGes-Anteile, S 5
  Einlage/verdeckte Einlage; Abs 3 S 2/3 „tritt an die Stelle"-Werte) — definieren WAS als
  Anschaffung/Veräußerung zählt, nicht WIE gerechnet wird → als Geltungsbedingung
  `anschaffung_veraeusserung_originaerer_erwerb` an p23_veraeusserungsgewinn benannt, ausgeklammert.

## Startmarke (auditierbar)
AN-Kern-Blöcke (Mantel 5 + N-WK 7 + N-DHF 1 + Vorsorge 2 + Haushaltsnahe 1 + Familie-Kern 2 +
SA-Kern 2 + agB-Kern 2 = **22 formalisiert**) gegen den vollen AN-nahen Nenner (die 22 plus:
Kind 2, SA 2, agB 2, KAP/R/V/SO 4, § 32b/§ 34/§ 10d/§ 24a 4, Riester/§ 35c/Mobilität/FW 4 =
**40 Regelungsbereiche**). Startmarke war **22/40 ≈ 55 %** aller AN-nahen Blöcke; der AN-Kern
selbst ~90 %.

**Fortschritt:** Charge 4 (§ 32b) verified_bedingt → 23/40 ≈ 57 %. Charge 5 §33b-Trio
(Behinderten-/Pflege-/Hinterbliebenen-Pauschbetrag) verified_bedingt → 24/40. § 24a
Altersentlastungsbetrag (Kohorten-Muster) verified_bedingt → 25/40. Charge 5 Paket B+C
(§10b, §33a Abs1+2, §10 Nr5, §10 Abs1a) → 30/40. Charge 6 KAP (§20/§32d, Verlust/Sparer-PB/
Abgeltung) → 31/40. Charge 7 Renten (§22 Nr1 Leibrente-Besteuerungsanteil, Kohorten,
strukturgeprueft) → 32/40 ≈ 80 %. Charge 8 Vermietung (§21 Einkuenfte + §7 Abs4 Gebaeude-AfA,
Negativ-Durchreichung) verified_bedingt → 33/40. Charge 9 (§34 Fuenftelregelung Tarif-Andockung
+ §10d Abs2 Verlustvortrag-Hoechstbetrag 70%) verified_bedingt → 35/40 ≈ 87,5 %. Damit ist
die Tarif-Mechanismen-Gruppe (§32b/§34/§10d/§24a) auf 4/4 geschlossen. Charge 10 (§35c energetisch
7/6%-Staffel + Energieberater-50%-Sondersatz, §21 Abs2 verbilligte Vermietung) verified_bedingt →
36/40 ≈ 90 % (Foerderungs-Gruppe geoeffnet 1/4; §21-Abs2-Nachtrag schliesst benannte
Vermietungs-Luecke). Charge 11 Riester (§10a SA-Abzug + Guenstigerpruefung, §§83-86 Zulage:
Grund/Kinder/Berufseinsteiger + Mindesteigenbeitrag 4% + Kuerzung; 5 Regeln, Andockung §31/§34)
verified_bedingt → 37/40 ≈ 92,5 % (Foerderungs-Gruppe 2/4). Charge 12 (§23 Veraeusserungsgewinn
+ 1.000-Freigrenze, §22 Nr3 sonstige Leistungen 256-Freigrenze, §101 Mobilitaetspraemie 14%;
FW §10e/f/g = begruendeter Nicht-Gegenstand Altfall) verified_bedingt → **40/40 = 100 %**.

## 🏁 PROGRAMM-MEILENSTEIN: AN-naher Nenner vollstaendig (40/40)

Alle 40 AN-nahen Regelungsbereiche sind **formalisiert (verified_bedingt) ODER als
Nicht-Gegenstand begruendet benannt**. Der AN-Kern (~100 %) war schon nach Charge 5 dicht;
Chargen 4-12 haben die uebrigen Einkunftsarten, Tarif-Mechanismen und Foerdertatbestaende
geschlossen. Startmarke 22/40 ≈ 55 % → 40/40. Naechste Ebene (jenseits dieses Nenners):
benannte Nachtraege (z.B. §23 Abs3 S7 Verlusttopf, §21 Abs2 Prognosekorridor, Riester-
Hinzurechnung) + die bewusst ausgeklammerten Grosskomplexe (Selbstaendige/EUER, Ausland/DBA).

**AN-Kern: die 🟡-Lücken (Kind, Sonderausgaben, agB) sind mit Charge 5 auf 4/4 geschlossen -
der AN-Kern ist damit auf echte ~100 %.** Judge-Nachzug erledigt (2026-07-13, neuer Judge
mistral-medium-3-5): alle 10 vormals strukturgeprueft-Regeln (_nb + Paket B/C + KAP + §22)
verified_bedingt.

## Priorisierungs-Vorschlag Charge 4 ff.
Kriterien: (a) Häufigkeit in realen AN-nahen Erklärungen, (b) § 2-Abhängigkeit (was der
Tarif/GdE braucht), (c) Formalisierbarkeit (klarer Wortlaut, wenig Ermessen). Julius-Grobreihen-
folge als Ausgangspunkt, hier geschärft:

1. **Charge 4 — § 32b Progressionsvorbehalt** (Lohnersatz: ALG I, Kurzarbeiter-, Eltern-,
   Krankengeld). *Warum zuerst:* sehr häufig im AN-Fall, greift direkt in den Tarif (§ 2-nah,
   dockt an p32a an), klarer Wortlaut (besonderer Steuersatz). Kleiner Zuschnitt, hoher Nutzen.
2. **Charge 5 — Kapitalvermögen** § 20 / § 32d / Sparer-Pauschbetrag § 20 Abs. 9 (Anlage KAP).
   Sehr häufig (fast jeder hat KAP-Erträge), aber eigener Sondertarif (25 % Abgeltung +
   Günstigerprüfung) — mittlerer Umfang.
3. **Charge 6 — Renten** § 22 Nr. 1 (Anlage R): Besteuerungsanteil nach Rentenbeginn-Kohorte.
   Häufig (Rentner), klarer Wortlaut (Tabelle), gut formalisierbar.
4. **Charge 7 — Vermietung** § 21 (Anlage V): Einnahmen − Werbungskosten (AfA Gebäude §7 Abs.4,
   Schuldzinsen, Erhaltungsaufwand). Häufig, aber umfangreicher (eigene AfA-Systematik).
5. **Charge 8 — Pauschbeträge/Familie-Rest**: § 33b (Behinderten-/Pflege-Pauschbetrag),
   § 33a Abs. 1 Unterhalt, § 33a Abs. 2 Ausbildungsfreibetrag, § 24a Altersentlastung. Kleine,
   klar tabellierte Zuschnitte — hohe Formalisierbarkeit, füllt die 🟡-Lücken in agB/Kind.
6. **Charge 9+ — § 10d Verlustabzug, § 34 Fünftelregelung, § 35c energetisch, Riester (§ 10a)**,
   danach Selbständige/EÜR und Ausland/DBA zuletzt (eigener Komplex).

Reihenfolge-Logik: erst die tarif-nahen Mechanismen (§ 32b) und häufigsten Einkunftsarten
(KAP, R, V), die den GdE/zvE eines Durchschnitts-AN wirklich verändern; dann die klar
tabellierten Pauschbetrags-Lücken (billig, schließen die 🟡-Felder); Sonderkomplexe zuletzt.

## Empfehlung
Charge 4 = **§ 32b Progressionsvorbehalt** als erster Vollabdeckungs-Zuschnitt: klein,
tarif-nah, sofort spürbar, dockt an p32a an. Schätzung folgt vor dem Lauf (Budget-Regel).
Die Landkarte selbst war $0; das +20-USD-Budget (gesamt ~24,7) trägt laut Grobschätzung die
nächsten 4–8 Zuschnitte. Judge-Nachzug `_nb` + Hersteller-ID laufen als geparkte Wecker weiter.
