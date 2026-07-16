# KStG-Nachträge Stufe-A-Landkarte (Paket 7: Zinsschranke, Verlustuntergang)

Autor: taxgraph-instructor, 2026-07-16. Freigabe: Julius ("wir machen also A,B und D" — B).

## Gültigkeitslage

- Freezes GII-Stand 2026-07-16: `estg_p4h`, `kstg_p8a`, `kstg_p8c`, `kstg_p8d` (je *_2026-07-16).
- § 4h in der Fassung Kreditzweitmarktförderungsgesetz (ATAD-Anpassung), anzuwenden ab
  VZ 2024 → **einfassig VZ 2024–2026**. Freigrenze 3 Mio. €, 30 % verrechenbares EBITDA,
  Zins- und EBITDA-Vortrag im Wortlaut.
- § 8c heutige Fassung: NUR >50 %-Erwerb = schädlich (die alte quotale 25–50 %-Stufe ist
  seit 2019 rückwirkend gestrichen — keine Altfassungs-Charge nötig). Sanierungsklausel
  (Abs. 1a) + Stille-Reserven-Klausel im Freeze.
- § 8d unverändert seit 2016 → einfassig.

## Kern-Zuschnitt

```
N1  § 4h Abs. 1 Kern: abziehbar = zinsertrag + min(zinssaldo, 30 % · ebitda_basis)
    zinssaldo = zinsaufwand − zinsertrag; nichtabziehbar → zinsvortrag (Fortschreibung
    = Nachtrag; zins_vortrag/ebitda_vortrag als INPUTS, p10d_2-bestand-Muster)
N2  § 4h Abs. 2 Ausnahmen als GELTUNGSBEDINGUNGEN + Freigrenze:
    a) zinssaldo < 3.000.000 (FREIGRENZE, nicht Freibetrag — Kante 2.999.999/3.000.000!)
    b) keine/anteilige Konzernzugehörigkeit  c) Escape (EK-Vergleich)
    b/c als Bedingungs-Inputs (bool), KEINE Konzern-Bilanz-Logik
N3  § 8a-Brücke: maßgebliches EINKOMMEN statt Gewinn (KapGes-Basis), Abs. 2/3
    Gesellschafter-Fremdfinanzierungs-Details = Nachtrag
N4  § 8c: schaedlicher_erwerb (bool-Input, > 50 % in 5 Jahren) → verlustbestand_nach_8c
    = 0 sonst unverändert; Stille-Reserven-Klausel + Sanierungsklausel (Abs. 1a) =
    benannte Nachträge (Inputs vorab qualifiziert)
N5  § 8d: antrag_8d (bool) + fortfuehrungs_voraussetzungen (bool) → § 8c-Suspension,
    Bestand wird fortführungsgebunden (Kennzeichnung, kein eigener Rechenpfad Stufe A)
N6  Verdrahtung + Goldens: § 8c/§ 8d wirken auf verlustbestand VOR K5-§ 10d-Abzug;
    § 4h-Add-back ERHÖHT den § 8-Abs.-1-Slot-Gewinn: Baseline gewinn_estg hat Zinsen
    VOLL abgezogen, der nichtabziehbare Teil wird hinzugerechnet (slot_eff =
    gewinn_estg + nichtabziehbare_zinsen). [Korrigiert n. dev-1-Rückfrage — die
    frühere Formulierung "mindert den Slot-Gewinn" war ein Vorzeichen-Lapsus.]
    Pflicht-Goldens: Freigrenzen-Kante (2.999.999 vs. 3.000.000 → Alles-oder-nichts),
    30 %-Deckel bindend/nicht bindend, § 8c-Untergang vor § 10d, § 8d-Suspension.
```

## Chargen-Caps (Startschwelle ≥ $0,15 mehrquellig)

| Charge | Cap | Bemerkung |
|---|---|---|
| N1 | $0,23 | bedingungsreich (Vorträge, Saldo-Logik) |
| N2 | $0,15 | Freigrenzen-Kante = Judge-Flag-Kandidat (Alles-oder-nichts) |
| N3 | $0,15 | 2-quellig (§ 8a + § 4h) |
| N4 | $0,15 | Untergang binär |
| N5 | $0,15 | Suspension binär |
| N6 | $0 | Golden/Runner, LLM-frei, Triangulation |

Paket-Cap-Vorschlag **$1,00**.

## Judge-Artefakt-Erwartungen

- N1/N2: Zuschnitts-Artefakte (Abs. 3-Definitionen, Abs. 4-6 Vortrags-Fortschreibung
  weggelassen) → faithful=False leere Liste möglich.
- N2-Freigrenze: Judge könnte "Freibetrag statt Freigrenze" flaggen — Wortlaut-Beweis
  ("erreicht oder übersteigt" bzw. "weniger als drei Millionen") ist die Widerlegung.
- Verfahren unverändert: STOPP + Meldung, Instructor-Adjudikation vorab.

## Organschaft (§§ 14 ff.): GEPARKT

Empfehlung an Julius (Standing-Delegation angewandt): NICHT in Paket 7 — eigenes
Großpaket (GAV-Voraussetzungen, Einkommenszurechnung, Mehr-/Minderabführungen,
Ausgleichszahlungen) mit wenig Nutzen ohne Konzern-Scope. Nur auf explizites
Julius-Wort. Kein Freeze angelegt.
