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
geblieben, ohne dass irgendetwas rot geworden wäre. Dieselbe Klasse wie 519199e
([[gate-polaritaet-normalfall-antwort]]).

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
SCHULD = {
    "vpf_keine_mahlzeitengestellung":
        "Gate-Eigenschaft offen: ein bestätigtes false (= Mahlzeiten wurden gestellt) schließt "
        "den GANZEN Verpflegungsmehraufwand aus (17 weitere askable Felder), statt nur zu "
        "kürzen. Die Kürzung ist seit 826bfdf modelliert — Mahlzeiten dürften die Regel gar "
        "nicht mehr ausschließen. Die Erfassungs-Hälfte ist behoben (frage_invertiert). "
        "BACKLOG gate-polaritaet-vpf-dhf.",
    "dhf_keine_pflicht_dienstwohnung":
        "Gate-Eigenschaft offen: ein bestätigtes false (= es IST eine Pflicht-Dienstwohnung) "
        "schließt die ganze doppelte Haushaltsführung aus (6 weitere askable Felder, gemessen "
        "348600 Cent). Die Norm betrifft nur die 2.000-Euro-Auslandsgrenze, nicht den Abzug dem "
        "Grunde nach. Die Erfassungs-Hälfte ist behoben (frage_invertiert). BACKLOG "
        "gate-polaritaet-vpf-dhf.",
}


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
