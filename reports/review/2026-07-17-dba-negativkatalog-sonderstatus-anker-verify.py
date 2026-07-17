#!/usr/bin/env python3
"""W3-Sonderstatus-Negativkatalog Anker-Verifikation (voll-Länge via gates._normalize).

Der dba_vorhanden=false-Pfad als eigene Katalog-Klasse (Komplement der 11 W1-Positiv-
Kataloge): für RU/BR/VAE/HK ist kein_dba_mit_quellenstaat = true → unilateraler § 34c
REGIERT. VIER Statusklassen, EINE Quelle (bmf_stand_dba_2026, sauberer Text-Layer, kein
OCR-Scan → echtes '§', keine '$'-Artefakte):

  BR  — Voll-DBA (1975) von DE GEKÜNDIGT; nur noch Schiff/Luft (S,L) + § 49-Befreiung.
  VAE — Voll-DBA befristet AUSGELAUFEN (bis 31.12.2021); ab 2022 nur S,L.
  RU  — Voll-DBA formal bestehend, aber Art. 5-22+24 einseitig SUSPENDIERT (Verbalnote
        08.08.2023) + seit 01.01.2024 via § 1 Abs. 3 S. 2 StAbwG neutralisiert.
  HK  — NIE Voll-DBA (China-DBA 28.03.2014 in HK nicht anwendbar); nur Luft/Schiff.

Anker aus zusammenhängenden dt. Blöcken; Unicode-Quotes („…") gemieden (_normalize
lässt sie stehen), Silbentrennungs-Umbrüche (Diskriminierungs-\nverbots) gemieden.
"""
import sys, os
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "pipeline"))
import quellen as Q  # noqa: E402

BMF = os.path.join(ROOT, "sources/bmf/bmf_stand_dba_2026.txt")
n = Q._normalize(open(BMF, encoding="utf-8").read())

# ---- POSITIV-Anker (voll-Länge, alle im bmf_stand-Freeze) -------------------
ANKER = {
 # RU — Suspendierungs-Mechanik (die Pointe; KEINE Kündigung, KEIN Auslauf)
 "ru_verbalnote_2023": "Die Russische Föderation hat mit Verbalnote vom 8. August 2023 ohne konkrete Angabe einer Rechtsgrundlage mit sofortiger Wirkung und bis auf Weiteres die",
 "ru_artikel_5_bis_22_24": "von Artikel 5 bis 22 und 24 des Abkommens zwischen der Bundesrepublik Deutschland und der Russischen Föderation zur Vermeidung der Doppelbesteuerung auf dem Gebiet der Steuern vom Einkommen und vom Vermögen vom 29. Mai 1996",
 "ru_aenderungsprotokoll_2007": "in der Fassung des Änderungsprotokolls vom 15. Oktober 2007 (BGBl. 2008 II S. 1399) sowie der Nummern 2 bis 7 des Protokolls zu diesem Abkommen",
 "ru_kein_voelkerrechtl_wegfall": "Diese einseitige Suspendierung führt völkerrechtlich nicht zu einer Aufhebung des Abkommens, so dass dieses weiterhin besteht",
 "ru_stabwg_seit_2024": "Jedoch werden seit dem 1. Januar 2024 deutsche Besteuerungsrechte durch das DBA mit der Russischen Föderation aufgrund des § 1 Absatz 3 Satz 2 Steueroasen-Abwehrgesetz",
 # BR — Kündigung des Voll-DBA, nur S/L-Rest
 "br_sonderabkommen_sl": "Sonderabkommen betreffend Einkünfte und Vermögen von Schifffahrt (S)- und Luftfahrt (L)-Unternehmen",
 "br_49_befreiung_2006": "Brasilien S, L (BStBl 2006 I S. 216)",
 # VAE — Befristung ausgelaufen + S/L-Rest ab 2022
 "vae_befristung_2021": "bis 31.12.2021",
 "vae_49_befreiung_2022": "Vereinigte Arabische Emirate S, L (BStBl 2022 S. 640)",
 # HK — China-DBA gilt nicht in der SAR; nur Luft/Schiff
 "hk_sar_status_1997": "Hongkong wurde mit Wirkung ab 1. Juli 1997 ein besonderer Teil der VR China",
 "hk_china_steuerrecht_nicht": "Das allgemeine Steuerrecht der VR China gilt dort nicht",
 "hk_china_dba_nicht_anwendbar": "ist das zwischen der Bundesrepublik Deutschland und der VR China abgeschlossene DBA vom 28. März 2014 in Hongkong nicht anwendbar",
 "hk_luftfahrt_ausnahme": "Vorgenannte Ausführungen zu Hongkong (außer Luftfahrtunternehmen)",
}

# ---- NEGATIV-Anker (müssen FEHLEN — trennen die vier Statusklassen sauber) --
NEGATIV = {
 # RU ist NICHT gekündigt und NICHT aufgehoben (nur suspendiert) — Klassentrennung
 "neg_ru_gekuendigt": "Abkommen mit der Russischen Föderation wurde gekündigt",
 "neg_ru_aufhebung_positiv": "führt völkerrechtlich zu einer Aufhebung des Abkommens",
 # VAE-Befristung ist 2021, nicht 2031
 "neg_vae_falschdatum_2031": "bis 31.12.2031",
 # China-DBA ist in HK NICHT anwendbar (positive Fassung fehlt)
 "neg_hk_anwendbar": "abgeschlossene DBA vom 28. März 2014 in Hongkong anwendbar",
 # gekündigtes BR-Voll-DBA (1975) steht NICHT mehr in der aktiven Einkommen-Liste
 "neg_br_altes_vollabkommen": "Brasilien 23.12.1975",
}

def main():
    ok = fehlt = 0
    print("=== POSITIV-Anker (voll-Länge, bmf_stand-Freeze) ===")
    for k, a in ANKER.items():
        hit = Q._normalize(a) in n
        print(f"  {'OK   ' if hit else 'FEHLT'} ({len(a):3d}) {k}")
        ok += hit; fehlt += (not hit)
    print("=== NEGATIV-Anker (müssen FEHLEN) ===")
    for k, a in NEGATIV.items():
        gone = Q._normalize(a) not in n
        print(f"  {'OK-fehlt' if gone else 'DA(!)   '} {k}")
        ok += gone; fehlt += (not gone)
    print("=== Klassen-Kontrolle: jeder Staat mind. 1 tragender Anker ===")
    staaten = {"RU": "ru_stabwg_seit_2024", "BR": "br_49_befreiung_2006",
               "VAE": "vae_49_befreiung_2022", "HK": "hk_china_dba_nicht_anwendbar"}
    for s, key in staaten.items():
        hit = Q._normalize(ANKER[key]) in n
        print(f"  {'OK' if hit else 'FEHLT'} {s} → {key}")
        ok += hit; fehlt += (not hit)
    print(f"\n== GESAMT: {ok} OK / {fehlt} FEHLER ==")
    return 0 if fehlt == 0 else 1

if __name__ == "__main__":
    sys.exit(main())
