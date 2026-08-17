"""Differential-Gate gegen die Zweig-Duplikation: _zweig_festzusetzende_est_gesamt und
_zweig_festzusetzende_est_rentner (produkt/haut/api.py) sind zwei unabhängig gepflegte
Funktionen, die sich mehrere Paragraphen INLINE duplizieren statt sie in eine gemeinsame
Funktion auszulagern (anders als z.B. _shared_steuer_sonder_agb, _shared_dba_sonstige,
_p35_partner_anteile — die sind bereits single-source und brauchen dieses Gate nicht, ein
Python-Funktionsaufruf mit gleichem Input kann nicht auseinanderlaufen).

Genau in dieser Bauart (zwei Kopien statt einer Funktion) lag der KiSt-Doppelbug
(kist-bemessungsgrundlage-doppelbug.md): derselbe Paragraph wurde zweimal gefixt, einmal in
JEDE Richtung — einmal auf 0 EUR für JEDEN Kirchensteuerpflichtigen, einmal 1.102 EUR zu
viel. Ein Fix, der nur in EINER der beiden Kopien ankommt, ist bis heute unsichtbar für
jeden Test, der nur eine Scheibe fährt.

WAS BEWUSST NICHT verglichen wird: die Endsummen (zahl_cent) beider Zweige. Dieselbe
§33b-Formel ergibt bei GLEICHEM Betrag ein VERSCHIEDENES End-Delta, weil sie in verschiedene
Progressionsstufen greift — gemessen in test_p33b_abs1_s1_wahlrecht_ring.py:
WAHLRECHT_TRUE_DELTA_GESAMT=135000 vs. WAHLRECHT_TRUE_DELTA_RENTNER=126000, für dieselbe
Formel. Ein Endsummen-Vergleich wäre entweder sofort rot (verschiedene Steuerarten,
verschiedene Bemessungsgrundlagen) oder würde bis zur Wirkungslosigkeit abgeschwächt.

Stattdessen: die runner.catala_*-Aufrufe MITSCHNEIDEN, während beide Zweige DENSELBEN
Feld-Snapshot rechnen, und die Aufrufe je Accessor auf Cent-Gleichheit prüfen. Das umgeht
die Progressions-Verwechslung vollständig — verglichen wird der INPUT/OUTPUT einer
einzelnen (mutmaßlich geteilten) Rechenstelle, nie ein Endbetrag, der durch beide Zweige
gelaufen ist.

STALLES-GEGEN-VERALTEN (Muster: _call_sites + AUSNAHMEN in
test_append_event_bindung_gate.py): die Menge der zu vergleichenden Accessoren ist KEINE
gepflegte Liste, sondern die AST-Schnittmenge der runner.catala_*-Namen, die beide
Zweig-Funktionskörper DIREKT aufrufen (nicht über einen Shared-Helper wie
_shared_steuer_sonder_agb — dessen Aufrufe sind aus genanntem Grund nicht Gegenstand
dieses Gates). Ein neuer, identisch dupliziert eingebauter Paragraph taucht darin
automatisch auf. Was strukturell gleich benannt, aber INHALTLICH verschieden ist (jeder
Zweig rechnet seine EIGENE Steuerbasis — anderer Einkunftsartenmix, andere Progression),
steht mit Begründung in AUSNAHMEN. test_ausnahmen_sind_begruendet und
test_ausnahmeliste_hat_keine_toten_eintraege halten diese Liste ehrlich, und
test_schnittmenge_ist_vollstaendig_zugeordnet stellt sicher, dass JEDER Name aus der
Schnittmenge entweder verglichen oder begründet ausgenommen ist — ein NEUER, unbenannter
gemeinsamer Aufruf lässt genau diesen Test rot werden, nicht schweigend durchrutschen.

NULL LLM.
"""
from __future__ import annotations

import ast
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden", "produkt/unsicherheit", "produkt/store",
            "produkt/traverser", "produkt/mapping"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API      # noqa: E402
import traverser as TR  # noqa: E402

API_PFAD = os.path.join(ROOT, "produkt", "haut", "api.py")
VZ = 2025


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


# --------------------------------------------------------------------- AST-Schnittmenge

def _catala_namen_in_funktion(funcname: str) -> set[str]:
    """Namen aller runner.catala_*-Aufrufe, die DIREKT (auch in verschachtelten Closures wie
    _festzusetzende) im Körper von `funcname` in api.py stehen. Läuft über den echten
    Quelltext, nicht über eine Erinnerung daran — wächst automatisch mit jeder neuen
    Codezeile, ohne dass diese Datei angefasst werden muss."""
    with open(API_PFAD, encoding="utf-8") as f:
        baum = ast.parse(f.read(), filename=API_PFAD)
    ziel = next(n for n in ast.walk(baum)
                if isinstance(n, ast.FunctionDef) and n.name == funcname)
    namen = set()
    for node in ast.walk(ziel):
        if (isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
                and node.func.attr.startswith("catala_")
                and isinstance(node.func.value, ast.Name) and node.func.value.id == "runner"):
            namen.add(node.func.attr)
    return namen


def _geteilte_catala_namen() -> set[str]:
    """Schnittmenge: von BEIDEN Zweigen direkt aufgerufene Accessoren — die Kandidaten für
    'müsste bei gleichem Input gleich sein'."""
    return (_catala_namen_in_funktion("_zweig_festzusetzende_est_gesamt")
            & _catala_namen_in_funktion("_zweig_festzusetzende_est_rentner"))


# Vergleichbar: SELBE Feld-IDs speisen den Aufruf in BEIDEN Zweigen (geprüft durch Lesen
# beider Funktionskörper, api.py Z. 1184-1220/1228-1256/1264 [gesamt] bzw.
# Z. 1587-1623/1679-1707 [rentner]) — Kirchensteuer-Überhang (§10 Abs.4b), §33b
# Behinderten-/Pflege-/Hinterbliebenen-Pauschbetrag (eigene Person), KAP-Verrechnung
# (§20 Abs.6), Sparer-Pauschbetrag (§20 Abs.9), §16 Abs.4-Freibetrag (identisches
# "rentner_veraeusserungsgewinn"-Feld — der Name ist NICHT rentner-spezifisch, s. Kommentar
# api.py:1108), §22 Nr.3-Freigrenze, §24b Entlastungsbetrag Alleinerziehende (Feldtripel
# fam_alleinstehend/fam_anzahl_kinder/fam_monate_ohne_voraussetzung, api.py:1142-1145 ==
# 1651-1654, byte-identisch).
VERGLEICHBAR = frozenset({
    "catala_behinderten_pb",
    "catala_hinterbliebenen_pb",
    "catala_pflege_pb",
    "catala_kapital_verrechnung",
    "catala_sparer_pb",
    "catala_p10_4b_erstattungsueberhang",
    "catala_p16_4_freibetrag",
    "catala_p22_nr3_einkuenfte",
    "catala_p24b_entlastung",
})

# Ausnahmen: Accessoren, die BEIDE Zweige direkt aufrufen, deren Argumente aber die JEWEILS
# EIGENE Steuerbasis des Zweigs sind — gesamt rechnet §19/§21-Lohn+Vermietung, rentner §22-
# Renten+eigene Gewinnanteile; dieselbe Formel auf verschiedene Einkunftsarten MUSS
# unterschiedliche Zahlen liefern, das ist keine Duplikations-Divergenz, sondern der Zweck
# der beiden Zweige. Belegt durch WAHLRECHT_TRUE_DELTA_GESAMT != _RENTNER in
# test_p33b_abs1_s1_wahlrecht_ring.py für IDENTISCHE §33b-Eingabe.
AUSNAHMEN = {
    "catala_gesamt_gde": (
        "Gesamtbetrag der Einkünfte je Zweig — gesamt speist ns/vv/gewinn, rentner speist "
        "renten (§22)/gewinn. Verschiedene einkuenfte_*-Keys, keine gemeinsame Rechenstelle."),
    "catala_gesamt_zve": (
        "zu versteuerndes Einkommen je Zweig — hängt an g2/rentner_g, der branch-eigenen "
        "Einkommensstruktur (§34-CHOOSER-Guard)."),
    "catala_gesamt_tarifliche": (
        "tarifliche ESt je Zweig — hängt an g2/rentner_g, s. catala_gesamt_zve."),
    "catala_est": (
        "die eigentliche Steuerfestsetzung — g2/rentner_g sind je Zweig eigene Dicts mit "
        "unterschiedlichen Einkunftsarten, mehrere Aufrufstellen je Zweig (Kapital-"
        "Günstigerprüfung, §32b)."),
    "catala_fuenftel": (
        "§34 Abs.1-Fünftelregelung auf zve2 (branch-eigenes zvE) — Default-Zweig des "
        "§34-CHOOSER, s. catala_gesamt_zve."),
    "catala_ermaessigter_durchschnittssatz": (
        "§34 Abs.3-Zweig des CHOOSER — bemessungsgrundlage_durchschnitt ist branch-eigenes "
        "zve2, est_gesamt_zzgl_progression branch-eigene catala_gesamt_tarifliche(g2)."),
    "catala_kapital_steuer": (
        "§32d Abs.6-Günstigerprüfung — est_regulaer_mit_kap/_ohne_kap sind branch-eigene "
        "catala_est(g2)-Ergebnisse, keine gemeinsame Eingabe."),
    "catala_p24a_altersentlastung": (
        "§24a-Bemessung unterscheidet sich strukturell: gesamt nimmt arbeitslohn (§19) + "
        "positive_andere_einkuenfte aus ns/vv/gewinn, rentner setzt arbeitslohn=0 fest "
        "(keine §19-Einkünfte im rentner-Ring) und speist stattdessen §23-Einkünfte "
        "(p23_eink) mit ein — andere Formel, kein Kopierfehler (s. api.py:1636-1645)."),
    "catala_p31_familienleistung": (
        "est_ohne_freibetraege/est_mit_freibetraegen kommen aus dem branch-eigenen "
        "_festzusetzende(...)/_festzusetzende_r(...) — rekursiv branch-eigene Steuerbasis."),
    "catala_p32b_1": (
        "§32b-Wrapper auf branch-eigenes zve32b/tarifliche_pre32b."),
    "catala_kist": (
        "§51a-Bemessung ist branch-eigene est_roh_ohne_kap — das ist die Stelle, an der der "
        "historische Doppelbug saß (kist-bemessungsgrundlage-doppelbug.md); der Bug lag in "
        "EINEM der beiden est_roh_ohne_kap-Tracking-Pfade, nicht darin, dass die Basen "
        "zwischen den Zweigen gleich sein müssten (sie sind es strukturell nicht)."),
    "catala_solz": (
        "SolZ-Bemessungsgrundlage ist branch-eigenes solz_info[_r]['est_mit_fb'], s. "
        "catala_kist."),
}


def test_schnittmenge_ist_vollstaendig_zugeordnet():
    """JEDER von beiden Zweigen geteilte catala-Name ist entweder in VERGLEICHBAR (wird
    Cent-gleich geprüft) oder in AUSNAHMEN (begründet, warum nicht) — kein Name fällt
    stillschweigend durch. Das ist der Wächter gegen Veralten: ein neuer, identisch in
    beide Zweige kopierter Paragraph muss HIER eingetragen werden, sonst schlägt dieser
    Test fehl."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    schnitt = _geteilte_catala_namen()
    zugeordnet = VERGLEICHBAR | AUSNAHMEN.keys()
    fehlend = schnitt - zugeordnet
    assert not fehlend, (
        f"Neue(r) von beiden Zweigen geteilte(r) Accessor(en) ohne Einordnung: "
        f"{sorted(fehlend)} — in VERGLEICHBAR (Cent-Vergleich) oder AUSNAHMEN "
        f"(mit Begründung) eintragen.")


def test_ausnahmen_sind_begruendet():
    """Muster test_append_event_bindung_gate.py: kein stilles Ausklammern."""
    for name, grund in AUSNAHMEN.items():
        assert grund and len(grund) > 20, f"{name}: Ausnahme ohne (ausreichende) Begründung"


def test_ausnahmeliste_und_vergleichsliste_haben_keine_toten_eintraege():
    """Jeder AUSNAHMEN-/VERGLEICHBAR-Eintrag muss noch tatsächlich von BEIDEN Zweigen
    aufgerufen werden — sonst täuscht die Liste eine Prüfung vor, die ein Refactor längst
    verschoben hat (Muster: test_ausnahmeliste_hat_keine_toten_eintraege)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    schnitt = _geteilte_catala_namen()
    for name in AUSNAHMEN:
        assert name in schnitt, (
            f"{name} steht in AUSNAHMEN, wird aber nicht mehr von beiden Zweigen "
            f"aufgerufen — toter Eintrag.")
    for name in VERGLEICHBAR:
        assert name in schnitt, (
            f"{name} steht in VERGLEICHBAR, wird aber nicht mehr von beiden Zweigen "
            f"aufgerufen — toter Eintrag, aus VERGLEICHBAR entfernen.")


# --------------------------------------------------------------------- Mitschnitt + Fahrt

def _bst(w):
    return {"wert": w, "zustand": "bestaetigt"}


def _felder() -> dict:
    """EIN Feld-Snapshot, der beide Zweige lauffähig macht, mit identischen Werten an den
    Stellen, die VERGLEICHBAR prüft. veranlagung=einzel + fam_anzahl_kinder=0 + kein store:
    Partner-/Kind-Zweigpfade bleiben inaktiv, sodass Aufrufhäufigkeit + Argumente pro Zweig
    stabil (aber nicht zwingend genau 1x, s. Multiset-Vergleich in
    test_geteilte_accessoren_sind_cent_gleich) und beide Zweige vergleichbar sind."""
    return {
        # gesamt-Minimalbasis (Präzedenz test_gewinn_partner_ring._basis)
        "bruttoarbeitslohn": _bst(6000000), "veranlagung": _bst("einzel"),
        "ep_arbeitstage": _bst(0), "ep_entfernung_km": _bst(0),
        "ep_oepnv_kosten": _bst(0), "ep_eigenes_kfz": _bst(False),
        # rentner-Minimalbasis (Präzedenz test_p33b._rentner_kegel, ohne HTTP-Deklarationsflags)
        "rentner_renten_art": _bst("gesetzliche_rente"), "rentner_jahresrente": _bst(2000000),
        "rentner_renten_beginn_jahr": _bst(2025), "rentner_alter_bei_rentenbeginn": _bst(65),
        "rentner_rentenfreibetrag": _bst(0),
        "vor_an_anteil_rv": _bst(0), "vor_ag_anteil_rv": _bst(0), "vor_rv_ausserhalb_lstb": _bst(0),
        "basis_kv": _bst(0), "basis_pv": _bst(0), "versicherungsart": _bst("gesetzlich_an"),
        "mit_anspruch_auf_zuschuss": _bst(False), "geburtsjahr": _bst(1970),
        "fam_alleinstehend": _bst(False), "fam_anzahl_kinder": _bst(0),
        "fam_monate_ohne_voraussetzung": _bst(0),
        # -- die geteilten Rechenstellen selbst (VERGLEICHBAR) — identisch für beide Zweige --
        "rentner_grad_der_behinderung": _bst(50), "rentner_hilflos_blind_taubblind": _bst(False),
        "rentner_pflegegrad": _bst(2), "rentner_gepflegter_hilflos": _bst(False),
        "rentner_hinterbliebenenbezuege": _bst(False),
        "kap_gewinn_aktien": _bst(100000), "kap_verlust_aktien": _bst(0),
        "kap_gewinn_sonstige": _bst(0), "kap_verlust_sonstige": _bst(0),
        "kist_gezahlt": _bst(100000), "kist_erstattet": _bst(250000),
        "rentner_veraeusserungsgewinn": _bst(6000000),
        "p22_nr3_einkuenfte": _bst(50000),
    }


def _mitschnitt(monkeypatch, runner_mod, namen):
    """Wrappt jeden Namen in `namen` auf dem runner-Modul; sammelt (args, ergebnis) je Aufruf.
    Gibt {name: [(args, ergebnis), ...]} zurück — dieselbe Liste wird zwischen den beiden
    Zweig-Läufen geleert (s. _vergleiche_zweige), damit ein Lauf den anderen nicht mit
    aufsammelt."""
    aufrufe: dict[str, list] = {n: [] for n in namen}
    for n in namen:
        orig = getattr(runner_mod, n)

        def _wrapper(*args, _orig=orig, _n=n, **kwargs):
            ergebnis = _orig(*args, **kwargs)
            aufrufe[_n].append((args, kwargs, ergebnis))
            return ergebnis
        monkeypatch.setattr(runner_mod, n, _wrapper)
    return aufrufe


def _fahre_zweig(quantitaet: str, bindung: dict, felder: dict) -> None:
    """Führt einen _zweig_*-Rechenkern über _bescheid_fn wirklich aus (nicht nur bauen) —
    dieselbe Konvention wie test_gewinn_partner_ring._zahl."""
    bf = API._bescheid_fn(quantitaet, VZ, bindung, felder, store=None, nur_bestaetigt=True,
                          solz_container=[None], extras={})
    assert bf is not None, f"{quantitaet}: bescheid_fn gab None (catala/cases?)"
    bf({fid: ev["wert"] for fid, ev in felder.items()})


def test_geteilte_accessoren_sind_cent_gleich(monkeypatch):
    """Der eigentliche Gate-Test: BEIDE Zweige mit DEMSELBEN Feld-Snapshot fahren, dabei
    VERGLEICHBAR mitschneiden, dann je Accessor auf exakte (args, ergebnis)-Gleichheit
    prüfen — nicht auf gleiche Endsumme (die MUSS wegen Progression verschieden sein,
    s. Moduldocstring)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    import runner as runner_mod

    felder = _felder()
    bindung = TR.lade_bindung()

    aufrufe = _mitschnitt(monkeypatch, runner_mod, VERGLEICHBAR)
    _fahre_zweig("festzusetzende_est_gesamt", bindung, felder)
    gesamt = {n: list(v) for n, v in aufrufe.items()}
    for v in aufrufe.values():
        v.clear()
    _fahre_zweig("festzusetzende_est_rentner", bindung, felder)
    rentner = {n: list(v) for n, v in aufrufe.items()}

    for name in sorted(VERGLEICHBAR):
        g_calls, r_calls = gesamt[name], rentner[name]
        assert g_calls and r_calls, (
            f"{name}: kein Aufruf mitgeschnitten (gesamt={len(g_calls)}x, "
            f"rentner={len(r_calls)}x) — Fixtur erreicht die Rechenstelle nicht.")
        # Multiset statt Einzelvergleich: derselbe Accessor kann INNERHALB eines Zweigs
        # mehrfach auftreten (gemessen für catala_behinderten_pb: einmal an der
        # ausserg-Aufbaustelle api.py:1192, ein zweites Mal INNERHALB der geteilten
        # _shared_steuer_sonder_agb für die §33b Abs.5 S.4-Rückrechnung, api.py:487 —
        # "eigener_pb_eur exakt wie an den ausserg-Aufbaustellen"). Das ist kein Risiko
        # (der Helfer ist selbst single-source für beide Zweige), aber die REIHENFOLGE der
        # Aufrufe ist nicht aussagekräftig — verglichen wird die MENGE der (Argumente,
        # Ergebnis)-Paare, sortiert nach repr() für einen stabilen Vergleich (dict-Keys
        # sind in beiden Zweigen in derselben Quelltext-Reihenfolge aufgebaut, repr() ist
        # damit deterministisch genug für eine Sortierung, nicht für Hashing).
        g_sorted = sorted(g_calls, key=repr)
        r_sorted = sorted(r_calls, key=repr)
        assert len(g_sorted) == len(r_sorted), (
            f"{name}: gesamt-Zweig ruft {len(g_sorted)}x auf, rentner-Zweig {len(r_sorted)}x "
            f"— unterschiedliche Aufrufhäufigkeit bei identischem Feld-Snapshot.\n"
            f"  gesamt:  {g_sorted!r}\n  rentner: {r_sorted!r}")
        for (g_args, g_kwargs, g_ergebnis), (r_args, r_kwargs, r_ergebnis) in zip(g_sorted, r_sorted):
            assert (g_args, g_kwargs) == (r_args, r_kwargs), (
                f"{name}: gesamt- und rentner-Zweig rufen mit VERSCHIEDENEN Argumenten auf, "
                f"obwohl derselbe Feld-Snapshot beide speist:\n"
                f"  gesamt:  args={g_args!r} kwargs={g_kwargs!r}\n"
                f"  rentner: args={r_args!r} kwargs={r_kwargs!r}\n"
                f"Das ist entweder ein Feld-Naht-Fehler (verschiedene Feld-IDs gelesen) oder "
                f"gehört nach AUSNAHMEN (falls die Divergenz beabsichtigt ist).")
            assert g_ergebnis == r_ergebnis, (
                f"{name}: GLEICHE Argumente ({g_args!r}), aber VERSCHIEDENES Ergebnis "
                f"gesamt={g_ergebnis!r} vs. rentner={r_ergebnis!r} — genau die Bugklasse aus "
                f"kist-bemessungsgrundlage-doppelbug.md (dieselbe Rechenstelle zweimal "
                f"gepflegt, einmal falsch).")
