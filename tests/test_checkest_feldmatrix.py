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
    _ABSENDER, _HID, _b, _fall_einzel, _fall_einzel_kirchensteuerpflichtig, braucht_eric,
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
    # KAP Stufe 2 (Zeile 37-39, § 36 Abs.2 Nr.2 Anrechnung Abzugsteuer) NICHT hier: kein
    # Alleinsteher. GEMESSEN 2026-08-10: p36_kapitalertragsteuer allein (Basisfixtur fuehrt
    # kap_kapitalertraege=0) ergibt rc=610001002 — "Kapitalertragsteuer und/oder Solidaritaets-
    # zuschlag zur Kapitalertragsteuer laut Bescheinigung(en) eingegeben, es wurden aber keine
    # zugehoerigen Kapitalertraege erklaert." Dieselbe Klasse wie basis_kv/versicherungsart:
    # Feld ohne seinen Block. p36_kapitalertragsteuer_kist zusaetzlich an kist_konfession="keine"
    # (Religionsschluessel 11) gekoppelt. Eigener Test statt MATRIX-Eintrag, s.
    # test_p36_kap_anrechnung_amtlich_plausibel_mit_kapitalertraegen unten.
    # § 35a NICHT hier: die drei Sum-Felder sind seit der Einzelaufstellung (2026-08-10)
    # askable:false, ein einzeln gesetztes Sum-Feld deklariert den alten Kz zwar noch, bleibt
    # aber OHNE Einz-Instanz beim selben rc=610001002 -- Art+Betrag GEHOEREN zusammen, die
    # Ein-Feld-Matrix misst das falsch. Eigener Test unten:
    # test_p35a_einzelaufstellung_alle_drei_toepfe_amtlich_plausibel.
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
# § 35a (hh_handwerker_arbeitskosten/hh_dienstleistungen/hh_minijob_aufwendungen) stand hier bis
# 2026-08-10 mit drei Eintraegen ("Einzelaufstellung fehlt", rc=610001002). ENTFERNT, nicht
# GESCHLOSSEN-markiert: die Sum-Felder sind jetzt askable:false und werden gar nicht mehr
# einzeln erklaert (kein MATRIX-Eintrag mehr fuer sie) — die Luecke ist mit einem eigenen Test
# geschlossen, s. test_p35a_einzelaufstellung_alle_drei_toepfe_amtlich_plausibel unten.
BEKANNTE_LUECKEN = {
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
    assert klasse not in CE.NICHT_GEPRUEFT_KLASSEN, (
        f"{feld_id}: rc={rc} [{klasse}] — XML bricht VOR der Pruefung ab. Ein leerer "
        f"Fehlerpuffer heisst hier NICHT fehlerfrei.")

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


def _ersetzen(s, feld_id, wert):
    """Setzt feld_id auf wert, ersetzt ein evtl. schon aktives Event (Store fail-closed B,
    store.py:232). `_fall_einzel_kirchensteuerpflichtig()` fuehrt kap_kapitalertraege=0 bereits
    aktiv — ein zweites _b() auf dasselbe Feld waere ein doppeltes aktives Event."""
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


@braucht_eric
def test_p36_kap_anrechnung_amtlich_plausibel_mit_kapitalertraegen():
    """KAP Stufe 2 (Zeile 37-39) im vollstaendigen Block: mit Kapitalertraegen UND
    kirchensteuerpflichtiger Konfession sind KapESt/SolZ/KiSt amtlich plausibel.

    GEMESSEN 2026-08-10: allein (kap_kapitalertraege=0, kist_konfession='keine') ergibt
    rc=610001002 fuer KapESt/SolZ ("keine zugehoerigen Kapitalertraege erklaert") — deshalb
    kein MATRIX-Eintrag, sondern dieser Block-Test mit den noetigen Begleitangaben.
    """
    s = _fall_einzel_kirchensteuerpflichtig()
    _ersetzen(s, "kap_kapitalertraege", 500000)
    _b(s, "p36_kapitalertragsteuer", 135680)
    _b(s, "p36_kapitalertragsteuer_solz", 7460)
    _b(s, "p36_kapitalertragsteuer_kist", 12211)
    rc, texte = _scharf(s)
    assert rc == CE.RC_OK, (
        f"KAP-Anrechnung (Zeile 37-39) macht die Erklaerung uneinreichbar (rc={rc}).\n"
        + "\n".join(f"   - {t}" for t in texte[:5]))


def test_p36_kap_anrechnung_kz_inert_ohne_angabe():
    """KAP Stufe 2 (Zeile 37-39): E1904701/E1904901/E1904801 duerfen nur im XML stehen, wenn
    der Nutzer die Abzugsteuer laut Steuerbescheinigung bestaetigt hat. Braucht kein ERiC.

    E1904701 = Anlage KAP Zeile 37 (Kapitalertragsteuer)
    E1904901 = Anlage KAP Zeile 38 (Solidaritaetszuschlag zur KapESt)
    E1904801 = Anlage KAP Zeile 39 (Kirchensteuer zur KapESt)
    """
    def _kz_im_xml(store):
        store = dict(store)
        store.setdefault("scheibe", "gesamt")
        bindung = API._scheibe_bindung(store)
        felder, sid = ST.materialisiere(store)
        felder = API._mit_ring_werten(felder, 2025)
        xml = EX.erzeuge_xml(est_mapping.deklariere(felder, bindung, snapshot_id=sid),
                             vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
        return "E1904701" in xml, "E1904901" in xml, "E1904801" in xml

    kapest_ohne, solz_ohne, kist_ohne = _kz_im_xml(_fall_einzel())
    assert not (kapest_ohne or solz_ohne or kist_ohne), (
        "E1904701/E1904901/E1904801 stehen im XML, obwohl der Nutzer keine Angabe zur "
        "Kapitalertragsteuer-Abzugsteuer gemacht hat.")

    s = _mit("p36_kapitalertragsteuer", 135680)
    _b(s, "p36_kapitalertragsteuer_solz", 7460)
    _b(s, "p36_kapitalertragsteuer_kist", 12211)
    kapest_mit, solz_mit, kist_mit = _kz_im_xml(s)
    assert kapest_mit and solz_mit and kist_mit, (
        f"Bei bestaetigter KapESt/SolZ/KiSt fehlt im XML: "
        f"{'E1904701 ' if not kapest_mit else ''}{'E1904901 ' if not solz_mit else ''}"
        f"{'E1904801' if not kist_mit else ''}")


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


# ---- § 35a Einzelaufstellung: eigener Test statt MATRIX-Zeilen -------------------------------
#
# Der Fund, der diese Datei ausgeloest hat (s. Modul-Docstring): drei Sum-Felder waren rc=610001002,
# weil checkESt neben der Summe eine Einzelaufstellung (Einz[maxOccurs=99] je Topf, XSD-Sektion
# St_Erm_1426027020_CType, s. reports/adjudikation/anlage-haushaltsnahe-einzelaufstellung-2026-08-10.md)
# verlangt. MATRIX setzt genau EIN Feld je Fall (_mit) — dafuer taugt ein Feldpaar (Art+Betrag je
# Instanz) nicht. Deshalb ein eigener Test, kein MATRIX-Eintrag.

def test_p35a_einzelaufstellung_alle_drei_toepfe_amtlich_plausibel():
    """Je Topf EIN vollstaendiger Einz-Posten (Art+Betrag, Instanz 1 = bare feld_id) → rc=0.

    Die Sum-Kz (E0104109/E0107208/E0111215) werden NICHT direkt gesetzt — sie kommen aus
    _mit_ring_werten (4), berechnet aus genau diesen Instanz-Feldern. Ein Test, der die Sum-Felder
    weiter direkt schriebe, wuerde den alten, jetzt abgeklemmten Pfad pruefen statt den echten.
    """
    s = _fall_einzel()
    for betrag_fid, art_fid, betrag, art in (
        ("hh_minijob_betrag", "hh_minijob_art", 100000, "Haushaltshilfe"),
        ("hh_dienstleistung_betrag", "hh_dienstleistung_art", 200000, "Gartenpflege"),
        ("hh_handwerker_betrag", "hh_handwerker_art", 300000, "Heizungswartung"),
    ):
        _b(s, betrag_fid, betrag)
        _b(s, art_fid, art)

    rc, texte = _scharf(s)
    assert rc == CE.RC_OK, (
        f"§35a Einzelaufstellung (alle drei Toepfe, je ein Art+Betrag-Posten) macht die "
        f"Erklaerung uneinreichbar (rc={rc}).\n" + "\n".join(f"   - {t}" for t in texte[:5]))


def test_p35a_ohne_daten_kein_einz_und_keine_sum_im_xml():
    """Gegenprobe zum Inert-Vertrag von _mit_ring_werten (4): keine §35a-Angaben → weder Sum-
    noch Einz-Kz im XML (keine Instanz-Σ > 0 heisst KEIN Eintrag, nicht Eintrag mit Wert 0).

    Ohne diese Gegenprobe waere eine Injektion, die die Sum-Kz bedingungslos mit 0 fuellt,
    unsichtbar gruen — Standardfalle bei Injektionen (s. test_kap_antrag_ist_inert_ohne_
    kapitalertraege oben, dieselbe Klasse Test fuer § 35a statt Anlage KAP).
    """
    store = dict(_fall_einzel())
    store.setdefault("scheibe", "gesamt")
    bindung = API._scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    felder = API._mit_ring_werten(felder, 2025)
    xml = EX.erzeuge_xml(est_mapping.deklariere(felder, bindung, snapshot_id=sid),
                         vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
    for kz in ("E0104109", "E0104206", "E0104108",   # Minijob (Sum, Art, Betrag)
               "E0107208", "E0107206", "E0107207",   # Dienstleistung
               "E0111215", "E0111217", "E0111214"):  # Handwerker
        assert kz not in xml, (
            f"{kz} steht im XML, obwohl der Fall keine §35a-Haushaltsnahe-Angaben fuehrt.")
