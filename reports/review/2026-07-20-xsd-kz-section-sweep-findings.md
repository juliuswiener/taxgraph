# XSD-Kz-Section-Sweep — konsolidierte Findings (Task #19–23)

Read-only. HOLD für Julius. Kein Self-Fix, kein Commit.

## Methodik

`elster_kz_grund`-Prosa ist als Oracle unbrauchbar — sie war beim L/S-Bug (Cluster 0)
selbst falsch (behauptete E0901201=Anlage S). Ersatz-Oracle: §-Zitat → erwartete Anlage
(§13 L, §15 G, §18 S, §19/§9 N, §20 KAP, §21 V, §22 Nr.1 R, §32 Kind, §33/§33b AgB,
§35a HA_35a, §10b SA, §10 Abs.1 Nr.2 VOR, §4/§6 EÜR), geprüft gegen den echten
Walker-Pfad (`xsd_verify.walk`, top-down lokale Elementnamen, kein Typ-Reverse-Lookup).

Zwei unabhängige Sweeps über alle 48 distinkten Kz: dev-2 (Task #19) und dev-1
(Task #20), dev-1 GEBLIND gegen dev-2s Ergebnis gefahren. Konvergent auf Cluster A+B;
Cluster C fand nur dev-1 (dev-2s #19 übersah ihn) — Beleg für den Wert des
Zweit-Sweeps.

## Cluster 0 — Anlage-Kz rentner_veraeusserungsgewinn (§16 Abs.4) — bereits gefixt

Bereits committet (SHA `73e3268`), hier nur als Präzedenz/Referenzfall:
`selbstaendig` zeigte auf E0901201 (Anlage L) statt E0804501 (Anlage S);
`land_forst` war benannte GAP (Ticket #16), jetzt auf E0901201 (Anlage L) verdrahtet.
Walk-verifiziert: gewerbe→`E10/G/.../E0801301`, selbstaendig→`E10/S/.../E0804501`,
land_forst→`E10/L/.../E0901201`. Tarif-Impact 0 (Ring liest Store-Feld
`rentner_veraeusserungsgewinn`, nie den Kz).

## Cluster A — §35a↔§33 Domänen-Swap (4 Felder)

Datei: `produkt/bindung/bindung_sonder_agb_35a.yaml`. Aktuell falsch:

| feld_id | aktueller Kz | Walker-Pfad (aktuell) | §-Zitat |
|---|---|---|---|
| `agb_aufwendungen` | E0104109 | `HA_35a/St_Erm/Minijobs/Sum` | § 33 Abs. 2 S. 1 EStG |
| `hh_minijob_aufwendungen` | E0161404 | `AgB/And_Aufw/Pflege/Sum` | § 35a Abs. 1 EStG |
| `hh_dienstleistungen` | E0161504 | `AgB/And_Aufw/Beh_Aufw/Sum` | § 35a Abs. 2 EStG |
| `hh_handwerker_arbeitskosten` | E0161804 | `AgB/And_Aufw/Sonst/Sum` | § 35a Abs. 3 EStG |

Root-Cause: Domänen-Swap — die vier Kz wurden vertauscht zwischen der allgemeinen
AgB-Sektion (§33, "andere außergewöhnliche Belastungen", Unterbuckets
Pflege/Beh_Aufw/Sonst) und der HA_35a-Sektion (§35a haushaltsnahe Aufwendungen,
Unterbuckets Minijobs/Hhn_BV_DL/Handw_L).

Korrekte Kz (Walk-verifiziert, doc-Text passend):

| feld_id | korrekter Kz | Walker-Pfad | Doku-Text |
|---|---|---|---|
| `hh_minijob_aufwendungen` | E0104109 | `HA_35a/St_Erm/Minijobs/Sum` | "Summe der Aufwendungen (abzüglich Erstattungen)" |
| `hh_dienstleistungen` | E0107208 | `HA_35a/St_Erm/Hhn_BV_DL/Sum` | "Summe der Aufwendungen (abzüglich Erstattungen)" |
| `hh_handwerker_arbeitskosten` | E0111215 | `HA_35a/St_Erm/Handw_L/Sum` | "Summe steuerlich berücksichtigungsfähiger Lohnanteile, Maschinen- und Fahrtkosten inkl. USt" |
| `agb_aufwendungen` | **E0161804 (Kandidat, Lead-Adjudikation)** | `AgB/And_Aufw/Sonst/Sum` | "Summe der Aufwendungen" (Sonst-Bucket) |

`agb_aufwendungen` ist der einzige nicht-deterministische Punkt: die AgB/And_Aufw-Sektion
hat DREI Unterbuckets (Pflege E0161404, Beh_Aufw E0161504, Sonst E0161804);
`agb_aufwendungen`s Fragetext ("größere außergewöhnliche Ausgaben ... z.B.
Krankheitskosten") ist generisch und passt strukturell am ehesten auf den
Sonst-Catch-all (E0161804) — aber Pflege/Beh_Aufw bleiben dann von KEINEM aktuellen
Bindungsfeld abgedeckt (kein stiller Drop, nur Transparenz: außerhalb des jetzigen
Feld-Katalogs, kein neuer Scope in diesem Fix).

Bug-kodierender Test: `tests/test_est_mapping.py:217-227`
(`test_scheibe2_sonder_35a_agb_1zu1_roundtrip`, Assertions Z.223-226).

Tarif-Impact 0: `produkt/haut/api.py:699-701,728` liest ausschließlich
`_c("hh_minijob_aufwendungen")` / `_c("hh_dienstleistungen")` /
`_c("hh_handwerker_arbeitskosten")` / `_c("agb_aufwendungen")` (Store-Feld), nie
den Kz-String.

## Cluster B — kap_kapitalertraege → Anlage-KAP-Elternzeile

Datei: `produkt/bindung/bindung_kap_vv_familie.yaml:9-21` (Person A),
`:116-128` (Person B, PARTNER_INSTANZ).

Aktuell: `kap_kapitalertraege` (§20 Abs.9 S.1, Sparer-Pauschbetrag) → E0121709,
Walker-Pfad `ESt1A_U/Ang_HH_unt_P_Unt_Leist/Ang_Unt_Pers/Ek_Bez_u_P/KapV/E0121709`
— landet in der Unterhaltsleistungs-Zusatzsektion des Hauptvordrucks, nicht in
Anlage KAP. Gegenprobe: Geschwisterfelder `kap_gewinn_aktien`/`kap_verlust_*`
(§20 Abs.6) liegen korrekt unter `KAP/KapErt_inl_StAbz/...` (E1900901/E1901201/
E1901301) — bestätigt E0121709 als echten Ausreißer, kein Schema-Artefakt.

Korrekter Kz: **E1900701** (`KAP/KapErt_inl_StAbz/Betr_lt_StBesch/E1900701`, Doku
"Kapitalerträge" — die KAP-Elternzeile, Geschwister-Slot zu E1900901 im selben
Block). Sparer-Pauschbetrag-Alternative (SpPB) geprüft und verworfen (dev-2,
Task #21 — Begründung dort).

Scope: 2 Instanzen betroffen — Person A (direkter 1:1-Kz) UND Person B
(`kap_kapitalertraege_partner`, PARTNER_INSTANZ-Eintrag `est_mapping.py:93`
reused denselben — dann falschen — Kz).

Bug-kodierende Tests (5, alle `tests/test_est_mapping.py`):
- `test_klasse_1_und_split_1zu1` (Z.68)
- `test_scheibe3_kapital_und_vv_1zu1_roundtrip` (Z.197)
- `test_aktien_subset_semantik_beide_deklariert` (Z.213-214)
- `test_neg_scheibe3_verfaelschtes_1zu1_bricht_roundtrip` (Z.273)
- `test_klasse_g_kapital_person_b` (Z.366,368 — Person-B-Bucket)

`tests/test_paket_b_e2e_http.py:766,1822,1870` erwähnt E0121709 nur in
Docstrings/Kommentaren (kein Literal-Assert gefunden) — separat verifizieren vor Fix.

Touch-Points: `bindung_kap_vv_familie.yaml:20` (Person A elster_kz),
`est_mapping.py:93` (PARTNER_INSTANZ-Wert).

Tarif-Impact 0: Ring nutzt `KAP_ERTRAEGE = "kap_kapitalertraege"`
(`produkt/haut/api.py:95`) als Store-Feld-Key, nie den Kz-String.

## Cluster C — Partner-Behinderung → PARTNER_INSTANZ-Reuse

Datei: `produkt/bindung/bindung_rentner.yaml:319-359`.

Aktuell: `rentner_grad_der_behinderung_partner`→E0505809,
`rentner_hilflos_blind_taubblind_partner`→E0505807 (§33b Abs.3, Ehegatten-eigene
Behinderung). Walker-Pfad beider: `E10/Kind/Ueb_PB_Beh_Hbl/Beh/...` — das ist NICHT
die AgB-Sektion, sondern der §33b-Abs.5-Übertragungsmechanismus für den
Behinderten-Pauschbetrag EINES KINDES auf die Eltern. Strukturell fremd zur
Fragestellung (Ehegatte hat selbst eine Behinderung).

Root-Cause: Header-Kommentar `bindung_rentner.yaml:320-324` behauptet explizit
"Person B hat eigene Kz (Klasse 1, NICHT g)" — falsche Prämisse. Tatsächlicher
Schema-Mechanismus: `AgB_67907_CType.Beh` (E10-2025.xsd Z.9622-9630) hat
`maxOccurs="2"`, indiziert über `Person: Enum_INDEXFELD_PERSON_AB_1_BaseCType`
(`PersonA`/`PersonB`, Z.7229/7234); die Sub-Blöcke
(`Ausw_Rentb_Besch_66196332_CType`/`Geh_Steh_Blind_Hilfl_66196332_CType`, Z.9638-9639)
sind TYPGLEICH zu denen, die bei Person A E0109708/E0109706 tragen — Person B nutzt
bei zweiter Block-Instanz DIESELBEN Kz, keine eigenen.

Person A (`rentner_grad_der_behinderung`/`rentner_hilflos_blind_taubblind`,
`bindung_rentner.yaml:227-261`) ist korrekt gebunden (E0109708/E0109706, AgB/Beh),
unbetroffen.

Fix-Muster bereits im Code vorhanden und produktiv (`est_mapping.py:86-97`
PARTNER_INSTANZ, Klasse g) — exakt analog `kap_kapitalertraege_partner`
(`bindung_kap_vv_familie.yaml:127-128`: `elster_kz: null` + Grund "kein
distinktes Partner-Kz, Instanz-Reuse"). Für Cluster C fehlen nur die 2 Einträge
in PARTNER_INSTANZ.

Bug-kodierender Test: `tests/test_est_mapping.py:253-263`
(`test_ehegatte_behinderung_partner_1zu1`) — Docstring behauptet wörtlich "Klasse 1,
NICHT g (Person B hat eigene Kz, kein Person-A-Reuse)", Assertions Z.261-262 auf
`deklaration["E0505809"]`/`["E0505807"]` (flacher Pfad statt `person_b`-Bucket).

Touch-Points: `est_mapping.py` PARTNER_INSTANZ (+2 Einträge:
`rentner_grad_der_behinderung_partner: E0109708`,
`rentner_hilflos_blind_taubblind_partner: E0109706`); `bindung_rentner.yaml:320-324`
(Header) + `:325-359` (beide Feldblöcke, elster_kz→null); `test_est_mapping.py:253-263`;
`produkt/konsistenz/partner_check.py:17-18` (nur Kommentar, K2-Guard-Logik
feld_id-basiert, unbetroffen).

Unbetroffen (feld_id-basiert, gegengecheckt): `partner_check.py`-Logik selbst,
`api.py:107 RENTNER_PARTNER`-Tuple, `test_partner_konsistenz.py`/
`test_partner_konsistenz_wiring.py`.

## Sweep-Vollständigkeit

48 distinkte Kz (37 reale Bindungsfelder + 11 geerntete Verzweigungs-/
Negations-/Aggregations-Kz), gegen E10-2025.xsd + E77-2025.xsd gewalkt:
**41 OK, 7 Mismatch** (Cluster A: 4, Cluster B: 1, Cluster C: 2). Rest inkl.
R-Sektion (Renten-Verzweigung, §22 Nr.1), N/N_DHH (§9/§19), Kind (§32/§24b),
V (§21), VOR (§10 Abs.1 Nr.2), EUER (§4/§6) — alle sektionsrichtig. Volle
Kz→Sektion→§-Tabelle auf Anfrage (dev-1-Skript, nicht persistiert).

## Gesamt-Tarif-Impact

Alle 4 Cluster (0/A/B/C) sind reine ELSTER-Submission-Bugs, 0 Tarif-Impact: der
Ring (`produkt/haut/api.py`) liest in jedem Fall das Store-Feld über `_c(...)`/
`felder.get(...)`, nie den `deklaration`-Kz-Wert aus `est_mapping.py`. Bestätigt
je Cluster einzeln (s.o.), konsistent mit dem bereits gefixten Cluster-0-Muster.

## Status

Alle 3 offenen Cluster (A/B/C) HOLD für Julius-Adjudikation (insb. Cluster A:
`agb_aufwendungen`-Zielbucket). Kein Self-Fix, keine Commits.
