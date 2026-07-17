#!/usr/bin/env python3
"""W1-GB-DBA-Katalog Anker-Verifikation (voll-Länge via gates._normalize).

Freeze: sources/dba/dba_gb_abkommen_2010.txt (Art. 23 Abs. 1, deutsche Methode).
DE/EN-interleaved + BGBl-Seitenköpfe → alle Anker aus ZUSAMMENHÄNGENDEN deutschen
Blöcken, keine Kopfzeile/kein EN-Block, keine hyphen-Break-Überschreitung.
Muster wie estr-verify (ES/TR). Negativtests + Protokoll-Art.-23-Unberührt-Check.
"""
import sys, os
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "pipeline"))
import quellen as Q  # noqa: E402

ABK = os.path.join(ROOT, "sources/dba/dba_gb_abkommen_2010.txt")
P14 = os.path.join(ROOT, "sources/dba/dba_gb_protokoll_2014.txt")
P21 = os.path.join(ROOT, "sources/dba/dba_gb_protokoll_2021.txt")

nabk = Q._normalize(open(ABK, encoding="utf-8").read())

# --- Art. 23 Abs. 1 Anker (deutsche Blöcke, voll-Länge) ---------------------
ANKER = {
 "a_freistellung": "Von der Bemessungsgrundlage der deutschen Steuer werden die Einkünfte aus dem Vereinigten Königreich sowie die im Vereinigten Königreich gelegenen Vermögenswerte ausgenommen, die nach diesem Abkommen im Vereinigten Königreich tatsächlich besteuert werden und nicht unter Buchstabe b fallen",
 "a_schachtel_gesellschaft": "wenn diese Dividenden an eine in Deutschland ansässige Gesellschaft (jedoch nicht an eine Personengesellschaft) von einer im Vereinigten Königreich ansässigen Gesellschaft gezahlt werden",
 "a_schachtel_10prozent": "deren Kapital zu mindestens 10 vom Hundert unmittelbar der deutschen",
 "b_anrechnung_intro": "wird unter Beachtung der Vorschriften des deutschen Steuerrechts über die Anrechnung ausländischer Steuern die Steuer des Vereinigten Königreichs angerechnet",
 "b_aa_dividenden": "aa) Dividenden, die nicht unter Buchstabe a fallen",
 "b_bb_veraeusserung": "bb) Einkünfte, die nach Artikel 13 Absatz 2 (Veräußerungsgewinne) im Vereinigten Königreich besteuert werden",
 "b_cc_aufsichtsrat": "cc) Aufsichtsrats- und Verwaltungsratsvergütungen",
 "b_dd_kuenstler": "dd) Einkünfte, die nach Artikel 16 (Künstler und Sportler) im Vereinigten Königreich besteuert werden",
 "c_aktivitaet_umschalt": "Statt der Bestimmungen des Buchstabens a sind die Bestimmungen des Buchstabens b anzuwenden auf Einkünfte im Sinne der Artikel 7 und 10",
 "c_aktivitaet_astg": "aus unter § 8 Absatz 1 des deutschen Außensteuergesetzes fallenden Tätigkeiten bezogen hat",
 "d_progression": "Deutschland behält aber das Recht, die nach den Bestimmungen dieses Abkommens von der deutschen Steuer ausgenommenen Einkünfte und Vermögenswerte bei der Festsetzung seines Steuersatzes zu berücksichtigen",
 "e_switchover_intro": "Ungeachtet der Bestimmungen des Buchstabens a wird die Doppelbesteuerung durch Steueranrechnung nach Buchstabe b vermieden, wenn",
 "e_aa_qualifikationskonflikt": "in den Vertragsstaaten Einkünfte oder Vermögen unterschiedlichen Abkommensbestimmungen zugeordnet oder verschiedenen Personen zugerechnet werden",
 "e_bb_notifikation": "Deutschland nach gehöriger Konsultation mit der zuständigen Behörde des Vereinigten Königreichs auf diplomatischem Weg dem Vereinigten Königreich andere Einkünfte notifiziert",
}

# --- Negativtests: verfälschte Anker MÜSSEN fehlen -------------------------
NEGATIV = {
 "neg_schachtel_25statt10": "deren Kapital zu mindestens 25 vom Hundert unmittelbar der deutschen",  # 25% ist TR, nicht GB
 "neg_astg_falsch": "aus unter § 9 Absatz 1 des deutschen Außensteuergesetzes fallenden",            # § 9 statt § 8
 "neg_switchover_erfunden": "Ungeachtet der Bestimmungen des Buchstabens a wird die Freistellung nach Buchstabe a beibehalten",
}

# --- Randnotiz-Anker aus anderen Freezes -----------------------------------
BMF = os.path.join(ROOT, "sources/bmf/bmf_stand_dba_2026.txt")
nbmf = Q._normalize(open(BMF, encoding="utf-8").read())
n21 = Q._normalize(open(P21, encoding="utf-8").read())
PPT_ANKER = "der Erhalt dieser Vergünstigung einer der Hauptzwecke einer Gestaltung oder Transaktion war"


def main():
    ok = fehlt = 0
    print("=== POSITIV-Anker (voll-Länge, müssen OK sein) ===")
    for name, a in ANKER.items():
        hit = Q._normalize(a) in nabk
        print(f"  {'OK   ' if hit else 'FEHLT'} ({len(a):3d}) {name}: {a[:60]}...")
        ok += hit; fehlt += (not hit)
    print("=== Randnotiz-Anker (andere Freezes) ===")
    ppt = Q._normalize(PPT_ANKER) in n21
    print(f"  {'OK   ' if ppt else 'FEHLT'} e2_ppt_art30a (protokoll_2021): {PPT_ANKER[:55]}...")
    ok += ppt; fehlt += (not ppt)
    gb_fundstelle = Q._normalize("Vereinigtes Königreich") in nbmf
    print(f"  {'OK   ' if gb_fundstelle else 'FEHLT'} gb_in_dba_fundstellenliste (bmf_stand)")
    ok += gb_fundstelle; fehlt += (not gb_fundstelle)
    print("=== MLI-Abwesenheit (GB NICHT in I.2-Positivliste 01.01.2025) ===")
    import re
    raw = open(BMF, encoding="utf-8").read()
    gb_mli = bool(re.search(r"(Vereinigtes Königreich|Großbritannien)[^\n]*01\.01\.2025", raw))
    fr_mli = bool(re.search(r"Frankreich[^\n]*01\.01\.2025", raw))
    print(f"  GB in MLI-2025-Liste: {gb_mli} (muss False) | Frankreich in MLI-2025-Liste: {fr_mli} (muss True)")
    ok += (not gb_mli) + fr_mli; fehlt += gb_mli + (not fr_mli)
    print("=== NEGATIV-Anker (verfälscht, müssen FEHLEN) ===")
    for name, a in NEGATIV.items():
        gone = Q._normalize(a) not in nabk
        print(f"  {'OK-fehlt' if gone else 'DA(!)   '} {name}")
        ok += gone; fehlt += (not gone)
    print("=== Fassungsketten-Check: Art. 23 in 2014/2021 UNBERÜHRT ===")
    for pf, name in [(P14, "protokoll_2014"), (P21, "protokoll_2021")]:
        if not os.path.exists(pf):
            print(f"  ? {name}: Datei fehlt"); continue
        t = open(pf, encoding="utf-8").read()
        n23 = t.count("Artikel 23")
        aendert23 = ("Artikel 23" in t and ("wird wie folgt geändert" in t or "erhält folgende Fassung" in t))
        print(f"  {name}: 'Artikel 23'-Erwähnungen={n23}, aendert_Art23_wortlaut={aendert23}")
    print(f"\n== GESAMT: {ok} OK / {fehlt} FEHLER ==")
    return 0 if fehlt == 0 else 1

if __name__ == "__main__":
    sys.exit(main())
