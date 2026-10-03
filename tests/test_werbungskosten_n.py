"""catala_werbungskosten_n — Roh-WK-Summe Anlage N (Gesamtsteuer-Ring MVP Stufe 1).

Belegt: (a) Stufe 1 = Entfernungspauschale, (b) ohne EP-Slots = 0 (dHf/Verpflegung/AM haben
kein Catala-Modul), (c) ROH — kein § 9a-Pauschbetrag-Günstiger (der sitzt im Tarif; ein hier
angehobener Betrag waere doppelter Abzug).
"""
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "golden"))


def _runner():
    try:
        import runner
        return runner
    except Exception as e:  # Catala-Toolchain fehlt
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")


def test_stufe1_ist_entfernungspauschale():
    runner = _runner()
    s = {"veranlagungszeitraum": 2025, "arbeitstage": 220, "entfernung_km_roh": 30,
         "oepnv_kosten_jahr": 0, "eigenes_oder_ueberlassenes_kfz": True}
    assert runner.catala_werbungskosten_n(s) == 2156


def test_ohne_ep_slots_ist_null():
    runner = _runner()
    # dHf/Verpflegung/AM haben kein Modul -> keine EP-Slots => WK-Summe 0 (nicht erfunden)
    assert runner.catala_werbungskosten_n({"veranlagungszeitraum": 2025}) == 0


def test_roh_kein_9a_pauschbetrag():
    runner = _runner()
    # 50 Tage x 10 km x 0,30 = 150 EUR. Roh bleibt 150 — NICHT auf den Pauschbetrag 1230
    # angehoben (den wendet der Tarif an; hier waere es Doppelabzug).
    s = {"veranlagungszeitraum": 2025, "arbeitstage": 50, "entfernung_km_roh": 10,
         "oepnv_kosten_jahr": 0, "eigenes_oder_ueberlassenes_kfz": True}
    assert runner.catala_werbungskosten_n(s) == 150


# ---- Stufe 1b: doppelte Haushaltsführung (dHf) ----

def _dhf_registry_seed():
    import yaml
    doc = yaml.safe_load(open(os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")))
    return next(r for r in doc["regeln"]
                if r["rule_id"] == "p9_1_3_nr5_doppelte_haushaltsfuehrung")["test_seed"]


def test_dhf_konsistenz_runner_registry():
    """KONSISTENZ-GATE runner↔registry: _dhf_abzug MUSS die Registry-Rechenwege (test_seed von
    p9_1_3_nr5) cent-genau reproduzieren. Divergenz (runner-Formel läuft von der Registry-hinweis-
    Formel weg) → ROT. Kopplung: bei Registry-Änderung diese Formel nachziehen.

    VZ: Die Registry-Regel liest die Fassung ab VZ 2026 (norm_source estg_p9_abs1nr5_2026-07-09);
    nur dort gilt die 2.000-€-Auslandsgrenze. Ein Auslands-Seed MUSS deshalb sein `vz:` nennen —
    ohne Tag würde er stillschweigend als 2025 gerechnet, und 2025 kennt keine Auslandsgrenze.
    Inlands-Seeds sind VZ-unabhängig (Inlandsgrenze 1.000 € in allen drei VZ) und laufen als 2025."""
    runner = _runner()
    import yaml  # noqa: F401
    for c in _dhf_registry_seed():
        assert c["inputs"]["im_inland"] or "vz" in c, (
            f"Auslands-Seed ohne vz-Tag: {c['inputs']} — die Auslandsgrenze gilt erst ab VZ 2026")
        got = runner._dhf_abzug(c["inputs"], c.get("vz", 2025))
        exp = int(c["expected"])
        assert got == exp, (f"runner↔registry-Divergenz: {c['inputs']} → runner {got} ≠ "
                            f"registry {exp} ({c['rechenweg']})")


def test_werbungskosten_n_mit_dhf():
    runner = _runner()
    # EP (2156) + dHf (Miete 1400 → gekappt 1000 × 12 = 12000) = 14156 EUR
    s = {"veranlagungszeitraum": 2025, "arbeitstage": 220, "entfernung_km_roh": 30,
         "oepnv_kosten_jahr": 0, "eigenes_oder_ueberlassenes_kfz": True,
         "unterkunftskosten_monat": 1400, "monate": 12, "im_inland": True}
    assert runner.catala_werbungskosten_n(s) == 2156 + 12000


# ---- Stufe 1b: dHf, Auslandsgrenze nur ab VZ 2026 ----
# Die 2.000-€-Grenze für eine Unterkunft im Ausland ist neu: Art. 2 Nr. 3 Buchst. b Doppelbuchst. aa
# StÄndG 2025 (BGBl. 2025 I Nr. 363), anzuwenden erstmals für VZ 2026 (§ 52 Abs. 1 S. 1 EStG).
# Davor nannte Nr. 5 S. 4 nur den Inlandsbetrag; für das Ausland gibt es keine feste Kappung
# (BMF-Reisekosten v. 25.11.2020, Rz. 112 und 124). Entscheidung 2026-09-26: bis VZ 2025 ungekappt.

def _dhf_fall(vz, miete, im_inland):
    return {"veranlagungszeitraum": vz, "unterkunftskosten_monat": miete, "monate": 12,
            "im_inland": im_inland}


@pytest.mark.parametrize("vz", [2024, 2025])
def test_dhf_ausland_ist_vor_vz2026_ungekappt(vz):
    """AK1/AK2: 2.500 € × 12 im Ausland → 30.000 €, kein Deckel (davor: 24.000 €)."""
    runner = _runner()
    assert runner._dhf_abzug(_dhf_fall(vz, 2500, False), vz) == 30000
    assert runner.catala_werbungskosten_n(_dhf_fall(vz, 2500, False)) == 30000


def test_dhf_ausland_ist_ab_vz2026_auf_2000_gekappt():
    """AK3 (Kontrollfall): dieselben Eingaben in VZ 2026 → 2.000 × 12 = 24.000 €."""
    runner = _runner()
    assert runner._dhf_abzug(_dhf_fall(2026, 2500, False), 2026) == 24000
    assert runner.catala_werbungskosten_n(_dhf_fall(2026, 2500, False)) == 24000


@pytest.mark.parametrize("vz", [2024, 2025, 2026])
def test_dhf_inland_bleibt_in_jedem_vz_auf_1000_gekappt(vz):
    """AK4 (Kontrollfall): Inland 1.400 × 12 → 12.000 € in jedem der drei VZ."""
    runner = _runner()
    assert runner._dhf_abzug(_dhf_fall(vz, 1400, True), vz) == 12000


def test_dhf_params_tragen_die_auslandsgrenze_nur_ab_vz2026():
    """AK6 aus Sicht des Runners: 2024/2025 liefern KEINE Auslandsgrenze (None), 2026 liefert
    2.000; die Inlandsgrenze ist in allen drei VZ 1.000. Die Grenze kommt aus params/<vz> — an
    EINER Stelle, nicht je Aufrufer (Entscheidung auslandsgrenze-2000-euro-gilt-erst-ab-vz-2026)."""
    runner = _runner()
    assert runner._dhf_params(2024) == {"cap_monat_inland": 1000, "cap_monat_ausland": None}
    assert runner._dhf_params(2025) == {"cap_monat_inland": 1000, "cap_monat_ausland": None}
    assert runner._dhf_params(2026) == {"cap_monat_inland": 1000, "cap_monat_ausland": 2000}


@pytest.mark.parametrize("auslandsblock", [
    pytest.param({}, id="schluessel_fehlt"),
    pytest.param({"cap_monat_ausland": {"wert": None}}, id="wert_none"),
    pytest.param({"cap_monat_ausland": None}, id="block_leer"),
])
def test_dhf_fehlende_auslandsgrenze_ist_keine_grenze_kein_keyerror(monkeypatch, auslandsblock):
    """Fehlt cap_monat_ausland oder ist sein Wert None, heisst das 'keine Grenze'. Der Runner
    darf dann weder mit KeyError/TypeError abbrechen noch eine Grenze erfinden — weder in der
    dHf (Nr. 5) noch bei der Übernachtung nach 48 Monaten (Nr. 5a, verweist auf Nr. 5).
    Die Inlandsgrenze bleibt davon unberührt."""
    runner = _runner()
    monkeypatch.setattr(runner, "_load_yaml_path",
                        lambda _pfad: {"cap_monat_inland": {"wert": 1000}, **auslandsblock})
    assert runner._dhf_params(2025) == {"cap_monat_inland": 1000, "cap_monat_ausland": None}
    assert runner._dhf_abzug(_dhf_fall(2025, 2500, False), 2025) == 30000
    assert runner._dhf_abzug(_dhf_fall(2025, 1400, True), 2025) == 12000
    assert runner._uebernachtung_monatsgrenze(2026, False) is None   # die Params entscheiden, kein zweiter Schalter
    assert runner._uebernachtung_monatsgrenze(2026, True) == 1000


# ---- Stufe 1b: Verpflegung (Engine-Vorarbeit; Haut/×Tage nach dev-2s Tage-Bindung) ----

def test_verpflegung_konsistenz_runner_registry():
    """KONSISTENZ-GATE runner↔registry: _verpflegung_pauschale (je Reise-Tag) reproduziert die
    Registry-test_seeds von p9_4a. Divergenz runner↔registry → ROT. Kopplung wie dHf."""
    runner = _runner()
    import yaml
    doc = yaml.safe_load(open(os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")))
    seed = next(r for r in doc["regeln"]
                if r["rule_id"] == "p9_4a_verpflegungsmehraufwand")["test_seed"]
    for c in seed:
        got = runner._verpflegung_pauschale(c["inputs"], 2025)
        assert got == int(c["expected"]), (f"runner↔registry-Divergenz: {c['inputs']} → runner "
                                           f"{got} ≠ registry {c['expected']}")


def test_verpflegung_staffel_zweige():
    """Alle vier Staffel-Zweige (§ 9 Abs. 4a S. 3): An-/Abreise → 14, voller Tag (≥24 h) → 28,
    eintägig > 8 h → 14, ≤ 8 h → 0."""
    runner = _runner()
    vp = lambda **kw: runner._verpflegung_pauschale(kw, 2025)
    assert vp(an_oder_abreisetag=True, abwesenheit_stunden=8) == 14
    assert vp(an_oder_abreisetag=False, abwesenheit_stunden=24) == 28
    assert vp(an_oder_abreisetag=False, abwesenheit_stunden=10) == 14
    assert vp(an_oder_abreisetag=False, abwesenheit_stunden=6) == 0


def test_verpflegung_abzug_summe():
    """_verpflegung_abzug = Σ Tage × Pauschale (Jahres-Abzug): Einzel-Tag reproduziert die
    Registry-Pauschale, Mehr-Tage summiert linear."""
    runner = _runner()
    va = lambda **kw: runner._verpflegung_abzug(kw, 2025)
    assert va(tage_24h=1) == 28
    assert va(tage_an_abreise=1) == 14
    assert va(tage_ueber_8h_eintaegig=1) == 14
    assert va(tage_24h=10) == 280
    assert va(tage_24h=5, tage_an_abreise=2, tage_ueber_8h_eintaegig=3) == 5 * 28 + 2 * 14 + 3 * 14


def test_werbungskosten_n_mit_verpflegung():
    runner = _runner()
    # EP (2156) + Verpflegung (tage_24h=10 → 280) = 2436 EUR
    s = {"veranlagungszeitraum": 2025, "arbeitstage": 220, "entfernung_km_roh": 30,
         "oepnv_kosten_jahr": 0, "eigenes_oder_ueberlassenes_kfz": True, "tage_24h": 10}
    assert runner.catala_werbungskosten_n(s) == 2156 + 280


# ---- Stufe 1b: Übernachtung Auswärtstätigkeit (§ 9 Abs. 1 S. 3 Nr. 5a) ----

def _ueb_registry_seed(rule_id):
    import yaml
    doc = yaml.safe_load(open(os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")))
    return next(r for r in doc["regeln"] if r["rule_id"] == rule_id)["test_seed"]


# Bis 2026-09-26 stand hier eine Ausnahmeliste für den Registry-Seed (bisher=47, monate=12): er
# erwartete 16.800 ungekappt, obwohl 47+12 die 48-Monats-Schwelle überspannt und damit die
# Geltungsbedingung seiner eigenen Regel (zeitraum_vollstaendig_vor_48_monate) verletzt. Der Seed
# ist auf (bisher=36, monate=12) gezogen — der letzte Zeitraum, der vollständig davor liegt —,
# damit ist jeder Seed im Geltungsbereich seiner Regel und die Ausnahmeliste entbehrlich. Den
# Überspannfall (47/12 → 12.400, monatsweise geteilt) hält test_uebernachtung_split_an_der_schwelle.


def _ueb_registry_inputs(inputs):
    """Registry-Signatur-Slots -> Store-Feld-IDs des Runners."""
    return {"uebernachtung_kosten_monat": inputs["uebernachtungskosten_monat"],
            "uebernachtung_monate": inputs["monate"],
            "uebernachtung_monate_bisher": inputs["monate_bisher_am_ort"],
            "uebernachtung_im_inland": True}      # Registry-Linie ist die Inlands-MVP-Linie


def test_uebernachtung_konsistenz_runner_registry():
    """KONSISTENZ-GATE runner↔registry: _uebernachtung_abzug MUSS die Registry-Rechenwege
    (test_seed von p9_1_3_nr5a_uebernachtung_vor_48 / _nach_48) reproduzieren. Kopplung wie dHf.

    Keine Ausnahmeliste mehr: jeder Seed liegt im Geltungsbereich seiner Regel, jeder muss
    unverändert durchgehen. Ein Seed, der die Schwelle überspannt, wäre hier ein Fehler im
    Seed — deshalb prüft der Test unten zusätzlich, dass keiner es tut."""
    runner = _runner()
    for rule_id in ("p9_1_3_nr5a_uebernachtung_vor_48",
                    "p9_1_3_nr5a_uebernachtung_nach_48"):
        for c in _ueb_registry_seed(rule_id):
            i = c["inputs"]
            bisher, monate = i["monate_bisher_am_ort"], i["monate"]
            assert not (bisher < 48 < bisher + monate), (
                f"{rule_id}: Seed {i} überspannt die 48-Monats-Schwelle — er liegt außerhalb der "
                f"Geltungsbedingung seiner Regel und würde die alte binäre Lesart festschreiben")
            got = runner._uebernachtung_abzug(_ueb_registry_inputs(i), 2025)
            exp = int(c["expected"])
            assert got == exp, (f"runner↔registry-Divergenz ({rule_id}): {i} → runner "
                                f"{got} ≠ registry {exp} ({c['rechenweg']})")


def test_uebernachtung_monatsgrenze_verzweigt_nach_vz():
    """Die Auslandsgrenze ist VZ-abhängig — EINE Stelle, hier geprüft.

    2.000 €/Monat gilt erst ab VZ 2026 (StÄndG 2025, BGBl. 2025 I Nr. 363). Für VZ 2024/2025 nennt
    Nr. 5 S. 4 a.F. nur die Inlandsgrenze, für Ausland bleibt es bei der Notwendigkeitsprüfung
    (BMF-Reisekosten 25.11.2020 Rz. 124). Inland ist in allen VZ gekappt."""
    runner = _runner()
    g = runner._uebernachtung_monatsgrenze
    assert g(2025, True) == 1000 and g(2026, True) == 1000     # Inland: unverändert
    assert g(2024, False) is None and g(2025, False) is None   # Ausland: keine Grenze
    assert g(2026, False) == 2000                              # Ausland: neu ab VZ 2026


def test_uebernachtung_ausland_nach_48_ungekappt_vz2025():
    """VZ 2025, Ausland, über der 48-Monats-Schwelle: 12 × 2.500 € bleiben VOLL stehen (30.000 €).
    Inland kappt derselbe Fall auf 12.000 € — der Unterschied ist genau der Ortsunterschied."""
    runner = _runner()
    s = {"uebernachtung_kosten_monat": 2500, "uebernachtung_monate": 12,
         "uebernachtung_monate_bisher": 48}
    assert runner._uebernachtung_abzug({**s, "uebernachtung_im_inland": False}, 2025) == 30000
    assert runner._uebernachtung_abzug({**s, "uebernachtung_im_inland": True}, 2025) == 12000


def test_uebernachtung_ausland_nach_48_gekappt_ab_vz2026():
    """VZ 2026, Ausland, über der 48-Monats-Schwelle: 12 × min(2.500, 2.000) = 24.000 €.
    Dieselben Eingaben ergeben in VZ 2025 noch 30.000 € — die Zahl hängt am VZ, nicht am Fall."""
    runner = _runner()
    s = {"uebernachtung_kosten_monat": 2500, "uebernachtung_monate": 12,
         "uebernachtung_monate_bisher": 48, "uebernachtung_im_inland": False}
    assert runner._uebernachtung_abzug(s, 2026) == 24000
    assert runner._uebernachtung_abzug(s, 2025) == 30000


def test_uebernachtung_split_an_der_schwelle():
    """Die Schwelle ist ein Zeitpunkt, kein Jahresschalter: bisher=40, monate=12 → 8 Monate
    ungekappt + 4 Monate gekappt. Beide Seiten der Grenze einzeln geprüft, damit ein Off-by-one
    am Schwellenmonat auffällt (bisher=47/monate=3 → 1 × 2.000 + 2 × 1.000)."""
    runner = _runner()
    u = lambda **kw: runner._uebernachtung_abzug(
        {"uebernachtung_kosten_monat": 2000, **kw}, 2025)
    assert u(uebernachtung_monate=12, uebernachtung_monate_bisher=40) == 8 * 2000 + 4 * 1000
    assert u(uebernachtung_monate=3, uebernachtung_monate_bisher=47) == 2000 + 2 * 1000
    # Genau auf der Schwelle: der GANZE Zeitraum ist gekappt (kein Monat mehr davor).
    assert u(uebernachtung_monate=12, uebernachtung_monate_bisher=48) == 12 * 1000
    # Der letzte Zeitraum, der vollständig davor liegt: 36 + 12 = 48 → alles ungekappt.
    assert u(uebernachtung_monate=12, uebernachtung_monate_bisher=36) == 12 * 2000
    # Der Registry-Randfall (bisher=47, monate=12, 1.400/Monat) als regulärer Fall: 1 Monat
    # ungekappt (der 48.) + 11 gekappt. 16.800 wäre die alte binäre Lesart — 4.800 € zu viel.
    assert runner._uebernachtung_abzug(
        {"uebernachtung_kosten_monat": 1400, "uebernachtung_monate": 12,
         "uebernachtung_monate_bisher": 47, "uebernachtung_im_inland": True}, 2025) == 12400


# ---- Front V+V: § 21 Einkünfte aus Vermietung und Verpachtung ----

def test_vermietung_konsistenz_runner_registry():
    """KONSISTENZ-GATE runner↔registry: catala_vermietung_einkuenfte reproduziert die Registry-
    test_seeds von p21_vermietung_einkuenfte (Einnahmen − WK, Verlust möglich). Divergenz → ROT."""
    runner = _runner()
    import yaml
    doc = yaml.safe_load(open(os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")))
    seed = next(r for r in doc["regeln"]
                if r["rule_id"] == "p21_vermietung_einkuenfte")["test_seed"]
    for c in seed:
        got = runner.catala_vermietung_einkuenfte(c["inputs"])
        assert got == int(c["expected"]), (f"runner↔registry-Divergenz: {c['inputs']} → runner "
                                           f"{got} ≠ registry {c['expected']}")


def test_vermietung_verlust_moeglich():
    """§ 21 kann negativ sein (Werbungskosten > Einnahmen) — kein Floor auf 0 im Accessor."""
    runner = _runner()
    assert runner.catala_vermietung_einkuenfte(
        {"einnahmen": 8000, "gebaeude_afa": 6000, "schuldzinsen": 4000}) == -2000


# ---- Kombiniert § 19 + § 21: Arbeitnehmer MIT Vermietung (Konvergenz-Schritt) ----

def test_einkuenfte_ns_accessor():
    """catala_einkuenfte_nichtselbststaendig = summe_der_einkuenfte des einzel-Tarifs: Bruttolohn
    minus WK, mindestens § 9a-Pauschbetrag 1230. Bruttolohn 0 -> 0 (reiner Vermieter unberührt)."""
    runner = _runner()
    ns = lambda bl, wk=0: runner.catala_einkuenfte_nichtselbststaendig(
        {"veranlagungszeitraum": 2025, "bruttoarbeitslohn": bl, "werbungskosten": wk})
    assert ns(40000) == 38770          # 40000 − 1230 (§ 9a bindet, WK 0)
    assert ns(40000, 2156) == 37844    # WK 2156 > 1230 -> tatsächliche WK
    assert ns(40000, 500) == 38770     # WK 500 < 1230 -> § 9a-Floor
    assert ns(0) == 0                  # kein Job -> keine § 19-Einkünfte


def _gesamt(runner, ns, vv):
    return runner.catala_est({"gesamtfall": True, "veranlagungszeitraum": 2025,
                              "veranlagung": "einzel",
                              "einkuenfte_nichtselbststaendig": ns, "einkuenfte_vermietung": vv})


def test_kombiniert_loss_offset_mindert_lohn():
    """K2-KERN: der § 21-Verlust mindert nach § 2 Abs. 3 den § 19-Lohn — die kombinierte ESt ist
    KLEINER als die reine § 19-ESt (Verlust NICHT verschluckt). Übersteigt der Verlust das
    Gesamteinkommen, floort die ESt auf 0 (K2: keine Negativsteuer)."""
    runner = _runner()
    nur_19 = _gesamt(runner, 38770, 0)
    mit_verlust = _gesamt(runner, 38770, -5000)
    assert mit_verlust < nur_19, (mit_verlust, nur_19)     # 5388 < 6919
    assert mit_verlust == 5388 and nur_19 == 6919
    assert _gesamt(runner, 38770, -50000) == 0             # Verlust > Einkommen -> Floor 0


def test_kombiniert_gewinn_summiert():
    """Positiver § 21 addiert sich zum § 19-Lohn (§ 2 Abs. 3): kombiniert > reine § 19-ESt."""
    runner = _runner()
    assert _gesamt(runner, 38770, 18770) == 13452
    assert _gesamt(runner, 38770, 18770) > _gesamt(runner, 38770, 0)
