# Phase-1-Deliverable: Arbeitnehmerfall end-to-end

Bruttoarbeitslohn rein, festzusetzende ESt raus, entlang der Stufenfolge des § 2 EStG:

    Summe der Einkuenfte (§ 2 Abs. 3 Satz 1)
      -> Gesamtbetrag der Einkuenfte (§ 2 Abs. 3)
      -> Einkommen (§ 2 Abs. 4)
      -> zu versteuerndes Einkommen (§ 2 Abs. 5)
      -> tarifliche Einkommensteuer (§ 2 Abs. 5, § 32a)
      -> festzusetzende Einkommensteuer (§ 2 Abs. 6)


Oracle: die Pauschbetraege § 9a (1 230 Euro) und § 10c (36 Euro) stammen aus der Gesetzesfassung (GETTSIM als Prueinstanz), der Tarif aus GETTSIMs § 32a-Engine. MVP-Scope: nur Einkuenfte aus nichtselbstaendiger Arbeit, ohne Vorsorgeaufwendungen. Im MVP sind die Uebergaenge GdE/Einkommen/zvE bis auf den § 10c-Abzug Identitaeten.


## VZ 2024

Pauschbetraege (Gesetz, GETTSIM-Prueinstanz): § 9a = 1230 Euro, § 10c = 36 Euro.

- zvE-Ableitung (§ 9a Satz 2 + § 10c): 507/507 exakt gleich der GETTSIM-Parameter-Rechnung (Abweichungen: 0).

- Einzelveranlagung: 0 von 507 Faellen weichen in der festzusetzenden ESt ab (erwartet: nur die § 32a-Grundtarif-Approximation, je 1 Euro).

- Zusammenveranlagung: 287 von 507 Faellen weichen ab (erwartet: die dokumentierte Splitting-Rundung, § 32a Abs. 5, 1-2 Euro; Wortlaut = Catala).


## VZ 2025

Pauschbetraege (Gesetz, GETTSIM-Prueinstanz): § 9a = 1230 Euro, § 10c = 36 Euro.

- zvE-Ableitung (§ 9a Satz 2 + § 10c): 506/506 exakt gleich der GETTSIM-Parameter-Rechnung (Abweichungen: 0).

- Einzelveranlagung: 4 von 506 Faellen weichen in der festzusetzenden ESt ab (erwartet: nur die § 32a-Grundtarif-Approximation, je 1 Euro).

  - brutto 49338: zvE 48072, Catala 10014, GETTSIM-Tarif 10013 (+1)
  - brutto 53661: zvE 52395, Catala 11551, GETTSIM-Tarif 11550 (+1)
  - brutto 63552: zvE 62286, Catala 15316, GETTSIM-Tarif 15315 (+1)
  - brutto 69322: zvE 68056, Catala 17672, GETTSIM-Tarif 17671 (+1)
- Zusammenveranlagung: 303 von 506 Faellen weichen ab (erwartet: die dokumentierte Splitting-Rundung, § 32a Abs. 5, 1-2 Euro; Wortlaut = Catala).


## VZ 2026

Pauschbetraege (Gesetz, GETTSIM-Prueinstanz): § 9a = 1230 Euro, § 10c = 36 Euro.

- zvE-Ableitung (§ 9a Satz 2 + § 10c): 505/505 exakt gleich der GETTSIM-Parameter-Rechnung (Abweichungen: 0).

- Einzelveranlagung: 6 von 505 Faellen weichen in der festzusetzenden ESt ab (erwartet: nur die § 32a-Grundtarif-Approximation, je 1 Euro).

  - brutto 32888: zvE 31622, Catala 4678, GETTSIM-Tarif 4679 (-1)
  - brutto 35736: zvE 34470, Catala 5511, GETTSIM-Tarif 5512 (-1)
  - brutto 41925: zvE 40659, Catala 7418, GETTSIM-Tarif 7419 (-1)
  - brutto 64172: zvE 62906, Catala 15368, GETTSIM-Tarif 15369 (-1)
  - brutto 64384: zvE 63118, Catala 15452, GETTSIM-Tarif 15453 (-1)
- Zusammenveranlagung: 310 von 505 Faellen weichen ab (erwartet: die dokumentierte Splitting-Rundung, § 32a Abs. 5, 1-2 Euro; Wortlaut = Catala).


## Bewertung

Die zvE-Ableitung (§ 9a + § 10c) stimmt exakt mit der GETTSIM-Parameter-Rechnung ueberein. In der Einzelveranlagung schlaegt nur die bekannte § 32a-Grundtarif-Approximation durch (je 1 Euro, GETTSIM-seitig, Catala = Wortlaut). In der Zusammenveranlagung schlaegt die dokumentierte Splitting-Rundung durch (§ 32a Abs. 5; amtlich als Wortlaut bestaetigt, siehe reports/s02-divergenzen.md). Die Kette Bruttolohn -> festzusetzende ESt ist damit end-to-end erklaert und im MVP-Scope gruen.

