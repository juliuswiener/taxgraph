#!/usr/bin/env python3
"""W1-DK-DBA-Katalog Anker-Verifikation (voll-Länge via gates._normalize).

ZWEI-FREEZE: a/Schachtel/b-Enum ankern am OCR-Grundtext dba_dk_abkommen_1995;
der PROGRESSIONSVORBEHALT ankert am Änderungsprotokoll dba_dk_protokoll_2020
(Art. 24 Abs. 1 a S. 2 ALT AUFGEHOBEN, c NEU gefasst = Prog). Alt-Prog-S. 2 aus
dem Grundtext ist NUR Fassungs-Vermerk, NIE aktiver Anker.

OCR-Disziplin: Anker aus zusammenhängenden deutschen Blöcken, hyphen-frei gewählt;
verbotene OCR-Fehlerwörter ('verstehenden', 'ebenfalis') NICHT im Anker; der ECHTE
BGBl-Druckfehler 'Anrechung' (bildbestätigt) IST amtlich und DARF im Anker stehen.
"""
import sys, os, re
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "pipeline"))
import quellen as Q  # noqa: E402

ABK = os.path.join(ROOT, "sources/dba/dba_dk_abkommen_1995.txt")
P20 = os.path.join(ROOT, "sources/dba/dba_dk_protokoll_2020.txt")
BER = os.path.join(ROOT, "sources/dba/dba_dk_berichtigung_2021.txt")
BEK = os.path.join(ROOT, "sources/dba/dba_dk_bekanntmachung_2022.txt")
BMF = os.path.join(ROOT, "sources/bmf/bmf_stand_dba_2026.txt")

n95 = Q._normalize(open(ABK, encoding="utf-8").read())
n20 = Q._normalize(open(P20, encoding="utf-8").read())
nbmf = Q._normalize(open(BMF, encoding="utf-8").read())

# --- POSITIV Grundtext 1995 (a/Schachtel/b-Enum) ---------------------------
ANKER_95 = {
 "a_freistellung": "Soweit nicht Buchstabe b anzuwenden ist, werden von der Bemessungsgrundlage der deutschen Steuer die Einkünfte aus Dänemark sowie die in Dänemark gelegenen",
 "a_schachtel_10prozent": "von einer in Dänemark ansässigen Gesellschaft gezahlt werden, deren Kapital zu mindestens 10 vom Hundert unmittelbar der",
 "b_anrechnung_intro": "der Vorschriften des deutschen Steuerrechts über die Anrechung ausländischer Steuern die dänische Steuer",  # 'Anrechung' = echter Druckfehler, amtlich
 "b_aa_dividenden": "aa) Dividenden, die nicht unter Buchstabe a fallen",
 "b_bb_artikelliste": "Einkünfte, die in Dänemark nach den Artikeln 13 Absatz 1 Satz 2, 15 Absatz 4, 16, 17, 18 Absatz 4 und 23 besteuert",
}
# --- POSITIV Protokoll 2020 (Prog = neu gefasster Buchstabe c) --------------
ANKER_20 = {
 "prog_neu_c_2020": "Die Bundesrepublik Deutschland behält das Recht, die nach diesem Abkommen von der deutschen Steuer ausgenommenen Einkünfte und Vermögenswerte bei der Festsetzung ihres Steuersatzes zu berücksichtigen",
}
VERBOTENE_OCR = ("verstehenden", "ebenfalis")   # OCR-Fehler, dürfen NICHT im Anker sein
ECHTER_DRUCKFEHLER = "Anrechung"                 # amtlich, MUSS im b-intro-Anker sein

def main():
    ok = fehlt = 0
    print("=== POSITIV Grundtext 1995 (voll-Länge) ===")
    for k, a in ANKER_95.items():
        hit = Q._normalize(a) in n95
        print(f"  {'OK   ' if hit else 'FEHLT'} ({len(a):3d}) {k}")
        ok += hit; fehlt += (not hit)
    print("=== POSITIV Protokoll 2020 (Prog) ===")
    for k, a in ANKER_20.items():
        hit = Q._normalize(a) in n20
        print(f"  {'OK   ' if hit else 'FEHLT'} ({len(a):3d}) {k}")
        ok += hit; fehlt += (not hit)
    print("=== OCR-Disziplin ===")
    verbmix = [a for a in {**ANKER_95, **ANKER_20}.values() for w in VERBOTENE_OCR if w in a]
    print(f"  {'OK   ' if not verbmix else 'FEHLER'} kein verbotenes OCR-Wort ({'/'.join(VERBOTENE_OCR)}) in Ankern")
    ok += (not verbmix); fehlt += bool(verbmix)
    druck_da = ECHTER_DRUCKFEHLER in ANKER_95["b_anrechnung_intro"]
    print(f"  {'OK   ' if druck_da else 'FEHLER'} echter Druckfehler '{ECHTER_DRUCKFEHLER}' im b-intro-Anker (amtlich, beibehalten)")
    ok += druck_da; fehlt += (not druck_da)
    print("=== ZWEI-FREEZE-Negativtest: Prog-neu-c ankert NICHT am aufgehobenen 1995er S. 2 ===")
    prog_in_95 = Q._normalize(ANKER_20["prog_neu_c_2020"]) in n95
    print(f"  {'OK-fehlt' if not prog_in_95 else 'DA(!)'} Prog-neu-c NICHT im 1995-Grundtext (nur am 2020er) — Katalog ankert Prog am Protokoll, nicht am aufgehobenen a S. 2")
    ok += (not prog_in_95); fehlt += prog_in_95
    print("=== Fassungskette: Protokoll 2020 hebt a S. 2 auf + fasst c neu ===")
    raw20 = open(P20, encoding="utf-8").read()
    s2_auf = "Artikel 24 Absatz 1 Buchstabe a Satz 2 wird aufgehoben" in raw20
    c_neu = "Artikel 24 Absatz 1 Buchstabe c wird wie folgt gefasst" in raw20
    print(f"  {'OK' if s2_auf else 'FEHLT'} a-S.2-aufgehoben-Marker | {'OK' if c_neu else 'FEHLT'} c-neu-gefasst-Marker")
    ok += s2_auf + c_neu; fehlt += (not s2_auf) + (not c_neu)
    print("=== Ketten-Belege Berichtigung 2021 + Bekanntmachung 2022 ===")
    ber = "Berichtigung" in open(BER, encoding="utf-8").read() and "1. Oktober 2020" in open(BER, encoding="utf-8").read()
    bek = "Inkrafttreten des Protokolls" in open(BEK, encoding="utf-8").read()
    print(f"  {'OK' if ber else 'FEHLT'} Berichtigung 2021 (zum Protokoll 01.10.2020) | {'OK' if bek else 'FEHLT'} Bekanntmachung 2022 (Inkrafttreten)")
    ok += ber + bek; fehlt += (not ber) + (not bek)
    print("=== MLI-Abwesenheit + ErbSt-Falle-Disziplin ===")
    raw = open(BMF, encoding="utf-8").read()
    dk_mli = bool(re.search(r"(Dänemark|Daenemark)[^\n]*01\.01\.2025", raw))
    dk_ertrag = bool(re.search(r"(Dänemark|Daenemark)[^\n]*22\.11\.1995[^\n]*01\.01\.1997", raw))  # Einkommen-DBA-Fundstelle
    print(f"  DK in MLI-2025-Liste: {dk_mli} (muss False) | DK-Einkommen-DBA-Fundstelle (22.11.1995 → 01.01.1997) präsent: {dk_ertrag} (muss True)")
    print("  (WARNUNG I.3: 'Dänemark 22.11.1995' steht auch in der ErbSt-Liste — Katalog referenziert die EINKOMMEN-DBA-Zeile, nicht ErbSt)")
    ok += (not dk_mli) + dk_ertrag; fehlt += dk_mli + (not dk_ertrag)
    print(f"\n== GESAMT: {ok} OK / {fehlt} FEHLER ==")
    return 0 if fehlt == 0 else 1

if __name__ == "__main__":
    sys.exit(main())
