"""Der Antrag auf den ermäßigten Satz nach § 34 Abs. 3 EStG erreicht die ELSTER-Erklärung.

Vault backlog/taxgraph/p34-antrag-ohne-kennzahl-erreicht-elster-nicht.md (AK1, AK3). Gemessen am
2026-10-03 auf 9ad51c19: `antrag_ermaessigter_satz` trägt `elster_kz: null`, und kein anderes Feld schrieb
die Antragszeile. Wer Abs. 3 beantragte, bekam in der Vorschau den ermäßigten Satz — im XML stand nur der
Veräußerungsgewinn (E0801301/E0804501/E0901201), nie der Antrag. Das Finanzamt sah ihn nicht.

Die Antragszeile steht laut amtlichem Schema im selben Container wie der Veräußerungsgewinn
(E10-2025.xsd:15134 / 16197 / 12826, je „Veräußerungsgewinn laut Zeile …, für den der ermäßigte
Steuersatz des § 34 Abs. 3 EStG … beantragt wird"):

    Anlage G  E0801301 -> E0801602     .../VAe_G_FB_Antr/
    Anlage S  E0804501 -> E0805003     .../VAe_Gew/Vor_FB/
    Anlage L  E0901201 -> E0901704     .../VAe_G_v_FB/VAe_G_FB_Antr/

Der Zweig `VAe_G_kein_FB` (E0801903 / E0805305 / E0902002) hat als Basis E0801401 / E0804601 / E0901302;
TaxGraph schreibt die nie, also bleibt dieser Zweig unerreichbar und wird nicht beschrieben (letzter Test).

Schreibbedingung (Instructor-Entscheid 2026-09-26): Antrag bestätigt UND `_abs3_eligible` UND
0 < netto_vg <= 5 Mio EUR — dieselbe Bedingung, unter der der Ring Abs. 3 rechnet. Dann stimmen Vorschau
und Erklärung überein. Ein Antrag ohne Berechtigung schreibt nichts. Der Wert ist der Betrag der
Basiszeile (der Gewinn „laut Zeile"), also bei jedem Schreibfall <= 5 Mio.

Nicht gebaut (Eintrag): Partner-Pfad (AK2), Sperrgrund für Partner-Gewinn neben A-Antrag (AK2b).

NULL LLM."""
from __future__ import annotations

import os
import sys
import xml.etree.ElementTree as ET

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/eingang", "produkt/mapping", "golden", "elster/submission"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API         # noqa: E402
import api_constants as AC  # noqa: E402
import audit              # noqa: E402
import elster_xml as EX   # noqa: E402
import validate_xsd as VX  # noqa: E402

from _kegel import kegel_fuer  # noqa: E402

# Art -> (Basis-Kz, Antrags-Kz im FB-Antr-Zweig, Antrags-Kz im kein-FB-Zweig)
ARTEN = {"gewerbe": ("E0801301", "E0801602", "E0801903"),
         "selbstaendig": ("E0804501", "E0805003", "E0805305"),
         "land_forst": ("E0901201", "E0901704", "E0902002")}
ALLE_ANTRAGS_KZ = tuple(k for _, a, b in ARTEN.values() for k in (a, b))
_ABSENDER = dict(absender_name="Maier Hans", absender_strasse="Musterstr. 55", absender_plz="55555",
                 absender_ort="Musterort", absender_steuernummer="9181081508155")

VG_CENT = 50_000_000           # 500.000 EUR: über dem Freibetrag (FB 0), unter 5 Mio


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _fall(tmp_path, monkeypatch, fid, scheibe="gesamt", vz=2025, **gesetzt):
    """Legt einen Fall mit vollem Kegel an. `gesetzt` gewinnt gegen die Standardwerte."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, r = API.fall_anlegen({"scheibe": scheibe, "veranlagungszeitraum": vz, "fall_id": fid})
    assert st == 201, r
    basis = {"bruttoarbeitslohn": 6_000_000, "kein_gewinn": False, "betriebseinnahmen": 0,
             "sonstige_betriebsausgaben": 0, "afa_jahresbetrag": 0,
             "rentner_veraeusserungsgewinn": VG_CENT, "rentner_veraeusserungs_betriebsart": "gewerbe",
             "rentner_alter_55_oder_berufsunfaehig": True, "rentner_freibetrag_erstmalig": True,
             "geburtsjahr": 1955, "antrag_ermaessigter_satz": True,
             "dauernd_berufsunfaehig": False, "ermaessigung_einmal_genutzt": False}
    if scheibe == "rentner_gesamt":
        # Standardwert 0 = „ungültiges Jahr" (gesperrt); ein früheres Jahr verlangt die Fixierung des Rentenfreibetrags
        basis["rentner_renten_beginn_jahr"] = 2025
    basis.update(gesetzt)
    # nur Felder, die die Scheibe führt (rentner_gesamt kennt keinen bruttoarbeitslohn)
    basis = {k: v for k, v in basis.items() if v is not None and k in AC.SCHEIBEN[scheibe]["felder"]}
    for feld, wert in kegel_fuer(scheibe, basis):
        st, r = API.event(fid, _laie(feld, wert))
        assert st == 201, (feld, wert, st, r)


def _dekl(fid):
    st, d = API.deklaration(fid)
    assert st == 200, d
    return d


def _zahl(fid):
    st, e = API.ergebnis(fid)
    assert st == 200, e
    return e["grund"], e["zahl_cent"]


# --------------------------------------------------------------------------------------- AK1

@pytest.mark.parametrize("scheibe", ["gesamt", "rentner_gesamt"])
@pytest.mark.parametrize("art", sorted(ARTEN))
def test_antrag_schreibt_die_antrags_kz_der_anlage(tmp_path, monkeypatch, scheibe, art):
    """AK1: Antrag bestätigt, berechtigt, 0 < netto_vg <= 5 Mio -> die Antrags-Kz der gewählten Anlage steht in
    der Deklaration, mit dem Gewinn der Basiszeile; die Antrags-Kz der beiden anderen Anlagen und alle drei
    kein-FB-Kz fehlen. Kontrolle: der Ring rechnet in diesem Fall Abs. 3 (Zahl weicht vom Fall ohne Antrag ab)."""
    basis, antrag, _ = ARTEN[art]
    _fall(tmp_path, monkeypatch, "mit", scheibe, rentner_veraeusserungs_betriebsart=art)
    d = _dekl("mit")["deklaration"]
    assert d.get(basis) == VG_CENT // 100, f"Basis-Kz {basis} fehlt, der Fall misst nichts: {d.get(basis)}"
    assert d.get(antrag) == VG_CENT // 100, f"{art}: Antrags-Kz {antrag} fehlt oder falsch: {d.get(antrag)}"
    fremd = {k: d[k] for k in ALLE_ANTRAGS_KZ if k != antrag and k in d}
    assert fremd == {}, f"nur {antrag} darf stehen, zusätzlich: {fremd}"
    _fall(tmp_path, monkeypatch, "ohne", scheibe, rentner_veraeusserungs_betriebsart=art,
          antrag_ermaessigter_satz=False)
    assert _zahl("mit") != _zahl("ohne"), "Kontrolle: Abs. 3 muss rechnen, sonst misst der Test nichts"


# --------------------------------------------------------------------------------------- AK3

@pytest.mark.parametrize("art", sorted(ARTEN))
@pytest.mark.parametrize("antrag", [False, None], ids=["antrag_nein", "antrag_unbeantwortet"])
def test_ohne_antrag_steht_keine_antrags_kz(tmp_path, monkeypatch, art, antrag):
    """AK3: ohne Antrag (bestätigtes „nein" oder nie beantwortet) steht keine der sechs Antrags-Kz. Der Gewinn
    selbst (Basis-Kz) steht weiter — der Fall ist deklarierbar."""
    basis = ARTEN[art][0]
    _fall(tmp_path, monkeypatch, "o", rentner_veraeusserungs_betriebsart=art, antrag_ermaessigter_satz=antrag)
    d = _dekl("o")["deklaration"]
    assert d.get(basis) == VG_CENT // 100
    assert {k: d[k] for k in ALLE_ANTRAGS_KZ if k in d} == {}


# ------------------------------------------------------------- Antrag ohne Berechtigung schreibt nichts

NICHT_BERECHTIGT = {
    # Alter 35, nicht berufsunfähig (bestätigtes „nein"): § 34 Abs. 3 S. 1 nicht erfüllt, der Ring rechnet Abs. 1
    "zu_jung": dict(geburtsjahr=1990),
    # Abs. 3 S. 4: einmal im Leben, schon genutzt
    "schon_genutzt": dict(ermaessigung_einmal_genutzt=True),
    # 0 < netto_vg: 40.000 EUR Gewinn, Freibetrag § 16 Abs. 4 (45.000) deckt alles -> netto 0
    "netto_null": dict(rentner_veraeusserungsgewinn=4_000_000),
}


@pytest.mark.parametrize("fall", sorted(NICHT_BERECHTIGT))
def test_antrag_ohne_berechtigung_schreibt_nichts(tmp_path, monkeypatch, fall):
    """Ein Antrag ohne Berechtigung schreibt keine Antrags-Kz, und die Vorschau stimmt damit überein: der Ring
    rechnet denselben Betrag wie im Fall ohne Antrag (Abs. 1 statt Abs. 3)."""
    kw = NICHT_BERECHTIGT[fall]
    _fall(tmp_path, monkeypatch, "a", **kw)
    d = _dekl("a")["deklaration"]
    assert d.get("E0801301") == kw.get("rentner_veraeusserungsgewinn", VG_CENT) // 100, "Fall misst nichts"
    assert {k: d[k] for k in ALLE_ANTRAGS_KZ if k in d} == {}, f"{fall}: Antrag ohne Berechtigung schrieb eine Kz"
    _fall(tmp_path, monkeypatch, "b", antrag_ermaessigter_satz=False, **kw)
    assert _zahl("a") == _zahl("b"), f"{fall}: Vorschau rechnet den Antrag, die Erklärung schreibt ihn nicht"


def test_antrag_ueber_fuenf_millionen_sperrt_statt_zu_schreiben(tmp_path, monkeypatch):
    """Über 5 Mio EUR sperrt `abs3_ueber_5mio_offen` die Deklaration (409); es entsteht keine Erklärung,
    also auch keine Antrags-Kz neben einem Gewinn, der die Grenze reißt."""
    _fall(tmp_path, monkeypatch, "g", rentner_veraeusserungsgewinn=600_000_000)
    st, d = API.deklaration("g")
    assert st == 409 and d["grund"] == "abs3_ueber_5mio_offen", (st, d.get("grund"))


def test_antrag_mit_freibetrag_nein_schreibt_den_gewinn_ohne_abzug(tmp_path, monkeypatch):
    """Grenzfall zur Bedingung `0 < netto_vg`: 40.000 EUR, aber `rentner_freibetrag_erstmalig` = nein -> kein
    Freibetrag, netto_vg = 40.000 > 0, berechtigt (Jahrgang 1955) -> die Antrags-Kz steht."""
    _fall(tmp_path, monkeypatch, "f", rentner_veraeusserungsgewinn=4_000_000, rentner_freibetrag_erstmalig=False)
    d = _dekl("f")["deklaration"]
    assert d.get("E0801602") == 40_000, d.get("E0801602")


# ----------------------------------------------------------------- Schema: Container, je Veranlagungsjahr

def _xsd_doku(vz: int, kz: str) -> str:
    """Die Dokumentation des Elements `kz` im amtlichen E10-<vz>.xsd (Text, wie das Schema ihn führt)."""
    import xsd_verify as XV
    schema = XV._find_schema(vz)
    if not schema:
        pytest.skip(f"E10-{vz}.xsd nicht gefunden")
    xs = "{http://www.w3.org/2001/XMLSchema}"
    doku = [d.text or "" for e in ET.parse(schema).getroot().iter(xs + "element") if e.get("name") == kz
            for d in e.iter(xs + "documentation")]
    assert len(doku) == 1, f"VZ {vz}: {kz} steht {len(doku)}x mit Doku im Schema"
    return doku[0]


def test_die_verzweigungstabelle_traegt_die_drei_antrags_kz():
    """Die Tabelle, die Art -> Antrags-Kz lenkt, steht hier ein zweites Mal, ausgeschrieben: ein Zahlendreher
    in est_mapping.VERZWEIGUNG wäre schemagültig und stünde in der falschen Zeile (wie bei § 35c)."""
    import est_mapping as EM
    assert EM.VERZWEIGUNG["p34_abs3_antragsbetrag"] == {
        "art_feld": "rentner_veraeusserungs_betriebsart",
        "kz": {"gewerbe": "E0801602", "selbstaendig": "E0805003", "land_forst": "E0901704"}}


@pytest.mark.parametrize("vz", [2024, 2025])
def test_antrags_kz_steht_im_container_der_basiszeile(vz):
    """Kz und Container aus dem amtlichen E10-<vz>.xsd, nicht aus dem Gedächtnis: jede Antrags-Kz existiert
    genau einmal, hängt im selben Container wie die Basiszeile ihrer Anlage, und ihre Schema-Doku nennt den
    Antrag nach § 34 Abs. 3 EStG und die Basiszeile („laut Zeile $<Basis>.Vordruckzeile$")."""
    try:
        pfade = EX.kz_pfade(vz)
    except EX.XmlFehler as e:
        pytest.skip(str(e))
    for art, (basis, antrag, _) in ARTEN.items():
        assert antrag in pfade, f"VZ {vz}: {antrag} ({art}) steht nicht im Schema"
        assert pfade[antrag][:-1] == pfade[basis][:-1], (vz, art, pfade[antrag], pfade[basis])
        doku = _xsd_doku(vz, antrag)
        assert "§ 34 Abs. 3 EStG" in doku and "beantragt" in doku, (vz, antrag, doku)
        assert f"${basis}.Vordruckzeile$" in doku, f"VZ {vz}: {antrag} bezieht sich nicht auf {basis}: {doku}"


def test_fuer_vz_2026_liegt_kein_schema_vor():
    """Befund, kein Raten: das ERiC-Paket 44.2.4.0 enthält kein E10-2026.xsd. Ob die drei Antrags-Kz für 2026
    existieren, ist damit nicht prüfbar. Wird ein 2026er Schema eingespielt, schlägt dieser Test an und die
    Schemaprüfung oben gehört auf 2026 erweitert."""
    import xsd_verify as XV
    assert XV._find_schema(2026) is None, "2026er Schema vorhanden: test_antrags_kz_steht_im_container… ausdehnen"


# ----------------------------------------------------------------------------- XML und xmllint

@pytest.mark.parametrize("vz", [2024, 2025])
@pytest.mark.parametrize("art", sorted(ARTEN))
def test_antrag_steht_im_xml_und_besteht_die_schema_pruefung(tmp_path, monkeypatch, vz, art):
    """Der Antragsfall als echtes XML (erzeuge_xml): die Antrags-Kz steht am Pfad des amtlichen Schemas, und
    xmllint nimmt das XML gegen elster11_E10_<vz>_extern.xsd an."""
    basis, antrag, _ = ARTEN[art]
    _fall(tmp_path, monkeypatch, "x", vz=vz, rentner_veraeusserungs_betriebsart=art)
    d = _dekl("x")
    d.pop("fall_id", None)
    try:
        xml = EX.erzeuge_xml(d, vz=vz, hersteller_id="74931", abgabefaehig=False, **_ABSENDER)
    except EX.XmlFehler as e:
        pytest.skip(str(e))
    xml = xml if isinstance(xml, str) else xml.decode("utf-8")
    wurzel = ET.fromstring(xml)
    treffer = {e.tag.split("}")[-1]: e.text for e in wurzel.iter()
               if e.tag.split("}")[-1] in (basis, antrag)}
    assert treffer == {basis: str(VG_CENT // 100), antrag: str(VG_CENT // 100)}, treffer
    if not VX.find_schema(str(vz)):
        pytest.skip(f"elster11_E10_{vz}_extern.xsd nicht gefunden")
    pfad = tmp_path / "antrag.xml"
    pfad.write_text(xml, encoding="utf-8")
    ok, meldung = VX.validate(str(pfad), str(vz))
    assert ok, meldung


# ----------------------------------------------------------- der kein-FB-Zweig bleibt unerreichbar

def test_kein_fb_zweig_wird_nicht_beschrieben(tmp_path, monkeypatch):
    """E0801903 / E0805305 / E0902002 sitzen im Zweig `VAe_G_kein_FB`, dessen Basiszeilen (E0801401 / E0804601 /
    E0901302) TaxGraph nicht schreibt. Eine Antragszeile ohne Basiszeile im selben Zweig wäre schemawidrig
    unvollständig; deshalb schreibt der Antrag nur in den FB-Antr-Zweig. Gegenprobe zu den Tests oben."""
    _fall(tmp_path, monkeypatch, "k")
    d = _dekl("k")["deklaration"]
    assert not {k: d[k] for k in ("E0801401", "E0804601", "E0901302", "E0801903", "E0805305", "E0902002") if k in d}
