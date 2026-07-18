# Deklarations-Vollständigkeits-Audit (dev-2, LLM-frei)

**Auftrag:** richtungs-agnostische Härtung des bestehenden Deklarations-Rings. Read-only, KEIN Kz-Eintrag
(nur Vorschläge). Deterministisch via Skript (scratchpad/dekl_audit.py). Stand nach e26f82b.

## KOPFZEILE: Der Deklarations-Ring ist SAUBER
- **Anker-Integrität: 92/92 anker_ref OK, 0 defekt** (jeder Zitatanker _normalize-voll-Länge in seiner Quelle).
- **est_mapping × Bindung: 0 tote Registry-Einträge, 0 Präzedenz-Kollisionen** (kein Feld gleichzeitig in
  special-Registry UND mit elster_kz → keine still ignorierte 1:1-Bindung).
- **Alle 50 null-Kz-Felder sind gerechtfertigt** (Voraussetzungs-Flags / berechnet / special-Klasse) bis auf
  2 echte auflösbare Kandidaten (unten). Kein Loch, das beißt.

## TEIL 1 — null-Kz-Kandidaten (Vorschlag, KEIN Eintrag)

Nach kz_extract-Verifikation bleiben **2 echt auflösbare** (mit Instanz-/Zweig-Vorbehalt) + 1 deferred:

| Feld | Kandidat-Kz | Beleg | Vorbehalt |
|---|---|---|---|
| am_anschaffungskosten (Anlage N Arbeitsmittel) | E0204402 [Einz] „Betrag" / E0204403 [Sum] „Summe" | E0204401 Art + E0204402 Betrag + E0204403 Summe, alle CType-Hash m1215341292 | **REPEATED-Instanz** (Due-Diligence): E0204402 ist PER-POSTEN-Betrag; Single-Total-MVP zielt eher auf E0204403 [Sum] (offen ob Eingabe- oder computed-Kz). KEIN trivialer 1:1 — Instanz-Fall wie Per-Kind/Multi-§21 |
| rentner_veraeusserungsgewinn (§16 Abs.4 Freibetrag) | **E0801301** [VAe_G_FB_Antr] „Veräußerungsgewinn vor Abzug des Freibetrags nach § 16 Abs…" | eindeutiges §16-FB-Kz; Anlage G (E08…), parallel Anlage S (E09…, E0901201) | Anlage-Zweig-abhängig (G/S/L+F) → Verzweigung wie Klasse f, nicht 1:1 |
| kist_gezahlt / kist_erstattet (§10 Abs.1 Nr.4 SA) | — nicht gefunden | Schnell-Pass fand nur LStB-KiSt (E0200501) + Kapital-KiSt (E1900601), NICHT die Mantelbogen-SA-„gezahlte/erstattete KiSt" (Vordruck-kurz-Kz 103/104) | deferred: braucht gezielten Mantelbogen-SA-Sektions-Lookup |

**KEINE Kandidaten (Regex-Fehlalarm korrigiert):** vv_entgelt_quote_prozent (berechnet→Klasse c, kein
dedizierter Kz — Lookup bestätigt), berufsausbildung_aufwendungen (§10 Nr.7-Label schema-weit absent),
rentner_renten_art (Art-Weiche, steuert Klasse-f-Verzweigung, bewusst kein eigener Kz).

## TEIL 2 — Anker-Integrität
**92/92 OK, 0 defekt, 0 ohne Anker.** Jeder anker_ref.zitatanker normalisiert wörtlich in seine
anker_ref.datei (Norm-/Vordruck-Quelle). Keine toten/schwachen Anker.

## TEIL 3 — est_mapping × Bindung-Deckung
- **Registry-Feld ohne Bindung: 0** (alle 12 special-Klasse-Felder existieren in der Bindung — kein toter Code).
- **Präzedenz-Kollision (special-Klasse UND elster_kz): 0** (die est_mapping-if-Kette prüft special vor 1:1;
  ein Feld mit beidem würde sein elster_kz still verlieren — kommt NICHT vor). Bestätigt u.a. dass die
  neuen Partner-Behinderungsfelder korrekt NUR 1:1 sind (nicht in PARTNER_INSTANZ).
- Deckung: 92 Felder = 30 (1:1 Klasse 1/b) + 12 (special a/d/e/f/g) + 50 (null-Kz Klasse c, gerechtfertigt).

## TEIL 4 — Deklarations-Coverage-Landkarte

| Scheibe | Felder | 1:1-Kz | special | null-c | askable | luecken |
|---|---|---|---|---|---|---|
| an_gesamt (§19/§26 + Flags + Partner-VOR) | 11 | 2 | 4 (g) | 5 | 11 | 0 |
| kap_vv_familie (§20/§21/§32/§35a-Familie) | 20 | 5 | 6 (a/d/e) | 9 | 20 | 18 |
| n_vor_gwg (§9/§9a Anlage N + VOR + GWG + Verpflegung) | 34 | 11 | 0 | 23 | 31 | 8 |
| rentner (§22/§24a/§33b/§16) | 14 | 7 | 2 (f) | 5 | 14 | 16 |
| sonder_agb_35a (§10/§10b/§33/§35a) | 13 | 5 | 0 | 8 | 13 | 19 |
| **Σ** | **92** | **30** | **12** | **50** | **89** | **61** |

Die 50 null-c + 61 luecken-Einträge sind der maschinenlesbare GAP-Bestand (bewusste Lücken mit Grund,
nicht Vergessen) — das ist die Fail-closed-Grundlage: nichts still übergangen.

## Empfehlung (nach Due-Diligence geschärft)
Der Ring ist deklarations-seitig gesund; die Richtungsentscheidung (Kapital/UI/DBA) hat eine saubere Basis.
**Kein wirklich trivialer 1:1-Nachzug offen** — Due-Diligence hat gezeigt: am_anschaffungskosten ist ein
REPEATED-Instanz-Fall (Arbeitsmittel = Art+Betrag+Summe je Posten, nicht Einzel-1:1), und
rentner_veraeusserungsgewinn → E0801301 braucht eine Anlage-Zweig-Verzweigung (G/S/L+F). Beide sind kleine
STRUKTUR-Entscheidungen, keine Einzeiler. Der relativ sauberere Kandidat ist rentner_veraeusserungsgewinn
(Zweig statt Instanz-Modell). Beide nur auf OK (Kz-Zitatanker-Doktrin). Kein Eintrag ohne Review.
