"""Jeder Befund, den der Preflight erzeugt, muss den Nutzer auch ERREICHEN.

ANLASS, gemessen am 2026-08-27: `preflight()` bekam sechs neue Plausibilitätsprüfungen
(Lohnsteuer über dem Lohn, Kirchensteuer, Schulgeld, …) unter dem Schlüssel
`widersprueche_plausibilitaet`. Sie liefen, sie hoben den Status auf RED — und beim Nutzer kam
NICHTS an. `api.preflight_check()` führt eine fest verdrahtete Liste der Schlüssel, die es
ausliefert; ein Schlüssel, der dort nicht steht, ist totes Wiring.

Die Wirkung wäre ausgerechnet die Form gewesen, die Julius am selben Tag als neunten Befund
gemeldet hat: ein roter Zustand OHNE einen einzigen Grund daneben. Der Nutzer liest „stimmt was
nicht" und erfährt nicht, was.

Diese Datei ist deshalb kein Test einer Prüfung, sondern ein ERREICHBARKEITS-GATE: es vergleicht,
was `preflight()` erzeugen KANN, mit dem, was `preflight_check()` durchlässt. Die nächste neue
Prüfung fällt hier auf, bevor sie stillschweigend ins Leere läuft — ohne dass jemand daran denken
muss.

Dieselbe Klasse gab es hier schon dreimal (SCHEIBEN.felder-Wiring, § 35c, Kegel-Feld) — jedes Mal
lief Code, den niemand erreichte, und jedes Mal fiel es nur zufällig auf.
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "produkt/konsistenz", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API        # noqa: E402
import api_auth          # noqa: E402
import audit             # noqa: E402
import preflight as PF   # noqa: E402


def _ausgelieferte_schluessel() -> set[str]:
    """Die Schlüssel, die `preflight_check()` tatsächlich ausliefert — aus dem Quelltext gelesen.

    Aus dem QUELLTEXT und nicht aus einem Aufruf: die Liste steht als Literal in einer
    for-Schleife, und ein Aufruf zeigte nur die Schlüssel, die bei DIESEM Fall etwas zu sagen
    haben. Genau daran würde das Gate vorbeimessen — ein leerer Befund und ein nicht
    ausgelieferter Befund sehen im Ergebnis gleich aus.
    """
    quelle = open(os.path.join(ROOT, "produkt", "haut", "api.py"), encoding="utf-8").read()
    anfang = quelle.index("def preflight_check")
    rumpf = quelle[anfang:quelle.index("\ndef ", anfang + 10)]
    return {s for s in PF.preflight({}) if s != "status" and f'"{s}"' in rumpf}


def test_jeder_preflight_befund_wird_auch_ausgeliefert():
    """DAS GATE. `preflight({})` nennt alle Sorten, die das Modul kennt — auch die, die gerade
    nichts zu melden haben. Jede davon muss in der Auslieferungsliste stehen."""
    erzeugt = {s for s in PF.preflight({}) if s != "status"}
    ausgeliefert = _ausgelieferte_schluessel()
    fehlend = erzeugt - ausgeliefert
    assert not fehlend, (
        f"Diese Preflight-Befunde erreichen den Nutzer NICHT: {sorted(fehlend)}. Sie heben den "
        f"Status auf RED und liefern keinen einzigen Grund dazu — der Nutzer liest „stimmt was "
        f"nicht“ und erfährt nicht, was. In api.preflight_check() gehört je Sorte eine Zeile "
        f"(typ, bereich, schluessel, textfeld) in die Liste.")


def test_das_gate_misst_die_liste_und_nicht_einen_leeren_aufruf(tmp_path, monkeypatch):
    """Vorbedingung des Gates: die gelesene Liste ist nicht leer und nicht alles.

    Ohne diesen Test könnte `_ausgelieferte_schluessel()` still auf die leere Menge fallen (ein
    umbenanntes `preflight_check`, ein anderer Schnitt) — dann meldete das Gate oben ALLES als
    fehlend, oder, schlimmer, bei einer leeren `erzeugt`-Menge gar nichts mehr."""
    ausgeliefert = _ausgelieferte_schluessel()
    assert len(ausgeliefert) >= 5, (
        f"Aus api.preflight_check() wurden nur {len(ausgeliefert)} Schlüssel gelesen — das Gate "
        f"misst dann nicht mehr, was es behauptet: {sorted(ausgeliefert)}")


def test_ein_plausibilitaets_widerspruch_steht_wirklich_in_der_antwort(tmp_path, monkeypatch):
    """Und der Weg vom Ende her: ein echter Widerspruch, durch den ECHTEN Endpunkt.

    Das Gate oben liest Quelltext; dieser Test lässt den Fall wirklich laufen. Beides zusammen,
    weil keins von beidem allein reicht: der Quelltext-Vergleich sähe eine falsche Zuordnung
    (Schlüssel vorhanden, aber `textfeld` daneben) nicht, und ein einzelner Durchlauf sagt nichts
    über die Sorten, die er zufällig nicht auslöst."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", "pruefer")
    fid = "preflight-erreichbar"
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    # `p36_lohnsteuer`, nicht `lohnsteuer`: die einbehaltene Lohnsteuer hängt an der
    # Abschlusszahlung (§ 36). Genau Julius' gemessener Fall — 12.123.213 EUR bei 40.000 EUR Lohn.
    for feld, wert in (("bruttoarbeitslohn", 4000000), ("p36_lohnsteuer", 1212321300)):
        API.event(fid, {"feld_id": feld, "wert": wert, "zustand": "bestaetigt",
                        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                     "haftung": "nutzer"},
                        "schreiber": "ui:laie",
                        "signal": {"signal_1": None, "signal_2": f"klick@{feld}"}})

    st, b = API.preflight_check(fid)
    assert st == 200
    plausi = [i for i in b["items"] if i.get("bereich") == "plausibilitaet"]
    assert plausi, (
        f"Lohnsteuer 12.123.213 EUR bei 40.000 EUR Lohn — und in der Antwort steht kein Wort "
        f"davon: status={b['status']!r}, items={b['items']}")
    # Geprüft wird der FELDNAME, nicht das Wort: „Lohnsteuer" gehört in den Satz, `p36_lohnsteuer`
    # nicht. Mein erster Versuch verbot das Wort und wurde zu Recht rot — der Meldetext war
    # richtig, die Behauptung darüber falsch.
    assert plausi[0]["text"], f"Leerer Meldetext: {plausi[0]}"
    assert "p36_" not in plausi[0]["text"] and "_" not in plausi[0]["text"], (
        f"Der Text nennt eine Feldkennung statt der Sache — der Nutzer hat sie nie gesehen: "
        f"{plausi[0]['text']!r}")
    assert "40.000" in plausi[0]["text"] and "12.123.213" in plausi[0]["text"], (
        f"Die Meldung nennt nicht BEIDE Zahlen, zwischen denen der Nutzer entscheiden soll: "
        f"{plausi[0]['text']!r}")
    assert b["status"] == "RED", f"Ein Widerspruch, aber der Status bleibt {b['status']!r}."
