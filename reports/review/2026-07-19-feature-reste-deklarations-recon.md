# Feature-Reste Deklarations-Recon (Julius #2-Front-Prep) — Read-only dev-2, 2026-07-19

Scope: die 3 Julius-#2-Kandidaten deklarations-seitig gescoped + priorisiert (Aufwand × Promotbarkeit ×
Wert). Read-only, KEIN Fix. Parallel zu dev-1s §24a-over-tax-Empirik. Alle Source-Werte gegen dated
Fassung verankert (§35-4×-Lehre gelebt).

## Priorisierungs-Tabelle

| # | Kandidat | Deklarations-Aufwand | Promotbarkeit | Wert | Empfehlung |
|---|---|---|---|---|---|
| **(a1)** | **p15_1_2 MitunternehmerEinkuenfte** (Einkünfte-Seite) | MODERATE — snapshot-ready catala (4 Inputs), aber FULLY un-wired: 4-Feld-Bindung + est_mapping-Klasse + Ring-Accessor neu | **HOCH** — snapshot verified_bedingt + faithful=True → materialize byte-equal + wire (kein Re-Run, Muster §10d/§16-4) | **MITTEL-HOCH** — Mitunternehmer = häufige PersG-Rechtsform; speist einkuenfte_gewinn (schon von §24a/§33/§10d genutzt) | **PRIO 1** |
| **(a2)** | **§35-Nr.2 anteiliger Messbetrag** (Ermäßigung-Seite) | KLEIN — 1 Feld `anteiliger_gewst_messbetrag` (FA-Grundlagenbescheid §35 Abs.2, gesondert+einheitlich festgestellt), null-Kz-MVP wie gewst_messbetrag | **HOCH** — reuse §35-4×-Accessor (runner.py:665 `mb*4`) als Summand; Kz E0802104 schon vorgemerkt (api.py:271) | **MITTEL** — koppelt an a1 (Mitunternehmer will §35-Anrechnung) | **PRIO 2** (Naht nach a1; dev-1-Ring für Accessor) |
| **(c)** | **Kz-MITTEL betriebseinnahmen/afa** | KLEIN-MODERATE — Kz-Promotion, aber Anlage-EÜR-Vordruck-Zeile-Cross-Check nötig | **BLOCKIERT** — Vordruck-PDF fehlt lokal + Aggregat-vs-Itemisierung-Entscheidung offen (E6001201-Summe ODER dokumentiert-Bucket) | **NIEDRIG** — dokumentiert-Aggregat ist ehrlich für MVP (aggregat-genau ≠ detail-genau) | **PRIO 3 / HOLD** (Vordruck + Granularitäts-Entscheidung) |
| **(b)** | **dHf-Mahlzeit-Kürzung §9 Abs.4a S.8** | MODERATE — neue Formalisierung: Kürzungs-Staffel + Mahlzeiten-Zähler-Feld | **NIEDRIG** — echter Build (kein Snapshot); common-case (keine Mahlzeit) schon via Flag gewired | **NIEDRIG** — seltener Edge-Case (benannte Lücke, bindung Z.340) | **PRIO 4 / DEFER** (Aufwand > Wert) |

## Detail

### (a1) p15_1_2 MitunternehmerEinkuenfte — PRIO 1
Snapshot `p15_1_2_mitunternehmer_einkuenfte.json`: queue=verified_bedingt, faithful=True, module=MitunternehmerEinkuenfte.
Inputs: `gewinnanteil` + `verguetung_taetigkeit` + `verguetung_darlehen` + `verguetung_ueberlassung`
(= die §15 Abs.1 S.1 Nr.2 Sondervergütungen). Output: `einkuenfte_mitunternehmer` (money).
Status: NICHT materialisiert (kein rules/estg/p15*-Dir, nicht in clerk.toml, kein rules.yaml-Eintrag) +
NICHT gebunden (bindung/est_mapping-Treffer = nur Kommentar-Erwähnungen „Mitunternehmeranteil"). = reines
verified_bedingt-Snapshot, wire-up-fähig wie §10d/§16-4. Promotion: catala byte-equal materialisieren +
4 Felder binden + est_mapping (gewinnanteil 1:1, 3 Sondervergütungen = slot_beitrag-Summanden?) + Ring-
Accessor der einkuenfte_mitunternehmer in einkuenfte_gewinn (betriebsart=gewerbe) faltet.
⚠ Adjudikations-Punkte für Instructor: (1) §-Anker fehlt im Snapshot (Feld „anker"=leer) → vor Materialisierung
Zitatanker §15 Abs.1 S.1 Nr.2 setzen. (2) est_mapping der 3 Sondervergütungen (eigene Kz je Vergütungsart
in Anlage G, oder Aggregat?).

### (a2) §35-Nr.2 anteiliger Messbetrag — PRIO 2
§35 Abs.1 S.1 Nr.2 (Source estg_p35_2026-07-14): Mitunternehmer (§15 Abs.1 S.1 Nr.2) → „um das Vierfache
des … festgesetzten **anteiligen** Gewerbesteuer-Messbetrags". Abs.2 S.1: anteiliger Messbetrag gesondert+
einheitlich festgestellt (FA-Grundlagenbescheid, Abs.3 S.2). = SELBE 4×-Mechanik wie Nr.1, nur andere Quelle
(Feststellungsbescheid statt eigener Messbescheid). 1 neues Feld, reuse `_gewst_messbetrag_cent`-Accessor
als Summand (Ermäßigung = 4×(eigener + anteiliger Messbetrag)). Kz-Kandidat E0802104 „sofern bekannt" schon
in api.py:271 vorgemerkt. Naht: braucht a1 zuerst (Mitunternehmer-Einkünfte müssen fließen, bevor die
Anrechnung greift).

### (c) Kz-MITTEL betriebseinnahmen/afa — PRIO 3 / HOLD
Aus EÜR-Kz-Report (2026-07-19): STARK-Block bereits promotet (§16-vg E0801301/E0901201, gwg E6002301,
sonstige_BA E6004901). Rest-MITTEL: `betriebseinnahmen` (E6001201-Summe vs itemisiert E6000101/301/401/501)
+ `afa_jahresbetrag` (E6002101 bewegliche vs Asset-Typ-Split via Anlage AVEÜR). Meine Felder = vorberechnete
Skalare, Anlage EÜR itemisiert. → braucht (1) Anlage-EÜR-Vordruck-Zeile-Cross-Check (PDF fehlt lokal) +
(2) Instructor-Granularitäts-Entscheidung (Aggregat-Zeile vs dokumentiert-Bucket wie V+V-WK). Ohne Vordruck
= verfrüht; dokumentiert-Aggregat ist ehrlich für MVP.

### (b) dHf-Mahlzeit-Kürzung §9 Abs.4a S.8 — PRIO 4 / DEFER
Source estg_p9_abs4a_2026-07-09 S.8: Mahlzeitengestellung → Verpflegungspauschale kürzen: „1. für Frühstück
um 20 Prozent, 2. für Mittag- und Abendessen um jeweils 40 Prozent" der Satz-3-Nr.1-Tagespauschale (28€ →
5,60€ / 11,20€); „die Kürzung darf die ermittelte Verpflegungspauschale nicht übersteigen" (Cap). S.10:
gezahltes Entgelt mindert Kürzungsbetrag. Status: NICHT materialisiert (benannte Lücke, bindung_n_vor_gwg
Z.340 „Mahlzeitenkürzung (S.8) = benannte Lücken (Folge-Nachtrag)"). Common-case (keine Mahlzeitengestellung
→ volle Pauschale) via Flag `vpf_keine_mahlzeitengestellung` schon gewired. Build = neue Kürzungs-Staffel +
Feld für Mahlzeiten-Zähler (Frühstück/Mittag/Abend-Counts). Aufwand MODERATE, Wert NIEDRIG (seltener
Edge-Case) → defer.

## Fazit / Empfehlung #2-Front-Wahl
**(a) Mitunternehmer-Bundle (a1→a2) = klarer #2-Kandidat**: a1 snapshot-ready (faithful=True → wire-up wie
§10d), füllt echte Rechtsform-Lücke (PersG-Mitunternehmer), speist die schon-genutzte gewinn-Basis; a2 als
kleine Naht danach (reuse 4×-Accessor). (c) HOLD auf Vordruck+Granularitäts-Entscheidung. (b) defer
(Aufwand > Wert, common-case schon wired). Alle read-only-scoped, kein Fix appliziert.
