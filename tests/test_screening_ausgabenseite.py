"""Fünf Screening-Flags für die Ausgabenseite — und die drei Stellen, an denen sie tot wären.

GEMESSEN 2026-08-21 im echten Nutzerlauf. Julius bekam:

    "Wurden die Handwerkerleistungen NICHT öffentlich gefördert …"     — er hatte keine
    "Hast du für dieses Gebäude in früheren Jahren schon einmal …"     — er hat kein Gebäude
    "Lief deine Tätigkeit am selben Ort durchgehend …"                 — er war an keinem Ort

Die Vorauswahl existierte, aber nur für die EINNAHMEN (kein_gewinn/_kap/_vuv/_sonstige/_kind).
Für Ausgaben und Ermäßigungen gab es kein einziges Flag: 143 der 316 fragbaren Felder lagen in
Regeln, die keine Antwort abschalten konnte.

WIRKUNG, gemessen: ein Arbeitnehmer (einzel, alle vier Einkunftsarten verneint, keine Kinder) sah
200 Fragen der Scheibe `gesamt`; mit den fünf neuen Antworten sind es 141 — 59 Fragen weniger für
fünf zusätzliche Antworten.

DREI STELLEN MÜSSEN ZUSAMMENSTIMMEN, sonst ist das Flag tote Verdrahtung. Genau dafür ist dieser
Test da — jede einzelne davon war im Repo schon einmal die Fehlerursache:

  1. BINDUNG (bindung_screening_ausgaben.yaml) — das Feld existiert und ist fragbar.
  2. SCHEIBE  (api_constants.SCHEIBEN["gesamt"]["felder"]) — eine HANDGESCHRIEBENE Liste. Ein Feld,
     das in der Bindung steht und dort nicht, wird nie gefragt. Dieselbe Klasse wie der tote
     § 35c-Teil-Ring.
  3. REGEL_BEDINGUNGEN (bindung_regel_bedingungen.yaml) — ohne Eintrag schaltet die Antwort nichts
     ab. Das Flag würde gefragt und bliebe wirkungslos: der Fix, der nichts bewirkt.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/traverser", "produkt/store", "produkt/haut"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api_constants as AK  # noqa: E402
import store as ST          # noqa: E402
import traverser as TR      # noqa: E402

BINDUNG = TR.lade_bindung()
BEDINGUNGEN = TR.lade_regel_bedingungen()
SCHEIBE_GESAMT = set(AK.SCHEIBEN["gesamt"]["felder"])

TS = "2026-08-21T10:00:00+00:00"
HERKUNFT = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

# Der Nutzer, an dem gemessen wurde: Arbeitnehmer, keine der vier Einkunftsarten, keine Kinder.
BASIS = {"veranlagung": "einzel", "kein_gewinn": True, "kein_kap": True,
         "kein_vuv": True, "kein_sonstige": True, "kein_kind": True}


def _store(antworten: dict):
    s = ST.leerer_store(2025, fall_id="screening-ausgaben")
    for f, w in antworten.items():
        ST.append_event(s, feld_id=f, wert=w, zustand="bestaetigt", herkunft=HERKUNFT,
                        schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"klick@{f}"}, ts=TS)
    return s


def _verneint(flag: str) -> bool:
    """Der Wert, mit dem der Nutzer den Sachverhalt VERNEINT.

    Bei `kein_*`/`keine_*` ist das `True` (das Feld benennt die Abwesenheit, die Frage fragt nach
    der Anwesenheit, `frage_invertiert` dreht sie um). Seit 2026-08-26 steht mit
    `vpf_auswaertige_taetigkeit` auch ein POSITIV benanntes Feld in der Liste — dort verneint
    `False`. Es musste hinein, weil es die Existenzfrage von ZWEI Regeln ist (Verpflegung und
    Übernachtung) und Julius die Übernachtungsfrage dreimal ohne Anlass bekam.
    """
    return bool(BINDUNG[flag].get("frage_invertiert"))


def _fragen(antworten: dict) -> list[str]:
    """Nur Felder der Scheibe `gesamt` — das ist der Fragebogen, den der Nutzer wirklich sieht."""
    return [f for f in TR.naechste_fragen(_store(antworten), BINDUNG) if f in SCHEIBE_GESAMT]


@pytest.mark.parametrize("flag", sorted(AK.AUSGABEN_SCREENING))
def test_flags_haengen_in_der_scheibe_gesamt(flag):
    """Stelle 2. Ohne diesen Eintrag existiert das Feld, wird aber nie gefragt — das Flag ist
    dann unbeantwortbar und schaltet nie etwas ab."""
    assert flag in BINDUNG, f"{flag} steht in AUSGABEN_SCREENING, aber in keiner Bindungsdatei."
    assert BINDUNG[flag].get("askable") is True, f"{flag} ist nicht fragbar."
    assert flag in SCHEIBE_GESAMT, (
        f"{flag} ist gebunden, steht aber nicht in SCHEIBEN['gesamt']['felder'] "
        f"(api_constants.py). Die Liste ist handgeschrieben — das Feld wird nie gefragt.")


@pytest.mark.parametrize("flag", sorted(AK.AUSGABEN_SCREENING))
def test_jedes_flag_schaltet_wirklich_etwas_ab(flag):
    """Stelle 3, und die eigentliche Prüfung: nicht ob ein regel_bedingungen-Eintrag existiert,
    sondern ob die Antwort MESSBAR Fragen entfernt. Ein Eintrag, der auf eine Regel zeigt, die
    ohnehin schon ausgeschlossen ist, sähe in einer Existenzprüfung genauso aus."""
    ohne = _fragen(BASIS)
    mit = _fragen({**BASIS, flag: _verneint(flag)})
    entfallen = set(ohne) - set(mit) - {flag}
    assert entfallen, (
        f"Ein bestätigtes {flag} entfernt KEINE einzige Frage. Entweder fehlt der Eintrag in "
        f"bindung_regel_bedingungen.yaml, oder er zeigt auf eine Regel, die für diesen Nutzer "
        f"schon aus einem anderen Grund ausgeschlossen ist. Das Flag kostet dann eine Frage und "
        f"bringt nichts — der Fix, der nichts bewirkt.")


@pytest.mark.parametrize("flag", sorted(AK.AUSGABEN_SCREENING))
def test_die_gegenrichtung_nimmt_niemandem_etwas(flag):
    """DIE POLARITÄTSFALLE, die hier schon dreimal Geld gekostet hat. Wer den Sachverhalt HAT,
    antwortet mit `false` — und muss dann MEHR Fragen sehen, nicht weniger. Ein Flag, das in
    beide Richtungen abschaltet, nähme den Pauschbetrag ausgerechnet dem Behinderten weg."""
    ohne = set(_fragen(BASIS))
    verneint = set(_fragen({**BASIS, flag: not _verneint(flag)}))
    verloren = ohne - verneint - {flag}
    assert not verloren, (
        f"{flag}=false (der Nutzer HAT den Sachverhalt) entfernt Fragen: {sorted(verloren)}\n"
        f"Das ist die Polaritätsfalle: das Flag muss nur bei `true` abschalten. Prüfe, ob es "
        f"versehentlich als Gate an einer ECHTEN Regel hängt statt über regel_bedingungen zu "
        f"wirken — ein askable bool an einer geltungsbedingung IST ein Gate.")


@pytest.mark.parametrize("flag", sorted(AK.AUSGABEN_SCREENING))
def test_unbeantwortet_schaltet_nichts_ab(flag):
    """fail-closed: solange niemand geantwortet hat, bleiben alle Fragen stehen. Ein Flag, das
    schon durch seine blosse Existenz vorauswählt, würde dem Nutzer Abzüge nehmen, nach denen er
    nie gefragt wurde."""
    assert BINDUNG[flag].get("beispielwert") is not _verneint(flag), (
        f"{flag}: der beispielwert entspricht der Verneinung — der Normalfall darf nichts "
        f"vorwegnehmen.")
    ohne = set(_fragen(BASIS))
    ziel_regeln = [r for r, eintraege in BEDINGUNGEN.items()
                   if any(e.get("feld") == flag for e in eintraege)]
    assert ziel_regeln, f"{flag} steht in keinem regel_bedingungen-Eintrag."
    for regel in ziel_regeln:
        felder = {f for f, b in BINDUNG.items()
                  if b.get("askable") and b["quelle"]["regel_id"] == regel and f in SCHEIBE_GESAMT}
        if felder:
            assert felder & ohne, (
                f"{regel} ist ohne Antwort auf {flag} bereits ausgeschlossen — dann belegt "
                f"test_jedes_flag_schaltet_wirklich_etwas_ab nichts über dieses Flag.")


def test_flags_sind_invertiert_deklariert():
    """Das Feld benennt die ABWESENHEIT, die Frage fragt nach der ANWESENHEIT. Bis 2026-08-20 riet
    die Erfassungsschicht am Feldnamen (`startswith('kein_')`) — das hätte `keine_*` verfehlt und
    dem Nutzer das Gegenteil seiner Antwort gespeichert."""
    # Massgeblich ist der FELDNAME, nicht die blosse Zugehoerigkeit zur Liste: seit 2026-08-26
    # steht mit `vpf_auswaertige_taetigkeit` ein positiv benanntes Feld darin, bei dem ein Kreuz
    # direkt „ja" heisst und eine Umkehr das Gegenteil speichern wuerde.
    falsch = [f for f in AK.AUSGABEN_SCREENING
              if bool(BINDUNG[f].get("frage_invertiert")) != f.startswith(("kein_", "keine_"))]
    assert not falsch, (
        f"{falsch}: `frage_invertiert` passt nicht zum Feldnamen. Benennt der Name die "
        f"Abwesenheit ('kein…'), muss die Umkehr gesetzt sein; benennt er die Anwesenheit, darf "
        f"sie es nicht — sonst speichert die Oberfläche das Gegenteil der Antwort.")


def test_kein_flag_traegt_eine_elster_kennzahl():
    """Anwendbarkeits-Flags, keine Deklarationsfelder. Eine Kz hier hiesse, dass die Antwort im
    XML landet — sie steuert aber nur, welche Fragen kommen."""
    mit_kz = [f for f in AK.AUSGABEN_SCREENING if BINDUNG[f].get("elster_kz")]
    assert not mit_kz, f"{mit_kz} tragen eine elster_kz."


def test_die_gemessene_wirkung_bleibt_erhalten():
    """Die Zahl aus dem Nutzerlauf, als Ratsche. Sie darf besser werden, nicht schlechter —
    sonst hat jemand ein Flag entwertet, ohne dass ein Einzeltest es merkt."""
    ohne = _fragen(BASIS)
    mit = _fragen({**BASIS, **{f: True for f in AK.AUSGABEN_SCREENING}})
    ersparnis = len(ohne) - len(mit)
    assert ersparnis >= 55, (
        f"Die fünf Flags ersparen nur noch {ersparnis} Fragen (gemessen am 2026-08-21: 59, "
        f"{len(ohne)} -> {len(mit)}). Entweder wurde ein regel_bedingungen-Eintrag entfernt oder "
        f"eine Zielregel umbenannt.")
