# KAP Stufe 2 — muss einbehaltene KapESt/SolZ/KiSt erklärt werden? JA.

**Datum:** 2026-08-10
**Auftrag:** Klären ohne Bau — Team-lead maß E1904701/E1904901/E1904801 (Zeilen 37–39,
einbehaltene KapESt/SolZ/KiSt) als ungebunden, checkESt rc=0 trotzdem. Frage: Anrechnungsvoraussetzung
oder holt das Finanzamt es aus Bankmeldungen?
**Ergebnis:** Erklärungspflicht bestätigt. Ohne Zeilen 37–39 keine Anrechnung — bei
Günstigerprüfung zahlt der Nutzer die einbehaltene Abgeltungsteuer real ein zweites Mal.
**Status:** offen, Stufe 2 bereit zum Bau (Freigabe ausstehend).

## Kernfrage: Erklärungspflicht bestätigt (nicht automatisch)

Drei unabhängige Belege, alle in dieselbe Richtung:

**1. `040_Anleitung_Anlage_KAP_2025.pdf`, S. 1** — explizite Bedingung für Anrechnung/Erstattung:

> „Füllen Sie die Anlage KAP bitte **stets** auch aus, wenn Ihr Finanzamt einbehaltene inländische
> Kapitalertragsteuer, einbehaltenen Solidaritätszuschlag, einbehaltene Kirchensteuer im
> Zusammenhang mit anderen Einkunftsarten **anrechnen oder erstatten soll**."

Kein Konjunktiv, kein Vorbehalt — Ausfüllen ist Bedingung für die Anrechnung, nicht Kür.

**2. `040_Anleitung_Anlage_KAP_2025.pdf`, S. 2, Abschnitt „Zeile 4 Günstigerprüfung"** — konkret
für den Günstigerprüfungs-Fall (das ist der Fall des Nutzers, `kap_antrag_guenstigerpruefung`
steht bereits, Stufe 1 seit 2026-08-10):

> „Die entsprechenden Steuerabzugsbeträge tragen Sie bitte in die Zeilen 37 bis 42 … ein."

Direkte Handlungsanweisung, keine Kann-Formulierung. Und getrennt davon, S. 2 oben:

> „Haben Sie in Zeile 4 die Günstigerprüfung … beantragt? Dann müssen Sie die **Steuerbescheinigung**
> nur auf Anforderung Ihres Finanzamts einreichen."

Das entlastet nur die **Beleg-Vorlage** (Nachweis-PDF), nicht die **Betrags-Eintragung**. Die
Zahlen selbst müssen in jedem Fall in die Zeilen 37–42.

**3. § 36 Abs. 2 Nr. 2 EStG** (`sources/gesetze-im-internet/estg_p36_2026-07-11.txt`):

> „Auf die Einkommensteuer werden angerechnet: … 2. die durch Steuerabzug erhobene
> Einkommensteuer, soweit sie entfällt auf a) die **bei der Veranlagung erfassten** Einkünfte …"

Anrechnung hängt an "bei der Veranlagung erfasst" — d. h. an dem, was in der Erklärung steht, nicht
an einem aus Bankmeldedaten rekonstruierten Datensatz. Es gibt (anders als bei der
Lohnsteuerbescheinigung nach § 41b EStG, die der Arbeitgeber **elektronisch an das Finanzamt**
übermitteln muss und die vorausgefüllt wird) **keine gesetzliche Pflicht der Bank, den KapESt-Abzug
individuell dem Finanzamt des Steuerpflichtigen elektronisch zu melden**. Die Bank führt die
KapESt anonymisiert über die KapESt-Anmeldung ab; die Zuordnung zum einzelnen Steuerfall läuft
ausschließlich über die Steuerbescheinigung, die der Steuerpflichtige selbst hält und (auf
Verlangen) vorlegt. Genau deshalb betont § 36 Abs. 2 Nr. 2 Satz 2 die Bescheinigung als
Anrechnungsvoraussetzung, statt einen Amtsermittlungs-Automatismus vorauszusetzen.

**Fazit:** Kein Automatismus. Ohne erklärte Zeilen 37–39 rechnet das Finanzamt die einbehaltene
Steuer nicht an. checkESt rc=0 heißt nur „strukturell zulässig ohne diese Angabe" (Anlage KAP OHNE
Steuerabzugsbeträge ist ein valider Vordruck-Zustand, z. B. bei ausländischen Erträgen ohne
inländischen Steuerabzug) — nicht „inhaltlich vollständig". Team-lead-Hypothese (real over-tax bei
Günstigerprüfung) bestätigt: 5.000 EUR Kapitalerträge, ~1.250 EUR einbehalten, unsere Erklärung
versteuert die Erträge (tariflich oder mit 25 %) erneut, ohne den bereits gezahlten Betrag
gegenzurechnen — die ~1.250 EUR sind schlicht weg.

Aktueller Code-Stand bestätigt die Lücke: `produkt/haut/api.py` bindet nur
`kap_antrag_guenstigerpruefung` (E1900401, Zeile 4) und `kap_sparer_pauschbetrag_genutzt`
(E1901401, Zeile 16/17) — siehe `api_constants.py:119` (`KAP_ANTRAG_FELDER`). Keine Spur von
Zeile 37–39 irgendwo in `produkt/` oder `pipeline/`.

## XSD-Nebenfrage geklärt: zweite Kz-Serie gehört zu Anlage KAP-BET, nicht zur Hauptanlage

Die im Vorreport offene Frage („sechs Kz im Schema, drei im Vordruck") war ein Vordruck-Fehlgriff,
kein Schema-Widerspruch. Direkter XSD-Strukturbeweis
(`~/02_Software/eric/.../ERiC-44.2.4.0/.../Schema/2025/E10-2025.xsd`):

```
Zeile 19919: <xs:complexType name="KAP_BET_67907_CType">
Zeile 19927:   <xs:element name="St_Abz_Betr_Ert_m_o_inl_StAbz" type="St_Abz_Betr_Ert_m_o_inl_StAbz_1800123897_CType" .../>
Zeile 20075: <xs:complexType name="St_Abz_Betr_Ert_m_o_inl_StAbz_1800123897_CType">
Zeile 20077:   <xs:element name="E1904702" .../>   <!-- Kapitalertragsteuer -->
              ... E1904802, E1904902, E1905002, E1905102, E1905202
```

E1904702/…/E1905202 hängt strukturell **unter** `KAP_BET_67907_CType` — das ist der Container für
die separate **Anlage KAP-BET** (`xs:element name="KAP_BET" minOccurs="0" maxOccurs="2"`,
Zeile 8292), nicht die Hauptanlage KAP. Kreuzcheck gegen `Anlage_KAP_BET_2025.pdf` bestätigt: die
KAP-BET hat einen eigenen Block „Steuerabzugsbeträge zu Erträgen in den Zeilen 8 bis 24" (Zeilen
29–34, Kz 290/490 bis 295/495) mit denselben sechs Bezeichnungen (KapESt, SolZ, KiSt, Angerechnete
ausl. Steuer, Anrechenbare noch nicht angerechnete ausl. Steuer, Fiktive ausl. Quellensteuer) —
6 Felder im Schema, 6 Felder im (richtigen) Vordruck. Kein Widerspruch, kein `m_o` = "mit oder
ohne"-Rätsel — schlicht die falsche Anlage zum Vergleich herangezogen im Vorreport.

Zur Vollständigkeit: die Hauptanlage-KAP-Zeilen 43–45 (3 Felder, Kz 286/486–288/488,
„Anzurechnende Steuern zu Erträgen in den Zeilen 28 bis 34") sind eine **dritte** Kz-Serie,
E1905502/E1905602/E1905702, liegend in derselben `St_Abz_Betr_Inl_u_Inv_Ert_2368107_CType` wie
Zeilen 37–42 (also Hauptanlage, nicht KAP-BET).

**Für Stufe 2/3 relevant:** E1904701/E1904901/E1904801/E1905001/E1905101/E1905201 (Zeilen 37–42)
liegen strukturell eindeutig in der Hauptanlage KAP, nicht in KAP-BET — Anlage KAP-BET ist für
diesen Task irrelevant (separates Datenmodell, im aktuellen Produkt nicht abgebildet, kein
Scope-Beitrag).

## Empfehlung (kein Bau, nur Vorschlag)

1. **Stufe 2** — Zeilen 37–39 (E1904701 KapESt, E1904901 SolZ, E1904801 KiSt) verbindlich bauen.
   Größter Hebel, behebt den Doppelabzug bei Günstigerprüfung.
2. **Stufe 3** — Zeile 41 / `q` (E1905101, „Anrechenbare noch nicht angerechnete ausländische
   Steuer", § 32d Abs. 1 S. 5) danach, kleinerer Betrag, wie im Vorreport skizziert.
3. Zeile 40 (E1905001, bereits angerechnete ausl. Steuer) und Zeile 42 (E1905201, fiktive
   Quellensteuer) sind Rand-Tatbestände (auslandsbezogen, selten) — außerhalb der beiden
   priorisierten Stufen, kein akuter Geldfehler bekannt.

## Quellen

- `sources/bfinv_raw/040_Anleitung_Anlage_KAP_2025.pdf` (S. 1–2)
- `sources/bfinv_raw/Anlage_KAP_2025.pdf` (S. 3, Zeilen 37–45)
- `sources/bfinv_raw/Anlage_KAP_BET_2025.pdf` (S. 2, Zeilen 29–37)
- `sources/gesetze-im-internet/estg_p36_2026-07-11.txt` (Abs. 2 Nr. 2)
- `~/02_Software/eric/doc_extract/ERiC-44.2.4.0/Dokumentation/Datenarten/ElsterErklaerung/ESt/Schema/2025/E10-2025.xsd`
  (Zeilen 19384, 19785–19919, 19919–20077+)
- `reports/adjudikation/anlage-kap-kz-zuordnung-2026-08-10.md` (Vorreport, Stufe-1-Zuordnung)
- `produkt/haut/api_constants.py:119`, `produkt/haut/api.py:2549`/`2585` (aktueller Stand: nur
  Zeile 4/16-17 gebunden)
