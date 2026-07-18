# charge30 Materialisierung — §31 / §24a / §24b (3 High-Value KAT-1) — dev-2, 2026-07-18

**Status: gebaut + grün, freeze-ready.** + Triage-Befund §24a/§24b (faithful=false).

## Materialisierte Module (aus verified_bedingt-Snapshots, kein Re-Run)
| Datei | Module | Snapshot | catala_a byte-gleich | typecheck | Seeds |
|---|---|---|---|---|---|
| `rules/estg/p31/familienleistungsausgleich.catala_en` | Familienleistungsausgleich | p31_familienleistungsausgleich (faithful=True) | ✓ True | ✓ | 2/2 |
| `rules/estg/p24a/altersentlastungsbetrag.catala_en` | Altersentlastungsbetrag | p24a_altersentlastungsbetrag (faithful=false) | ✓ True | ✓ | 2/2 |
| `rules/estg/p24b/entlastungsbetrag.catala_en` | Entlastungsbetrag | p24b_entlastungsbetrag (faithful=false) | ✓ True | ✓ | 5/5 |

- §31 Günstigerprüfung: est_nach = est_ohne (Grundregel) / est_mit + kindergeld (exception wenn günstiger).
  Anker §31 "steuerliche Freistellung eines Einkommensbetrags in Höhe des Existenzminimums" (voll-Länge).
- §24a: min((prozentsatz/100)·(arbeitslohn+andere); hoechstbetrag). Anker §24a "Prozentsatz ermittelter
  Betrag des Arbeitslohns und der" (voll-Länge).
- §24b: alleinstehend + kinder → 4260 + 240·(kinder−1), monatsanteilig gekürzt. Anker §24b "allein stehenden
  Steuerpflichtigen ein Kind im Sinne" (voll-Länge).
- Export clerk.toml (include_dirs p31/p24a/p24b + [[target]]), Import verifiziert. golden 121/121 NO-OP,
  Suite 532 passed.

## ⚠ TRIAGE-BEFUND §24a/§24b (faithful=false) — GETRIAGED-ARTEFAKT, KEIN echter Defekt
Beide Snapshots: **faithful=false MIT abweichungen=[]** (keine Wert-Abweichung) + `bedingungen` = getriagte
Geltungsbedingungen. Exakt das §35a-Arbeitskosten-Artefakt-Muster ([[altfassung-aenderungsbefehl-judge-artefakt]]):
der Judge misst gegen die VOLLE Norm (inkl. der getriagten Bedingungen), die Teilregel nimmt diese als INPUT/
Andockung — daher faithful=false, aber KEINE echte Rechen-Abweichung (abweichungen=[]).
- **§24a (Altersentlastungsbetrag)** bedingungen: `prozentsatz_hoechstbetrag_aus_kohortentabelle` (Prozentsatz +
  Höchstbetrag kommen aus der Kohortentabelle nach Jahr der Vollendung des 64. Lj = INPUT), `bemessung_netto_
  nach_ausschluessen`, `alter_64_vor_bezugsjahr_vollendet` (Geltungsbedingung). → alle drei getriagt zu Input/
  Integration; die reine Prozentsatz×Bemessung-gedeckelt-Formel ist byte-gleich korrekt.
- **§24b (Entlastungsbetrag)** bedingungen: `alleinstehend_im_sinne_des_absatzes_3` (Def. Alleinstehend =
  Geltungsbedingung/Input-Flag), `kinder_mit_freibetrag_oder_kindergeld_im_haushalt`, `monate_ohne_voraussetzung_
  sind_volle_kalendermonate`. → getriagt; die Grundbetrag+Erhöhung−Kürzung-Formel ist byte-gleich korrekt.
**VERDIKT: byte-gleiche Materialisierung sicher** (kein Re-Run, kein Neubau). Die getriagten Bedingungen sind
Andock-Auflagen für dev-1s Accessor (Kohorten-Lookup §24a; Alleinstehend-Prüfung §24b), keine Regel-Defekte.

## Andock (dev-1-Accessor)
§31 → p32a hinzurechnung_kindergeld/freibetraege_kinder (Günstiger-Integration, 2 Tarif-Läufe est_ohne/est_mit).
§24a → p32a altersentlastungsbetrag (Z.466). §24b → p32a entlastungsbetrag_alleinerziehende (Z.467).

## Zur Freigabe
Instructor verifiziert Byte-Gleichheit + Seeds + golden. Triage §24a/§24b = getriagt-Artefakt bestätigt (sicher).
