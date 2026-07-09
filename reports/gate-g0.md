# Gate G0: Go/No-Go fuer Catala

Abschluss-Deliverable Phase 0. Bewertung der vier Gate-Kriterien mit Belegen.
Die Entscheidung trifft Julius, nicht diese Auswertung.

Stand: 2026-07-09. Toolchain: Catala/Clerk 1.2.0, OCaml 4.14.2, GETTSIM 1.2.1,
Python 3.12. Details in `docs/setup.md`.

## Kriterium 1: Compiler stabil genug (keine Blocker-Bugs im Spike-Umfang)?

**Erfuellt.** Catala 1.2.0 und Clerk liessen sich ueber opam sauber bauen und
haben den gesamten Spike-Umfang ohne Compiler-Absturz oder Blocker verarbeitet:
literate Files mit Modulen, Strukturen, Enums, Default/Exception-Logik,
Scope-Aufrufe ueber Modulgrenzen, Kompilation nach OCaml und nach Python.

Aufgetretene Reibungspunkte, alle ohne Workaround mit semantischer Abweichung
loesbar (siehe `docs/setup.md` und `reports/s03-ergonomie.md`):

- Cross-Modul-Scope-Aufrufe im Interpreter/Test brauchen `--whole-program`
  (`clerk test -W`).
- Modulname muss dem (grossgeschriebenen) Dateinamen entsprechen.
- `Decimal.floor`/`ceiling` der stdlib sind Cap-Funktionen; fuer die gesetzliche
  Abrundung wird `Decimal.truncate` verwendet.

Es waren Werkzeug- und Syntaxdetails, keine Compiler-Bugs.

## Kriterium 2: Python-Backend produziert korrekte, aufrufbare Artefakte?

**Erfuellt.** Das Modul `Einkommensteuertarif` wurde nach Python kompiliert
(`clerk build p32a-python`) und ist als Bibliothek aufrufbar
(`oracle/gettsim/harness.py` ruft `grundtarif` und `splittingtarif` mit
`Money`-Ein- und Ausgaben). Die Ausgaben stimmen mit dem Catala-Interpreter
ueberein (dieselben Testwerte, z. B. zvE 30 000 VZ 2026 -> 4 217 Euro) und
werden im Differentialtest ueber tausende Faelle gegen GETTSIM gehalten.

Randbedingung: die von Catala erzeugte Python-Runtime nutzt PEP-695-Syntax und
verlangt Python >= 3.12; dokumentiert und im 3.12-venv geloest.

## Kriterium 3: Default Logic bildet S0.3 ohne Verrenkungen ab?

**Erfuellt.** Siehe `reports/s03-ergonomie.md`. § 4 Abs. 5 Nr. 6b und 6c
inklusive des gegenseitigen Ausschlusses (Nr. 6c Satz 3) sind mit
`label`/`exception`/`under condition` fast eins zu eins zum Gesetzestext
abgebildet. 8 Testfaelle gruen. Der 2025 an der Graph-Modellierung gescheiterte
Ausschlussfall ist hier ergonomisch darstellbar.

## Kriterium 4: Differentialtest gruen oder alle Divergenzen erklaert?

**Erfuellt im Sinne von "alle Divergenzen erklaert".** Details in
`reports/s02-divergenzen.md`. Pro VZ 1000 geseedete zvE-Werte plus Randwerte,
Grundtarif und Splitting, Vergleich auf Euro-Ebene.

**Grundtarif (Absatz 1): praktisch deckungsgleich.** 1 bis 3 Divergenzen je VZ
ueber je rund 1000 Punkte, jede genau 1 Euro, an Zonen-Innenpunkten. Ursache:
die im publizierten Format (2 Nachkommastellen) angegebenen Tarifkoeffizienten
gegen GETTSIMs voll aufgeloeste Progressionsfaktor-Rekonstruktion. Bemerkenswert:
auch fuer VZ 2026, wo Catala die woertlichen Gesetzeskoeffizienten verwendet,
tritt genau eine solche Divergenz auf; dort weicht GETTSIM vom Gesetzeswortlaut
ab, Catala entspricht dem Wortlaut. Fuer VZ 2024/2025 (abgeleitete Koeffizienten)
bleibt der literale BGBl-Abgleich offen (siehe offene Punkte).

**Splitting (Absatz 5): systematische, vollstaendig erklaerte Divergenz.** Rund
570 bis 600 von je 1000 Faellen weichen um genau 1 Euro ab. Ursache ist eine
Rundungsinterpretation: das literale § 32a Abs. 5 i.V.m. Abs. 1 Satz 6 berechnet
`2 * abrunden(Tarif(Z/2))` und liefert stets gerade Euro-Betraege (so die
amtliche Splittingtabelle). GETTSIM rundet erst am Ende: `abrunden(2 * Tarif(Z/2))`.
Catala folgt dem Wortlaut. Nach Handover-Vorgabe wird der Widerspruch
dokumentiert und eskaliert, nicht stillschweigend aufgeloest.

## Offene Punkte (an Julius)

1. **Splitting-Rundung (§ 32a Abs. 5).** Bestaetigung, dass der Wortlaut
   (gerade Betraege, `2 * abrunden(Tarif(Z/2))`) gegenueber GETTSIMs Verfahren
   massgeblich ist. Abgleich gegen amtliche Splittingtabelle oder
   Programmablaufplan empfohlen. Catala ist aktuell auf den Wortlaut gesetzt.
2. **Literale Tarifkoeffizienten VZ 2024/2025.** Die geschlossenen Koeffizienten
   fuer 2024/2025 sind aus den GETTSIM-Zonenparametern abgeleitet (Methode gegen
   die VZ-2026-Literalfassung validiert). Ein literaler Abgleich gegen BGBl 2024
   I Nr. 386 (2024) und BGBl 2024 Nr. 449 (2025) steht noch aus.
3. **Abrundung des zvE auf volle Euro (§ 32a Abs. 1 S. 1).** Catala rundet das
   zvE ab, GETTSIM nicht. Im Testgrid (ganzzahlige Euro) nicht ausgeloest; bei
   nicht ganzzahligem zvE separat zu pruefen.

## Empfehlung

**Go fuer Catala.** Alle vier Kriterien sind erfuellt. Der Compiler traegt den
Spike-Umfang stabil, das Python-Backend ist als Oracle-Schnittstelle nutzbar,
die Default-Logik bildet die schwierige Ausnahmestruktur natuerlich ab, und der
Differentialtest zeigt am Grundtarif praktische Deckungsgleichheit mit GETTSIM.
Die verbleibenden Divergenzen sind vollstaendig erklaert und laufen auf zwei
steuerfachliche Klaerungen hinaus (Splitting-Rundung, literale Koeffizienten
2024/2025), nicht auf Werkzeugmaengel. Der Fallback (eigene Regel-IR) ist nicht
noetig.
