# Charge-5 Paket C — Zuschnitte § 10 Abs. 1 Nr. 5 + § 10 Abs. 1a

Vollabdeckung Charge 5, letztes Paket (schließt die restlichen Kind-/SA-Lücken). Stufe A, $0,
via skip-judge (DeepInfra-Judge geparkt). Quelle: Bestands-Freeze `estg_p10_2026-07-11.txt`
(ganzer § 10, beide Passagen verifiziert — kein neuer Freeze).

## § 10 Abs. 1 Nr. 5 — Kinderbetreuungskosten (p10_1_5_kinderbetreuung)

Wortlaut: „80 Prozent der Aufwendungen, höchstens 4 800 Euro je Kind, für Dienstleistungen zur
Betreuung eines … Kindes … welches das 14. Lebensjahr noch nicht vollendet hat …".

Rechenkern: `min(0,80 × aufwendungen, 4.800)`. Signatur: `aufwendungen money -> kinderbetreuung_abzug
money`. Präzision: 0,80 in decimal, Cent-Schnitt zuletzt (praezisions_lint). Selbsttragend (Formel-
Deckel, kein Konditional-Vorrang). Scope: Altersgrenze (< 14 / behindert), Haushaltszugehörigkeit =
§ 2-Integration/Anwendbarkeit; „je Kind" = die Regel rechnet je Kind, Aggregation upstream.
Seeds: aufw 3.000 → 2.400; aufw 6.000 → 0,80×6.000 = 4.800 (Deckel-Grenze); aufw 8.000 → 6.400 > 4.800
→ 4.800; aufw 0 → 0.

## § 10 Abs. 1a Nr. 1 — Realsplitting/Unterhalt an Ex-Ehegatten (p10_1a_realsplitting)

Wortlaut: „Unterhaltsleistungen an den geschiedenen oder dauernd getrennt lebenden … Ehegatten, wenn
der Geber dies mit Zustimmung des Empfängers beantragt, bis zu 13 805 Euro im Kalenderjahr" (S. 1);
Höchstbetrag erhöht um KV/PV-Beiträge (S. 2, analog § 33a).

Rechenkern: `min(unterhaltsleistungen, 13.805 + kv_pv_beitraege)`. Signatur: `unterhaltsleistungen
money, kv_pv_beitraege money -> realsplitting_abzug money`. Der 13.805-Höchstbetrag ist eine
Norm-Konstante im Wortlaut (steht im auszug → selbsttragend, kein Input nötig — anders als § 33a
grundfreibetrag, der aus § 32a kam). Scope: Antrag + Zustimmung des Empfängers = Anwendbarkeit
(§ 2-Integration); der Empfänger versteuert korrespondierend (§ 22 Nr. 1a, außerhalb).
Seeds: unterhalt 10.000/kv 0 → 10.000; unterhalt 15.000/kv 0 → 13.805 (Deckel); unterhalt 15.000/kv
2.000 → Deckel 15.805, min → 15.000; unterhalt 20.000/kv 0 → 13.805.

## Nach den Läufen
Beide skip-judge (Judge geparkt bis deepinfra-Erholung, dann gebündelter Nachzug mit den 4 Paket-A/B-
strukturgeprueft-Regeln). Landkarte: § 10 Nr. 5 → Kind 3/4, § 10 Abs. 1a → SA. Damit Charge 5 komplett;
AN-Kern-🟡-Lücken weitgehend geschlossen.
