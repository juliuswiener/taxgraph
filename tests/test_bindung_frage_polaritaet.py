"""Gate: fragt `fragetext_laie` entgegengesetzt zum Feldnamen, MUSS die Umkehr deklariert sein.

Die Erfassungsschicht kehrt bool-Antworten genau dann um, wenn die Bindung `frage_invertiert:
true` sagt (app.js boolAntwort, api.py _wert_klartext). Vorher riet sie am Feldnamen
(`startswith("kein_")`). Diese Heuristik traf `kein_kap` und verfehlte jedes Feld mit der
Verneinung in der MITTE des Namens:

    vpf_keine_mahlzeitengestellung   „Hat dir dein Arbeitgeber ... Mahlzeiten ... gestellt?"
    dhf_keine_pflicht_dienstwohnung  „Ist deine Zweitwohnung ... eine Dienst- oder Werkswohnung ...?"

Beide speicherten das Gegenteil der Nutzerantwort; gemessene Wirkung 3.486,00 EUR (dHf) bzw.
87,00 EUR + eine gesperrte Berechnung (Verpflegung), s. tests/test_ui_frage_polaritaet.py.

Der Gate hier ersetzt die Heuristik nicht durch eine bessere Heuristik im Produktivcode — dort
steht jetzt eine Deklaration. Er benutzt sie als PRÜFER: wenn der Feldname verneint und der
Fragetext nicht (oder umgekehrt), muss jemand die Richtung erklärt haben.

GRENZEN, ausdrücklich:

  1. Der Prüfer erkennt Verneinung an Wörtern, nicht an Bedeutung. Ein invertiertes Feld mit
     einem POSITIVEN Namen (etwa `arbeitgeber_zahlte_alles` unter der Frage „Hast du selbst
     gezahlt?") trägt kein Signal und bleibt unsichtbar. Dagegen hilft nur das Lesen der Frage.
  2. Die Rückrichtung — positiver Name, verneinte Frage — wird NICHT gegatet. Gemessen: sie
     erzeugt auf dem heutigen Bestand nur Fehltreffer, weil Klammer-Zusätze wie „(nicht
     gewerblich)", „(nicht bar)", „(nicht nur vorübergehend)" Präzisierungen sind und keine
     Polaritätswechsel. Ein Gate mit vier bekannten Fehltreffern wird abgeschaltet, nicht gelesen.

Auf dem Bestand vom 2026-08-20 trennt der Prüfer die 15 verneinend benannten askable bool-Felder
ohne Fehltreffer in 7 umzukehrende und 8 selbstverneinende.

NULL LLM.
"""
from __future__ import annotations

import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import traverser as TR  # noqa: E402

BINDUNG = TR.lade_bindung()

# Verneinung im Feldnamen: ganze snake_case-Token, nicht Teilzeichenketten. „keine" als Token
# trifft `vpf_keine_mahlzeitengestellung`; eine Teilzeichenketten-Suche träfe auch `kleinbetrag`.
NAME_NEGATION = {"kein", "keine", "keinen", "keiner", "nicht", "ohne", "weder", "nie"}
FRAGE_NEGATION = re.compile(r"\b(kein|keine|keinen|keiner|nicht|ohne|weder|nie|niemals)\b", re.I)

# Felder, deren Frage die Umkehr braucht, obwohl der Name das nicht verrät — der Fall, den der
# Prüfer nicht sehen kann (Grenze 1). Leer: heute trägt jedes invertierte Feld eine Verneinung im
# Namen. Ein Eintrag hier ist eine bewusste Aussage „geprüft, Name gibt nichts her", kein Freibrief.
INVERTIERT_OHNE_NAMENSSIGNAL: dict[str, str] = {}


def _askable_bools() -> list[tuple[str, dict]]:
    return sorted((fid, b) for fid, b in BINDUNG.items()
                  if b.get("askable") and b.get("typ") == "bool")


def _name_verneint(fid: str) -> bool:
    return bool(NAME_NEGATION & set(fid.split("_")))


def _frage_verneint(b: dict) -> bool:
    return bool(FRAGE_NEGATION.search(b.get("fragetext_laie") or ""))


def test_es_gibt_ueberhaupt_askable_bools():
    """Ohne diese Zusicherung wäre jeder Test unten vakuum-grün, sobald die Bindung nicht mehr
    lädt oder die Feldstruktur sich ändert (`bindungen` statt `felder` — genau daran ist der
    erste Sweep-Versuch zu diesem Fund stillschweigend vorbeigelaufen)."""
    felder = _askable_bools()
    assert len(felder) >= 70, f"nur {len(felder)} askable bool-Felder gefunden — Bindung geladen?"
    assert any(_name_verneint(fid) for fid, _ in felder), "kein verneinend benanntes Feld gefunden"
    assert any(b.get("frage_invertiert") for _, b in felder), "kein invertiertes Feld gefunden"


def test_verneinter_name_ohne_verneinte_frage_muss_umkehren():
    """Der eigentliche Gate. Feldname behauptet eine Abwesenheit, die Frage fragt nach der
    Anwesenheit — dann ist die Antwort des Nutzers die Verneinung des gespeicherten Werts."""
    fehlt = []
    for fid, b in _askable_bools():
        if _name_verneint(fid) and not _frage_verneint(b) and not b.get("frage_invertiert"):
            fehlt.append(f"{fid}: „{b.get('fragetext_laie')}\"")
    assert not fehlt, (
        "Feldname verneint, Fragetext nicht — die Antwort des Nutzers wird ungekehrt gespeichert:\n"
        + "\n".join(f"   - {z}" for z in fehlt)
        + "\nEntweder `frage_invertiert: true` in die Bindung (wenn die Frage wirklich in die "
          "Gegenrichtung fragt) oder den Fragetext auf die Richtung des Feldnamens umstellen.")


def test_verneinte_frage_darf_nicht_zusaetzlich_umkehren():
    """Die Kehrseite. Trägt die Frage die Verneinung selbst („Hast du KEINE Bankverbindung?"),
    ist eine Umkehr die doppelte Verneinung — der Wert kippt ins Gegenteil. Genau diesen Fehler
    hätte die alte Präfix-Heuristik gemacht, wenn das Feld `kein_bankverbindung` geheißen hätte."""
    zuviel = []
    for fid, b in _askable_bools():
        if b.get("frage_invertiert") and _frage_verneint(b):
            zuviel.append(f"{fid}: „{b.get('fragetext_laie')}\"")
    assert not zuviel, (
        "frage_invertiert gesetzt, obwohl die Frage die Verneinung selbst trägt (doppelte "
        "Verneinung — der gespeicherte Wert ist das Gegenteil der Antwort):\n"
        + "\n".join(f"   - {z}" for z in zuviel))


def test_umkehr_ohne_namenssignal_ist_benannt():
    """Grenze 1 sichtbar halten: ein invertiertes Feld ohne Verneinung im Namen ist für den
    Prüfer unsichtbar. Es muss dann wenigstens hier stehen, mit Begründung — sonst wächst
    unbemerkt eine Klasse heran, die kein Gate mehr findet."""
    unbenannt = [fid for fid, b in _askable_bools()
                 if b.get("frage_invertiert") and not _name_verneint(fid)
                 and fid not in INVERTIERT_OHNE_NAMENSSIGNAL]
    assert not unbenannt, (
        f"invertiert, aber ohne Verneinung im Namen und ohne Eintrag in "
        f"INVERTIERT_OHNE_NAMENSSIGNAL: {unbenannt}")


@pytest.mark.parametrize("fid", sorted(INVERTIERT_OHNE_NAMENSSIGNAL))
def test_eintraege_ohne_namenssignal_gelten_noch(fid):
    """Ein Eintrag, dessen Feld inzwischen anders heißt oder nicht mehr invertiert, ist tote
    Ausnahme — er würde einen echten künftigen Fund decken."""
    b = BINDUNG.get(fid)
    assert b is not None, f"{fid} ist nicht mehr gebunden — Eintrag entfernen."
    assert b.get("frage_invertiert"), f"{fid} ist nicht mehr invertiert — Eintrag entfernen."
    assert not _name_verneint(fid), f"{fid} trägt die Verneinung jetzt im Namen — Eintrag entfernen."


def test_frage_invertiert_nur_auf_askable_bool():
    """`frage_invertiert` auf einem cent-Feld oder einem nicht gefragten Feld ist wirkungslos und
    liest sich wie eine Zusicherung. Das Schema verbietet es; hier steht die Messung dazu, damit
    ein gelockertes Schema nicht unbemerkt durchgeht."""
    falsch = [fid for fid, b in BINDUNG.items()
              if b.get("frage_invertiert") and not (b.get("askable") and b.get("typ") == "bool")]
    assert not falsch, f"frage_invertiert auf Nicht-bool oder nicht-askable Feldern: {falsch}"


# ------------------------------------------------- Die zwei Verwender OHNE Browser
#
# Die Erfassungsschicht hat drei Verwender der Umkehr. Zwei davon laufen ohne Browser und gehören
# deshalb HIERHER: die Oberflächen-Tests (tests/test_ui_frage_polaritaet.py) hängen an playwright,
# und playwright ist in CI nicht installiert — dort überspringt die ganze Datei. Was nur dort
# stünde, prüfte in CI niemand.
#
#   api.fragen()          reicht `frage_invertiert` an die Oberfläche (ohne das rät sie wieder)
#   api._wert_klartext()  baut den Erklär-Kontext fürs Modell
#
# Der dritte, app.js, braucht den Browser und bleibt in der anderen Datei.

def test_fragen_reicht_die_umkehr_an_die_oberflaeche(tmp_path, monkeypatch):
    """Die Oberfläche kann nur umkehren, was sie erfährt. Fehlt der Schlüssel in der Antwort von
    /fragen, ist `q.frage_invertiert` dort `undefined` — und jede Umkehr fällt still aus, ohne
    dass irgendwo etwas rot wird.

    Gemessen wird der echte Payload aus api.fragen(), nicht ein Nachbau."""
    import api as API
    import audit

    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, _ = API.fall_anlegen({"scheibe": "an_gesamt", "veranlagungszeitraum": 2025,
                              "fall_id": "polaritaet"})
    assert st in (200, 201), f"Fall nicht angelegt: {st}"
    st, body = API.fragen("polaritaet")
    assert st == 200, f"/fragen liefert {st}"

    fragen = {q["feld_id"]: q for q in body["fragen"]}
    assert fragen, "/fragen liefert keine einzige Frage — der Test misst sonst nichts."

    ohne_schluessel = [fid for fid, q in fragen.items() if "frage_invertiert" not in q]
    assert not ohne_schluessel, (
        f"/fragen liefert diese Felder ohne 'frage_invertiert': {ohne_schluessel[:5]} — "
        f"die Oberfläche sieht dort undefined und kehrt nicht um.")

    falsch = [fid for fid, q in fragen.items()
              if q["frage_invertiert"] != bool(BINDUNG[fid].get("frage_invertiert"))]
    assert not falsch, f"/fragen widerspricht der Bindung bei: {falsch}"

    # Und die Messung hat überhaupt Griff: mindestens ein invertiertes Feld war dabei.
    assert any(q["frage_invertiert"] for q in fragen.values()), (
        "kein einziges invertiertes Feld im Payload — der Test wäre vakuum-grün.")


@pytest.mark.parametrize("fid,gespeichert,erwartet", [
    # invertiert: gespeichertes true heisst „nein" auf die gestellte Frage
    ("kein_kap", True, "nein"),
    ("kein_kap", False, "ja"),
    ("vpf_keine_mahlzeitengestellung", True, "nein"),
    ("vpf_keine_mahlzeitengestellung", False, "ja"),
    ("dhf_keine_pflicht_dienstwohnung", True, "nein"),
    # nicht invertiert: die Frage traegt die Verneinung selbst
    ("stammdaten_keine_bankverbindung", True, "ja"),
    # GEDREHT 2026-08-26 mit der Frage selbst (Julius: „1 ändern auf positive fragen"). Beide
    # Fragen tragen die Verneinung nicht mehr im Text — „Wurden Nebenkosten vereinbart?" statt
    # „Wurden KEINE Nebenkosten vereinbart?" —, also sind beide Felder jetzt `frage_invertiert`
    # und ein gespeichertes `true` (= „nicht vereinbart") heisst auf die gestellte Frage „nein".
    # Dass diese beiden Zeilen sich mitdrehen MUSSTEN, ist der Beleg, dass die Umkehr greift:
    # bliebe die Anzeige gleich, waere die neue Frage mit der alten Antwort beschriftet.
    ("vv_nebenkosten_nicht_vereinbart", True, "nein"),
    ("vv_nebenkosten_nicht_vereinbart", False, "ja"),
    ("hh_handwerker_keine_foerderung", False, "ja"),
    ("hh_handwerker_keine_foerderung", True, "nein"),
])
def test_erklaer_kontext_nennt_die_antwort_nicht_den_rohwert(fid, gespeichert, erwartet):
    """`_wert_klartext` sagt dem Modell, was der Nutzer geantwortet hat. Vorher riet es am
    Feldnamen und erzählte bei vpf/dhf das Gegenteil — die Erklärung argumentierte dann gegen
    die eigene Angabe des Nutzers, und zwar mit der Autorität einer Auskunft."""
    import api as API

    assert fid in BINDUNG, f"{fid} nicht mehr gebunden — Fall anpassen."
    assert API._wert_klartext(fid, gespeichert, BINDUNG) == erwartet, (
        f"{fid}={gespeichert} muss dem Modell als „{erwartet}\" gemeldet werden, gemeldet wird "
        f"„{API._wert_klartext(fid, gespeichert, BINDUNG)}\".")


def test_pruefer_trennt_den_bestand_ohne_fehltreffer():
    """Der Prüfer selbst unter Beobachtung. Er ist nur so viel wert wie seine Trennschärfe: auf
    dem gemessenen Bestand (2026-08-20: 15 verneinend benannte Felder) stimmt seine Erwartung mit
    der Deklaration in JEDEM Fall überein. Weicht das eines Tages ab, ist entweder ein neues Feld
    falsch deklariert oder die Wortliste zu grob — beides will gesehen werden, nicht überlesen."""
    abweichung = []
    for fid, b in _askable_bools():
        if not _name_verneint(fid):
            continue
        erwartet = not _frage_verneint(b)
        if bool(b.get("frage_invertiert")) != erwartet:
            abweichung.append(f"{fid}: Prüfer erwartet frage_invertiert={erwartet}, "
                              f"deklariert ist {bool(b.get('frage_invertiert'))}")
    assert not abweichung, "\n".join(abweichung)
