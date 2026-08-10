"""Feld-Matrix gegen die amtliche Pruefung: jedes Kz-tragende Feld einmal EINZELN scharf.

WARUM DIESER TEST EXISTIERT — die Luecke zwischen zwei guten Gates.

Es gab schon zwei Gates, die sich nicht ueberlappen:

  test_ring_deklaration_differential   jedes Betragsfeld hat einen Weg ins XML
                                       (Kz / nicht_deklariert / Aggregat / Ausschluss)
                                       -> kennt ERiC NICHT
  test_checkest_durchstich (Ratsche)   das XML ist amtlich plausibel
                                       -> kennt ERiC, faehrt aber EINEN Fall

Am 2026-08-10 fiel dazwischen ein Abgabe-Blocker: der Differential-Test setzt selbst
`kap_kapitalertraege = 500000` und war gruen, weil E1900701 im XML ankommt. Gegen checkESt
gefahren ergibt derselbe Wert rc=610001002:

    "Auf den Anlagen KAP und / oder KAP-BET wurden Kapitalertraege erklaert, die dem
     inlaendischen Steuerabzug unterlegen haben. Bitte geben Sie auf der Anlage KAP auch
     einen Grund fuer die Angabe der Kapitalertraege an."

Die Ratsche sah es nicht, weil ihre Fixtur `kap_kapitalertraege = 0` fuehrt (Option A:
Nullwerte nicht deklarieren — korrekt, deckt aber genau deshalb den Fall nicht ab).
Zusammenstecken lassen sich die beiden nicht: die Differential-Szenarien haben keine
Stammdaten und sind strukturell nicht abgabefaehig, koennen ERiC also nie erreichen.
Deshalb setzt diese Matrix auf der Ratschen-Fixtur auf, die Stammdaten fuehrt.

WAS DIESE MATRIX FINDET: Felder, die wir erklaeren, ohne das mitzuliefern, was das Finanzamt
dazu verlangt — Antragsgruende, Pflicht-Begleitangaben, Feldkopplungen.

WAS SIE NICHT FINDET: Rechenfehler. ERiC prueft Plausibilitaet, nicht Richtigkeit. Ob 8400
Cent die korrekte Kuerzung sind, sagen Golden Cases und der GETTSIM-Crosscheck.

KOSTEN: ~0,75 s je Lauf, die ganze Datei unter 10 s.
"""

from __future__ import annotations

import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("elster", "produkt/haut", "produkt/import", "produkt/mapping",
            "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, HERE)

import api as API                   # noqa: E402
import checkest_gate as CE          # noqa: E402
import elster_xml as EX             # noqa: E402
import est_mapping                  # noqa: E402
import store as ST                  # noqa: E402
import traverser as TR              # noqa: E402

from test_checkest_durchstich import (  # noqa: E402
    _ABSENDER, _HID, _b, _fall_einzel, braucht_eric,
)


def _scharf(store) -> tuple[int, list[str]]:
    """Store -> Ring-Werte -> Deklaration -> Abgabe-XML -> amtliches Plugin. Kein Versand.

    Der Weg MUSS ueber `_mit_ring_werten` gehen, sonst misst die Matrix den falschen Pfad.
    Die erste Fassung ging direkt store -> est_mapping.deklariere und uebersprang damit alle
    Ring-Injektionen — E0205508 (Verpflegungskuerzung) und die KAP-Antragsfelder entstehen aber
    genau dort. Ein Bau, der nur in `_mit_ring_werten` sitzt, waere hier unsichtbar gruen
    geblieben. Gefunden 2026-08-10 von p33b-2b beim Bau des KAP-Antrags.

    `_fall_einzel()` liefert einen rohen Store ohne `scheibe`; die setzen wir hier, weil
    _scheibe_bindung()/_cfg() sie brauchen. Die Ratschen-Fixtur selbst bleibt unangetastet.
    """
    store = dict(store)
    store.setdefault("scheibe", "gesamt")
    bindung = API._scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    felder = API._mit_ring_werten(felder, 2025)
    xml = EX.erzeuge_xml(est_mapping.deklariere(felder, bindung, snapshot_id=sid),
                         vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split())
             for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    return rc, texte


def _mit(feld_id, wert):
    """Ratschen-Basisfall plus EIN zusaetzliches Feld.

    Der Store ist fail-closed gegen Ueberschreiben eines aktiven Events (store.py:232),
    darum wird ein bereits in der Fixtur gesetztes Feld ersetzt statt doppelt angelegt.
    """
    s = _fall_einzel()
    aktiv = None
    for e in reversed(s.get("events") or []):
        if e.get("feld_id") == feld_id and not e.get("ersetzt_durch"):
            aktiv = e["event_id"]
            break
    if aktiv:
        ST.append_event(s, feld_id=feld_id, wert=wert, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                        schreiber="laie",
                        signal={"signal_1": {"typ": "laie_eingabe"},
                                "signal_2": "laie_bestaetigt"},
                        ersetzt=aktiv)
    else:
        _b(s, feld_id, wert)
    return s


# Felder, die ALLEIN eine sinnvolle Erklaerung ergeben. Kuratiert, nicht generiert — und
# das ist eine gemessene Entscheidung, keine Bequemlichkeit:
#
# Der erste Wurf zog alle 55 Kz-tragenden askable Felder automatisch aus der Bindung. 29
# davon wurden rot, aber nur eine Minderheit waren echte Funde. Die Mehrheit war Rauschen
# aus zwei Quellen:
#   (a) Feld ohne seinen Block  — kind_kv ohne Anlage Kind ("Tragen Sie bitte den Vornamen
#       des Kindes ein"), p35c ohne Gebaeude-Standort, vv_einnahmen ohne Laufende_Nummer_V,
#       gewst_hebesatz ohne gewst_messbetrag. Der Fall ist unvollstaendig, nicht der Bau.
#   (b) untauglicher Probewert  — GdB=5 ("ungueltiger Grad der Behinderung", zulaessig sind
#       20/30/.../100), Pflegegrad=5 ("ungueltiger Schluesselwert").
# `instanz_gruppe` trennt (a) nur teilweise: p35c_* und p33a_* brauchen ebenfalls Kontext,
# fuehren aber keine Instanzgruppe. Ein Gate mit 70 % Rauschen wird weggeklickt statt
# gelesen — deshalb hier nur, was fuer sich steht.
#
# Ein Feld gehoert in diese Liste, wenn es ohne Begleit-Instanz erklaerbar ist. Der Rest
# braucht eine Block-Matrix (minimal vollstaendige Anlage je Block) — eigener Bau, siehe
# BACKLOG checkest-blockmatrix.
MATRIX = [
    # Anlage KAP — Betraege laut Steuerbescheinigung
    ("kap_kapitalertraege", 500000),
    ("kap_gewinn_aktien", 200000),
    ("kap_verlust_aktien", 100000),
    ("kap_verlust_sonstige", 50000),
    # § 35a — die drei Toepfe, jeder fuer sich erklaerbar
    ("hh_handwerker_arbeitskosten", 300000),
    ("hh_dienstleistungen", 200000),
    ("hh_minijob_aufwendungen", 100000),
    # § 33 / Vorsorge / Anlage N — je ein Vertreter, der allein steht
    ("agb_aufwendungen", 500000),
    ("tage_24h", 10),
    ("vor_rv_ausserhalb_lstb", 100000),
    # basis_kv steht NICHT hier: es braucht `versicherungsart`, sonst bricht schon der
    # Writer ab ("Art (versicherungsart) unbestaetigt — Kz-Zweig offen"). Gehoert damit in
    # die Block-Matrix, nicht in die Einzelfeld-Matrix.
]

# Bekannte, GEMESSENE Beanstandungen mit Begruendung. Ein Eintrag hier ist eine Schuld,
# kein Freibrief: er benennt, was fehlt, damit der Test nicht dauerrot ist und trotzdem
# niemand vergisst, dass der Fall heute nicht einreichbar ist.
# Anlage KAP stand hier bis 2026-08-10 mit vier Eintraegen (Antragsgrund fehlt, rc=610001002).
# GESCHLOSSEN: E1900401 (Antrag Guenstigerpruefung) + E1901401 (in Anspruch genommener
# Sparer-Pauschbetrag) werden jetzt in _mit_ring_werten injiziert; alle vier Faelle erreichen
# rc=0. Der Test hat das selbst erzwungen — ein Eintrag hier, der sauber durchlaeuft, schlaegt
# fehl, damit eine geschlossene Luecke nicht als Dauer-Ausnahme stehen bleibt.
BEKANNTE_LUECKEN = {
    "hh_handwerker_arbeitskosten": (
        "§ 35a: checkESt verlangt eine EINZELAUFSTELLUNG, wir liefern nur die Summe. Von dieser "
        "Matrix beim ersten Lauf gefunden (2026-08-10, rc=610001002): '...es wurde aber keine "
        "Einzelaufstellung der Handwerkerleistungen vorgenommen.' BACKLOG p35a-einzelaufstellung."),
    "hh_dienstleistungen": (
        "§ 35a, gleiche Ursache wie hh_handwerker_arbeitskosten: Einzelaufstellung fehlt. "
        "Der Befund ist damit systematisch ueber alle drei § 35a-Toepfe, nicht punktuell."),
    "hh_minijob_aufwendungen": (
        "§ 35a Minijobs, gleiche Ursache: 'Gesamtbetrag der Aufwendungen angegeben, es wurde "
        "aber kein(e Einzelaufstellung)...' — dritter von drei Toepfen."),
}


@braucht_eric
@pytest.mark.parametrize("feld_id,wert", MATRIX)
def test_feld_einzeln_bleibt_amtlich_plausibel(feld_id, wert):
    """Ein Feld anschalten darf die Erklaerung nicht uneinreichbar machen.

    Faellt ein Feld hier neu durch, fehlt eine Begleitangabe, die das Finanzamt zu genau
    diesem Feld verlangt — dieselbe Klasse wie der KAP-Antragsgrund. Der Fehlertext von
    ERiC steht in der Assertion, er benennt die fehlende Angabe meist woertlich.
    """
    rc, texte = _scharf(_mit(feld_id, wert))

    klasse = CE.klassifiziere_rc(rc)
    assert klasse != "io_gate_nicht_geprueft", (
        f"{feld_id}: rc={rc} — XML bricht VOR der Pruefung ab. Ein leerer Fehlerpuffer "
        f"heisst hier NICHT fehlerfrei.")

    if feld_id in BEKANNTE_LUECKEN:
        assert rc != CE.RC_OK, (
            f"{feld_id} ist als bekannte Luecke eingetragen, laeuft aber sauber durch "
            f"(rc={rc}). Wenn die Luecke geschlossen wurde: Eintrag aus BEKANNTE_LUECKEN "
            f"entfernen — sonst deckt der Test die Regression nicht mehr ab.")
        return

    assert rc == CE.RC_OK, (
        f"{feld_id}={wert} macht die Erklaerung uneinreichbar (rc={rc}).\n"
        + "\n".join(f"   - {t}" for t in texte[:5])
        + "\nEntweder fehlt eine Begleitangabe (dann bauen) oder es ist eine bewusste "
          "Luecke (dann mit gemessener Begruendung in BEKANNTE_LUECKEN eintragen).")


def test_kap_antrag_ist_inert_ohne_kapitalertraege():
    """Der KAP-Antrag darf NUR erscheinen, wenn Kapitalertraege erklaert werden.

    Braucht kein ERiC. Ohne diese Gegenprobe waere ein Bau, der den Antrag bedingungslos setzt,
    gruen — und wir wuerden fuer jeden Nutzer die Guenstigerpruefung beantragen, auch fuer die
    ohne einen einzigen Kapitalertrag. Die Standard-Falle bei Injektionen: das Feuern testen und
    das Schweigen vergessen.

    E1900401 = Anlage KAP Zeile 4 (Antrag Guenstigerpruefung, § 32d Abs. 6)
    E1901401 = Anlage KAP Zeile 16 (in Anspruch genommener Sparer-Pauschbetrag)
    """
    def _kz_im_xml(store):
        store = dict(store)
        store.setdefault("scheibe", "gesamt")
        bindung = API._scheibe_bindung(store)
        felder, sid = ST.materialisiere(store)
        felder = API._mit_ring_werten(felder, 2025)
        xml = EX.erzeuge_xml(est_mapping.deklariere(felder, bindung, snapshot_id=sid),
                             vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
        return "E1900401" in xml, "E1901401" in xml

    antrag_ohne, pb_ohne = _kz_im_xml(_fall_einzel())          # Fixtur fuehrt kap = 0
    assert not antrag_ohne, (
        "E1900401 steht im XML, obwohl der Fall keine Kapitalertraege erklaert — dann "
        "beantragen wir die Guenstigerpruefung fuer jeden Nutzer.")
    assert not pb_ohne, (
        "E1901401 steht im XML ohne Kapitalertraege — ein Sparer-Pauschbetrag ohne Ertraege, "
        "auf die er entfaellt.")

    antrag_mit, pb_mit = _kz_im_xml(_mit("kap_kapitalertraege", 500000))
    assert antrag_mit and pb_mit, (
        f"Bei 5.000 EUR Kapitalertraegen fehlt im XML: "
        f"{'E1900401 ' if not antrag_mit else ''}{'E1901401' if not pb_mit else ''} — "
        f"ohne beide ist die Erklaerung uneinreichbar (rc=610001002).")


def test_matrix_geht_durch_den_ring_pfad():
    """Der Weg dieser Matrix MUSS die Ring-Injektionen mitnehmen — sonst misst sie das Falsche.

    Braucht kein ERiC: geprueft wird der Pfad, nicht die Plausibilitaet.

    Die erste Fassung von `_scharf` ging direkt store -> est_mapping.deklariere und uebersprang
    `_mit_ring_werten`. Damit waere jeder Bau, der dort sitzt, hier unsichtbar gruen geblieben —
    die Matrix haette Abwesenheit von Beanstandungen gemeldet, ohne den Code je auszufuehren.
    Dieser Test haelt die Differenz fest, statt sie einer Messung zu ueberlassen, die niemand
    wiederholt.
    """
    store = _mit("tage_24h", 10)
    for feld, wert in [("vpf_fruehstuecke_gestellt_anzahl", 5),
                       ("vpf_mittagessen_gestellt_anzahl", 5)]:
        _b(store, feld, wert)
    store = dict(store)
    store["scheibe"] = "gesamt"

    bindung = API._scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)

    ohne = est_mapping.deklariere(dict(felder), bindung, snapshot_id=sid)
    mit_ring = API._mit_ring_werten(felder, 2025)
    mit = est_mapping.deklariere(mit_ring, bindung, snapshot_id=sid)

    kuerzung = (mit_ring.get("p9_4a_kuerzung_nach_entgelt") or {}).get("wert")
    assert kuerzung and kuerzung > 0, (
        f"Vorbedingung kaputt: der Ring kuerzt bei 5 Fruehstuecken + 5 Mittagessen nicht "
        f"({kuerzung}). Ohne Kuerzung sagt dieser Test nichts aus.")

    assert "E0205508" not in ohne.get("deklaration", {}), (
        "E0205508 entsteht schon OHNE _mit_ring_werten — dann ist dieser Test kein Nachweis "
        "mehr, dass der Ring-Pfad laeuft. Der Nachweis muss neu gebaut werden.")
    assert "E0205508" in mit.get("deklaration", {}), (
        f"Der Ring kuerzt {kuerzung} Cent, aber E0205508 kommt nicht in der Deklaration an — "
        f"_scharf() umgeht die Ring-Injektionen. Genau dieser Fehler machte die Matrix blind "
        f"fuer alles, was in _mit_ring_werten gebaut wird.")


@braucht_eric
def test_basisfall_ohne_zusatzfeld_ist_sauber():
    """Kontrolle: die Fixtur selbst ist rc=0.

    Ohne diesen Test waere eine Matrix, in der ALLES rot ist, nicht von einer kaputten
    Fixtur zu unterscheiden.
    """
    rc, texte = _scharf(_fall_einzel())
    assert rc == CE.RC_OK, (
        f"Basisfall der Matrix ist selbst nicht plausibel (rc={rc}) — dann sagt kein "
        f"einziger Matrix-Eintrag etwas aus.\n" + "\n".join(f"   - {t}" for t in texte[:5]))


@braucht_eric
def test_bekannte_luecken_sind_begruendet():
    """Jeder Freistellungs-Eintrag nennt Messung und Grund — kein stilles Ausklammern."""
    for feld, grund in BEKANNTE_LUECKEN.items():
        assert feld in dict(MATRIX), (
            f"{feld} steht in BEKANNTE_LUECKEN, wird aber gar nicht gefahren — "
            f"toter Eintrag, der eine Abdeckung vortaeuscht.")
        assert len(grund) > 20, f"{feld}: Begruendung zu duenn, um spaeter noch zu tragen"
