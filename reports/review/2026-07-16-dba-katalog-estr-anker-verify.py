"""W1 Anker-Voll-Längen-Verifikation ES + TR (Paket 6, DBA-Methoden-Kataloge).

Prueft JEDEN Katalog-Zitatanker voll-Länge via gates._normalize gegen die
gefreezten zweisprachigen Grundabkommen (ES = DE/ES, TR = DE/EN; Anker NUR aus
zusammenhaengenden deutschen Bloecken) + den amtlichen MLI-/Fassungs-Beleg
bmf_stand_dba_2026. Negativtests Pflicht (Auflage 3): verfaelschte Anker MUESSEN
fehlen. LLM-frei, $0, read-only.

Run: python reports/review/2026-07-16-dba-katalog-estr-anker-verify.py
"""
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "pipeline"))
from gates import _normalize  # noqa: E402


def load(rel):
    return _normalize(open(os.path.join(ROOT, rel), encoding="utf-8").read())


ES = load("sources/dba/dba_es_abkommen_2011.txt")
TR = load("sources/dba/dba_tr_abkommen_2011.txt")
BMF = load("sources/bmf/bmf_stand_dba_2026.txt")

# (Name, Haystack, Anker, muss_fehlen)
ANKER = [
    # --- ES Art. 22 Abs. 2 (deutsche Methode) ---
    ("ES-frei-a", ES, "Von der Bemessungsgrundlage der deutschen Steuer werden die Einkünfte aus dem Königreich Spanien sowie die im Königreich Spanien gelegenen Vermögenswerte ausgenommen, die nach diesem Abkommen im Königreich Spanien tatsächlich besteuert werden und nicht unter Buchstabe b fallen", False),
    ("ES-schachtel10", ES, "Kapital zu mindestens 10 vom Hundert unmittelbar der deutschen Gesellschaft gehört", False),
    ("ES-anr-b", ES, "wird unter Beachtung der Vorschriften des deutschen Steuerrechts über die Anrechnung ausländischer Steuern die spanische Steuer angerechnet", False),
    ("ES-enum-i", ES, "Dividenden, die nicht unter Buchstabe a fallen", False),
    ("ES-enum-ii", ES, "Einkünfte, die nach Artikel 13 Absätze 2 und 3 im Königreich Spanien besteuert werden können", False),
    ("ES-enum-iii", ES, "Einkünfte, die nach Artikel 14 Absatz 3 im Königreich Spanien besteuert werden können", False),
    ("ES-enum-iv", ES, "Einkünfte, die nach Artikel 15 im Königreich Spanien besteuert werden können", False),
    ("ES-enum-v", ES, "Einkünfte, die nach Artikel 16 im Königreich Spanien besteuert werden können", False),
    ("ES-enum-vi", ES, "Einkünfte, die nach Artikel 17 Absätze 2 und 3 im Königreich Spanien besteuert werden können", False),
    ("ES-enum-vii", ES, "Einkünfte aus unbeweglichem Vermögen (einschließlich Einkünften aus der Veräußerung dieses Vermögens)", False),
    ("ES-umschalt-c", ES, "Statt der Bestimmungen des Buchstabens a sind die", False),
    ("ES-prog-d", ES, "von der deutschen Steuer ausgenommenen Einkünfte und Vermögenswerte bei der Festsetzung ihres Steuersatzes zu berücksichtigen", False),
    # --- TR Art. 22 Abs. 2 (deutsche Methode) ---
    ("TR-frei-a", TR, "Von der Bemessungsgrundlage der deutschen Steuer werden die Einkünfte aus der Türkei ausgenommen, die nach diesem Abkommen in der Türkei besteuert werden können und nicht unter Buchstabe b fallen", False),
    ("TR-schachtel25", TR, "25 Prozent unmittelbar der deutschen Gesellschaft gehört", False),
    ("TR-anr-b", TR, "wird unter Beachtung der Vorschriften des deutschen Steuerrechts über die Anrechnung ausländischer Steuern die türkische Steuer angerechnet", False),
    ("TR-enum-aa", TR, "Dividenden, die nicht unter Buchstabe a fallen", False),
    ("TR-enum-bb", TR, "Zinsen", False),
    ("TR-enum-cc", TR, "Lizenzgebühren", False),
    ("TR-enum-dd", TR, "Einkünfte, die nach Artikel 13 Absätze 2 und 5 in der", False),
    ("TR-enum-ee", TR, "Einkünfte, die nach Protokollziffer 6 zu Artikel 15 in der", False),
    ("TR-enum-ff", TR, "Aufsichtsrats- und Verwaltungsratsvergütungen", False),
    ("TR-enum-gg", TR, "Einkünfte, die nach Artikel 17 besteuert werden können", False),
    ("TR-umschalt-c", TR, "Statt der Bestimmungen des Buchstabens a sind die", False),
    ("TR-prog-d", TR, "Die Bundesrepublik Deutschland behält aber das Recht, die nach den Bestimmungen dieses Abkommens von der deutschen Steuer ausgenommenen Einkünfte bei der Festsetzung ihres Steuersatzes zu berücksichtigen", False),
    # --- bmf_stand_dba_2026 (MLI-/Fassungs-Beleg) ---
    ("BMF-mli-rule", BMF, "erfassten Steuerabkommens aus Gründen der Rechtssicherheit und -klarheit jedoch erst nach Abschluss eines nachfolgenden Anwendungsgesetzgebungsverfahrens", False),
    ("BMF-anwendungsgesetz", BMF, "Das Gesetz zur Anwendung des Mehrseitigen Übereinkommens vom 24. November 2016 und zu weiteren", False),
    ("BMF-spanien-mli-2025", BMF, "Spanien 2024 205 2025 5 01.01.2025", False),
    ("BMF-frankreich-mli-2025", BMF, "Frankreich 2024 205 2025 5 01.01.2025", False),
    ("BMF-luxemburg-base", BMF, "Luxemburg 23.04.2012", False),
    ("BMF-luxemburg-prot2023", BMF, "06.07.2023 2023 334 2024 899 2024 147 2024 906 01.01.2024", False),
    # --- Negativtests (muessen FEHLEN) ---
    ("NEG-ES-25statt10", ES, "25 vom Hundert unmittelbar der deutschen Gesellschaft", True),
    ("NEG-TR-10statt25", TR, "10 Prozent unmittelbar der deutschen Gesellschaft", True),
    ("NEG-BMF-tuerkei-mli", BMF, "Türkei 2024 205 2025 5 01.01.2025", True),
]


def main():
    ok = 0
    fail = []
    for name, hay, anker, muss_fehlen in ANKER:
        hit = _normalize(anker) in hay
        good = (not hit) if muss_fehlen else hit
        tag = "OK " if good else "PROBLEM"
        extra = " (Negativ, muss fehlen)" if muss_fehlen else ""
        print(f"{tag} {name}{extra} -> {'hit' if hit else 'fehlt'}")
        if good:
            ok += 1
        else:
            fail.append(name)
    print(f"\n{ok}/{len(ANKER)} Checks OK" + (f"  FEHLER: {fail}" if fail else "  (Negativtests greifen beidseitig)"))
    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main())
