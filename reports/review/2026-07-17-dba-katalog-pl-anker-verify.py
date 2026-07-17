#!/usr/bin/env python3
"""W1-PL-DBA-Katalog Anker-Verifikation (voll-Länge via gates._normalize).

EIN Freeze dba_pl_abkommen_2003 (OCR deutsche Spalte, Bildscan, S. 1318/1319
bildverifiziert), einfassig. Anker aus der deutschen OCR-Spalte, Silbentrennungs-
Hyphene exakt wie OCR ('gele- genen'). OCR-ARTEFAKTE gemieden: Spalten-Bleed-
Einzelzeichen am Zeilenende ('a','b','C)','d','si','P','W') + '$'/'$&' statt
Paragraph-Zeichen in der AStG-Passage. Muster GB/DK. Nachnutzung der 8
Instructor-Anker (meta erwartete_anker) + aa/c-AStG ergänzt.
"""
import sys, os, re
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "pipeline"))
import quellen as Q  # noqa: E402

ABK = os.path.join(ROOT, "sources/dba/dba_pl_abkommen_2003.txt")
BMF = os.path.join(ROOT, "sources/bmf/bmf_stand_dba_2026.txt")
n = Q._normalize(open(ABK, encoding="utf-8").read())

ANKER = {
 "titel": "Methoden zur Vermeidung der Doppelbesteuerung",
 "a_freistellung": "werden vorbehaltlich des Buchstabens b die Einkünfte aus der Republik Polen sowie die in der Republik Polen gele- genen Vermögenswerte ausgenommen, die nach diesem Abkommen in der Republik Polen besteuert werden kön- nen",
 "a_schachtel_10prozent": "deren Kapital zu mindestens 10 vom Hundert unmittel- bar der deutschen Gesellschaft gehört",
 "a_wirtschaftszonen_rueckausnahme": "Anspruch auf die Steuervergünstigung nach dem Gesetz vom 20. Oktober 1994 über die besonderen Wirtschafts- zonen in der Republik Polen hat",
 "b_anrechnung_intro": "Beachtung der Vorschriften des deutschen Steuerrechts über die Anrechnung ausländischer Steuern die polnische Steuer angerechnet",
 "b_aa_streubesitz": "aa) Dividenden, die nicht unter Buchstabe a Satz 2 fallen",
 "b_bb_artikelliste": "Einkünfte, die nach Artikel 11 Absatz 2, Artikel 12 Absatz 2, Artikel 13 Absatz 2, Artikel 15 Absatz 3, Arti- kel 16 Absatz 1 und Artikel 17 in der Republik Polen besteuert werden können",
 "c_aktivitaet_umschalt": "Statt der Bestimmungen des Buchstabens a sind die",
 "c_aktivitaet_astg": "Absatz 1 Nummern 1 bis 6 des deutschen Außen- steuergesetzes fallenden Tätigkeiten",
 "d_progression": "nach diesem Abkommen von der deutschen Besteuerung ausgenommenen Einkünfte und Vermögenswerte bei der Festsetzung des Steuersatzes für andere Einkünfte und Vermögenswerte zu berücksichtigen",
}
NEGATIV = {
 "neg_schachtel_25statt10": "deren Kapital zu mindestens 25 vom Hundert unmittel- bar der deutschen Gesellschaft gehört",
 "neg_astg_paragraph_korrekt": "unter § 8 Absatz 1 Nummern 1 bis 6 des deutschen",   # §-korrekt FEHLT: Freeze trägt OCR-'$', mein Anker meidet die Stelle
 "neg_wirtschaftszonen_erfunden": "Anspruch auf die Steuervergünstigung nach dem Gesetz vom 20. Oktober 1999 über die besonderen",
}
VERBOTENE_BLEED = ("$", "$&")  # Anker dürfen die $-statt-§-Artefakt-Stelle nicht enthalten

def main():
    ok = fehlt = 0
    print("=== POSITIV-Anker (voll-Länge, OCR-Hyphen exakt) ===")
    for k, a in ANKER.items():
        hit = Q._normalize(a) in n
        print(f"  {'OK   ' if hit else 'FEHLT'} ({len(a):3d}) {k}")
        ok += hit; fehlt += (not hit)
    print("=== NEGATIV-Anker (müssen FEHLEN) ===")
    for k, a in NEGATIV.items():
        gone = Q._normalize(a) not in n
        print(f"  {'OK-fehlt' if gone else 'DA(!)   '} {k}")
        ok += gone; fehlt += (not gone)
    print("=== OCR-Disziplin: kein '$'-Artefakt in POSITIV-Ankern ===")
    dollarmix = [k for k, a in ANKER.items() if any(b in a for b in VERBOTENE_BLEED)]
    print(f"  {'OK   ' if not dollarmix else 'FEHLER'} kein '$'/'$&' in Ankern ({dollarmix or 'keine'})")
    ok += (not dollarmix); fehlt += bool(dollarmix)
    print("=== MLI-Abwesenheit (PL NICHT in I.2-Positivliste 01.01.2025) ===")
    raw = open(BMF, encoding="utf-8").read()
    pl_mli = bool(re.search(r"Polen[^\n]*01\.01\.2025", raw))
    pl_dba = bool(re.search(r"Polen[^\n]*14\.05\.2003", raw))
    print(f"  PL in MLI-2025-Liste: {pl_mli} (muss False) | PL-DBA-Fundstelle (14.05.2003) präsent: {pl_dba} (muss True)")
    ok += (not pl_mli) + pl_dba; fehlt += pl_mli + (not pl_dba)
    print(f"\n== GESAMT: {ok} OK / {fehlt} FEHLER ==")
    return 0 if fehlt == 0 else 1

if __name__ == "__main__":
    sys.exit(main())
