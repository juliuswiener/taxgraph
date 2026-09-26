"""Ein „nein" des Normalfalls darf keine Regel abschalten.

`relevanz()` behandelt JEDES askable bool-Feld an einer Geltungsbedingung als Gate: ein
bestätigtes `false` schließt die ganze Regel aus. Das ist für echte Tatbestandsvoraussetzungen
richtig und für Deklarationsfelder falsch — und der Unterschied ist von außen nicht sichtbar,
weil beide gleich aussehen: bool, askable, an einer Geltungsbedingung.

Gemessen am 2026-08-16, unmittelbar nachdem die Anlage V einreichbar war:

    vv_nutzung_ferienwohnung  = false  ->  p21_vermietung_einkuenfte ausgeschlossen
    vv_nutzung_an_angehoerige = false  ->  dito
    vv_nutzung_kurzfristig    = false  ->  dito
    vv_nebenkosten_nicht_vereinbart = false -> dito

Der normale Vermieter antwortet auf alle vier mit „nein" — und verlor damit die komplette
Anlage V aus dem Dialog: 155 statt 174 Fragen, kein einziges vv_-Betragsfeld mehr. Das XML war
davon unberührt (est_mapping.deklariere kennt relevanz() nicht), die Erklärung wäre also leer
geblieben, ohne dass irgendetwas rot geworden wäre. Dieselbe Klasse wie 519199e.

Behoben durch `gate: false` in der Bindung — „Deklaration, keine Rechen-Voraussetzung". Die
Geltungsbedingung erscheint dann als OFFENE ANNAHME statt als beantwortetes Gate, wird also nie
still als erfüllt verbucht.

Was hier geprüft wird:

  1. Die fünf umgestellten Felder: BEIDE Antworten lassen die Regel stehen — und sie werden
     weiterhin gefragt. `gate: false` darf nicht die bequeme Art werden, ein Feld verschwinden
     zu lassen.
  2. Der Sweep über ALLE bool-Gates: die Antwort des Normalfalls (`beispielwert`) darf keine
     Regel ausschließen, die noch weitere askable Felder hat. Genau diese Prüfung hätte den
     Anlage-V-Fund am selben Tag gefunden, an dem er entstand.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import store as ST      # noqa: E402
import traverser as TR  # noqa: E402

BINDUNG = TR.lade_bindung()


def _relevanz_mit(feld: str, wert) -> dict:
    s = ST.leerer_store(2025, fall_id="gate-polaritaet")
    ST.append_event(s, feld_id=feld, wert=wert, zustand="bestaetigt",
                    herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                              "haftung": "nutzer"},
                    schreiber="ui:laie",
                    signal={"signal_1": None, "signal_2": f"klick@{feld}"},
                    ts="2026-08-16T12:00:00+00:00")
    return TR.relevanz(s, BINDUNG)


# Feld -> Regel, die es NICHT abschalten darf. Alle fünf sind Angaben, die der Vordruck verlangt,
# ohne dass die Regel von ihnen abhinge.
DEKLARATION_STATT_GATE = [
    ("vv_nutzung_ferienwohnung", "p21_vermietung_einkuenfte"),
    ("vv_nutzung_an_angehoerige", "p21_vermietung_einkuenfte"),
    ("vv_nutzung_kurzfristig", "p21_vermietung_einkuenfte"),
    ("vv_nebenkosten_nicht_vereinbart", "p21_vermietung_einkuenfte"),
    # § 35c: hier lag der Fehler in BEIDE Richtungen. „nein" (noch nie beantragt — der
    # Normalfall) schloss die Ermäßigung aus; „ja" ließ sie unangetastet stehen, obwohl genau
    # dann der 40.000-EUR-Objektdeckel anzurechnen wäre. Die Kumulation über Jahre ist nicht
    # modelliert (benannter Nachtrag), also ist die Bedingung eine offene Annahme.
    ("p35c_bereits_ermaessigung_frueher", "p35c_sanierung_ermaessigung"),
    # 2026-08-20, beide aus SCHULD hierher: die Norm schliesst in keinem der beiden Faelle aus.
    # § 9 Abs. 4a S. 8-9 sagt "sind die Verpflegungspauschalen zu KÜRZEN … die Kürzung darf die
    # ermittelte Verpflegungspauschale nicht übersteigen" — kürzen, nie streichen.
    ("vpf_keine_mahlzeitengestellung", "p9_4a_verpflegungsmehraufwand"),
    # § 9 Abs. 1 S. 3 Nr. 5 S. 4 Hs. 2 sagt über die Dienstwohnung nur, dass die 2.000-Euro-Grenze
    # bei einer Unterkunft im AUSLAND dann NICHT gilt. Sie hebt eine Obergrenze auf, ist also
    # günstig — als Gate schloss sie aus, und zwar auch im Inland, wo die Norm sie nicht erwähnt.
    ("dhf_keine_pflicht_dienstwohnung", "p9_1_3_nr5_doppelte_haushaltsfuehrung"),
    # 2026-09-26, GWG-Brutto-Pfad. § 9b Abs. 1 EStG nimmt nur die ABZIEHBARE Vorsteuer aus den
    # Anschaffungskosten; wer sie nicht abziehen darf, zieht brutto ab (Anleitung EÜR 2025
    # Z. 1126: "Kleinunternehmer geben den Bruttobetrag an"). "Netto? nein" ist damit eine
    # Angabe über die EINGABE, keine Voraussetzung — als Gate nahm sie dem Kleinunternehmer die
    # ganze GWG-Familie aus dem Dialog, samt der Folgefrage, die seinen Abzug rettet.
    # GEMESSEN vor dem Umbau, über den HTTP-Pfad: netto=nein -> p6_2_gwg_sofortabzug
    # "ausgeschlossen", alle drei gwg-Felder aus /fragen.
    ("gwg_netto_ohne_vorsteuer", "p6_2_gwg_sofortabzug"),
    # 2026-09-26, Vollständigkeitstest. § 10 Abs. 1 Nr. 5 S. 2 EStG nimmt Unterricht, Vermittlung
    # besonderer Fähigkeiten und Freizeitbetätigungen aus dem Abzug aus, S. 4 verlangt Rechnung und
    # unbare Zahlung. Beides sind Voraussetzungen des ABZUGS einer qualifizierenden Aufwendung —
    # nicht der Einkunftsart. "Betreuung war keine reine Betreuung" nimmt keine Einkünfte weg; die
    # Aufwendung ist dann unqualifiziert und mindert die Bemessungsgrundlage nicht.
    # GEMESSEN vor dem Eintrag, über den echten Pfad (API.event + API.ergebnis, Kind 1, Betrag
    # 6.000 EUR): beide Antworten lassen die Regel stehen (Status "unentschieden", nie
    # "ausgeschlossen"), und ein "nein" oder ein gar nicht gesetztes Feld liefert KEINE stille
    # Zahl, sondern den Sperrgrund kinderbetreuung_reine_betreuung_offen bzw.
    # kinderbetreuung_zahlung_offen. Kind 2 trägt weiter bei, wenn Kind 1 "nein" sagt — die Sperre
    # läuft je Kind-Instanz, nicht regelweit.
    ("kind_betreuung_reine_betreuung", "p10_1_5_kinderbetreuung"),
    ("kind_betreuung_rechnung_ueberweisung", "p10_1_5_kinderbetreuung"),
]


@pytest.mark.parametrize("feld,regel", DEKLARATION_STATT_GATE)
@pytest.mark.parametrize("wert", [True, False])
def test_deklarationsfeld_schaltet_die_regel_nicht_ab(feld, regel, wert):
    """Beide Antworten, beide Male muss die Regel stehen bleiben."""
    rel = _relevanz_mit(feld, wert)
    assert rel[regel]["status"] != "ausgeschlossen", (
        f"{feld}={wert} schließt {regel} aus. Das Feld ist Deklaration, keine Voraussetzung — "
        f"eine ganze Anlage verschwindet aus dem Dialog, ohne dass eine Zahl rot wird.")


@pytest.mark.parametrize("feld,regel", DEKLARATION_STATT_GATE)
def test_deklarationsfeld_wird_weiter_gefragt(feld, regel):
    """`gate: false` darf das Feld nicht stillegen — der Vordruck verlangt die Angabe.

    Ohne diese Hälfte wäre der bequemste Weg durch den Test oben, das Feld auf askable: false
    zu setzen: die Regel bliebe stehen, die Angabe fehlte im XML, und checkESt fiele erst
    Wochen später darüber.
    """
    b = BINDUNG.get(feld)
    assert b is not None, f"{feld} ist gar nicht mehr gebunden."
    assert b.get("askable") is True, f"{feld} wird nicht mehr gefragt."
    assert b.get("gate") is False, f"{feld} trägt kein gate: false mehr — dann ist es ein Gate."


def test_gate_false_reicht_nicht_der_rechenpfad_fuehrt_eine_zweite_liste():
    """Die Naht, an der `gate: false` allein wirkungslos bleibt.

    GEMESSEN 2026-08-20: `gate: false` bei dhf_keine_pflicht_dienstwohnung setzte den Status in
    relevanz() korrekt von "ausgeschlossen" auf "unentschieden" — und die Steuer blieb auf den
    Cent gleich. Der Abzug hing gar nicht am Gate, sondern an einer ZWEITEN, unabhängigen
    Repräsentation derselben Bedingung:

        bescheid_zweige.py:158/399   all(f.get(b, {}).get("wert") is True for b in DHF_BEDINGUNGEN)

    Zwei Listen, dieselbe Fachfrage, keine Verbindung — die Bindung sagte "keine Voraussetzung",
    der Rechenpfad sagte weiter "Voraussetzung", und nur der Rechenpfad zählt für die Zahl. Erst
    das Herausnehmen aus DHF_BEDINGUNGEN (eigenes Tupel DHF_AUSLANDSGRENZE) brachte die 3.178 EUR
    zurück.

    Dieser Test hält beide Repräsentationen zusammen: was in der Bindung `gate: false` trägt, darf
    im Rechenpfad keine Voraussetzung sein. Ohne ihn kann jemand ein Feld aus DHF_BEDINGUNGEN
    entfernen und das `gate: false` vergessen — oder umgekehrt, wie es hier zwei Jahre lang war.
    """
    import api_constants as AK
    voraussetzungs_tupel = {"DHF_BEDINGUNGEN": AK.DHF_BEDINGUNGEN,
                            "UEBERNACHTUNG_BEDINGUNGEN": AK.UEBERNACHTUNG_BEDINGUNGEN}
    kein_gate = {fid for fid, b in BINDUNG.items() if b.get("gate") is False}
    for name, tupel in voraussetzungs_tupel.items():
        doppelt = kein_gate & set(tupel)
        assert not doppelt, (
            f"{sorted(doppelt)} trägt in der Bindung `gate: false`, steht aber in {name} und ist "
            f"damit im Rechenpfad weiterhin Abzugsvoraussetzung. Die Bindung allein entscheidet "
            f"nicht über die Zahl — beide Stellen müssen dasselbe sagen.")


# ---------------------------------------------------------------- Sweep über alle bool-Gates

# Gates, deren „nein" die Regel zu Recht ausschließt: ein WAHLRECHT, das nicht ausgeübt wurde.
# Hier ist die Regel wirklich nicht anwendbar, nicht bloß eine Zeile des Vordrucks unbeantwortet.
ECHTES_OPT_IN = {
    "antrag_ermaessigter_satz":
        "§ 34 Abs. 3 wirkt nur auf Antrag; ohne Antrag gibt es die Regel nicht.",
}

# Gemessene, HEUTE offene Polaritätsfehler derselben Klasse — Schuld, kein Freibrief.
#
# STAND 2026-08-20: von den ZWEI hier gestapelten Fehlern ist der erste behoben, der zweite offen.
#
#   (1) ERFASSUNG — behoben. Die Oberfläche speicherte das Gegenteil der Nutzerantwort, weil sie
#       die Umkehr am Feldnamen riet (`startswith("kein_")`) und die Verneinung in der Mitte
#       dieser beiden Namen nicht sah. Jetzt deklariert die Bindung sie (`frage_invertiert`),
#       Gate in tests/test_bindung_frage_polaritaet.py, Nutzerpfad in
#       tests/test_ui_frage_polaritaet.py. Der Normalfall — „nein, keine Mahlzeiten" bzw. „nein,
#       keine Pflicht-Dienstwohnung" — speichert seither `true` und BEHÄLT seine Regel.
#
#   (2) GATE-EIGENSCHAFT — offen, Julius' Entscheidung. Ein bestätigtes `false` schließt weiter
#       die ganze Regel aus. Betroffen ist jetzt der jeweils andere Nutzer: wer wahrheitsgemäß
#       „ja" antwortet (Mahlzeiten wurden gestellt / es IST eine Pflicht-Dienstwohnung), verliert
#       den ganzen Abzug statt nur die Kürzung bzw. die Auslandsgrenze zu bekommen. Gemessen am
#       an_gesamt-Ring: dHf 348600 Cent Unterschied (314300 gegen 662900), Verpflegung geht bei
#       `false` gar nicht erst durch (grund=verpflegung_reduktion_offen).
#       Fachlich spricht beides für `gate: false`: die Mahlzeitenkürzung ist seit 826bfdf
#       modelliert, Mahlzeiten schließen die Regel also gar nicht mehr aus; und § 9 Abs. 1 S. 3
#       Nr. 5 S. 4 Hs. 2 betrifft nur die 2.000-Euro-Auslandsgrenze, nicht den Abzug dem Grunde
#       nach. Das ändert die Rechen-Semantik und bleibt deshalb liegen.
#
# Die Prüfung unten misst über `beispielwert` als FELDWERT, nicht über die Antwort des Nutzers —
# sie belegt (2), nicht (1).
# LEER seit 2026-08-20 — und das ist ein Ergebnis, kein Versehen. Hier standen die beiden Felder
# vpf_keine_mahlzeitengestellung und dhf_keine_pflicht_dienstwohnung, deren Gate-Eigenschaft dem
# Nutzer bei wahrheitsgemässer Antwort den ganzen Abzug nahm statt ihn zu kürzen bzw. gar nichts
# zu tun. Beide tragen jetzt `gate: false` (§ 9 Abs. 4a S. 8-9: "zu KÜRZEN … die Kürzung darf die
# ermittelte Verpflegungspauschale nicht übersteigen"; § 9 Abs. 1 S. 3 Nr. 5 S. 4 Hs. 2: die
# Dienstwohnung HEBT die Auslandsgrenze AUF, sie schliesst nichts aus).
#
# Ein Eintrag hier ist eine Schuld, kein Freibrief: test_schuld_eintraege_sind_noch_offen wird rot,
# sobald das Feld kein Gate mehr ist. Genau so ist diese Liste leer geworden.
SCHULD: dict[str, str] = {}


def _bool_gates() -> list[tuple[str, str, bool, int]]:
    """(feld, regel, beispielwert, Zahl der ÜBRIGEN askable Felder der Regel) je echtem Gate."""
    askable_je_regel: dict[str, list[str]] = {}
    for fid, b in BINDUNG.items():
        if b.get("askable"):
            askable_je_regel.setdefault(b["quelle"]["regel_id"], []).append(fid)
    out = []
    for fid, b in sorted(BINDUNG.items()):
        q = b.get("quelle", {})
        if not (b.get("askable") and b.get("gate", True)
                and "geltungsbedingung" in q and b.get("typ") in ("bool", "boolean")):
            continue
        bw = b.get("beispielwert")
        if not isinstance(bw, bool):
            continue
        rid = q["regel_id"]
        out.append((fid, rid, bw, len([f for f in askable_je_regel.get(rid, []) if f != fid])))
    return out


def test_normalfall_antwort_laesst_die_regel_stehen():
    """Der Sweep. Ein Gate, dessen Normalfall-Antwort die eigene Regel abschaltet, nimmt dem
    typischen Nutzer alle übrigen Fragen dieser Regel — still, ohne Fehlermeldung.

    Die Einschränkung „Regel hat weitere askable Felder" ist keine Bequemlichkeit, sondern die
    Trennlinie: die fünf Screening-Flags (`kein_gewinn` & Co.) sind das einzige Feld ihrer
    Pseudo-Regel. Deren Ausschluss kostet nichts — die Wirkung läuft über
    `regel_bedingungen` auf die echten Regeln.
    """
    schaden = []
    for fid, rid, bw, rest in _bool_gates():
        if rest == 0 or fid in ECHTES_OPT_IN or fid in SCHULD:
            continue
        if _relevanz_mit(fid, bw)[rid]["status"] == "ausgeschlossen":
            schaden.append(f"{fid}={bw} schließt {rid} aus und nimmt {rest} weitere Fragen mit")
    assert not schaden, (
        "Gate-Polarität verdreht — der Normalfall schaltet seine eigene Regel ab:\n"
        + "\n".join(f"   - {z}" for z in schaden)
        + "\nEntweder ist es Deklaration (dann gate: false in der Bindung), ein echtes Wahlrecht "
          "(dann nach ECHTES_OPT_IN mit Begründung) oder ein Fehler (dann Fragetext/Polarität "
          "richtigstellen).")


@pytest.mark.parametrize("feld", sorted(SCHULD))
def test_schuld_eintraege_sind_noch_offen(feld):
    """Ein Eintrag in SCHULD ist eine Schuld, kein Freibrief: läuft er sauber, muss er raus —
    sonst bleibt eine geschlossene Lücke für immer als Dauerausnahme stehen (dieselbe Mechanik
    wie BLOCKIERTE_BLOECKE in der Blockmatrix)."""
    treffer = [(rid, bw, rest) for fid, rid, bw, rest in _bool_gates() if fid == feld]
    assert treffer, f"{feld} ist kein bool-Gate mehr — Eintrag aus SCHULD entfernen."
    rid, bw, rest = treffer[0]
    assert rest > 0, f"{feld}: {rid} hat keine weiteren askable Felder mehr — Eintrag entfernen."
    assert _relevanz_mit(feld, bw)[rid]["status"] == "ausgeschlossen", (
        f"{feld} ist als offener Polaritätsfehler eingetragen, verhält sich aber korrekt. "
        f"Wenn der Fund behoben wurde: Eintrag aus SCHULD entfernen.")


def test_schuld_eintraege_sind_begruendet():
    for feld, grund in SCHULD.items():
        assert "BACKLOG" in grund, (
            f"{feld}: Begründung ohne BACKLOG-Verweis — dann findet den Punkt später niemand.")


# ------------------------------------------------- Vollständigkeit der `gate: false`-Felder

# Jedes Feld mit `gate: false` schaltet eine Regel nicht mehr ab. Das ist eine starke Aussage,
# und bis zum 2026-09-26 war sie fuer die Haelfte der Felder UNBEZEUGT: DEKLARATION_STATT_GATE
# ist eine handgepflegte Liste, und nichts erzwang, dass ein neues `gate: false` dort auftaucht
# oder sonst einen Beleg bekommt. GEMESSEN am 2026-09-26: 14 Felder trugen `gate: false`,
# 8 standen in DEKLARATION_STATT_GATE, 6 nicht — darunter die vier p16_4-Voraussetzungen und die
# zwei kind_betreuung_*.
#
# Dieser Test schliesst die Luecke: jedes `gate: false`-Feld ist ENTWEDER in
# DEKLARATION_STATT_GATE registriert ODER hier namentlich mit einer Pruefdatei belegt. Ein neues
# `gate: false` ohne das eine oder das andere macht ihn rot.
#
# WARUM NICHT EINFACH ALLE REGISTRIEREN: DEKLARATION_STATT_GATE traegt je Eintrag eine Begruendung
# und pinnt sie ueber `test_deklarationsfeld_schaltet_die_regel_nicht_ab`. Die vier p16_4-Felder
# haben ihren eigenen, schaerferen Beleg (tests/test_p16_4_freibetrag_fragt_den_gewinn.py) und
# gehoeren dort nicht zusaetzlich hinein — zwei Listen, dieselbe Frage ist die Bauart, die dieser
# Test gerade verhindern soll.

# gate: false -> Datei, die seine Berechtigung belegt (namentlich, nicht per Ordner-Scan)
BELEGT_ANDERSWO: dict[str, str] = {
    "rentner_alter_55_oder_berufsunfaehig":
        "tests/test_p16_4_freibetrag_fragt_den_gewinn.py — § 16 Abs. 4 S. 1-2: die Voraussetzungen "
        "des FREIBETRAGS, nicht der Einkunftsart. Ein 'nein' entfernt einen Abzug, keine Einkunft.",
    "rentner_alter_55_oder_berufsunfaehig_partner":
        "tests/test_p16_4_freibetrag_fragt_den_gewinn.py — Partner-Spiegel zu Person A.",
    "rentner_freibetrag_erstmalig":
        "tests/test_p16_4_freibetrag_fragt_den_gewinn.py — § 16 Abs. 4 S. 2, nur einmal zu gewaehren.",
    "rentner_freibetrag_erstmalig_partner":
        "tests/test_p16_4_freibetrag_fragt_den_gewinn.py — Partner-Spiegel zu Person A.",
}


def _gate_false_felder() -> set[str]:
    return {fid for fid, b in BINDUNG.items() if b.get("gate") is False}


def test_jedes_gate_false_feld_ist_registriert_oder_belegt():
    """Die Luecke selbst: kein `gate: false` ohne Registrierung und ohne Beleg.

    Ohne diesen Test kann jemand `gate: false` an ein Feld schreiben, das tatsaechlich eine
    Rechen-Voraussetzung ist — die Regel bliebe dann stehen, obwohl die Antwort sie abschalten
    muesste, und niemand merkte es.
    """
    registriert = {fid for fid, _ in DEKLARATION_STATT_GATE}
    ungedeckt = []
    for fid in sorted(_gate_false_felder()):
        if fid in registriert or fid in BELEGT_ANDERSWO:
            continue
        ungedeckt.append(fid)
    assert not ungedeckt, (
        f"{ungedeckt} tragen `gate: false`, stehen aber weder in DEKLARATION_STATT_GATE noch in "
        f"BELEGT_ANDERSWO. Ein `gate: false` behauptet: die Antwort schaltet die Regel NICHT ab. "
        f"Entweder in DEKLARATION_STATT_GATE registrieren (mit Begruendung) oder hier einen "
        f"namentlichen Beleg eintragen — sonst ist die Behauptung unbezeugt.")


@pytest.mark.parametrize("feld", sorted(BELEGT_ANDERSWO))
def test_belegte_gate_false_felder_tragen_wirklich_gate_false(feld):
    """Die Gegenrichtung: ein Beleg fuer ein Feld, das kein `gate: false` mehr traegt, ist
    veraltet und muss raus — sonst waechst hier eine zweite Dauerausnahme-Liste."""
    b = BINDUNG.get(feld)
    assert b is not None, f"{feld} ist gar nicht mehr gebunden — Beleg entfernen."
    assert b.get("gate") is False, (
        f"{feld} traegt kein `gate: false` mehr. Der Beleg in BELEGT_ANDERSWO ist damit "
        f"gegenstandslos — entfernen oder die Ursache klaeren.")


def test_belegte_gate_false_felder_lassen_die_regel_stehen():
    """Der Beleg muss auch wirken: beide Antworten lassen die Regel des Feldes stehen. Das ist
    dieselbe Pruefung wie bei DEKLARATION_STATT_GATE, nur fuer die namentlich belegten Felder —
    die Zahl der Abdeckung ist damit 14 von 14, nicht 8 von 14."""
    for feld, _ in sorted(BELEGT_ANDERSWO.items()):
        rid = BINDUNG[feld]["quelle"]["regel_id"]
        for wert in (True, False):
            st = _relevanz_mit(feld, wert)[rid]["status"]
            assert st != "ausgeschlossen", (
                f"{feld}={wert} schliesst {rid} aus, obwohl das Feld `gate: false` traegt und "
                f"namentlich belegt ist. Der Beleg traegt nicht.")
