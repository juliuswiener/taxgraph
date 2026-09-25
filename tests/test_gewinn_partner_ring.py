"""Gewinneinkünfte des Ehegatten (§§ 13-18) im RING — Stufe 2 zu
BACKLOG partnerseite-gewinneinkuenfte-fehlt-strukturell.

Stufe 1 (2026-08-12) hat die Deklarationsseite gebaut: einkuenfte_gewinn_partner, die vier
§ 15-Mitunternehmer-Felder und die vier § 16 Abs. 4-Felder existieren, sind gebunden und gehen
ins XML. Die RECHENSEITE fehlte — api.py war für den _bescheid_fn-Refactor gesperrt. Wirkung
in der Zwischenzeit: ein Paar deklariert den Partner-Gewinn korrekt, aber /ergebnis rechnete ihn
nicht mit. Die angezeigte Steuer war zu niedrig (UNDER-tax), die abgegebene Erklärung richtig.

Die Tests hier messen beide Richtungen:
  - der Partner-Gewinn erhöht die Steuer bei Zusammenveranlagung (sonst fehlt eine Einkunftsart)
  - bei EINZELveranlagung darf er es NICHT (sonst zahlt jemand für den Gewinn eines anderen)

§ 16 Abs. 4 ist per Person zu gewähren ("Der Veräußerungsgewinn wird ... nur berücksichtigt,
soweit er 45.000 Euro übersteigt" — S. 1 knüpft an den Steuerpflichtigen an, bei
Zusammenveranlagung also an jeden Ehegatten einzeln), deshalb ein zweiter, eigener
Freibetrags-Aufruf statt einer gemeinsamen Summe.

NULL LLM.
"""
from __future__ import annotations

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


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _basis(veranlagung: str) -> dict:
    """Minimaler bestätigter Fall für den gesamt-Ring: Arbeitslohn + Veranlagungsart."""
    return {
        "bruttoarbeitslohn": {"wert": 6000000, "zustand": "bestaetigt"},     # 60.000 EUR
        "veranlagung": {"wert": veranlagung, "zustand": "bestaetigt"},
        "ep_arbeitstage": {"wert": 0, "zustand": "bestaetigt"},
        "ep_entfernung_km": {"wert": 0, "zustand": "bestaetigt"},
        "ep_oepnv_kosten": {"wert": 0, "zustand": "bestaetigt"},
        "ep_eigenes_kfz": {"wert": False, "zustand": "bestaetigt"},
    }


def _basis_rentner(veranlagung: str) -> dict:
    """Minimaler bestätigter Fall für den rentner-Ring: 20.000 EUR gesetzliche Rente ab 2025
    (Präzedenz test_zweig_duplikation_differential._felder). Geburtsjahr 1970 hält § 24a
    heraus: dessen Altersentlastungsbetrag knüpft nur an Person A an und machte A und B sonst
    verschieden, ganz ohne § 34."""
    return {fid: {"wert": w, "zustand": "bestaetigt"} for fid, w in {
        "veranlagung": veranlagung,
        "rentner_renten_art": "gesetzliche_rente", "rentner_jahresrente": 2000000,
        "rentner_renten_beginn_jahr": 2025, "rentner_alter_bei_rentenbeginn": 65,
        "rentner_rentenfreibetrag": 0,
        "vor_an_anteil_rv": 0, "vor_ag_anteil_rv": 0, "vor_rv_ausserhalb_lstb": 0,
        "basis_kv": 0, "basis_pv": 0, "versicherungsart": "gesetzlich_an",
        "mit_anspruch_auf_zuschuss": False, "geburtsjahr": 1970,
        "fam_alleinstehend": False, "fam_anzahl_kinder": 0, "fam_monate_ohne_voraussetzung": 0,
    }.items()}


def _zahl(felder: dict, quantitaet: str = "festzusetzende_est_gesamt") -> int:
    """Die festzusetzende ESt des gewählten Rings in CENT für diesen Feld-Snapshot."""
    bindung = TR.lade_bindung()
    bf = API._bescheid_fn(quantitaet, 2025, bindung, felder,
                          store=None, nur_bestaetigt=True)
    assert bf is not None, "bescheid_fn gab None (catala/cases?)"
    return bf({f: ev["wert"] for f, ev in felder.items()})


@pytest.mark.parametrize("feld,betrag_cent", [
    ("einkuenfte_gewinn_partner", 3000000),      # 30.000 EUR laufender Gewinn
    ("gewinnanteil_partner", 3000000),           # 30.000 EUR § 15-Mitunternehmeranteil
])
def test_partner_gewinn_erhoeht_die_steuer_bei_zusammenveranlagung(feld, betrag_cent):
    """Ohne die Ring-Verdrahtung ist die Steuer mit und ohne Partner-Gewinn identisch —
    die Einkunftsart fehlt schlicht in der Bemessung (under-tax)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne = _zahl(_basis("zusammen"))
    mit_f = _basis("zusammen")
    mit_f[feld] = {"wert": betrag_cent, "zustand": "bestaetigt"}
    mit = _zahl(mit_f)
    assert mit > ohne, (
        f"{feld}={betrag_cent} ct bewegt die festgesetzte Steuer nicht: {ohne} -> {mit} ct. "
        f"Der Partner-Gewinn wird deklariert, aber nicht besteuert.")


def test_partner_gewinn_wirkt_nicht_bei_einzelveranlagung():
    """Gegenrichtung: bei Einzelveranlagung gibt es keinen Ehegatten in dieser Erklärung —
    ein Partner-Feld darf die eigene Steuer nicht erhöhen. Ohne diesen Test wäre eine
    Verdrahtung, die die Veranlagungsart ignoriert, ebenfalls grün."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne = _zahl(_basis("einzel"))
    mit_f = _basis("einzel")
    mit_f["einkuenfte_gewinn_partner"] = {"wert": 3000000, "zustand": "bestaetigt"}
    assert _zahl(mit_f) == ohne, (
        "Ein Partner-Gewinn erhöht die Steuer bei EINZELveranlagung — dort gibt es keinen "
        "Ehegatten, dessen Einkünfte mitzuveranlagen wären.")


def _mit_vg(a_cent: int, b_cent: int | None = None, basis=_basis) -> dict:
    """Zusammenveranlagung mit Veräußerungsgewinn bei A (und optional B), § 16 Abs. 4-Gates
    jeweils bestätigt-true."""
    f = basis("zusammen")
    f["rentner_veraeusserungsgewinn"] = {"wert": a_cent, "zustand": "bestaetigt"}
    f["rentner_alter_55_oder_berufsunfaehig"] = {"wert": True, "zustand": "bestaetigt"}
    f["rentner_freibetrag_erstmalig"] = {"wert": True, "zustand": "bestaetigt"}
    if b_cent is not None:
        f["rentner_veraeusserungsgewinn_partner"] = {"wert": b_cent, "zustand": "bestaetigt"}
        f["rentner_alter_55_oder_berufsunfaehig_partner"] = {"wert": True, "zustand": "bestaetigt"}
        f["rentner_freibetrag_erstmalig_partner"] = {"wert": True, "zustand": "bestaetigt"}
    return f


def test_p16_4_freibetrag_gilt_je_person():
    """§ 16 Abs. 4: der Freibetrag steht jedem Ehegatten eigenständig zu.

    Gemessen wird gegen den Fall, der bei EINEM gemeinsamen Freibetrag herauskäme — beide Male
    120.000 EUR Veräußerungsgewinn insgesamt, einmal auf eine Person, einmal auf zwei:

        A 120.000 allein   -> ein Freibetrag  -> netto 120.000 − 45.000 = 75.000 steuerbar
        A 60.000 + B 60.000-> zwei Freibeträge-> netto 2 × (60.000 − 45.000) = 30.000 steuerbar

    Die Aufteilung MUSS also die niedrigere Steuer ergeben. Beträge bewusst über der 45.000-
    Schwelle: bei 40.000 wäre netto beidseitig 0 und der Test grün, ohne etwas zu prüfen.
    Auch unter der Abschmelzgrenze des S. 3 (136.000), damit die kein Faktor ist."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    z_120_allein = _zahl(_mit_vg(12000000))
    z_60_und_60 = _zahl(_mit_vg(6000000, 6000000))
    # Exakter Wert statt `<`: die Richtungsprüfung blieb auch dann grün, wenn der zweite
    # Freibetrag nur zu einem Bruchteil ankommt. Herleitung VZ 2025, Splitting, und § 34
    # Abs. 1 Fünftelung auf die Veräußerungsgewinne BEIDER Ehegatten (bescheid_zweige.py
    # glättet sie, Entscheidung p34-fuenftelung-umfasst-beide-ehegatten):
    #   allein: Fünftel(zvE 133.698, ao 75.000) = 30.358 EUR
    #   je 60k: Fünftel(zvE  88.698, ao 30.000) = 16.848 EUR   → Differenz 13.510 EUR
    # Die zvE-Differenz ist exakt der zweite Freibetrag (133.698 − 88.698 = 45.000).
    # Bis 2026-09 stand hier 13.026 EUR: in ao ging nur der Netto-VG von A ein (15.000).
    delta = z_120_allein - z_60_und_60
    assert delta == 1351000, (
        f"Zwei Veräußerungsgewinne von je 60.000 EUR müssen 13.510 EUR günstiger sein als "
        f"120.000 EUR bei einer Person, gemessen {delta} ct ({z_60_und_60} vs. "
        f"{z_120_allein} ct) — prüfe, ob der Partner seinen eigenen § 16 Abs. 4-Freibetrag "
        f"bekommt und ob sich die § 34-Glättung geändert hat.")


@pytest.mark.parametrize("quantitaet,basis,vg_cent,erwartet_cent", [
    # Fünftel(zvE 88.698, ao 30.000) = 16.848 EUR, Herleitung s. test_p16_4_freibetrag_gilt_je_person.
    ("festzusetzende_est_gesamt", _basis, 6000000, 1684800),
    # Fünftel(zvE 126.526, ao 110.000) = 5 × 2 × T(19.263) = 14.570 EUR (vorher ao 55.000: 29.278).
    # Je 100.000 statt 60.000: ohne vg liegt das zvE hier bei 16.526, mit je 60.000 wäre die
    # Steuer nach der Glättung 0 und die Symmetrie unten prüfte nichts.
    ("festzusetzende_est_rentner", _basis_rentner, 10000000, 1457000),
])
def test_p34_fuenftelung_glaettet_auch_den_partner(quantitaet, basis, vg_cent, erwartet_cent):
    """§ 34 Abs. 1 bei Zusammenveranlagung: der § 16-Gewinn des Ehegatten wird geglättet wie der
    von Person A (Entscheidung p34-fuenftelung-umfasst-beide-ehegatten; § 26b behandelt beide
    „gemeinsam als ein Steuerpflichtiger"). Vorher ging nur der Netto-VG von A in die Fünftelung
    ein, der des Partners stand vollprogressiv im zvE — in BEIDEN Ringen, daher beide gemessen.

    Zwei Prüfungen: der exakte Wert mit zwei Gewinnen, und die Symmetrie — derselbe Gewinn
    ergibt dieselbe Steuer, gleich welcher Ehegatte verkauft hat. Verkaufte vorher nur der
    Partner, lief gar keine Fünftelung."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    beide = _zahl(_mit_vg(vg_cent, vg_cent, basis=basis), quantitaet)
    assert beide == erwartet_cent, (
        f"{quantitaet}: zwei Veräußerungsgewinne von je {vg_cent // 100} EUR ergeben {beide} ct, "
        f"erwartet {erwartet_cent} ct — geht der Netto-VG des Partners in die Fünftelung ein?")
    nur_a = _zahl(_mit_vg(vg_cent, basis=basis), quantitaet)
    nur_b = _zahl(_mit_vg(0, vg_cent, basis=basis), quantitaet)
    assert nur_b == nur_a, (
        f"{quantitaet}: derselbe Veräußerungsgewinn kostet beim Partner {nur_b} ct, bei "
        f"Person A {nur_a} ct — die Glättung hängt an der Formularposition.")
    # Ein Abs.-3-Antrag von A ohne eigenen Gewinn darf dem Partner die Fünftelung nicht nehmen:
    # _abs3_eligible liest nur Person A, und bei A gibt es nichts zu ermäßigen.
    antrag_a = _mit_vg(0, vg_cent, basis=basis)
    antrag_a["antrag_ermaessigter_satz"] = {"wert": True, "zustand": "bestaetigt"}
    antrag_a["dauernd_berufsunfaehig"] = {"wert": True, "zustand": "bestaetigt"}
    mit_antrag = _zahl(antrag_a, quantitaet)
    assert mit_antrag == nur_a, (
        f"{quantitaet}: A beantragt § 34 Abs. 3 ohne eigenen Gewinn, der Gewinn des Partners "
        f"kostet dann {mit_antrag} ct statt {nur_a} ct.")


def test_p35_anrechnung_gilt_auch_fuer_den_betrieb_des_partners():
    """§ 35 Abs. 1: die Ermäßigung bemisst sich nach der SUMME der gewerblichen Einkünfte
    (S. 2: "Summe der positiven gewerblichen Einkünfte / Summe aller positiven Einkünfte"), nicht
    je Person. Bei Zusammenveranlagung gehört der Gewerbebetrieb des Ehegatten also dazu.

    Vorher: sein Gewinn stand seit Stufe 2 im NENNER, sein Messbetrag aber nicht im ZÄHLER — ein
    Paar, bei dem nur der Ehegatte gewerblich tätig ist, bekam gar keine Anrechnung. Richtung
    over-tax: der Nutzer zahlt zu viel, es reklamiert niemand.

    Der Deckel auf die tatsächlich gezahlte Gewerbesteuer (S. 5) muss dabei JE BETRIEB gerechnet
    werden — die Hebesätze zweier Gemeinden sind verschieden."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    f = _basis("zusammen")
    f["einkuenfte_gewinn_partner"] = {"wert": 5000000, "zustand": "bestaetigt"}   # 50.000 EUR
    f["gewinn_betriebsart_partner"] = {"wert": "gewerbe", "zustand": "bestaetigt"}
    ohne_gewst = _zahl(f)

    mit_gewst = dict(f)
    mit_gewst["gewst_messbetrag_partner"] = {"wert": 175000, "zustand": "bestaetigt"}  # 1.750 EUR
    mit_gewst["gewst_hebesatz_partner"] = {"wert": 400, "zustand": "bestaetigt"}
    # § 35 ist eine STEUERERMÄSSIGUNG, kein Abzug von den Einkünften: sie mindert die Steuer
    # 1:1, ein Grenzsatz kommt nicht vor. Das zvE ist in beiden Fällen dasselbe — der
    # Partnergewinn steht auch im ohne_gewst-Fall. Die Differenz IST also die Anrechnung, und
    # die ist ohne Tarifkenntnis herleitbar: 4 × Messbetrag = 4 × 1.750 = 7.000 EUR (S. 1
    # Nr. 1), tatsächlich gezahlte Gewerbesteuer = 1.750 × 400 % = 7.000 EUR (S. 5) — beide
    # Deckel treffen sich exakt. Nur die Richtung zu prüfen liesse zu, dass die Anrechnung um
    # eine Grössenordnung danebenliegt.
    z_mit = _zahl(mit_gewst)
    assert ohne_gewst - z_mit == 700000, (
        f"Anrechnung {ohne_gewst - z_mit} Cent statt 700000 (ohne {ohne_gewst}, mit {z_mit}) — "
        "der Gewerbesteuer-Messbetrag des Ehegatten mindert die Steuer nicht (oder nicht in "
        "voller Höhe); sein Gewinn steht im § 35-Nenner, sein Messbetrag aber nicht im Zähler.")


def test_p35_hebesatz_je_betrieb_ermittelt_deckel_ist_summe():
    """§ 35 Abs. 1 S. 5 deckelt auf die TATSÄCHLICH zu zahlende Gewerbesteuer. Die ist je Betrieb
    Messbetrag × Hebesatz — und zwei Gemeinden haben verschiedene Hebesätze. Ein gemeinsamer
    Hebesatz auf die Messbetragssumme wäre eine andere Zahl.

    Gemessen wird an einem Fall, in dem dieser Deckel bindet: bei Hebesatz 400 ist
    Messbetrag × 4 exakt das Vierfache aus S. 1 Nr. 1, darunter greift S. 5. Person A steht bei
    300, der Ehegatte bei 500 — würde der Code A's Hebesatz auf beide Messbeträge anwenden, wäre
    die gezahlte Gewerbesteuer in BEIDEN verglichenen Fällen dieselbe und die Differenz 0.
    Genau das schliesst der feste Sollwert unten aus.

    "Je Betrieb" meint hier die GEZAHLTE Gewerbesteuer — Messbetrag mal EIGENER Hebesatz, dann
    summiert. Es meint NICHT, dass die Anrechnung je Betrieb gedeckelt würde; die läuft über die
    Summe (`min(4 * p35_messbetrag_ges, p35_gezahlt, ...)` in bescheid_zweige.py):

        so rechnet die Software:   min(4 × 2.000, 3.000 + 5.000) = 8.000 EUR
        betriebsbezogen gelesen:   min(4.000, 3.000) + min(4.000, 5.000) = 7.000 EUR

    Der Unterschied sind 1.000 EUR, und die betriebsbezogene Lesart ist bereits zweimal aus dem
    Gedächtnis aufgetaucht — sie steht nicht im Gesetz. Abs. 1 S. 2 bemisst den Höchstbetrag über
    "Summe der positiven gewerblichen Einkünfte / Summe aller positiven Einkünfte", S. 5 nennt
    keinen Betriebsbezug. Am 2026-08-13 gegen sources/ geprüft und bewusst so gebaut, siehe
    vault backlog/archive/taxgraph/p35-anrechnung-partner-offen.md. Wer das ändern will, braucht
    eine Quelle über den Gesetzeswortlaut hinaus."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    gemeinsam = _basis("zusammen")
    gemeinsam["einkuenfte_gewinn"] = {"wert": 5000000, "zustand": "bestaetigt"}
    gemeinsam["gewinn_betriebsart"] = {"wert": "gewerbe", "zustand": "bestaetigt"}
    gemeinsam["gewst_messbetrag"] = {"wert": 100000, "zustand": "bestaetigt"}      # 1.000 EUR
    gemeinsam["gewst_hebesatz"] = {"wert": 300, "zustand": "bestaetigt"}
    nur_a = _zahl(gemeinsam)

    beide = dict(gemeinsam)
    beide["einkuenfte_gewinn_partner"] = {"wert": 5000000, "zustand": "bestaetigt"}
    beide["gewinn_betriebsart_partner"] = {"wert": "gewerbe", "zustand": "bestaetigt"}
    beide["gewst_messbetrag_partner"] = {"wert": 100000, "zustand": "bestaetigt"}   # 1.000 EUR
    beide["gewst_hebesatz_partner"] = {"wert": 500, "zustand": "bestaetigt"}
    # Der Partner bringt Gewinn (erhöht) UND Anrechnung (mindert) — geprüft wird nur, dass seine
    # Gewerbesteuer überhaupt mit dem EIGENEN Hebesatz zählt: mit Hebesatz 500 statt 300 muss die
    # Steuer niedriger ausfallen, weil der S.-5-Deckel höher liegt.
    beide_niedriger_hebesatz = dict(beide)
    beide_niedriger_hebesatz["gewst_hebesatz_partner"] = {"wert": 300, "zustand": "bestaetigt"}
    z_beide = _zahl(beide)
    z_niedriger = _zahl(beide_niedriger_hebesatz)
    # Eine Steuerermäßigung bei identischem zvE: der Partnergewinn steht in beiden Fällen drin,
    # nur sein Hebesatz unterscheidet sie. Die Differenz IST also die Differenz der Anrechnung,
    # ein Grenzsatz kommt nicht vor. Gezahlte Gewerbesteuer, je Betrieb mit dem eigenen Hebesatz:
    #   A          1.000 × 300 % = 3.000 EUR   (in beiden Fällen gleich)
    #   Partner    1.000 × 500 % = 5.000 EUR   gegen   1.000 × 300 % = 3.000 EUR
    # Deckel auf die Summe: min(8.000, 8.000) = 8.000 gegen min(8.000, 6.000) = 6.000 EUR.
    assert z_niedriger - z_beide == 200000, (
        f"Differenz {z_niedriger - z_beide} Cent statt 200000 (Hebesatz 500: {z_beide}, "
        f"Hebesatz 300: {z_niedriger}) — bei 0 zählt der Hebesatz des Ehegatten gar nicht, "
        "bei 100000 wäre der S.-5-Deckel je Betrieb statt auf die Summe gerechnet (s. Docstring).")
    assert nur_a is not None


def test_p35_partner_wirkt_nicht_bei_einzelveranlagung():
    """Gegenrichtung: ohne Zusammenveranlagung gibt es keinen Ehegattenbetrieb in dieser
    Erklärung — sein Messbetrag darf die eigene Steuer nicht mindern."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    f = _basis("einzel")
    f["einkuenfte_gewinn"] = {"wert": 5000000, "zustand": "bestaetigt"}
    f["gewinn_betriebsart"] = {"wert": "gewerbe", "zustand": "bestaetigt"}
    f["gewst_messbetrag"] = {"wert": 100000, "zustand": "bestaetigt"}
    f["gewst_hebesatz"] = {"wert": 400, "zustand": "bestaetigt"}
    ohne = _zahl(f)
    mit = dict(f)
    mit["gewst_messbetrag_partner"] = {"wert": 500000, "zustand": "bestaetigt"}
    mit["gewst_hebesatz_partner"] = {"wert": 400, "zustand": "bestaetigt"}
    mit["einkuenfte_gewinn_partner"] = {"wert": 5000000, "zustand": "bestaetigt"}
    mit["gewinn_betriebsart_partner"] = {"wert": "gewerbe", "zustand": "bestaetigt"}
    assert _zahl(mit) == ohne, (
        "Ein Partner-Gewerbebetrieb mindert die Steuer bei EINZELveranlagung.")


def _sperrgrund(felder: dict):
    """Der Guard-Sperrgrund für Scheibe gesamt zu diesem Feld-Snapshot (None = frei)."""
    import api_constants as AC
    bindung = TR.lade_bindung()
    return API._an_gesamt_sperrgrund(felder, AC.SCHEIBEN["gesamt"], 2025, None, bindung)


def test_p16_4_gate_gilt_auch_fuer_den_partner():
    """§ 16 Abs. 4 S. 1+2: der Freibetrag setzt Alter ≥ 55 (oder Berufsunfähigkeit) UND
    erstmalige Inanspruchnahme voraus. Für Person A sperrt der Guard, solange das nicht bestätigt
    ist — ohne denselben Spiegel für den Partner bekäme dessen Veräußerungsgewinn den Freibetrag
    ungeprüft, und das ist UNDER-tax (der Fehler geht zulasten des Fiskus, reklamiert also
    niemand). Der Spiegel gehört zwingend zur Ring-Verdrahtung, nicht in einen Folgeschritt."""
    ohne_gates = _basis("zusammen")
    ohne_gates["rentner_veraeusserungsgewinn_partner"] = {"wert": 6000000, "zustand": "bestaetigt"}
    assert _sperrgrund(ohne_gates) == "p16_4_gate_offen", (
        "Partner-Veräußerungsgewinn ohne bestätigte § 16 Abs. 4-Bedingungen sperrt nicht — "
        "der Freibetrag würde ungeprüft gewährt.")

    mit_gates = dict(ohne_gates)
    mit_gates["rentner_alter_55_oder_berufsunfaehig_partner"] = {"wert": True, "zustand": "bestaetigt"}
    mit_gates["rentner_freibetrag_erstmalig_partner"] = {"wert": True, "zustand": "bestaetigt"}
    assert _sperrgrund(mit_gates) != "p16_4_gate_offen", (
        "Mit beiden bestätigten Bedingungen darf das Gate nicht mehr sperren — sonst ist der "
        "Fall unerreichbar statt geprüft.")


def test_p16_4_partner_gate_sperrt_nicht_bei_einzelveranlagung():
    """Gegenrichtung: bei Einzelveranlagung rechnet der Ring den Partner-vg gar nicht mit, also
    darf ein dort stehender Altwert den eigenen Bescheid auch nicht blockieren. Ohne diesen Test
    wäre ein Spiegel, der die Veranlagungsart übergeht, ebenfalls grün — und würde Einzelfälle
    an einer Frage aufhängen, die sie nichts angeht."""
    f = _basis("einzel")
    f["rentner_veraeusserungsgewinn_partner"] = {"wert": 6000000, "zustand": "bestaetigt"}
    assert _sperrgrund(f) != "p16_4_gate_offen", (
        "Ein Partner-Veräußerungsgewinn sperrt den Bescheid bei EINZELveranlagung.")


def test_partner_veraeusserungsgewinn_wird_ueberhaupt_besteuert():
    """Ergänzung zum Test darüber: dieser vergleicht zwei Aufteilungen und wäre auch dann grün,
    wenn der Partner-vg komplett ignoriert würde (dann wäre er sogar noch günstiger). Hier also
    die andere Richtung — ein Partner-vg über dem Freibetrag MUSS die Steuer erhöhen."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne = _zahl(_mit_vg(6000000))
    mit = _zahl(_mit_vg(6000000, 6000000))
    assert mit > ohne, (
        f"Ein Partner-Veräußerungsgewinn von 60.000 EUR (15.000 über dem Freibetrag) bewegt "
        f"die Steuer nicht: {ohne} -> {mit} ct.")
