# Offene Instructor-Fragen (2026-07-21, Correctness-Sweep-Runde 2)

Instructor-Session war beim Fund NICHT am Bus → hier geparkt, sobald sie connectet routen.

## §35-Mitu-Zähler (api.py:814 gesamt / :1081 rentner) — Over-tax-Verdacht
`p35_zaehler = max(0, laufender_gewinn) if gewinn_betriebsart=="gewerbe" else 0`.
`laufender_gewinn` enthält mitu (Mitunternehmer-Anteil), aber der Zähler wird 0,
wenn die DIREKTE Betriebsart ≠ "gewerbe". Ein reiner Mitunternehmer (mitu>0, kein
eigenes Gewerbe) verliert die §35-GewSt-Anrechnung komplett → Over-tax.
**Frage:** Ist der Mitunternehmer-Gewinn (§15 Abs.1 Nr.2) für den §35-Zähler IMMER
gewerbesteuerpflichtig einzubeziehen — oder gibt es Fälle (freiberufliche
Mitunternehmerschaft §18 Abs.4) wo mitu NICHT in den §35-Zähler gehört? Braucht der
Fix ein separates Betriebsart-Feld für mitu?

## §34c DBA-Anrechnung — Wiring (Plan fertig, scope-p34c-Report)
Engine-Slot `anzurechnende_auslaendische_steuern` existiert schon in
FestzusetzendeEstGesamt(-Zusammen), catala p34c_1/p34c_2 promoted+inert. gesamt+rentner
erreichbar, an_gesamt nicht. Bau ~1-1.5h.
- **Q1 (kritisch, Under-tax):** Muss der Fragetext klarstellen, dass
  `dba_auslaendische_einkuenfte` eine TEILMENGE der bereits erfassten Einkünfte ist
  (Welteinkommensprinzip, nur Höchstbetrags-Nenner)? Sonst Doppel-Nichterfassung/GdE-Lücke.
- **Q2:** §34c vs §35-GewSt-Reihenfolge bei gemeinsamem Vorkommen — Unabhängigkeit korrekt
  oder Gesetzes-Vorrang (§2 Abs.6)?
- **Q3:** Stufe-1 (nur Anrechnung) jetzt, Stufe-2 (Abzug-Wahlrecht §34c Abs.2 +
  Einkunftsart-Routing) als Folgeticket — freigeben?
- **Q4:** an_gesamt bewusst Backlog (Einzel-Scope hat den Slot nicht) — akzeptabel?

## Backlog (Julius: vormerken, nicht jetzt bauen)
- §34a Thesaurierungsbegünstigung — nirgends implementiert (neu).
- §34 Abs.2 Nr.2-4 Abfindungen/mehrjährige Vergütungen — ao nur auf §16-vg verdrahtet (neu).
