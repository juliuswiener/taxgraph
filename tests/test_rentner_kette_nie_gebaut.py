"""P5.4 Rechenweg-Kette (Erklaer-UI, `extras["kette"]`) fehlt im Rentner-Zweig komplett.

Instructor-Auftrag "Bruch 2": Vorhersage vorher zwei moegliche Bauarten -- (a) dieselbe Info wird
irgendwo BERECHNET und dann auf dem Weg zur API verworfen (Bauart von `_chat_parse()`, 0d0fec3/
71f92d6: `rechenweg` kam vom Modell, wurde beim Parsen weggelassen), oder (b) andere Bauart mit
gleichem Symptom: nie berechnet.

Codelese-Vorhersage (vor der Messung notiert, Instructor-Nachricht zitiert): `_zweig_festzusetzende_
est_rentner` (bescheid_zweige.py:940-1297) enthaelt KEINEN einzigen Verweis auf `catala_gesamt_kette`
oder `extras["kette"]` -- nur `_zweig_festzusetzende_est_gesamt` (Zeile 902-905) setzt es, und nur bei
kinder==0. `grep catala_.*kette` im ganzen Baum: genau eine Funktion, `catala_gesamt_kette`
(runner.py:1639), keine rentner-eigene. Vorhersage: (b), kein Verlust-beim-Transport.

Gemessen ueber den ECHTEN Nutzerpfad (HTTP: POST /fall -> POST /fall/<id>/event -> GET /fall/<id>/
ergebnis, ueber server.py, nicht API._bescheid_fn direkt und nicht mal API.ergebnis() python-intern --
das ist genau die Faussstelle, an der die Nachbarinstanz heute (2026-08-30) einen Befund verlor, weil
`_bescheid_fn` einen Wächter überspringt, den der HTTP-Pfad passiert). Bestaetigt: (b). 200.000 EUR
Jahresrente, Erstjahr 2025, kinderlos, alle Pflichtfelder beantwortet -> grund=bestaetigt,
zahl_cent=5917000, kette=None.

Hebel-Beleg (Instructor-Auflage: erst zeigen dass der Hebel existiert, bevor eine gemessene
Abwesenheit etwas beweist): derselbe Betragsrahmen (59.170 EUR) auf scheibe=gesamt statt
rentner_gesamt, ebenfalls kinderlos+bestaetigt, LIEFERT eine vollstaendige kette (vier Stufen).
Einzige geaenderte Groesse ist die Scheibe -- der P5.4-Aufruf existiert im gesamt-Zweig, im
Rentner-Zweig nirgends.

Root Cause: `catala_gesamt_kette(s)` ruft intern `_gesamt_out(s)` -- denselben Catala-"Gesamtfall"-
Scope, den auch `catala_est`/`catala_gesamt_zve`/`catala_gesamt_tarifliche` fuer JEDEN gesamtfall=True-
Sachverhalt verwenden. `rentner_g` (bescheid_zweige.py:1075) traegt bereits `"gesamtfall": True` und
wird im Rentner-Zweig laufend an genau diese Funktionen uebergeben (z.B. Zeile 1154/1174/1236) --
`catala_gesamt_kette(rentner_g)` ist damit strukturell derselbe Aufruf wie im gesamt-Zweig, nur nie
geschrieben. Kein neuer Ring-Code noetig, nur der fehlende Aufruf (P5.4 wurde am 2026-07-30 nur fuer
scheibe=gesamt gebaut und nie auf rentner_gesamt nachgezogen).

Fix im selben Commit: Aufruf `extras["kette"] = runner.catala_gesamt_kette(rentner_g)` nachgezogen,
1:1 an der Stelle, die Zeile 902-905 im gesamt-Zweig entspricht (nach der kinder>0/kinder==0-
Verzweigung, gleiche kinder==0-Bedingung wie dort -- § 31-Zweig-Ambiguitaet, siehe Kommentar Zeile 902).

Aufrufer-Pruefung nach dem Fix: `extras["kette"]` wird an genau EINER Backend-Stelle weiter gelesen,
`api.py:614` (`"kette": extras.get("kette")`), reine Durchreiche in die /ergebnis-Antwort, keine
zweite Verlust-Stelle zwischen bescheid_zweige.py und der API-Antwort (im Unterschied zum ersten
Bruch bei `_chat_parse()` war hier ueberhaupt keine Zwischenstation zu pruefen). Instructor-Nachfrage
(2026-08-31): "liest irgendwer extras['kette'] auf dem Rentner-Pfad, und merkt der Zweig seinen
Verlust?" -- ja, echter Leser mit echtem Verlust: `app.js:2351` (`if (r.kette) {...} else {...}`)
ist scheibe-agnostisch, feuert bei JEDEM grund=bestaetigt-Ergebnis. Der Kommentar direkt an der
else-Zeile (app.js:2398, VOR diesem Fix geschrieben) listet den Fallback-Fall explizit als
"(Rentner, Kinder, an_gesamt)" -- die UI-Seite kannte die Rentner-Luecke also bereits als
beobachtete Tatsache. Der Unterschied zu den anderen beiden: an_gesamt hat eine eigene, strukturell
andere Zweigfunktion (`_zweig_festzusetzende_est`, Zeile 156, "§2 Gesamtsteuer MVP" -- baut gar
keinen gesamtfall=True-Sachverhalt, kann `catala_gesamt_kette` architektonisch nicht bedienen) und
Kinder>0 hat eine dokumentierte Begruendung im Code selbst (Zeile 902, § 31-Zweig-Ambiguitaet).
Fuer Rentner/kinder==0 gab es weder das eine noch das andere -- nur eine Luecke, die die UI schon
kannte, aber niemand im Ring geschlossen hatte. Vor jedem gesamt-Nutzer bekommt ein rentner_gesamt-
Nutzer derselben Groessenordnung also NIE die Rechenweg-Erklaerung, ohne dass irgendwo dafuer ein
Grund steht -- das ist der "echte Verlust", nicht nur ein still liegendes Feld.

Nebenbefund (Instructor-Hinweis 2026-08-31, NICHT Teil dieses Fixes): ein anderer Worker
("sackgasse-schnitt") hat bei HEAD 5af945c GET /fragen -> HTTP 500 RentenfreibetragFixierungOffen
("aa-Folgejahr 0<2025 ohne fixierten Rentenfreibetrag") fuer rentner_gesamt gemeldet. Eigene
Gegenprobe (primaeres Kegel wie oben, /fragen nach jedem Event aufgerufen, HEAD fa9453d): NICHT
reproduziert -- alle Aufrufe lieferten 200. Der primaere aa-Zweig mit renten_beginn_jahr==VZ nimmt
in `catala_renten_einkuenfte` (runner.py:911) den sicheren Erstjahr-Pfad, nicht den Fixierungs-Ast
(runner.py:913-916). Vermutung, nicht verifiziert: der Trigger liegt bei den `_partner`-Rentenfeldern
(vgl. `test_rentner_partner_kegel_guard.py`, dort bereits einmal ein strukturell verwandter 500er
auf /ergebnis gefixt) -- meine Bruch-2-Kegel und mein permanenter Test beruehren keine Partnerfelder
und keinen /fragen-Aufruf, daher unabhaengig von diesem Defekt. Nicht weiter verfolgt: gehoert nicht
zu diesem Fix, main hat es ausdruecklich als getrennten Meldepunkt eingeordnet.

NULL LLM."""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402
import audit              # noqa: E402


def _req(base: str, method: str, path: str, body: dict | None = None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


RENTNER_KEGEL = [
    ("veranlagung", "einzel"),
    ("rentner_renten_art", "gesetzliche_rente"),
    ("rentner_jahresrente", 20_000_000),   # 200.000 EUR in Cent
    ("rentner_renten_beginn_jahr", 2025),
    ("rentner_alter_bei_rentenbeginn", 65),
    ("rentner_rentenfreibetrag", 0),
    ("rentner_grad_der_behinderung", 0),
    ("rentner_hilflos_blind_taubblind", False),
    ("rentner_hinterbliebenenbezuege", False),
    ("rentner_pflegegrad", 0),
    ("rentner_gepflegter_hilflos", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
    ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0),
    ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0),
    ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
    ("mit_anspruch_auf_zuschuss", False),
]

_STAMM = [
    ("stammdaten_nachname", "Meier"), ("stammdaten_vorname", "Klaus"),
    ("stammdaten_geburtsdatum", "01.01.1970"),
    ("stammdaten_strasse", "Teststr."), ("stammdaten_hausnummer", "1"),
    ("stammdaten_plz", "10115"), ("stammdaten_wohnort", "Berlin"),
    ("stammdaten_keine_bankverbindung", True),
    ("stammdaten_art_est_erklaerung", True),
    ("kist_konfession", "keine"),
]
_PFLICHT_ZUSATZ = [
    ("basis_kv", 0), ("basis_pv", 0), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("mit_anspruch_auf_zuschuss", False),
    ("ep_arbeitstage", 0), ("ep_eigenes_kfz", False), ("ep_entfernung_km", 0),
    ("ep_oepnv_kosten", 0),
]
# Gleicher Betragsrahmen wie RENTNER_KEGEL (59.170 EUR Zahllast), scheibe=gesamt: Hebel-Beleg.
GESAMT_KONTROLLE_KEGEL = [
    ("bruttoarbeitslohn", 5_917_000), ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0),
    ("vor_rv_ausserhalb_lstb", 0),
    ("kein_gewinn", True), ("kein_vuv", True), ("kein_sonstige", True), ("kein_kap", True),
] + _STAMM + _PFLICHT_ZUSATZ + [("veranlagung", "einzel")]


def _anlegen(base, fid, scheibe, kegel):
    st, resp = _req(base, "POST", "/fall", {"scheibe": scheibe, "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201, resp
    for feld, wert in kegel:
        st, resp = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201, (feld, wert, resp)


def test_hebel_existiert_gesamt_kinderlos_hat_kette(base):
    """Kontrolle: derselbe Betragsrahmen, scheibe=gesamt statt rentner_gesamt, kinderlos,
    bestaetigt -- kette MUSS gesetzt sein. Ohne diesen Beleg waere ein rentner_gesamt-Nullbefund
    unterhalb der Schwelle, an der ein Unterschied ueberhaupt entstehen kann (Instructor-Auflage)."""
    _anlegen(base, "kette-kontrolle", "gesamt", GESAMT_KONTROLLE_KEGEL)
    st, erg = _req(base, "GET", "/fall/kette-kontrolle/ergebnis")
    assert st == 200, erg
    assert erg["grund"] == "bestaetigt", erg
    assert erg["kette"] is not None, "Hebel existiert nicht -- Kontrolllauf muesste eine kette liefern"
    assert erg["kette"]["festzusetzende_est"] * 100 == erg["zahl_cent"]


def test_rentner_gesamt_kinderlos_bestaetigt_hat_kette(base):
    """200.000 EUR Rente, kinderlos, bestaetigt -- kette ist jetzt gesetzt (Fix: catala_gesamt_kette
    wird auch im Rentner-Zweig aufgerufen, 1:1 gesamt-Praezedenz). War xfail bis zu diesem Commit."""
    _anlegen(base, "kette-rentner", "rentner_gesamt", RENTNER_KEGEL)
    st, erg = _req(base, "GET", "/fall/kette-rentner/ergebnis")
    assert st == 200, erg
    assert erg["grund"] == "bestaetigt", erg
    assert erg["kette"] is not None, (
        f"kette fehlt bei rentner_gesamt trotz grund=bestaetigt (zahl_cent={erg['zahl_cent']}) -- "
        "die Erklaer-UI (P5.4) zeigt jedem Rentner-Fall keinen Rechenweg, jedem gesamt-Fall schon")
