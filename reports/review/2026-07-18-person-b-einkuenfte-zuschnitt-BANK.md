# Zusammen + Person-B-Einkünfte (catala_gesamt-Scheiben) — GEBANKTER Zuschnitt (Recon-Teil)

**Status:** concept-first Recon PAUSIERT (UI-Politur hat Vorrang, Instructor msg 2823). Gebankt für nach-UI.
Kein Bau. Recon-Teil unten fertig; est_mapping-Klasse-g-Detail + Größe offen für Fortsetzung.

## Kern-Befund (Recon soweit)
- **Engine-Seite OK:** catala_gesamt_zusammen (FestzusetzendeEstGesamtZusammen, engine p32a Zeile 542-545)
  nimmt EINZELNE, KOMBINIERTE einkuenfte_*-Slots (einkuenfte_nichtselbststaendig „beider Ehegatten",
  einkuenfte_kapitalvermoegen, einkuenfte_vermietung, einkuenfte_sonstige). KEIN _a/_b-Split nötig — der
  Accessor summiert A+B in den EINEN Slot je Einkunftsart. Splitting-Tarif intern korrekt.
- **GAP = fehlende Person-B-Deklarations-Felder** (die den Accessor A+B summieren ließen):
  - §19 + VOR: _partner EXISTIERT schon (bruttoarbeitslohn_partner, vor_an/ag/rv_partner — an_gesamt).
  - §33b: disability _partner existiert (rentner_grad_der_behinderung_partner, _hilflos_partner).
  - **FEHLT: vv_*_partner (§21 V+V), kap_*_partner (§20 Kapital), renten_*_partner (§22 Rente).**
    → Ehepaar wo BEIDE V+V/Kapital/Rente haben: Person-B nicht summiert.
- **est_mapping Klasse g (PARTNER_INSTANZ):** deckt aktuell nur §19+VOR (4 Felder → Anlage-N-Instanz-B-Kz).
  Für V+V/Kapital/Rente Person-B analog erweitern (Person-B-Anlage-V/KAP/R-Instanz-Kz).

## Offen (Fortsetzung nach UI)
- (2) Feld-Liste je Scheibe: welche vv_*/kap_*/renten_* Person-B-Zwillinge genau (V+V ~5, Kapital ~4,
  Rente ~5 inkl. renten_art/beginn/alter/rentenfreibetrag_partner).
- (3) est_mapping Klasse g je Art (Person-B-Instanz-Kz E07xx/E19xx/E18xx — Hash/Vordruck-Beleg).
- (4) Größe: MITTEL — _partner-Felder + Klasse-g-Ausbau (meine Zone) + Accessor-A+B-Summierung je Scheibe
  (dev-1). K2: Person-B-Felder nur bei veranlagung==zusammen (partner_check-Muster erweitern).

## Muster-Vorwissen (aus §33b/Splitting)
- Person-B-Behinderung nutzt EIGENE distinkte Kz (E05058), NICHT Klasse-g-Reuse — bei V+V/Kapital/Rente
  prüfen ob Person-B eigene Anlage-Instanz-Kz hat (Anlage V/KAP/R je Ehegatte) → dann Klasse 1, sonst g.
- partner_check (konsistenz) erweitern: neue Person-B-Einkunfts-Felder gesetzt + veranlagung!=zusammen → Widerspruch.
