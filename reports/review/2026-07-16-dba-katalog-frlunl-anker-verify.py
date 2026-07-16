"""Voll-Länge-Anker-Verifikation FR/LU/NL DBA-Kataloge (Paket 3, Auflage 1).

Importiert `gates._normalize` direkt (kein Nachbau) und prüft jeden Katalog-Anker
der drei Reports (2026-07-16-dba-katalog-{fr,lu,nl}.md) gegen den jeweiligen Freeze
in sources/dba/. Ausgabe je Anker OK/FEHLT (Zeichenzahl).

DE/NL- bzw. DE/FR-Interleave + versetzte BGBl-Kopfzeilen: jeder Anker MUSS aus einem
zusammenhängenden deutschen Block stammen. Ein Anker über die Spalten-/Seitengrenze
verschwindet nach _normalize (Kopfzeile dazwischen) und meldet FEHLT.

NEGATIVTEST am Ende: ein bewusst verfälschter Anker MUSS FEHLT liefern, sonst ist das
Gate wirkungslos (grün ohne Beweiskraft). Exit != 0, falls ein Positiv-Anker FEHLT
ODER der Negativtest fälschlich OK meldet.

Lauf:  python reports/review/2026-07-16-dba-katalog-frlunl-anker-verify.py
"""
import sys
sys.path.insert(0, "pipeline")
from gates import _normalize

DBA = "sources/dba"


def _norm_freeze(freeze_file):
    return _normalize(open(f"{DBA}/{freeze_file}", encoding="utf-8").read())


def check(freeze_file, anchors):
    norm = _norm_freeze(freeze_file)
    ok = True
    print(f"\n=== {freeze_file} ===")
    for label, anchor in anchors:
        hit = _normalize(anchor) in norm
        if not hit:
            ok = False
        print(f"  [{'OK  ' if hit else 'FEHLT'}] {len(anchor):3d}z  {label}: {anchor[:58]}...")
    return ok


# ------- Anker je Freeze (identisch zu den drei Katalog-Reports) -------------
SUITE = {
    "dba_lu_abkommen_2012.txt": [
        ("b Anrechnung (base, unverändert)",
         "wird unter Beachtung der Vorschriften des deutschen Steuerrechts über die Anrechnung ausländischer Steuern die Steuer Luxemburgs angerechnet"),
        ("c Aktivitätsvorbehalt (base)",
         "Statt der Bestimmungen des Buchstabens a sind die Bestimmungen des Buchstabens b anzuwenden auf Einkünfte im Sinne der Artikel 7 und 10"),
        ("d Progressionsvorbehalt (base)",
         "von der deutschen Steuer ausgenommenen Einkünfte und Vermögenswerte bei der Festsetzung ihres Steuersatzes zu berücksichtigen"),
    ],
    "dba_lu_protokoll_2023.txt": [
        ("a Freistellung (Protokoll, neu gefasst)",
         "Von der Bemessungsgrundlage der deutschen Steuer werden die Einkünfte aus Luxemburg sowie die in Luxemburg gelegenen Vermögenswerte ausgenommen, die nicht unter Buchstabe b fallen"),
        ("Schachtel >=10% (Protokoll)",
         "deren Kapital zu mindestens 10 Prozent unmittelbar der deutschen Gesellschaft gehört"),
        ("f Rückfall/subject-to-tax (Protokoll, ex-e)",
         "Ungeachtet der Bestimmungen des Buchstabens a wird die Doppelbesteuerung durch Steueranrechnung nach Buchstabe b vermieden, soweit"),
        ("Bagatell Art.14 Abs.1a 34-Tage (Protokoll)",
         "wenn die Arbeit an weniger als 35 Arbeitstagen im Kalenderjahr jeweils ganz oder teilweise im erstgenannten Staat"),
    ],
    "dba_nl_abkommen_2012.txt": [
        ("a Freistellung Satz1 (base, contig-Stück)",
         "Von der Bemessungsgrundlage der deutschen Steuer werden die Einkünfte aus den Niederlanden ausgenommen, die"),
        ("a subject-to-tax (base, contig)",
         "nach diesem Abkommen tatsächlich in den Niederlanden besteuert werden und nicht unter Buchstabe b fallen"),
        ("Schachtel >=10% (base)",
         "deren Kapital zu mindestens 10 Prozent unmittelbar der deutschen Gesellschaft gehört"),
        ("b Anrechnung (base)",
         "wird unter Beachtung der Vorschriften des deutschen Steuerrechts über die Anrechnung ausländischer Steuern die niederländische Steuer angerechnet"),
        ("c Aktivitätsvorbehalt (base)",
         "Statt der Bestimmungen des Buchstabens a sind die Bestimmungen des Buchstabens b anzuwenden auf Einkünfte im Sinne der Artikel 7 und 10"),
        ("d Progressionsvorbehalt (base)",
         "von der deutschen Steuer ausgenommenen Einkünfte bei der Festsetzung ihres Steuersatzes zu berücksichtigen"),
    ],
    "dba_nl_protokoll_2021.txt": [
        ("Art.22 Abs.1b Enum-Tweak (Prot 2021)",
         "In Artikel 22 Absatz 1 Buchstabe b des Abkommens"),
    ],
    "dba_nl_protokoll_2025.txt": [
        ("a Freistellung VZ2026 (Prot 2025, neu, contig)",
         "Von der Bemessungsgrundlage der deutschen Steuer werden die Einkünfte aus den Niederlanden ausgenommen, die nach diesem Abkommen tatsächlich in den Niederlanden besteuert werden und nicht unter Buchstabe b fallen"),
        ("Bagatell Art.14 Abs.1a 35-Tage VZ2026 (Prot 2025)",
         "wenn die unselbständige Arbeit an weniger als 35 Arbeitstagen im Kalenderjahr ganz oder teilweise im erstgenannten"),
    ],
    "dba_fr_zusabk_2015.txt": [
        ("Art.20 Abs.1a Freistellung (ZusAbk, neu)",
         "die Einkünfte aus Frankreich sowie die in Frankreich gelegenen Vermögenswerte ausgenommen, die nach diesem Abkommen in Frankreich besteuert werden können"),
        ("Art.20 Abs.1c Anrechnung (ZusAbk, neu)",
         "wird unter Beachtung der Vorschriften des deutschen Rechts über die Anrechnung ausländischer Steuern auf die deutsche Steuer angerechnet"),
        ("Art.20 Abs.1c Anrechnungs-Enum (ZusAbk)",
         "auf die unter Artikel 7 Absatz 4, Artikel 11, Artikel 13 Absatz 6 und Artikel 13 b fallenden Einkünfte"),
        ("Art.20 Abs.1d Umschaltklausel (ZusAbk, neu)",
         "Doppelbesteuerung durch Steueranrechnung nach Buchstabe c vermieden, wenn die Bundesrepublik gegenüber Frankreich auf diplomatischem Weg andere Einkünfte notifiziert"),
        ("Art.13 Abs.5a Grenzgänger (ZusAbk, neu)",
         "können Einkünfte aus nichtselbständiger Arbeit von Personen, die im Grenzgebiet eines Vertragsstaats arbeiten und ihre ständige Wohnstätte"),
        ("Art.13 Abs.5a Grenzgänger Zuweisung (ZusAbk)",
         "im Grenzgebiet des anderen Vertragsstaats haben („Grenzgänger“), nur in diesem anderen Staat besteuert werden"),
        ("Art.13a Fiskalausgleich 1,5% (ZusAbk, neu)",
         "Diese Entschädigung wird auf 1,5 vom Hundert der gesamten Bruttojahresvergütungen der Grenzgänger festgelegt"),
    ],
}

positiv_ok = all(check(f, a) for f, a in SUITE.items())

# ---------------- NEGATIVTEST -----------------------------------------------
# 1) Verfälschter Anker (Zahl gekippt) MUSS FEHLT liefern.
# 2) Über die Seitengrenze gespannter NL-Anker (Satz 1 + Fortsetzung, Kopfzeile
#    dazwischen) MUSS im Base-Freeze FEHLT liefern — beweist die Interleave-Falle.
print("\n=== NEGATIVTEST (müssen FEHLEN) ===")
neg = [
    ("dba_lu_protokoll_2023.txt", "Zahl gekippt: 20 statt 10 Prozent",
     "deren Kapital zu mindestens 20 Prozent unmittelbar der deutschen Gesellschaft gehört"),
    ("dba_nl_abkommen_2012.txt", "über Seitengrenze gespannt (Kopfzeile dazwischen)",
     "werden die Einkünfte aus den Niederlanden ausgenommen, die nach diesem Abkommen tatsächlich"),
]
neg_ok = True
for f, label, anchor in neg:
    hit = _normalize(anchor) in _norm_freeze(f)
    # erwartet: NICHT gefunden -> gut
    if hit:
        neg_ok = False
    print(f"  [{'FEHLT (gut)' if not hit else 'OK (SCHLECHT!)'}] {label}")

print("\n" + "=" * 60)
gesamt = positiv_ok and neg_ok
print("GESAMT:", "ALLE OK + Negativtest greift" if gesamt
      else "FEHLER — Positiv-Anker fehlt oder Negativtest greift nicht")
sys.exit(0 if gesamt else 1)
