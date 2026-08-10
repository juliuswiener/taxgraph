# § 35a Haushaltsnahe Aufwendungen — XSD-Struktur der Einzelaufstellung

**Datum:** 2026-08-10
**Anlass:** `p35a-einzelaufstellung` (BACKLOG.yaml) — Abgabe-Blocker rc=610001002 auf allen drei
§35a-Töpfen (Minijob, Dienstleistungen, Handwerker), obwohl die Summenfelder korrekt gebunden
und deklariert sind.
**Status:** Struktur belegt, direkt im XSD gegengelesen. Keine Code-Änderung, reine Recherche.

## Befund: JEDER Topf ist ein Einz[99]+Sum[1]-Paar, kein Sonderfall

`~/02_Software/eric/.../Schema/2025/E10-2025.xsd`, Sektion `St_Erm_1426027020_CType`
(Zeilen 10026–10137 — verifiziert per Direktlektüre, nicht nur vom Recherche-Agent übernommen):

```xml
<xs:complexType name="St_Erm_1426027020_CType">
  <xs:sequence>
    <xs:element name="Minijobs"   type="Minijobs_m1586336596_CType"   minOccurs="0" maxOccurs="1"/>
    <xs:element name="Hhn_BV_DL"  type="Hhn_BV_DL_m1586336596_CType"  minOccurs="0" maxOccurs="1"/>
    <xs:element name="Handw_L"    type="Handw_L_m1586336596_CType"    minOccurs="0" maxOccurs="1"/>
    ...
  </xs:sequence>
</xs:complexType>
```

Alle drei Container-Typen folgen demselben Muster — EIN `Einz`-Element mit `maxOccurs="99"`
(die Einzelaufstellung) plus EIN `Sum`-Element mit `maxOccurs="1"` (die Summe, das ist unser
heute gebundenes Kz):

| Topf | Container | `Einz` (maxOccurs=99) → Kz je Zeile | `Sum` (maxOccurs=1) → Kz |
|---|---|---|---|
| Minijob | `Minijobs_m1586336596_CType` | `Einz_m1752952991_CType`: **E0104206** (Art der Tätigkeit, String) + **E0104108** (Betrag) | `Sum_m1752952991_CType`: **E0104109** |
| Dienstleistungen | `Hhn_BV_DL_m1586336596_CType` | `Einz_2025170028_CType`: **E0107206** (Art der Tätigkeit/Aufwendungen, String) + **E0107207** (Aufwendungen) | `Sum_2025170028_CType`: **E0107208** |
| Handwerker | `Handw_L_m1586336596_CType` | `Einz_m1590406119_CType`: **E0111217** (Art der Aufwendungen, String) + **E0170601** (Rechnungsbetrag) + **E0111214** (darin Lohnanteile) | `Sum_m1590406119_CType`: **E0111215** |

`E0104109`/`E0107208`/`E0111215` sind exakt die drei Kz, die heute in
`produkt/bindung/bindung_sonder_agb_35a.yaml:189,206,223` gebunden sind (`hh_minijob_aufwendungen`,
`hh_dienstleistungen`, `hh_handwerker_arbeitskosten` — verifiziert per Direktlektüre). Wir füllen
in allen drei Töpfen nur `Sum`, nie ein `Einz`-Element — das erklärt rc=610001002 direkt und
identisch für alle drei.

**Korrektur der Ausgangsvermutung** (aus dem Vordruck `sources/bfinv/haushaltsnahe_2025.txt`
abgeleitet): Zeilen 6–8 (Handwerker) zeigen im Papierformular keine Kz-Zahl neben den drei
Beispielzeilen, was wie ein Freitext-/Beleg-Container ohne eigene Kz aussah. Das XSD widerlegt
das — die Einzelposten-Felder tragen sehr wohl eigene Kz (s. Tabelle). Der Vordruck druckt sie
nur nicht ab, weil Papier-Formularfelder keine Kz-Beschriftung tragen (Kz sind ein
ELSTER-internes Konzept, keine Vordruck-Beschriftung — das ist Konvention im ganzen Formularsatz,
nicht §35a-spezifisch). Auch Minijob (Zeile 4) und Dienstleistungen (Zeile 5), die im Papierformular
nur EINE Box zeigen, haben im XSD dieselbe `Einz[99]`-Struktur — die elektronische Übermittlung
erlaubt dort ebenso mehrere Einträge, auch wenn das Papier nur eine Zeile vorsieht.

## Codebase-Präzedenz: der Mechanismus existiert bereits, §35a ist nur noch kein Konsument

Repeated-Instance ist kein Neubau — der Kern läuft produktiv für drei andere Anlagen:

- `produkt/mapping/est_mapping.py:294`: `_INSTANZ_RE = re.compile(r"^(?P<base>[a-z][a-z0-9_]*)__(?P<idx>[1-9][0-9]*)$")`.
  Instanz 1 = Basis-`feld_id` unverändert, Instanz 2..N = `base__<n>`. Kein `#`, kein neues
  Store-Zeichen — das Suffix bleibt im bestehenden `feld_id`-Pattern (Kommentar Z. 287–293
  begründet das explizit).
- Bindungs-Property `instanz_gruppe` (z. B. `produkt/bindung/bindung_kap_vv_familie.yaml:244`
  `vv_einnahmen` → `instanz_gruppe: vv_objekt`, verifiziert per Direktlektüre) markiert, welche
  Basis-Kz instanzfähig ist.
- Ergebnis-Bucket `anlage_instanzen: {gruppe: [{index, felder{kz: wert}, dokumentiert}]}`,
  `est_mapping.py:324` (`_deklariere_instanz`) — **Kz-Reuse je Instanz**, kein neuer Kz pro
  Instanz (bestätigt Kz-Instanz-Recon 2026-07-18 laut Kommentar Z. 292–293: "alle drei Anlagen
  V/R/Kind = Reuse, kein distinkter Instanz-Kz").
- Bisherige Konsumenten: `vv_objekt` (Anlage V, Vermietung/Verpachtung), `kind` (Anlage Kind,
  `bindung_kap_vv_familie.yaml:514`), Multi-Rente. §35a/Haushaltsnahe ist **nicht** in dieser
  Liste — wäre ein vierter Konsument desselben, bereits gebauten Kerns.
- Design-Dokumente: `reports/review/2026-07-18-repeated-instance-kern.md`,
  `reports/review/2026-07-18-store-modell-ZUSCHNITT.md`. Tests: `tests/test_instanz_kern.py`.

## Anleitung-PDF: keine zusätzliche Aussage

`sources/bfinv_raw/014_Anleitung_Anlage_Haushaltsnahe-Aufwendungen_2025.pdf` (2 Seiten) erklärt
zu Zeile 4–9 nur Kürzungsregeln (Erstattungen, Pflegegeld) und allgemeine Belegpflicht ("Bitte
reichen Sie entsprechende Belege in Kopie ein, wenn Sie von Ihrem Finanzamt dazu aufgefordert
werden."). Keine explizite Aussage zu "gesonderte Aufstellung", zur Anzahl möglicher Zeilen über
die drei gedruckten hinaus, oder zu einem elektronischen Mehrzeilen-Modus. Die Itemisierungspflicht
ist ausschließlich aus der XSD-`Einz[99]`-Struktur und der checkESt-Ablehnung ableitbar, **nicht**
aus der Anleitung selbst. Nicht bestätigt, nicht geraten — für den Bau reicht das XSD als
Primärquelle.

## Für den Bau (nicht Teil dieses Reports, nur die Stichworte)

1. Drei neue `instanz_gruppe`-Basisfelder (Art-der-Tätigkeit/Aufwendungen-Text + Betrag je Zeile,
   Handwerker zusätzlich Rechnungsbetrag getrennt von Lohnanteil) je Topf, analog `vv_einnahmen`.
2. Kz-Zuordnung pro Instanz-Feld: `E0104206`/`E0104108` (Minijob), `E0107206`/`E0107207`
   (Dienstleistungen), `E0111217`/`E0170601`/`E0111214` (Handwerker) — alle oben verifiziert.
3. Die drei heutigen Summenfelder bleiben (`Sum`-Kz), müssen aber vermutlich aus den Instanzen
   berechnet statt vom Nutzer direkt erfragt werden — sonst kann Summe und Einzelaufstellung
   auseinanderlaufen (Präzedenzfrage, nicht in diesem Report entschieden).

## Quellen

- `~/02_Software/eric/doc_extract/ERiC-44.2.4.0/Dokumentation/Datenarten/ElsterErklaerung/ESt/Schema/2025/E10-2025.xsd`,
  Zeilen 10026–10137 (per Direktlektüre verifiziert, nicht nur Agent-Zitat)
- `produkt/bindung/bindung_sonder_agb_35a.yaml:178–228`
- `produkt/mapping/est_mapping.py:285–330`
- `produkt/bindung/bindung_kap_vv_familie.yaml:243–244`
- `sources/bfinv/haushaltsnahe_2025.txt` (amtlicher Vordruck)
- `sources/bfinv_raw/014_Anleitung_Anlage_Haushaltsnahe-Aufwendungen_2025.pdf`
