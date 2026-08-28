"""Der Partner bekommt dieselben Existenzfragen wie der Nutzer — und kein Ich-Kreuz darf ihm mehr
etwas wegnehmen.

ANLASS, vier echte Durchgaenge: die Ankreuzliste fragte nur nach „dir", nie nach „euch". Ein
Ehepaar, das alle achtzehn vorhandenen Kreuze verneinte, bekam in der Scheibe `gesamt` 88 Fragen,
davon 32 zum Partner — derselbe Mensch allein veranlagt bekam 56 und keine einzige Partner-Frage.
Fuenf Fragen nach den Kapitalertraegen des Partners trotz angekreuztem „keine Kapitalertraege",
drei nach Verguetungen aus einer nie erhobenen Gesellschafterstellung.

DER TEURE TEIL WAR NICHT DIE ZAHL DER FRAGEN. Neun Partner-Felder hingen bereits an einem
ICH-Kreuz. `keine_behinderung_pflege` fragt woertlich „Hast du selbst oder hat eines deiner Kinder
eine amtlich festgestellte Behinderung…?" — der Partner kommt darin nicht vor. Wer nur einen
behinderten PARTNER hat, antwortet wahrheitsgemaess „nein" und verliert dessen Pauschbetrag fuer
immer. Ueber den echten Rechenweg gemessen (rentner_gesamt, VZ 2025, zusammen, Partner-GdB 50):
5.532,00 EUR gegen 5.834,00 EUR — 302,00 EUR zu viel Steuer.

Deshalb steht hier nicht nur „wieviele Fragen entfallen", sondern vor allem die Gegenrichtung: ein
Kreuz, das dem Partner etwas wegnimmt, wonach es nie gefragt hat, muss diesen Test rot machen.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/import"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API              # noqa: E402
import api_constants as AK     # noqa: E402
import audit                   # noqa: E402
import store as ST             # noqa: E402
import traverser as TR         # noqa: E402

BINDUNG = TR.lade_bindung()
SCHEIBE_GESAMT = set(AK.SCHEIBEN["gesamt"]["felder"])

TS = "2026-08-28T10:00:00+00:00"
HERKUNFT = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

# Die Ich-Kreuze fragen ALLE woertlich in der ersten Person. Keines von ihnen darf ein
# Partner-Feld abschalten — das ist die Regel, deren Bruch 302 EUR gekostet hat.
ICH_KREUZE = frozenset(f for f, b in BINDUNG.items()
                       if b.get("screening") and not f.endswith("_partner"))

# Die vier Partner-Stammdaten tragen ein echtes Kz und werden von JEDER Zusammenveranlagung
# gebraucht. Ein Screening-Kreuz darf sie niemals abschalten.
PARTNER_STAMMDATEN = ("stammdaten_vorname_partner", "stammdaten_nachname_partner",
                      "stammdaten_geburtsdatum_partner", "kist_konfession_partner")


def _store(antworten: dict):
    s = ST.leerer_store(2025, fall_id="screening-partner")
    for f, w in antworten.items():
        ST.append_event(s, feld_id=f, wert=w, zustand="bestaetigt", herkunft=HERKUNFT,
                        schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"klick@{f}"}, ts=TS)
    return s


def _fragen(antworten: dict, bindung: dict | None = None) -> list[str]:
    """Nur Felder der Scheibe `gesamt` — das ist der Fragebogen, den der Nutzer wirklich sieht."""
    b = bindung if bindung is not None else BINDUNG
    return [f for f in TR.naechste_fragen(_store(antworten), b) if f in SCHEIBE_GESAMT]


def _alle_ich_verneint() -> dict:
    """Jedes Ich-Kreuz mit dem Wert, der den Sachverhalt VERNEINT.

    Bei `kein_*`/`keine_*` ist das `True` (das Feld benennt die Abwesenheit, `frage_invertiert`
    dreht die Frage um); bei positiv benannten Feldern wie `vpf_auswaertige_taetigkeit` `False`.
    """
    return {f: bool(BINDUNG[f].get("frage_invertiert")) for f in ICH_KREUZE}


ZUSAMMEN = {"veranlagung": "zusammen"}
EINZEL = {"veranlagung": "einzel"}


# ---- Stelle 1-3: das Kreuz muss gebunden, fragbar und in der Scheibe sein ----

@pytest.mark.parametrize("flag", sorted(AK.PARTNER_SCREENING))
def test_partner_kreuz_ist_gebunden_und_fragbar(flag):
    """Ohne Bindungseintrag existiert das Feld nicht; ohne Scheiben-Eintrag wird es nie gefragt
    und schaltet nie etwas ab — tote Verdrahtung, dieselbe Klasse wie der § 35c-Teil-Ring."""
    assert flag in BINDUNG, f"{flag} steht in PARTNER_SCREENING, aber in keiner Bindungsdatei."
    assert BINDUNG[flag].get("askable") is True, f"{flag} ist nicht fragbar."
    assert BINDUNG[flag].get("screening") is True, (
        f"{flag} ist nicht als `screening` deklariert und landet damit nicht in der Ankreuzliste.")
    assert flag in SCHEIBE_GESAMT, (
        f"{flag} ist gebunden, steht aber nicht in SCHEIBEN['gesamt']['felder'] — es wird nie "
        f"gefragt und schaltet deshalb nie etwas ab.")


@pytest.mark.parametrize("flag", sorted(AK.PARTNER_SCREENING))
def test_partner_kreuz_hat_eine_eigene_pseudoregel(flag):
    """Jedes Flag ist das EINZIGE fragbare Feld seiner Regel.

    relevanz() wertet die Gates EINER Regel konjunktiv aus und bricht beim ersten bestaetigten
    False ab. Laegen zwei Flags auf derselben Pseudoregel, naehme eine bejahte Frage der anderen
    ihre Antwort — genau der Fehler, wegen dem p2_einkunftsarten 2026-08-14 in vier Regeln
    zerlegt wurde.
    """
    rid = BINDUNG[flag]["quelle"]["regel_id"]
    geschwister = [f for f, b in BINDUNG.items()
                   if b["quelle"]["regel_id"] == rid and b.get("askable") and f != flag]
    assert not geschwister, (
        f"{flag} teilt sich die Regel {rid} mit {geschwister} — eine bejahte Frage nimmt der "
        f"anderen die Antwort.")


# ---- Der Kern: kein Ich-Kreuz darf ein Partner-Feld abschalten ----

def _partner_felder_an_ich_kreuz(bindung: dict) -> list[tuple[str, str]]:
    """(Partner-Feld, Ich-Kreuz) fuer jedes Partner-Feld, das an einem Ich-Kreuz haengt.

    Der wiederverwendbare Kern des Waechters — die Mutationsprobe unten ruft dieselbe Funktion
    mit einer absichtlich kaputten Bindung auf.
    """
    treffer = []
    for f, b in bindung.items():
        if not f.endswith("_partner"):
            continue
        bed = b.get("feld_bedingung")
        if bed and bed["feld"] in ICH_KREUZE:
            treffer.append((f, bed["feld"]))
    return sorted(treffer)


def test_kein_partner_feld_haengt_an_einem_ich_kreuz():
    """DER TEURE WAECHTER. Bis 2026-08-28 hingen neun Partner-Felder an einem Ich-Kreuz.

    Alle Ich-Kreuze sind woertlich in der ersten Person gestellt; `keine_behinderung_pflege` nennt
    ausdruecklich nur „dich selbst" und „deine Kinder". Ein Flag darf nur abschalten, wonach es
    auch gefragt hat — sonst verliert das Paar den Pauschbetrag des Partners (gemessen: 302 EUR).
    """
    treffer = _partner_felder_an_ich_kreuz(BINDUNG)
    assert not treffer, (
        "Diese Partner-Felder haengen an einem Kreuz, das nur nach dem NUTZER fragt — eine "
        f"wahrheitsgemaesse Verneinung nimmt dem Paar die Angaben des Partners: {treffer}")


def test_mutationsprobe_der_waechter_wuerde_den_geldfehler_finden():
    """Gegenprobe zum Waechter darueber: baut den Fehler absichtlich ein.

    Ohne diese Probe koennte `test_kein_partner_feld_haengt_an_einem_ich_kreuz` gruen sein, weil
    er nichts misst (leere Menge, falscher Schluessel, `_partner`-Suffix nie getroffen) statt
    weil nichts zu finden ist. Genau die Klasse „Pruefer misst Stellvertretermerkmal".
    """
    kaputt = {f: dict(b) for f, b in BINDUNG.items()}
    kaputt["rentner_grad_der_behinderung_partner"]["feld_bedingung"] = {
        "feld": "keine_behinderung_pflege", "wert": False,
        "grund": "absichtlich falsch fuer die Mutationsprobe — das Kreuz des Nutzers am Partner-Feld"}
    treffer = _partner_felder_an_ich_kreuz(kaputt)
    assert ("rentner_grad_der_behinderung_partner", "keine_behinderung_pflege") in treffer, (
        "Der Waechter findet den wieder eingebauten Geldfehler nicht — er misst etwas anderes "
        "als das, was er behauptet.")


def test_mutationsprobe_ein_stammdatenfeld_mitgaten_faellt_auf():
    """Zweite Mutationsprobe, die andere Richtung: ein Feld mitgaten, das NICHT mitgehoert.

    Vorname, Nachname, Geburtsdatum und Konfession des Partners tragen echte Kennzahlen
    (E0100801, E0100901, E0101001, E0101002) und werden von JEDER Zusammenveranlagung gebraucht.
    Haengte eines davon an einem Screening-Kreuz, verschwaende ein Pflichtfeld der Erklaerung,
    sobald das Paar den Sachverhalt verneint.
    """
    kaputt = {f: dict(b) for f, b in BINDUNG.items()}
    kaputt["stammdaten_vorname_partner"]["feld_bedingung"] = {
        "feld": "kein_kap_partner", "wert": False,
        "grund": "absichtlich falsch fuer die Mutationsprobe — Stammdatenfeld an einem Screening-Kreuz"}
    verneint = {**ZUSAMMEN, "kein_kap_partner": True}
    assert "stammdaten_vorname_partner" not in _fragen(verneint, kaputt), (
        "Die Probe baut den Fehler nicht ein — sie kann ihn also auch nicht nachweisen.")
    assert "stammdaten_vorname_partner" in _fragen(verneint), (
        "Der Vorname des Partners darf durch KEIN Screening-Kreuz entfallen: er traegt E0100801 "
        "und wird von jeder Zusammenveranlagung gebraucht.")


@pytest.mark.parametrize("feld", PARTNER_STAMMDATEN)
def test_partner_stammdaten_ueberleben_jedes_kreuz(feld):
    """Auch wenn das Paar ALLES verneint: die Stammdaten des Partners bleiben.

    Sie tragen ein Kz und gehen in die Erklaerung. Ein Feld, das in die Erklaerung geht, darf
    nicht durch ein Kreuz verschwinden.
    """
    alles_verneint = {**ZUSAMMEN, **_alle_ich_verneint(),
                      **{f: True for f in AK.PARTNER_SCREENING}}
    assert feld in _fragen(alles_verneint), (
        f"{feld} traegt eine Kennzahl und ist trotzdem durch die Kreuze entfallen.")


# ---- Wirkung: was die vier Kreuze abschalten ----

ABGESCHALTET = {
    "kein_kap_partner": ("kap_kapitalertraege_partner", "kap_gewinn_aktien_partner",
                         "kap_gewinn_sonstige_partner", "kap_verlust_aktien_partner",
                         "kap_verlust_sonstige_partner"),
    "kein_gewinn_partner": ("einkuenfte_gewinn_partner", "gewinn_betriebsart_partner",
                            "gewinn_bezeichnung_partner", "gewinnanteil_partner",
                            "gewst_hebesatz_partner", "gewst_messbetrag_partner",
                            "verguetung_taetigkeit_partner", "verguetung_darlehen_partner",
                            "verguetung_ueberlassung_partner",
                            "rentner_veraeusserungsgewinn_partner",
                            "rentner_veraeusserungs_betriebsart_partner"),
    "keine_behinderung_pflege_partner": ("rentner_grad_der_behinderung_partner",
                                         "rentner_hilflos_blind_taubblind_partner"),
}


@pytest.mark.parametrize("flag,felder", sorted(ABGESCHALTET.items()))
def test_verneintes_partner_kreuz_nimmt_seine_felder_weg(flag, felder):
    offen = set(_fragen(ZUSAMMEN))
    fehlend = [f for f in felder if f not in offen]
    assert not fehlend, (
        f"Diese Felder sind schon ohne jede Antwort nicht in der Warteschlange — der Test misst "
        f"dann nichts: {fehlend}")
    nachher = set(_fragen({**ZUSAMMEN, flag: True}))
    geblieben = [f for f in felder if f in nachher]
    assert not geblieben, f"{flag}=verneint laesst diese Felder stehen: {geblieben}"


@pytest.mark.parametrize("flag,felder", sorted(ABGESCHALTET.items()))
def test_partner_mit_dem_sachverhalt_bekommt_die_fragen_weiter(flag, felder):
    """GEGENPROBE (Gate-Polaritaet, Praezedenz 519199e): der Partner, der den Sachverhalt HAT,
    muss seine Fragen behalten — auch dann, wenn der Nutzer ihn fuer sich verneint hat."""
    antworten = {**ZUSAMMEN, **_alle_ich_verneint(), flag: False}
    offen = set(_fragen(antworten))
    fehlend = [f for f in felder if f not in offen]
    assert not fehlend, (
        f"{flag}=bejaht (der Partner HAT den Sachverhalt), trotzdem fehlen diese Fragen: {fehlend}")


@pytest.mark.parametrize("flag,felder", sorted(ABGESCHALTET.items()))
def test_unbeantwortetes_partner_kreuz_schliesst_nichts_aus(flag, felder):
    """FAIL-CLOSED: Schweigen schliesst nichts aus. Der Nutzer hat alle EIGENEN Kreuze verneint,
    zum Partner aber nichts gesagt — dann muessen alle Fragen stehen bleiben."""
    offen = set(_fragen({**ZUSAMMEN, **_alle_ich_verneint()}))
    fehlend = [f for f in felder if f not in offen]
    assert not fehlend, (
        f"{flag} ist unbeantwortet, trotzdem fehlen diese Partner-Fragen: {fehlend}. "
        f"Unbeantwortet heisst fragen.")


def test_vorlaeufiger_vorschlag_schliesst_nichts_aus():
    """Ein vorlaeufiger KI-Vorschlag ist keine Antwort des Nutzers und darf ihm keine Frage
    nehmen, die er nie gesehen hat."""
    s = ST.leerer_store(2025, fall_id="vorlaeufig")
    ST.append_event(s, feld_id="veranlagung", wert="zusammen", zustand="bestaetigt",
                    herkunft=HERKUNFT, schreiber="ui:laie",
                    signal={"signal_1": None, "signal_2": "klick"}, ts=TS)
    ST.append_event(s, feld_id="kein_kap_partner", wert=True, zustand="vorlaeufig",
                    herkunft={"herkunft": "llm", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="ki:berater", signal={"signal_1": "llm", "signal_2": None}, ts=TS)
    offen = set(TR.naechste_fragen(s, BINDUNG))
    assert "kap_kapitalertraege_partner" in offen, (
        "Ein vorlaeufiger Vorschlag hat dem Nutzer eine Frage weggenommen, die er nie gesehen hat.")


# ---- Einzelveranlagung: der Partner-Zweig existiert nicht ----

@pytest.mark.parametrize("flag", sorted(AK.PARTNER_SCREENING))
def test_alleinstehender_sieht_die_partner_kreuze_nicht(flag):
    assert flag not in _fragen(EINZEL), (
        f"{flag} wird einem Einzelveranlagten gestellt — er hat keinen Partner.")


def test_alleinstehender_bekommt_keine_partner_frage():
    """Auch der Alleinstehende MIT Behinderung nicht.

    Bis 2026-08-28 bekam er zwei: `behinderungsbedingte_aufwendungen_partner` und deren Wahlrecht
    liegen in p33_1_2_agb_abzug — einer Regel, die auch fuer Einzelveranlagte gilt — und hingen an
    `keine_behinderung_pflege`. Wer die mit „ja" beantwortete, wurde nach den Aufwendungen eines
    Partners gefragt, den er nicht hat.
    """
    antworten = {**EINZEL, **_alle_ich_verneint(), "keine_behinderung_pflege": False}
    partner_fragen = [f for f in _fragen(antworten) if f.endswith("_partner")]
    assert not partner_fragen, (
        f"Ein Einzelveranlagter bekommt Partner-Fragen: {partner_fragen}")


# ---- Der Geldbeweis: echter Rechenlauf ----

@pytest.fixture
def _isoliert(tmp_path, monkeypatch):
    """Faelle in tmp_path statt in die echte Ablage — sonst kollidieren Wiederholungslaeufe."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))


def _laie(fid, wert):
    return {"feld_id": fid, "wert": wert, "zustand": "bestaetigt", "herkunft": HERKUNFT,
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fid}"}}


_BASIS_ZUSAMMEN = [
    ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 6000000),
    ("rentner_renten_beginn_jahr", 2025), ("rentner_alter_bei_rentenbeginn", 0),
    ("rentner_grad_der_behinderung", 0), ("rentner_hilflos_blind_taubblind", False),
    ("rentner_pflegegrad", 0), ("rentner_gepflegter_hilflos", False),
    ("rentner_hinterbliebenenbezuege", False), ("veranlagung", "zusammen"),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("versicherungsart", "gesetzlich_an"), ("basis_kv", 0), ("basis_pv", 0),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
]


def _lauf(fall_id, zusatz):
    st, _ = API.fall_anlegen({"fall_id": fall_id, "scheibe": "rentner_gesamt",
                              "veranlagungszeitraum": 2025})
    assert st == 201
    for fid, wert in _BASIS_ZUSAMMEN + zusatz:
        st, resp = API.event(fall_id, _laie(fid, wert))
        assert st == 201, f"{fid}: {st} {resp}"
    return API.ergebnis(fall_id)


def test_partner_pauschbetrag_ueberlebt_die_verneinung_des_nutzers(_isoliert):
    """DIE 302 EUR. Der Nutzer hat selbst keine Behinderung und verneint sie wahrheitsgemaess;
    der Partner hat GdB 50.

    Bis 2026-08-28 nahm diese Antwort dem Paar die Frage nach dem GdB des Partners — und damit
    dessen Pauschbetrag. Gemessen: 5.532,00 EUR gegen 5.834,00 EUR.

    Der Test prueft BEIDES, weil eines allein zu wenig ist: dass die Frage noch gestellt wird
    (sonst kann der Wert nie in den Fall kommen) UND dass die Steuer mit dem Wert niedriger ist
    (sonst waere die Frage folgenlos — der Fix, der nichts bewirkt).
    """
    nutzer_verneint_seine_behinderung = {**ZUSAMMEN, "keine_behinderung_pflege": True}
    assert "rentner_grad_der_behinderung_partner" in _fragen(nutzer_verneint_seine_behinderung), (
        "Der Nutzer hat SEINE Behinderung verneint — die Frage nach dem Grad der Behinderung des "
        "PARTNERS muss trotzdem kommen, sonst ist dessen Pauschbetrag unerreichbar.")

    _, mit_gdb = _lauf("partner_gdb", [("rentner_grad_der_behinderung_partner", 50),
                                       ("rentner_hilflos_blind_taubblind_partner", False)])
    _, ohne_gdb = _lauf("partner_ohne_gdb", [])
    assert mit_gdb["grund"] == "bestaetigt" and ohne_gdb["grund"] == "bestaetigt"
    assert mit_gdb["zahl_cent"] < ohne_gdb["zahl_cent"], (
        f"Der Behinderten-Pauschbetrag des Partners senkt die Steuer nicht: "
        f"mit GdB {mit_gdb['zahl_cent']}, ohne {ohne_gdb['zahl_cent']}")
    assert ohne_gdb["zahl_cent"] - mit_gdb["zahl_cent"] == 30200, (
        f"Die Ersparnis hat sich geaendert: {(ohne_gdb['zahl_cent'] - mit_gdb['zahl_cent']) / 100:.2f} "
        f"EUR statt 302,00 EUR. Wenn das gewollt ist, ist der Anker hier nachzuziehen.")
