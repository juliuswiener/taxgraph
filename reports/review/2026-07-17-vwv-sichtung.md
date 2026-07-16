# Verwaltungsvorschriften-Sichtung (Paket 10b, Stufe A, dev-2)

taxgraph-dev-2, 2026-07-17. Read-only-Sichtung der Finanzverwaltungs-Richtlinien
(EStR/EStH, KStR/KStH, GewStR/GewStH, LStR/LStH, AEAO) gegen die 99 Registry-Regeln.
Ziel: „gelebtes Recht" als **Konkretisierung** anbinden (authority=verwaltung), NIE als
Ersatz für Gesetz-Anker. KEINE Freezes (Instructor-Zone), KEINE Registry-Änderung. LLM-frei, $0.

## Kern-Befund vorab (melde statt improvisiere)
1. **Route-Blocker:** Die amtlichen Handbücher (`esth./ksth./gewsth./lsth.bundesfinanzministerium.de`)
   liegen hinter **Radware-Bot-Wall + SPA**. Der requests-Session-Warmup aus `corpus/scrape_dba.py`
   (der auf `www.bundesfinanzministerium.de` funktioniert) **reicht auf diesen Subdomains NICHT**:
   `__uzm*`-Cookies werden gesetzt, aber die JS-Challenge bleibt ungelöst (Titel bleibt „Radware
   Page", `perfdrive`-Marker; 2 Versuche belegt). ⇒ **H-Text-Ernte (Teilaufgabe b) + Anker-Qualität
   (d) brauchen einen echten Browser-Solve** (JS-Challenge-Ausführung + SPA-Rendering, „Blob-URL-Trick"
   via Chrome-Automation), nicht plain-HTTP. Dieser Report liefert das **download-freie Rückgrat**
   (a Mapping, c Gültigkeit, b als priorisierter Ernte-Plan mit Kandidaten-Schätzung).
2. **Gültigkeit ist gespreizt:** KStR 2022 + LStR 2023 aktuell; **EStR-Korpus 2012 (~14 J.)**,
   **GewStR 2009 (~17 J.) — ältestes Werk**. Explizit ausgewiesen (Julius-Direktive: Zeitschwellen
   nie still). Die *Hinweise* (EStH/KStH/GewStH) werden häufiger nachgeführt als die *Richtlinien*.

## Teilaufgabe (a) — R→Regel-Mapping

Rangfolge: **R** = Richtlinie (bindet Verwaltung, Konkretisierung), **H** = Hinweis (enthält
BMF-Schreiben-Verweise + amtliche Rechenbeispiele = Golden-Quelle). „Kand." = Golden-Kandidat-
Potenzial aus H-Beispielen (nachrechenbar), sobald H-Text vorliegt.

### EStR 2012 / EStH 2023
| Registry-Regel | § EStG | Richtlinie (R) | Hinweis (H) / Beispiel | Kand. |
|---|---|---|---|---|
| p4_3_gewinn | §4(3) | R 4.5 | H 4.5 (Zu-/Abfluss) | mittel |
| p4_5_1_geschenke | §4(5)S1Nr1 | R 4.10(2-4) | H 4.10 (35/50-€-Grenze) | mittel |
| p4_5_2_bewirtung | §4(5)S1Nr2 | R 4.10(5-9) | H 4.10 (70%-Bewirtung) | mittel |
| p5_5_aktiver/passiver_rap | §5(5) | R 5.6 | H 5.6 | niedrig |
| p6_1_1_bewertung_av / _wertaufholung | §6(1)Nr1 | R 6.7, R 6.8 | H 6.7 (Teilwert), Wertaufholungsgebot | mittel |
| p6_1_3a_abzinsung | §6(1)Nr3a | R 6.11 | **H 6.11 (Abzinsung 5,5%)** | **HOCH** |
| p6_1_4_kfz_nutzungswert | §6(1)Nr4 | (BMF 18.11.2009 primär; R dünn) | 1%-Beispiele meist BMF | niedrig* |
| p6_1_5_einlage | §6(1)Nr5 | R 6.12 | H 6.12 | mittel |
| p6_2_gwg_sofortabzug | §6(2) | **R 6.13** (GWG) | **H 6.13 (410/800/1000-Grenzen)** | **HOCH** |
| p6_2a_sammelposten_zuf./aufl. | §6(2a) | R 6.13(5-6) | **H 6.13 (Pool 1/5)** | **HOCH** |
| p6a_pension_hoechstbetrag | §6a | R 6a (umfangreich) | **H 6a (Teilwert-Pension)** | **HOCH** |
| p7_1_lineare_afa | §7(1) | R 7.1-7.3 | H 7.x | mittel |
| p7_2_degressive_afa | §7(2) | R 7.4 | H 7.4 | mittel |
| p7_2a_ekfz_75 | §7(2a) | (neu; BMF) | — | keine |
| p7_4_gebaeude_afa | §7(4) | R 7.4 | **H 7.4 (2/2,5/3%-Sätze)** | **HOCH** |
| p7g_1_iab_bildung / p7g_5_sonder_afa | §7g | (kein R; BMF 15.06.2022) | **H 7g (IAB 50% / Sonder-AfA 20%)** | **HOCH** |
| p10_1_5_kinderbetreuung | §10(1)Nr5 | (BMF 14.03.2012) | H 10.5 (2/3, max 4000/6000) | mittel |
| p10_1_7_berufsausbildung | §10(1)Nr7 | R 10.9 | H 10.9 (6000€ Höchst) | mittel |
| p10b_spenden | §10b | R 10b | H 10b (20% GdE) | mittel |
| p10d_2_verlustvortrag_abzug | §10d(2) | R 10d | **H 10d (1 Mio + 60%)** | **HOCH** |
| p15a_1/_2/_3 (Verlustverrechnung) | §15a | (kein R) | **H 15a (Verlusttopf-Beispiele)** | mittel |
| p16_4_freibetrag | §16(4) | R 16 | **H 16 (45.000€, Abschmelz ab 136.000)** | **HOCH** |
| p21_2_verbilligte_vermietung_wk | §21(2) | R 21.x | **H 21 (66%/50%-Grenze)** | **HOCH** |
| p22_1_leibrente_besteuerungsanteil | §22Nr1 | R 22.4 | **H 22 (Ertragsanteil-Tabelle)** | **HOCH (Tabelle)** |
| p24a_altersentlastungsbetrag | §24a | (Kohortentabelle Gesetz) | H 24a (Prozent/Höchst-Tabelle) | mittel (Tabelle) |
| p32b_progressionsvorbehalt | §32b | R 32b | **H 32b (bes. Steuersatz)** | **HOCH** |
| p33_1_2_agb / p33_3_zumutbare_belastung | §33 | R 33.1-33.4 | H 33.1-33.3 (zumutbare Belastung — **schon test_seed via BFH VI R 75/14**) | geerntet |
| p33a_unterhalt / _ausbildungsfreibetrag | §33a | R 33a.1 | **H 33a (Höchstbetrag, Opfergrenze)** | **HOCH** |
| p33b_*_pauschbetrag | §33b | R 33b | H 33b (Pauschbetrags-Tabelle) | mittel (Tabelle) |
| p34_fuenftel_ao_est / p34_3_durchschnittssatz | §34 | R 34.x | **H 34 (Fünftelregelung-Beispiel)** | **HOCH** |
| p34c_1/_2 (Anrechnung ausl. Steuer) | §34c | R 34c | **H 34c (Höchstbetrags-Formel)** | **HOCH** |
| p35_1_gewst_anrechnung | §35 | (R dünn) | H 35 (3,8/4,0-fach GewSt-Messbetrag) | mittel |
| p35a_2_3_haushaltsnahe | §35a | (BMF 09.11.2016 primär) | H 35a (20%, Caps — **schon test_seed**) | geerntet |
| p35c_* (energetische Sanierung) | §35c | (kein R; BMF) | — (P9-R1: 4 Kandidaten via BMF, kein R) | keine |

### KStR 2022 / KStH 2022
| Registry-Regel | § KStG | Richtlinie (R) | Hinweis (H) | Kand. |
|---|---|---|---|---|
| p8_1_einkommen_koerperschaft | §8(1)(3) | R 8.1-8.9 (**vGA R 8.5**) | H 8.5 (vGA-Beispiele) | mittel |
| p8b_beteiligungsertraege | §8b | KStH zu §8b | **H 8b (95%/5%-Schema)** | **HOCH** |
| p8c_verlustuntergang | §8c | (BMF 28.11.2017) | — | keine |
| p8d_fortfuehrungsgebundener_verlust | §8d | (BMF) | — | keine |
| p9_spenden_hoechstbetrag | §9 KStG | R 9 KStR | H 9 (20% Einkommen) | mittel |
| p10_nichtabziehbar_addback | §10 KStG | R 10 KStR | H 10 | niedrig |
| p23_koerperschaftsteuer_satz | §23 | (15%, Gesetz) | — | keine |

### GewStR 2009 / GewStH 2016  ⚠ ältestes Werk
| Registry-Regel | § GewStG | Richtlinie (R) | Hinweis (H) | Kand. |
|---|---|---|---|---|
| p7_gewerbeertrag | §7 | R 7.1 | H 7.1 | niedrig |
| p8_1_hinzurechnung | §8Nr1 | **R 8.1** | **H 8.1 (25% × [1/1,1/5,1/2,1/4] − 200.000 FB)** | **HOCH** (deckt GewSt-Ketten-Goldens) |
| p9_1_kuerzung_grundbesitz / p9_kuerzungen | §9Nr1/2 | R 9.1-9.2 | **H 9.1 (1,2% Einheitswert), H 9.2 (erweiterte Kürzung)** | **HOCH** |
| p10a_gewerbeverlust | §10a | R 10a.1-10a.4 | **H 10a (1 Mio + 60%, Unternehmer-/Unternehmensidentität)** | **HOCH** |
| p11_steuermessbetrag | §11 | R 11.1-11.2 | **H 11 (FB 24.500, Messzahl 3,5%)** | **HOCH** |

### LStR 2023 / LStH 2023
| Registry-Regel | § EStG | Richtlinie (R) | Hinweis (H) | Kand. |
|---|---|---|---|---|
| p9_4a_verpflegungsmehraufwand | §9(4a) | R 9.6 LStR | LStH 9.6 (28/14 — **schon test_seed**) | geerntet |
| p9_1_3_nr5_doppelte_haushaltsfuehrung | §9(1)S3Nr5 | R 9.11 LStR | LStH 9.11 (1000€/Monat) | mittel |
| p9_1_3_nr5a_uebernachtung_* | §9(1)S3Nr5a | R 9.7/9.8 LStR | LStH | mittel |
| p9_1_3_nr6_7_afa / arbeitsmittel | §9(1)S3Nr6/7 | R 9.12 LStR | LStH (GWG-Grenze Arbeitsmittel) | mittel |

### R-los (Tarif/Zulage/Abgeltung/DBA/neue Normen — bewusst KEIN Richtlinien-Treffer)
§32a-Tarif, §31 Familienleistungsausgleich (nur R 31 Verweis), §20(9) Sparer-PB, §24b,
Riester-Zulagen §83-99 (p8x/p84/p85/p86/p101), §32d Abgeltungsteuer (BMF), §7(2a)/§35c/§8d
(neu, nur BMF), DBA-Katalog. → kein VwV-Anker möglich/nötig; Gesetz-Anker genügt.

**Mapping-Trefferquote (Schätzung):** von 99 Registry-Regeln haben **~58 eine konkretisierende
Richtlinie/Hinweis** (EStR ~40, GewStR ~6, KStR ~6, LStR ~6), **~41 sind R-los** (Tarif/Zulagen/
Abgeltung/DBA/junge Normen). Trefferquote ≈ **59 %**.

## Teilaufgabe (b) — H-Beispiele als Golden-Kandidaten (Ernte-PLAN, Text ausstehend)

Priorisierte H-Abschnitte mit **nachrechenbaren** Zahlen-Beispielen (Muster P9 Runde 1). Bereits
geerntete Exemplare (zumutbare Belastung, haushaltsnahe 35a, Verpflegung) ausgeschlossen.

**Erste Ernte-Welle (höchstes HIT-Potenzial, rule-covered):**
H 6.13 (GWG/Sammelposten), H 6.11 (Abzinsung 5,5%), H 6a (Pension-Teilwert), H 7.4 (Gebäude-AfA-
Sätze), H 10d (Verlustabzug 1 Mio+60%), H 16 (Freibetrag-Abschmelzung), H 34/H 34c (Fünftel/
Anrechnungshöchstbetrag), H 22 (Ertragsanteil-Tabelle); GewStH 8.1 (Hinzurechnung), GewStH 9.1/9.2
(Kürzungen), GewStH 10a (Gewerbeverlust), GewStH 11 (Messbetrag); KStH 8b (95/5).

**Kandidaten-Schätzung:** ~**12–18 nachrechenbare NEUE** Golden-Kandidaten erwartbar (netto nach
Abzug bereits geernteter/tabellarischer). Exakte Zahl + Zitatanker erst nach H-Text-Ernte
belegbar — **nicht vorab beziffert, um kein Falsch-Grün zu erzeugen**. GewStH 8.1/10a + H 10d
sind die wahrscheinlichsten Treffer (decken die bestehenden GewSt-/§10d-Ketten-Goldens amtlich ab).

## Teilaufgabe (c) — Gültigkeits-Check je Werk (Stand 2026-07-17)
| Werk | Fassung | In Kraft seit | Alter | Status |
|---|---|---|---|---|
| **EStR** | EStR **2012** (BStBl I 2012 Sondernr.1) | VZ 2012; keine EStÄR seither in Kraft (Ref-Entwurf EStÄR 2025 offen) | **~14 J.** | ⚠ R-Korpus alt |
| EStH | **EStH 2023** (amtl. 2024) | jährlich nachgeführt | aktuell | ✓ Hinweise frisch |
| **KStR** | KStR **2022** (ersetzt 2015) | VZ 2022 | ~4 J. | ✓ aktuell |
| KStH | KStH 2022 | — | ~4 J. | ✓ |
| **GewStR** | GewStR **2009** (BStBl I 2010 Sondernr.1) | EZ 2009 | **~17 J.** | ⚠⚠ ältestes Werk |
| GewStH | GewStH **2016** | — | ~10 J. | ⚠ |
| **LStR** | LStR **2023** (ersetzt 2015) | 2023 | ~3 J. | ✓ aktuell |
| AEAO | laufend (BMF-Schreiben) | fortlaufend | aktuell | ✓ (kaum Berechnungs-relevant) |

**Konsequenz:** Für GewSt-H-Beispiele (GewStR 2009 / GewStH 2016) ist die **Gegenprobe gegen den
aktuellen Gesetzesstand Pflicht** (Messzahl 3,5 %, FB 24.500 unverändert — aber z. B. §8-Nr.1-
Quoten/Freibetrag 200.000 sind neuer als 2009 ⇒ H-Beispiel kann veralteten Rechtsstand zeigen;
GENAU der P9-„veralteter Rechtsstand"-Prüffall). EStR-2012-Beispiele ebenso auf zwischenzeitliche
Gesetzesänderungen prüfen (z. B. GWG-Grenze 410→800 ab 2018; H 6.13 muss die 800er-Fassung zeigen).

## Teilaufgabe (d) — Freeze-Readiness
- **Anker-Tauglichkeit unbestätigt** bis erste echte H-Text-Extraktion. Erwartung: HTML-Handbücher
  liefern sauberen UTF-8-Text (keine U+0007-BEL-Artefakte wie Alt-BGBl-Scans), also `_normalize`-
  tauglich — aber **erst nach Radware-Browser-Solve verifizierbar**.
- **Blocker (s. o.):** requests-Warmup unzureichend auf esth./ksth./gewsth./lsth. → Chrome-Automation
  (JS-Challenge lösen, SPA rendern, Section-HTML extrahieren) ODER amtliche BStBl-PDF-Route der
  Handbücher (falls PDF-Gesamtausgabe ohne Radware beschaffbar). Zu klären mit Instructor.

## Repro
```
cd /home/julius/00_projects/168_TaxGraph/taxgraph
# Regel-Inventar + Norm je Regel:
python3 -c "import glob,yaml,os;[print(yaml.safe_load(open(f))['rule_id']) for f in sorted(glob.glob('pipeline/item_registry/*.yaml')) if 'rule_id' in (yaml.safe_load(open(f)) or {})]"
# Radware-Probe (belegt Blocker): Warmup-Session gegen esth → Titel bleibt 'Radware Page', perfdrive-Marker.
```
Read-only, kein Registry-/Code-Touch außer diesem Report.

## Fazit
- **Mapping (a) fertig:** ~59 % der 99 Regeln haben eine Verwaltungs-Konkretisierung; das
  Rückgrat steht download-frei. Verwendung: Richtlinien als **Konkretisierung** (Zitatanker
  authority=verwaltung) NEBEN dem Gesetz-Anker, nie als Ersatz.
- **Gültigkeit (c) fertig:** EStR 2012 / GewStR 2009 alt — GewSt-H-Beispiele zwingend gegen
  aktuellen Gesetzesstand gegenprüfen (Veralt-Risiko).
- **H-Ernte (b) + Anker-Qualität (d) blockiert** durch Radware+SPA auf den Handbuch-Subdomains;
  requests-Bypass unzureichend (belegt). Braucht Browser-Solve oder amtliche PDF-Route.
  **Rückfrage an Instructor:** Grünes Licht für Chrome-Automation-Ernte der ~15 priorisierten
  H-Abschnitte — oder alternative amtliche Beschaffungsroute?
