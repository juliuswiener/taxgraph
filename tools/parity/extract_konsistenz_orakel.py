#!/usr/bin/env python3
"""Erzeugt rust/fixtures/konsistenz_orakel.json: Snapshots samt Antworten des PYTHON-Orakels
(`produkt/konsistenz/*`, ueber `tools/parity/oracle_konsistenz.py`). Der Rust-Test
`rust/konsistenz/tests/orakel_werte.rs` spielt dieselben Snapshots gegen die Crate `konsistenz` und
vergleicht -- hermetisch, ohne Python zur Laufzeit.

Szenario = Snapshot + Scheibe + Vorjahr:
  * `snap`     `{feld_id: [wert, "b"|"v"]}` (b = bestaetigt, v = vorlaeufig); in Rust ein `Felder`,
               in Python das Dict `{"wert", "zustand"}`, nach `feld_id` sortiert wie `store.py:601-602`,
  * `scheibe`  `null` oder die Feldmenge der Scheibe (`api._scheibe_bindung`),
  * `vorjahr`  `null` oder `vorjahr_referenz` (`{"verlustvortrag_bestand": {"wert": ..}}`).
Antworten: `pf` (= `preflight(...)`, nur nicht-leere Schluessel, dazu `status`) und `inst`
(= `unvollstaendige_instanzen`). Dazu `konstanten` und `eur` (Cent -> Text).

Wertebereich wie in der Crate-Doku: Floats und Ganzzahlen ueber `i64` stehen nur dort im Fixture, wo
die Rust-Fassung sie wie Python behandelt (`flag_check`, `check_pauschalen`, `partner_check`); in den
Betrags-Pruefungen von `preflight.py` (`isinstance(wert, (int, float))`) zaehlt ein Float in Rust
absichtlich nicht (`// PARITAET:`), dort stehen keine Floats.

Neu erzeugen:   python3 tools/parity/extract_konsistenz_orakel.py
"""
from __future__ import annotations

import json
import os
import random
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
if os.environ.get("KO_ROOT"):
    ROOT = os.environ["KO_ROOT"]
sys.path.insert(0, os.path.join(ROOT, "tools", "parity"))

import oracle_konsistenz as O  # noqa: E402

OUT = os.environ.get("KO_OUT_JSON", os.path.join(ROOT, "rust", "fixtures", "konsistenz_orakel.json"))
SEED = 20261004

FC, PC, CP, PF = O._FC, O._PC, O._CP, O._PF
FLAG_NEGIERT = FC.FLAG_NEGIERT
PARTNER_FELDER = list(PC.PARTNER_FELDER)
RING = list(O._RING)

# Wertklassen an den Kippstellen der Pruefungen.
BETRAG = [1, 999, 1000, 1001, 1099, 1100, 1_200_000, 5_000_000_000, 0, -1, True, False, None, "1500"]
BETRAG_PY = BETRAG + [2.5, 0.0, -0.0, 999.99, 1000.0, 1000.5, 1099.9, 1100.0, 1234.5, 1e15, 1e22, 1e300,
                      2 ** 63, 2 ** 64 - 1,
                      # Ab 2**53 liegt `(f - fmod(f, 100)) / 100` nicht mehr auf einer Ganzzahl; `//` korrigiert
                      # dort (CPython `float_divmod`). Gemessen an 3.14: 3.6118e16 ohne, 3.6136e16 mit Korrektur.
                      3.611817872850673e16, 3.613642085502932e16, 3.630884210044771e16, 1e17,
                      9007199254740993.0]
LEER_ZUSTAENDE = [None, False, True, 0, 0.0, -0.0, 1, 5, 5.5, "", " ", "  ", " x ", "\u001f", " ", "0"]


def b(w):
    return [w, "b"]


def v(w):
    return [w, "v"]


SZ: list[dict] = []


def add(name, snap, scheibe=None, vorjahr=None):
    SZ.append({"n": name, "snap": snap, "scheibe": scheibe, "vorjahr": vorjahr})


# ------------------------------------------------------------------------------------ Flag
GRID_FLAGS = ("kein_vuv", "kein_gewinn", "kein_sonstige_partner")
KURZ = [6, 0, 8, 10, 2, 17]       # 1_200_000, 1, 0, True, 1000, 1000.0 -- Indizes in BETRAG_PY


def flag_szenarien():
    # Jede Basis jedes Flags: alle Werte aus BETRAG_PY, ein Flag, eine Basis.
    for flag, basen in FLAG_NEGIERT.items():
        for bi, basis in enumerate(basen):
            if bi == 0 and flag in GRID_FLAGS:
                nutze = range(len(BETRAG_PY))
            elif bi == 0:
                nutze = KURZ
            else:
                nutze = KURZ[:3]
            for i in nutze:
                add(f"flag_{flag}_{basis}_{i}", {flag: b(True), basis: b(BETRAG_PY[i])})
    # Alle Flags Ja, alle Basen belegt: Reihenfolge der Widersprueche (Flag-Tabelle, Basis-Tabelle).
    snap = {f: b(True) for f in FLAG_NEGIERT}
    for flag, basen in FLAG_NEGIERT.items():
        for k, basis in enumerate(basen):
            snap[basis] = b(1_000_00 + k)
    add("flag_alle_ja_alle_basen", snap)
    add("flag_alle_ja_alle_basen_scheibe_alle", dict(snap), scheibe=sorted(snap))
    # Stand des Flags x Scheibe.
    for flag in ("kein_vuv", "kein_sonstige_partner"):
        basis = FLAG_NEGIERT[flag][0]
        for zi, flagwert in enumerate([b(True), v(True), b(False), b(1), b(1.0), b("true"), b(None), None]):
            for si, scheibe in enumerate([None, [], [flag], [basis]]):
                snap = {basis: b(5000)}
                if flagwert is not None:
                    snap[flag] = flagwert
                add(f"flagstand_{flag}_{zi}_{si}", snap, scheibe=scheibe)
    # Instanz-Schluessel.
    schluessel = ["", "__2", "__02", "__1", "__10", "__x", "__2\n", "__0", "__20", "__", "__12a", "__99", "__3",
                  "_2", "2"]
    for i, suffix in enumerate(schluessel):
        add(f"flag_inst_{i}", {"kein_vuv": b(True), "vv_einnahmen" + suffix: b(5000)})
    add("flag_inst_gemischt", {"kein_vuv": b(True), "vv_einnahmen": b(5000), "vv_einnahmen__2": b(6000),
                               "vv_einnahmen__10": b(7000), "vv_einnahmen__3": v(8000),
                               "vv_einnahmen__x": b(9000), "vv_einnahmen__1": b(1100)})
    add("flag_inst_nur_instanzen", {"kein_vuv": b(True), "vv_einnahmen__3": b(4000), "vv_einnahmen__2": b(3000)})
    add("flag_vorlaeufiger_betrag", {"kein_vuv": b(True), "vv_einnahmen": v(5000)})
    add("flag_vorlaeufige_instanz", {"kein_vuv": b(True), "vv_einnahmen__2": v(5000), "vv_einnahmen": b(2000)})
    add("flag_betrag_null_bestaetigt", {"kein_vuv": b(True), "vv_einnahmen": b(None)})


# ------------------------------------------------------------------------------------ Partner
def partner_szenarien():
    veranlagungen = [b("einzel"), b("zusammen"), b("Zusammen"), b("zusammen "), b(5), b(True), b(None), v("einzel"),
                     v("zusammen"), None]
    allein = [b(True), b(False), b(1), b(1.0), b("true"), b(None), v(True), None]
    for vi, ver in enumerate(veranlagungen):
        for ai, al in enumerate(allein):
            snap = {"rentner_grad_der_behinderung_partner": b(50)}
            if ver:
                snap["veranlagung"] = ver
            if al:
                snap["fam_alleinstehend"] = al
            add(f"partner_ver{vi}_allein{ai}", snap)
    werte = [50, 1, 0, -1, True, False, None, "x", 0.5, 0.0, 2 ** 63, 2 ** 64 - 1, 1000, 5.5]
    for fi, feld in enumerate(PARTNER_FELDER):
        for wi, w in enumerate(werte):
            add(f"partner_feld{fi}_wert{wi}", {"veranlagung": b("einzel"), feld: b(w)})
        add(f"partner_feld{fi}_vorlaeufig", {"veranlagung": b("einzel"), feld: v(50)})
    add("partner_alle_felder", {"veranlagung": b("einzel"), **{f: b(100 + i) for i, f in enumerate(PARTNER_FELDER)}})
    add("partner_alle_felder_nicht_zusammen_null", {"veranlagung": b(None), **{f: b(100) for f in PARTNER_FELDER}})
    add("partner_ohne_veranlagung", {f: b(100) for f in PARTNER_FELDER})
    add("alleinerziehend_zusammen", {"veranlagung": b("zusammen"), "fam_alleinstehend": b(True)})
    add("alleinerziehend_einzel", {"veranlagung": b("einzel"), "fam_alleinstehend": b(True)})
    add("beide_richtungen", {"veranlagung": b("zusammen"), "fam_alleinstehend": b(True),
                             "kap_kapitalertraege_partner": b(500)})


# ------------------------------------------------------------------------------------ Pauschalen
def pauschal_szenarien():
    ausl_werte = [1, 100, 0, -1, True, False, None, "5", 2.5, 0.0, -0.0, 2 ** 63, 2 ** 64 - 1]
    for c in CP.PAUSCHAL_CHECKS:
        for ai, aw in enumerate(ausl_werte):
            for fi, af in enumerate(c["ausloeser_felder"]):
                add(f"pausch_{c['id']}_ausl{fi}_{ai}", {af: b(aw)})
        # Ausloeser vorlaeufig, beide Ausloeser, Ausloeser + Pauschal-Zustaende.
        add(f"pausch_{c['id']}_ausloeser_vorlaeufig", {c["ausloeser_felder"][0]: v(100)})
        add(f"pausch_{c['id']}_alle_ausloeser", {af: b(100 + i) for i, af in enumerate(c["ausloeser_felder"])})
        for pi, pf in enumerate(c["pauschal_felder"]):
            for zi, pw in enumerate(LEER_ZUSTAENDE):
                if pi > 0 and zi % 2:
                    continue
                add(f"pausch_{c['id']}_pf{pi}_{zi}", {c["ausloeser_felder"][0]: b(100), pf: b(pw)})
            add(f"pausch_{c['id']}_pf{pi}_vorlaeufig", {c["ausloeser_felder"][0]: b(100), pf: v(5)})
            add(f"pausch_{c['id']}_pf{pi}_fehlt_andere_gesetzt",
                {c["ausloeser_felder"][0]: b(100), **{p: b(7) for p in c["pauschal_felder"] if p != pf}})
    vv = CP.PAUSCHAL_CHECKS[-1]["pauschal_felder"]
    zust = [None, 0, False, 5, "  ", v(5)]
    rng = random.Random(SEED)
    for k in range(15):
        snap = {"vv_einnahmen": b(100)}
        for f in vv:
            z = rng.choice(zust)
            if z is not None or rng.random() < 0.3:
                snap[f] = z if isinstance(z, list) else b(z)
        add(f"pausch_vv_wk_zufall_{k}", snap)
    add("pausch_alle_drei_checks", {"kap_kapitalertraege": b(100), "bruttoarbeitslohn": b(100), "vv_einnahmen": b(100)})
    add("pausch_alle_drei_checks_vorlaeufig_gesetzt",
        {"kap_kapitalertraege": b(100), "bruttoarbeitslohn": b(100), "vv_einnahmen": b(100),
         "veranlagung": v("einzel"), "ep_arbeitstage": v(200), "vv_schuldzinsen": v(5)})


# ------------------------------------------------------------------------------------ Plausibilitaet
def plausi_szenarien():
    bruttos = [1, 10, 100, 3_000_000, 1_000_000_000]
    for brutto in bruttos:
        for feld in ("p36_lohnsteuer", "vor_an_anteil_rv", "vor_ag_anteil_rv"):
            for d in (-1, 0, 1):
                add(f"brutto_{feld}_{brutto}_{d}", {"bruttoarbeitslohn": b(brutto), feld: b(brutto + d)})
        for feld in ("kist_gezahlt", "kirchensteuer_arbeitgeber"):
            k = brutto * 3 // 10
            for d in ((-2, -1, 0, 1, 2) if brutto in (10, 3_000_000) else (-1, 0, 1)):
                add(f"brutto_kist_{feld}_{brutto}_{d}", {"bruttoarbeitslohn": b(brutto), feld: b(k + d)})
    add("brutto_alle_felder", {"bruttoarbeitslohn": b(1000), "p36_lohnsteuer": b(2000), "vor_an_anteil_rv": b(2000),
                               "vor_ag_anteil_rv": b(2001), "kist_gezahlt": b(301), "kirchensteuer_arbeitgeber": b(301)})
    add("brutto_vorlaeufig", {"bruttoarbeitslohn": v(1000), "p36_lohnsteuer": b(2000)})
    add("brutto_felder_vorlaeufig", {"bruttoarbeitslohn": b(1000), "p36_lohnsteuer": v(2000), "vor_an_anteil_rv": v(2000),
                                     "kist_gezahlt": v(2000)})
    add("brutto_null_und_negativ", {"bruttoarbeitslohn": b(0), "p36_lohnsteuer": b(5)})
    add("brutto_bool", {"bruttoarbeitslohn": b(True), "p36_lohnsteuer": b(5)})
    add("brutto_text", {"bruttoarbeitslohn": b("1000"), "p36_lohnsteuer": b(5000)})
    # Schulgeld.
    for betrag in (16_666_659, 16_666_660, 16_666_661, 16_666_662, 1, 0, -5, 5_000_000_000):
        for feld in ("schulgeld", "schulgeld__2", "schulgeld__0", "schulgeld__02", "schulgeld__1", "schulgeld__x",
                     "schulgeld__", "schulgeld_2", "Schulgeld", "schulgeld__2x", "schulgeld__x2"):
            add(f"schulgeld_{feld}_{betrag}", {feld: b(betrag)})
    add("schulgeld_vorlaeufig", {"schulgeld": v(20_000_000)})
    add("schulgeld_mehrere_kinder", {"schulgeld__3": b(20_000_000), "schulgeld": b(17_000_000), "schulgeld__2": b(30_000_000),
                                     "schulgeld__10": b(16_666_661)})
    add("schulgeld_bool_und_text", {"schulgeld": b(True), "schulgeld__2": b("99999999")})
    # KiSt-Abgleich.
    for e in (1, 7, 300, 1_000_000):
        for d in (-1, 0, 1):
            add(f"kist_abgleich_gezahlt_gross_{e}_{d}", {"kist_gezahlt": b(e * 10 + d), "kirchensteuer_arbeitgeber": b(e)})
            add(f"kist_abgleich_einbehalten_gross_{e}_{d}", {"kirchensteuer_arbeitgeber": b(e * 10 + d), "kist_gezahlt": b(e)})
    add("kist_abgleich_vorlaeufig", {"kist_gezahlt": v(100000), "kirchensteuer_arbeitgeber": b(5)})
    add("kist_abgleich_eines_fehlt", {"kist_gezahlt": b(100000)})
    # IBAN.
    for ki, keine in enumerate([b(True), b(1), b(False), b(None), v(True), None, b("true")]):
        for ii, iban in enumerate([b("DE12"), b(""), b(" "), b("\u001f"), b(" "), b(None), b(5), v("DE12"), None]):
            snap = {}
            if keine:
                snap["stammdaten_keine_bankverbindung"] = keine
            if iban:
                snap["stammdaten_iban"] = iban
            add(f"iban_{ki}_{ii}", snap)
    # Konfession offen.
    konf = [b("rk"), b(""), b(None), b(5), v("rk"), None]
    belege = [{}, {"kist_gezahlt": b(5000)}, {"kirchensteuer_arbeitgeber": b(5000)}, {"kist_erstattet": b(5000)},
              {"kist_gezahlt": b(0)}, {"kist_gezahlt": v(5000)}, {"kist_gezahlt": b(5000), "kist_erstattet": b(7000)},
              {"kirchensteuer_arbeitgeber": b(6000), "kist_erstattet": b(7000)},
              {"kist_gezahlt": b(5000), "kirchensteuer_arbeitgeber": b(5000), "kist_erstattet": b(5000)},
              {"kist_erstattet": b(-5)}, {"kist_gezahlt": b(True)}]
    laender = [None, b("by"), b(""), b(None), b(5), v("by")]
    for ki, k in enumerate(konf):
        for bi, bel in enumerate(belege):
            for li, land in enumerate(laender):
                if (ki + bi + li) % 3 == 0 or (ki == 0 and bi < 3) or (bi == 0):
                    snap = dict(bel)
                    if k:
                        snap["kist_konfession"] = k
                    if land:
                        snap["kist_bundesland"] = land
                    add(f"konfession_{ki}_{bi}_{li}", snap)
    # Verlustvortrag.
    for vj_i, vj in enumerate([None, {}, {"verlustvortrag_bestand": {"wert": 100}}, {"verlustvortrag_bestand": {"wert": 0}},
                               {"verlustvortrag_bestand": {"wert": True}}, {"verlustvortrag_bestand": {"wert": -5}},
                               {"verlustvortrag_bestand": {}}]):
        for neu_i, neu in enumerate([b(99), b(100), b(101), v(500), b(0), b(None), b(True), None, b(1)]):
            snap = {"verlustvortrag_bestand": neu} if neu else {}
            add(f"verlust_{vj_i}_{neu_i}", snap, vorjahr=vj)
    # Zusammenspiel: mehrere Plausi-Meldungen in einer Antwort (Reihenfolge der Bloecke).
    add("plausi_alle_bloecke", {
        "bruttoarbeitslohn": b(1000), "p36_lohnsteuer": b(5000), "schulgeld": b(20_000_000),
        "kist_gezahlt": b(100000), "kirchensteuer_arbeitgeber": b(5), "stammdaten_keine_bankverbindung": b(True),
        "stammdaten_iban": b("DE12"), "verlustvortrag_bestand": b(900), "fam_anzahl_kinder": b(3),
        "kinderbetreuungskosten": b(1000)}, vorjahr={"verlustvortrag_bestand": {"wert": 100}})


# ------------------------------------------------------------------------------------ Instanzen
INSTANZ_REIHEN = [("fam_anzahl_kinder", "kinderbetreuungskosten"), ("rentner_anzahl_renten", "rentner_jahresrente"),
                  ("p23_anzahl_verkaeufe", "p23_veraeusserungspreis"), ("gwg_anzahl", "gwg_anschaffungskosten_netto")]


def instanz_szenarien():
    rng = random.Random(SEED + 1)
    for ri, (anzahl, basis) in enumerate(INSTANZ_REIHEN):
        for n in (0, 1, 2, 3, 4, True, None, "3"):
            for mi in range(3):
                maske = rng.randrange(0, 64)
                snap = {anzahl: b(n)}
                for i in range(1, 6):
                    if maske & (1 << i):
                        fid = basis if i == 1 else f"{basis}__{i}"
                        snap[fid] = v(1000) if (maske & 1 and i == 2) else b(1000)
                add(f"instanz_reihe{ri}_n{n!r}_{mi}", snap)
        add(f"instanz_reihe{ri}_anzahl_vorlaeufig", {anzahl: v(3), basis: b(1000)})
        add(f"instanz_reihe{ri}_luecke_mitte", {anzahl: b(4), basis: b(1000), f"{basis}__4": b(1000)})
        add(f"instanz_reihe{ri}_nur_basis", {anzahl: b(3), basis: b(1000)})
        add(f"instanz_reihe{ri}_alle", {anzahl: b(3), basis: b(1000), f"{basis}__2": b(1000), f"{basis}__3": b(1000)})
    add("instanz_mehrere_reihen", {INSTANZ_REIHEN[0][0]: b(3), INSTANZ_REIHEN[0][1]: b(1), INSTANZ_REIHEN[1][0]: b(2),
                                   INSTANZ_REIHEN[1][1]: b(1)})
    # Kind-Namen (Etikett aus der Bindung).
    add("instanz_kind_vorname", {"fam_anzahl_kinder": b(3), "kind_vorname": b("A"), "kind_vorname__3": b("C")})


# ------------------------------------------------------------------------------------ vorlaeufige Ring-Betraege
def ring_szenarien():
    add("ring_alle_vorlaeufig", {f: v(150_000 + i) for i, f in enumerate(RING)})
    add("ring_alle_vorlaeufig_ohne_betrag", {f: v(0) for f in RING})
    add("ring_alle_bestaetigt", {f: b(150_000) for f in RING})
    add("ring_alle_vorlaeufig_negativ", {f: v(-150_000) for f in RING})
    add("ring_alle_vorlaeufig_eins", {f: v(1) for f in RING})
    add("ring_alle_vorlaeufig_bool", {f: v(True) for f in RING})
    add("ring_alle_vorlaeufig_text", {f: v("1500") for f in RING})
    add("ring_alle_vorlaeufig_null", {f: v(None) for f in RING})
    add("ring_gemischt", {f: (b(150_000 + i) if i % 3 == 0 else v(150_000 + i)) for i, f in enumerate(RING)})
    add("ring_einzeln_gross", {"p36_vorauszahlungen": v(1_212_321_300), "kist_gezahlt": v(99), "schulgeld": v(150_000)})
    add("ring_fremdes_feld_vorlaeufig", {"kein_feld_im_ring": v(150_000), "iban_unbekannt": v(5)})
    for k in range(0, len(RING), 7):
        add(f"ring_feld_{RING[k]}", {RING[k]: v(150_000)})


# ------------------------------------------------------------------------------------ Zufall
def zufalls_pool():
    pool = []
    for flag, basen in FLAG_NEGIERT.items():
        pool.append(flag)
        for basis in basen:
            pool.append(basis)
            pool += [basis + s for s in ("__2", "__3", "__10", "__02", "__0", "__1", "__x")]
    pool += PARTNER_FELDER
    for c in CP.PAUSCHAL_CHECKS:
        pool += list(c["ausloeser_felder"]) + list(c["pauschal_felder"])
    pool += ["veranlagung", "fam_alleinstehend", "bruttoarbeitslohn", "p36_lohnsteuer", "vor_an_anteil_rv",
             "vor_ag_anteil_rv", "kist_gezahlt", "kirchensteuer_arbeitgeber", "kist_erstattet", "kist_konfession",
             "kist_bundesland", "schulgeld", "schulgeld__2", "schulgeld__3", "schulgeld__0", "schulgeld__x",
             "stammdaten_keine_bankverbindung", "stammdaten_iban", "verlustvortrag_bestand", "fam_anzahl_kinder",
             "kinderbetreuungskosten", "kinderbetreuungskosten__2", "rentner_anzahl_renten", "p23_anzahl_verkaeufe",
             "gwg_anzahl", "vv_anzahl_objekte", "hh_dienstleistungen", "spenden_betrag", "tage_24h",
             "p36_vorauszahlungen", "basis_kv"]
    pool += RING[::9]
    return sorted(set(pool))


def zufalls_szenarien(n=100):
    rng = random.Random(SEED + 2)
    pool = zufalls_pool()
    texte = ["", " ", "\u001f", " ", "einzel", "zusammen", "DE12", "rk", "by", " x "]
    ganz = [0, 1, 999, 1000, 1001, 1500, 100_000, 1_212_321_300, 16_666_660, 16_666_661, 5_000_000_000, -150]

    def wert():
        r = rng.random()
        if r < 0.1:
            return None
        if r < 0.25:
            return rng.random() < 0.5
        if r < 0.45:
            return rng.randint(-3, 4)
        if r < 0.75:
            return rng.choice(ganz)
        if r < 0.9:
            return rng.randint(-10 ** 13, 10 ** 13)
        return rng.choice(texte)

    for k in range(n):
        snap = {}
        for _ in range(rng.randint(0, 30)):
            snap[rng.choice(pool)] = [wert(), "b" if rng.random() < 0.7 else "v"]
        if rng.random() < 0.7:
            snap["veranlagung"] = [rng.choice(["einzel", "zusammen", "zusammen", "getrennt"]), "b" if rng.random() < 0.8 else "v"]
        if rng.random() < 0.6:
            snap["fam_alleinstehend"] = [rng.random() < 0.8, "b" if rng.random() < 0.8 else "v"]
        if rng.random() < 0.5:
            f = rng.choice(PARTNER_FELDER)
            snap[f] = [wert(), "b"]
        if rng.random() < 0.4:
            brutto = rng.randint(1, 10 ** 12)
            snap["bruttoarbeitslohn"] = b(brutto)
            snap["kist_gezahlt"] = b(brutto * 3 // 10 + rng.randint(-2, 2))
        if rng.random() < 0.4:
            anzahl, basis = rng.choice(INSTANZ_REIHEN)
            snap[anzahl] = b(rng.randint(0, 4))
            maske = rng.randrange(0, 64)
            for i in range(1, 6):
                if maske & (1 << i):
                    snap[basis if i == 1 else f"{basis}__{i}"] = b(1000) if rng.random() < 0.8 else v(1000)
        scheibe = None
        if rng.random() < 0.5:
            scheibe = rng.sample(pool, rng.randint(0, 40))
        vorjahr = None
        if rng.random() < 0.3:
            vorjahr = rng.choice([{}, {"verlustvortrag_bestand": {"wert": wert()}}])
        add(f"zufall_{k:03d}", snap, scheibe, vorjahr)


# ------------------------------------------------------------------------------------ Antworten
def python_snapshot(snap: dict) -> dict:
    return {k: {"wert": w, "zustand": "bestaetigt" if z == "b" else "vorlaeufig"} for k, (w, z) in sorted(snap.items())}


def antworten(sz: dict) -> dict:
    snapshot = python_snapshot(sz["snap"])
    bindung = None if sz["scheibe"] is None else {f: {} for f in sz["scheibe"]}
    pf = PF.preflight(snapshot, bindung, vorjahr_referenz=sz["vorjahr"])
    kompakt = {k: x for k, x in pf.items() if k == "status" or x}
    return {"pf": kompakt, "inst": PF.unvollstaendige_instanzen(snapshot)}


EUR = [0, 1, 99, 100, 101, 149, 150, 199, 200, -1, -99, -100, -101, -149, -150, -151, -199, -200, 999, 1000, 99_900,
       100_000, 100_099, 1_000_00, 123_456, 1_212_321_300, 99_999_999, 100_000_000, 100_000_001, 1_000_000_000_00,
       10 ** 15, 10 ** 18, 2 ** 63 - 1, -(2 ** 63), -1_212_321_300, -100_000_000, -99_900]


TEXTE: list[str] = []
TEXT_INDEX: dict[str, int] = {}


def tabelle(x):
    """Lange Texte (grund, hinweis) einmal in `texte` ablegen; im Szenario steht `{"$t": index}`."""
    if isinstance(x, str) and len(x) > 60:
        if x not in TEXT_INDEX:
            TEXT_INDEX[x] = len(TEXTE)
            TEXTE.append(x)
        return {"$t": TEXT_INDEX[x]}
    if isinstance(x, list):
        return [tabelle(y) for y in x]
    if isinstance(x, dict):
        return {k: tabelle(y) for k, y in x.items()}
    return x


def main():
    flag_szenarien()
    partner_szenarien()
    pauschal_szenarien()
    plausi_szenarien()
    instanz_szenarien()
    ring_szenarien()
    zufalls_szenarien()
    namen = [s["n"] for s in SZ]
    assert len(namen) == len(set(namen)), "doppelte Szenarionamen"
    for s in SZ:
        a = antworten(s)
        s["pf"], s["inst"] = tabelle(a["pf"]), tabelle(a["inst"])
    doc = {
        "erzeugt_von": "tools/parity/extract_konsistenz_orakel.py",
        "orakel": "produkt/konsistenz/{flag_check,partner_check,check_pauschalen,check_nicht_gerechnet,preflight}.py",
        "konstanten": O._konstanten({}),
        "eur": [[c, PF._eur(c)] for c in EUR],
        "texte": TEXTE,
        "szenarien": SZ,
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, separators=(",", ":"), sort_keys=False)
        f.write("\n")
    status = {}
    for s in SZ:
        status[s["pf"]["status"]] = status.get(s["pf"]["status"], 0) + 1
    print(f"{OUT}: {len(SZ)} Szenarien, {os.path.getsize(OUT)} Bytes, Status {status}")


if __name__ == "__main__":
    main()
