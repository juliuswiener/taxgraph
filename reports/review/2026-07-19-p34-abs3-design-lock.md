# § 34 Abs. 3 Ermäßigter Durchschnittssatz — Design-Lock (Stufe-2) — dev-2, 2026-07-19

Vertiefung des Recon (2026-07-19-p34-abs3-ermaessigter-satz-deklarations-recon.md). Zwei Instructor-Aufträge:
(A) Judge-Fehltreffer WÖRTLICH gegen estg_p34 belegen (nicht behaupten). (B) 5-Mio-Excess-Ring-Split sauber
designen (>5Mio regulär besteuern, nicht droppen). Read-only, KEINE Ring-Naht gebaut. Alle Werte source-verifiziert
gegen estg_p34_2026-07-13.txt.

## (A) Judge-Fehltreffer WÖRTLICH — nicht_echt-Beleg

**Judge-Abweichung (Snapshot p34_3, judge_verdict, wörtlich):**
> „Der durchschnittliche Steuersatz wird als Quotient aus est_gesamt_zzgl_progression und
> bemessungsgrundlage_durchschnitt berechnet, nicht als tarifliche Einkommensteuer."

**Der Norm-Satz, den der Judge falsch las — § 34 Abs. 3 S. 2 EStG (estg_p34_2026-07-13, wörtlich):**
> „Der ermäßigte Steuersatz beträgt 56 Prozent **des durchschnittlichen Steuersatzes**, der sich ergäbe, wenn die
> tarifliche Einkommensteuer nach dem gesamten zu versteuernden Einkommen zuzüglich der dem Progressionsvorbehalt
> unterliegenden Einkünfte zu bemessen wäre, mindestens jedoch 14 Prozent."

**Der konkrete Fehltreffer:** Der Judge verwechselt zwei distinkte Größen im Norm-Wortlaut:
- „durchschnittlicher **Steuersatz**" = eine RATE (dimensionslos) = per Definition (tarifliche ESt) ÷ (Basis).
- „tarifliche **Einkommensteuer**" = ein BETRAG (€) — im Norm-Satz nur der ZÄHLER der Rate („der sich ergäbe,
  wenn die tarifliche Einkommensteuer … zu bemessen wäre").

Das Modell rechnet `durchschnittssatz = est_gesamt_zzgl_progression / bemessungsgrundlage_durchschnitt` — das IST
der „durchschnittliche Steuersatz" exakt nach S. 2: `est_gesamt_zzgl_progression` = die tarifliche ESt (Zähler),
`bemessungsgrundlage_durchschnitt` = das gesamte zvE zzgl. Progression (Nenner). Der Judge forderte, der Wert solle
„als tarifliche Einkommensteuer" (Betrag) berechnet werden — aber S. 2 verlangt einen SATZ (Rate), und ein Satz IST
ein Quotient. Der Judge hat das Wort „Steuersatz" als „Steuer" (Betrag) fehlgelesen. → **abweichung nicht_echt**
(Norm-Wortlaut deckt exakt die Quotient-Formel; keine Norm-Abweichung). Muster p10d_2 (2-Mio-Falschflag) /
p15_1_2 (Judge übersah Nr.2-S.2). ⚠ Instructor-Boundary-Review gegen estg_p34 bei Materialisierung (nicht mein
alleiniger Call — faithful≠Correctness gilt in BEIDE Richtungen).

Registry-Mechanik (bei Materialisierung, NICHT jetzt): abweichung nicht_echt-adjudizieren → offene=0 →
verified_bedingt-clean → byte-gleich materialisieren (wie p10d_2/p15_1_2). item_registry.py-Pfad (p34_3 ist
Snapshot-only, kein rules.yaml-Eintrag).

## (B) 5-Mio-Excess-Ring-Split — kritisches Naht-Design

**Norm § 34 Abs. 3 S. 1:** ermäßigter Satz nur auf „den Teil dieser außerordentlichen Einkünfte, der den Betrag von
insgesamt **5 Millionen Euro nicht übersteigt**". → der Überschuss >5Mio ist NICHT von Abs. 3 gedeckt.

**Modul-Grenze:** `ErmaessigterDurchschnittssatz.est_ao = min(ao, 5Mio) · ermaessigter_satz` — liefert NUR die ESt
auf den ≤5Mio-Teil. Der Überschuss (ao − 5Mio) hat im Modul KEINE Steuer. Würde der Ring nur `est_ao` ansetzen und
den Überschuss weglassen → **>5Mio-vg UNVERSTEUERT = K2-UNDER-TAX**.

**Ring-Split-Design (dev-1-Ring, bei Abs.3-Materialisierung — NACH Abs.1-Chooser):**
```
ao          = außerordentliche Einkünfte (VÄ-Gewinn §§14/16/18, ≤5Mio-Antrag)
ao_erm      = min(ao, 5Mio)                      # ermäßigt-besteuerter Teil
ao_excess   = max(0, ao − 5Mio)                  # REGULÄR-besteuerter Überschuss
zvE_rest    = zvE − ao                            # verbleibendes zvE (S.3)

est_erm     = ErmaessigterDurchschnittssatz(ao_erm, ...)          # Abs.3 S.2 (56%/14%)
est_rest    = Tarif(zvE_rest)                                      # S.3 allg. Tarif auf verbleibendes zvE
est_excess  = Tarif(zvE_rest + ao_excess) − Tarif(zvE_rest)       # ao_excess regulär ON TOP (Grenzsteuer)
                # ODER Abs.1-Fünftel auf ao_excess, je Abs.1-vs-Abs.3-Chooser — Design-Frage an Instructor

festzusetzende_est ⊇ est_rest + est_erm + est_excess
```
⚠ Die `est_excess`-Zeile ist die K2-kritische: der Überschuss wird NICHT gedroppt (das wäre Under-Tax).

⭐ **EXCESS-TREATMENT KORRIGIERT (Source-Read 2026-07-19, mein „regulär"-MVP war FALSCH = Over-Tax/K2):**
§ 34 nennt den >5Mio-Überschuss-Tarif NIRGENDS explizit (nur Abs. 3 S. 1 nennt „5 Millionen Euro"). S. 6
(„Absatz 1 Satz 4 ist entsprechend anzuwenden") importiert NUR die § 6b/§ 6c-Ausnahme (Abs. 1 S. 4), NICHT
das Excess-Treatment. → SYSTEMATIK: Abs. 3 S. 1 verschiebt „auf Antrag **abweichend von Absatz 1** die auf den
Teil … der 5 Mio **nicht übersteigt**" — NUR der ≤5Mio-Teil wird von Abs. 1 weg zum ermäßigten Satz verschoben.
Der Überschuss >5Mio bleibt außerordentliche Einkünfte iSd Abs. 2 Nr. 1 unter dem DEFAULT = **§ 34 Abs. 1
Fünftelregelung** (NICHT regulär). Regulär wäre HÖHER als Fünftel → **Over-Tax = K2-Verstoß** (ein bekannter
Over-Tax ist kein „safe"). → est_excess = Abs.1-Fünftel auf ao_excess, NICHT Grenzsteuer-regulär.
⚠ ABER: das ist SYSTEMATISCHE Auslegung, NICHT Wortlaut → braucht Kommentar-Beleg + Instructor-Adjudikation.

⚠⚠ **KOMMENTAR-ZWEITBELEG (Kirchhof/Seer EStG 22.Aufl 2023, § 34 Rn. 49-50) — SPANNUNG, ESKALIERT:**
Der Kommentar enthält ZWEI scheinbar widersprüchliche Aussagen zum >5Mio-Excess in benachbarten Randnummern:
- **Rn. 49 (S. 127811 pdftotext):** „Nur bis zu diesem Betrag [5 Mio] wird der ermäßigte Durchschnittssteuersatz
  gewährt. Darüber hinausgehende Gewinne nehmen nicht an der Steuerermäßigung teil und werden **nach dem
  normalen Tarif** besteuert." → liest sich wie REGULÄR (voll progressiv).
- **Rn. 50 (S. 127881 pdftotext, Wahlrecht-Abschnitt):** „Liegen die Voraussetzungen für eine Steuerermäßigung
  nach Abs. 3 nicht vor, kommt eine Steuerermäßigung nur nach Abs. 1 in Betracht. Das bedeutet, dass in den
  Fällen, in denen der Veräußerungsgewinn 5 Mio. Euro übersteigt, für den darüber hinausgehenden Betrag
  **Abs. 1 in Betracht kommt**." → liest sich wie ABS.1-FÜNFTEL.

**Meine Reconciliation (Instructor entscheidet final):** Rn. 50 ist die PRÄZISE, operative Wahlrechts-Aussage
(explizit „Abs. 1 in Betracht"); Rn. 49 ist die lose Überblicks-Formulierung („normaler Tarif" = nur Kontrast
zum Abs.3-56%-Satz). Systematisch stützt Rn. 50: der Überschuss bleibt außerordentliche Einkünfte iSd Abs. 2
Nr. 1, und Abs. 1 gilt „von Amts wegen" (Kirchhof/Seer § 16-Kommentar 86634: Abs.1 „von Amts wegen", Abs.3
„antragsgebunden") für ALLE ao Nr. 1 → Excess → **Abs. 1-Fünftel**. Der Überschuss verliert seine ao-Eigenschaft
oberhalb 5Mio NICHT.
⚠ **K2-BEIDSEITIG:** ist Rn.50 (Fünftel) richtig + wir bauen regulär → Over-Tax. Ist Rn.49-wörtlich (regulär)
richtig + wir bauen Fünftel → UNDER-Tax (schärfer). → Excess-Treatment ist NICHT gebaut-entscheidbar ohne
Instructor-Adjudikation; ich neige zu Rn.50 (Fünftel), aber die Rn.49-„normaler Tarif"-Formulierung erzeugt echte
Ambiguität → ESKALIERT. Registry-Anker (bei Materialisierung) = 2 Wortlaut-Anker (Abs.3 S.1 „5 Mio nicht
übersteigt" + Abs.1 mandatory) + Systematik-Doku + dieser Kommentar-Zweitbeleg.

**Div-by-Zero-Guard** (Recon-Boundary): `durchschnittssatz = est/bemessungsgrundlage` — wenn bemessungsgrundlage=0
→ Catala-Laufzeitfehler. Basis ⊇ ao (>0 wenn ao>0), aber defensiver Guard `if bemessungsgrundlage <= 0 then 0.14`
(min-Satz greift) bei Materialisierung erwägen.

⭐ **AVERAGE-RATE-BASIS geklärt (Source Abs.3 S.2, Boundary-Punkt (a)):** „56 Prozent des durchschnittlichen
Steuersatzes, der sich ergäbe, wenn die tarifliche Einkommensteuer nach dem **gesamten** zu versteuernden
Einkommen **zuzüglich der dem Progressionsvorbehalt unterliegenden Einkünfte** zu bemessen wäre, **mindestens
jedoch 14 Prozent**". → die Rate wird auf dem VOLLEN zvE (inkl. VOLLER ao, NICHT 5Mio-gekappt) zzgl. Progression
gebildet, dann auf den GEKAPPTEN ao (min(ao,5Mio)) angewandt. Modul-Inputs: `bemessungsgrundlage_durchschnitt` =
VOLLES zvE zzgl. Progression (nicht gekappt); `est_gesamt_zzgl_progression` = tarifliche ESt auf ebendieses volle
zvE. Der 14%-Floor ist in S.2 verbatim. Modul-Formel (durchschnittssatz auf volle Basis × ao_gekappt) = korrekt,
sofern der Ring die Inputs VOLL (nicht gekappt) speist.

⚠ BOUNDARY-PUNKT (b) für Instructor-Hand-Check (Rechenbeispiel): est_excess-STACKING-Reihenfolge. Wenn der
Überschuss via Abs.1-Fünftel läuft (s. Excess-Korrektur oben), sitzt der ≤5Mio-ermäßigt-Teil in der Progression
UNTER dem Excess — die Stapel-Ordnung (verbleibendes zvE → +≤5Mio-Teil → +Excess-Fünftel) ändert die Grenz-/
Fünftel-Basis. Rechen-kritisch, nicht offensichtlich → Hand-Prüfung gegen amtliches Beispiel bei Materialisierung.

## Deklarations-Felder (unverändert aus Recon, bestätigt)
NEU: antrag_ermaessigter_satz (flag, opt-in → ohne = Abs.1-Default over-tax-safe) · dauernd_berufsunfaehig (flag) ·
ermaessigung_einmal_genutzt (flag, S.4). DERIVE Alter≥55 aus geburtsjahr (gebunden). REUSE
rentner_veraeusserungsgewinn als ao_einkuenfte. Ring-DERIVED: est_gesamt_zzgl_progression + bemessungsgrundlage.
S.5-Multi-VÄ = benannte Lücke (MVP skalar).

## Sequenz
Stufe-2 nach § 34 Abs. 1 (dev-1). Prerequisit: Abs.1-vs-Abs.3-Chooser (auf Antrag). nicht_echt-Adjudikation +
byte-gleiche Materialisierung + Ring-Split = JOINT bei Abs.3-Build, Instructor-Boundary-Review vorweg. Dieser
Report = Design-Lock-Prep, KEIN Write appliziert.
