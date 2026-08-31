"""HTTP-Messung (server.py, echter Prozess): ein Rentner mit privatem Grundstuecksverkauf
(§23 EStG) bekam bis HEAD 78861cc STILL eine um 21.000,00 € zu niedrige Steuer `bestaetigt` --
keine der vier gesetzlichen Befreiungen (Spekulationsfrist, Eigennutzung, taeglicher Gebrauch,
originaerer Erwerb) wurde je erfragt, `catala_p23_veraeusserungsgewinn` rechnete bedingungslos.

Ziel dieses Tests ist NICHT, die Zahl zu korrigieren (dafuer fehlen die vier Befreiungs-
Antworten, die es noch nicht gibt), sondern den Ring zu SPERREN statt eine geratene Zahl
auszugeben -- "still falsch wird laut gesperrt". Der Fix: ein neues, eigenstaendiges Kreuz
`kein_p23_verkauf` (produkt/bindung/bindung_an_gesamt.yaml), Pflichtfeld im Kegel beider
Scheiben (gesamt, rentner_gesamt), als fremd_arten-Mitglied verdrahtet (produkt/haut/
api_constants.py) -- derselbe, unveraenderte generische Mechanismus, den `kein_sonstige`
und `kein_vuv` schon nutzen (produkt/bescheid/bescheid_deklaration.py, _an_gesamt_sperrgrund,
`any(felder.get(fl,{}).get("wert") is False for fl in cfg.get("fremd_arten", ()))`). Das Kreuz
kreuzt selbst nichts weg: keine bindung referenziert es in einem feld_bedingung, die vier
§23-Detailfelder waren nie an ein feld_bedingung gekoppelt (weder vorher noch durch diese
Aenderung) -- der Nutzer bekommt eine zusaetzliche Frage gestellt, keine Felder werden ihm
entzogen.

Reproduziert das "Rotes Kommando" aus dem Vault-Eintrag
backlog/taxgraph/p23-sonstige-kreuz-kopplung-rentner-reachable.md (dort mit identischem Kegel
und identischen Betraegen gemessen: Δ = 2.100.000 Cent = 21.000,00 € bei HEAD c745e59).

Vier Zusicherungen, nicht eine:
1. Das neue Kreuz erscheint im Fragenlauf BEIDER Scheiben, genau einmal, bevor irgendetwas
   sonst beantwortet ist (main-Auflage: "ein Fragenlauf, der dein Kreuz nicht enthaelt, sieht
   aus wie einer, der es enthaelt" -- Zaehlung statt Farbe).
2. Gegenprobe -- ein Rentner OHNE Verkauf beantwortet das neue Kreuz wahrheitsgemäss mit "kein
   Verkauf" (True) und bekommt GENAU dieselbe Zahl wie vor dem Kreuz (zahl_cent=5917000), in
   ZWEI getrennten Zustaenden: (a) die vier §23-Detailfelder bleiben unbeantwortet, (b) dieselben
   Felder werden ausdruecklich mit 0 beantwortet. Ein Veraeusserungsgewinn von 0 € ist der
   Normalfall bei Verkauf zum Einstandspreis, kein Randfall -- "unbeantwortet" und "0" muessen
   beide durchkommen, sonst sperrt die Pruefung jemanden, der wahrheitsgemaess geantwortet hat
   (main-Auflage nach der kap_gewinn-Nullblindheits-Messung von nebenan).
3. Der Defektfall (Verkauf bejaht) liefert den KONKRETEN Sperrgrund, den fremd_arten produziert
   -- nicht bloss "irgendeine Sperre" (sonst waere der Test auch gruen, wenn die Software aus
   einem ganz anderen Grund sperrt oder das Kreuz nie gestellt wird und der Nutzer fail-closed
   haengenbleibt).
4. Der Klartext zu diesem Sperrgrund ist nicht leer, erreicht den Browser 1:1 im /ergebnis-
   Response, UND nennt private Verkaeufe -- nicht nur die vier alten Gruende (Renten/sonstige,
   Vermietung, Kapitalertraege, Gewinn aus Betrieb), sonst sieht sich ein ueber `kein_p23_verkauf`
   gesperrter Nutzer im eigenen Sperrtext nicht wieder. Dieser vierte Teil ist zum Zeitpunkt der
   ersten Messung (HEAD 8c47a7b) absichtlich ROT: der bestehende, generische Klartext-Eintrag
   ("einkunftsart_nicht_ring_faehig") deckt die vier alten Gruende ab, nicht den fuenften. Die
   eine noch ausstehende Zeile in bescheid_deklaration.py::SPERRGRUND_KLARTEXT ist mit main
   koordiniert (geteilte Datei, Nachbarinstanz baut ~460 Zeilen weiter unten) und wird erst nach
   Meldung gesetzt.

`kein_sonstige` wird von diesem Test NICHT angefasst -- weder umbenannt noch seine Reichweite
geaendert noch in fremd_arten nachgezogen; das Kreuz behaelt hier exakt den Wert, den ein
ehrlicher Rentner ohnehin eintragen muss (`False`, da die eigene Rente sonstige Einkunft ist).

Gemessen bei HEAD 8c47a7b... (2026-08-30), /tmp-Klon, oracle/gettsim/_catala/{pkg,rt} aus dem
echten Repo kopiert (gitignored Build-Artefakte, sonst engine_unavailable).

NULL LLM.
"""
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
for sub in ("produkt/haut", "produkt/bescheid", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API                          # noqa: E402
import audit                               # noqa: E402
import server as SRV                       # noqa: E402
from bescheid_deklaration import SPERRGRUND_KLARTEXT   # noqa: E402

FLAG_ID = "kein_p23_verkauf"
ERWARTETER_SPERRGRUND = "einkunftsart_nicht_ring_faehig"


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    assert srv.server_address[0] == "127.0.0.1"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _req(base_url, method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base_url + path, data=data, method=method,
                                  headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, wert):
    return {"feld_id": fld, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _neuer_fall(base_url, fall_id, scheibe):
    st, r = _req(base_url, "POST", "/fall",
                 {"scheibe": scheibe, "veranlagungszeitraum": 2025, "fall_id": fall_id})
    assert st == 201, (st, r)


def _fragen(base_url, fall_id):
    st, r = _req(base_url, "GET", f"/fall/{fall_id}/fragen")
    assert st == 200, (st, r)
    return r["fragen"]


def _antworten(base_url, fall_id, paare):
    for fld, wert in paare:
        st, r = _req(base_url, "POST", f"/fall/{fall_id}/event", _laie(fld, wert))
        assert st == 201, (fld, st, r)


def _ergebnis(base_url, fall_id):
    st, r = _req(base_url, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200, (st, r)
    return r


# Identischer Kegel wie im Vault-"Rotes Kommando" (einzel, ein Rentner, gesetzliche Rente
# 20.000 €/Jahr) -- Werte unveraendert aus der dortigen Messung uebernommen, nicht neu erfunden.
# `kein_p23_verkauf` ist NEU (Pflichtfeld seit diesem Fix) und steht mit True ("kein Verkauf") an
# derselben Stelle wie seine drei Geschwister -- ein ehrlicher Rentner ohne Verkauf traegt hier
# wahrheitsgemaess True ein, ohne ueber seine Rente nachdenken zu muessen.
KEGEL_OHNE_VERKAUF = [
    ("veranlagung", "einzel"), ("rentner_renten_art", "gesetzliche_rente"),
    ("rentner_jahresrente", 20000000), ("rentner_renten_beginn_jahr", 2025),
    ("rentner_alter_bei_rentenbeginn", 65), ("rentner_rentenfreibetrag", 0),
    ("rentner_grad_der_behinderung", 0), ("rentner_hilflos_blind_taubblind", False),
    ("rentner_hinterbliebenenbezuege", False), ("rentner_pflegegrad", 0),
    ("rentner_gepflegter_hilflos", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    (FLAG_ID, True),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
]
# Derselbe Kegel, aber FLAG_ID=False ("ich HATTE einen Verkauf") -- der Defektfall.
KEGEL_MIT_VERKAUF = [(f, (False if f == FLAG_ID else w)) for f, w in KEGEL_OHNE_VERKAUF]

# Grundstuecksverkauf: Erloes 100.000 €, Anschaffung 50.000 € -> Gewinn 50.000 € -- identisch
# zum Vault-"Rotes Kommando". Keine der vier Befreiungen (Frist/Eigennutzung/taeglicher
# Gebrauch/originaerer Erwerb) wird hier oder sonstwo im Dialog je erfragt.
P23_VERKAUF = [
    ("p23_veraeusserungs_typ", "grundstueck"),
    ("p23_veraeusserungspreis", 10000000),            # 100.000 EUR
    ("p23_anschaffung_herstellungskosten", 5000000),  # 50.000 EUR -> Gewinn 50.000 EUR
    ("p23_werbungskosten", 0),
]
# Dieselben drei Betragsfelder, aber AUSDRUECKLICH mit 0 beantwortet statt unbeantwortet zu
# bleiben -- Gegenprobe B unten. Kein p23_veraeusserungs_typ (Enum, keine Betragsschwelle, fuer
# die Nullblindheits-Frage ohne Belang).
P23_BETRAEGE_NULL = [
    ("p23_veraeusserungspreis", 0),
    ("p23_anschaffung_herstellungskosten", 0),
    ("p23_werbungskosten", 0),
]

BASELINE_ZAHL_CENT = 5917000   # unveraendert seit dem Vault-"Rotes Kommando", HEAD c745e59


def test_kreuz_erscheint_im_fragenlauf_beider_scheiben(base):
    # Erwartung VOR der Zaehlung ausgesprochen (main-Auflage): genau EIN Treffer je Scheibe,
    # auf dem allerersten /fragen-Aufruf eines frischen Falls (das Kreuz traegt kein
    # feld_bedingung, ist also unbedingt offen wie seine drei Geschwister kein_gewinn/kein_kap/
    # kein_vuv).
    _neuer_fall(base, "q_gesamt", "gesamt")
    fragen_gesamt = _fragen(base, "q_gesamt")
    treffer_gesamt = [f for f in fragen_gesamt if f["feld_id"] == FLAG_ID]
    assert len(treffer_gesamt) == 1, (
        f"Erwartet: {FLAG_ID!r} genau 1x in /fragen (Scheibe gesamt) vor jeder Antwort. "
        f"Tatsaechlich: {len(treffer_gesamt)} Treffer unter {len(fragen_gesamt)} offenen Fragen. "
        f"Fehlt es (0 Treffer), waere das Kreuz fuer den Nutzer nie beantwortbar -- fail-closed "
        f"sperrt ihn dann dauerhaft und ausweglos, sobald es in fremd_arten haengt.")

    _neuer_fall(base, "q_rentner", "rentner_gesamt")
    fragen_rentner = _fragen(base, "q_rentner")
    treffer_rentner = [f for f in fragen_rentner if f["feld_id"] == FLAG_ID]
    assert len(treffer_rentner) == 1, (
        f"Erwartet: {FLAG_ID!r} genau 1x in /fragen (Scheibe rentner_gesamt) vor jeder Antwort. "
        f"Tatsaechlich: {len(treffer_rentner)} Treffer unter {len(fragen_rentner)} offenen Fragen.")


def test_gegenprobe_unbeantwortet_und_ausdruecklich_null_kommen_gleich_durch(base):
    # Zustand (a): §23-Detailfelder bleiben unbeantwortet, nur das neue Kreuz wird beantwortet.
    _neuer_fall(base, "r0a_unbeantwortet", "rentner_gesamt")
    _antworten(base, "r0a_unbeantwortet", KEGEL_OHNE_VERKAUF)
    erg_a = _ergebnis(base, "r0a_unbeantwortet")
    assert erg_a["grund"] == "bestaetigt" and erg_a["zahl_cent"] == BASELINE_ZAHL_CENT, (
        f"Zustand 'unbeantwortet': erwartet grund=bestaetigt, zahl_cent={BASELINE_ZAHL_CENT}. "
        f"Tatsaechlich: grund={erg_a['grund']!r}, zahl_cent={erg_a['zahl_cent']!r}.")

    # Zustand (b): dieselben Felder AUSDRUECKLICH mit 0 beantwortet -- ein Veraeusserungsgewinn
    # von 0 € (Verkauf zum Einstandspreis) ist der Normalfall, kein Randfall, und muss identisch
    # durchkommen wie Zustand (a).
    _neuer_fall(base, "r0b_null", "rentner_gesamt")
    _antworten(base, "r0b_null", KEGEL_OHNE_VERKAUF + P23_BETRAEGE_NULL)
    erg_b = _ergebnis(base, "r0b_null")
    assert erg_b["grund"] == "bestaetigt" and erg_b["zahl_cent"] == BASELINE_ZAHL_CENT, (
        f"Zustand 'ausdruecklich 0': erwartet grund=bestaetigt, zahl_cent={BASELINE_ZAHL_CENT}. "
        f"Tatsaechlich: grund={erg_b['grund']!r}, zahl_cent={erg_b['zahl_cent']!r}. Wenn dieser "
        f"Fall sperrt, waehrend Zustand (a) durchkommt, behandelt die Pruefung 'unbeantwortet' "
        f"und 'ausdruecklich 0' unterschiedlich -- ein wahrheitsgemaess mit 0 antwortender "
        f"Rentner (Verkauf zum Einstandspreis) waere dann faelschlich gesperrt.")


@pytest.mark.xfail(strict=True, reason=(
    "fail-open auf dem Screening-Kreuz selbst (Instructor-Auftrag 2026-08-30, Anker-Variante fuer "
    "rentner_gesamt: P23_SCREENING aus RENTNER_KEGEL heraus, kegel-unabhaengig in RENTNER_FELDER "
    "verankert). Ein Rentner MIT §23-Verkauf, der nur das Kreuz kein_p23_verkauf nie beantwortet "
    "-- die vier Detailfelder aber wahrheitsgemaess mit Gewinn 50.000 EUR ausfuellt --, bekommt "
    "grund=bestaetigt, zahl_cent=8017000 statt einer Sperre. Live gemessen: derselbe Rentner mit "
    "wahrheitsgemaess False (Verkauf bejaht) sperrt korrekt mit einkunftsart_nicht_ring_faehig, "
    "der einzige Unterschied ist unbeantwortet statt bestaetigt. Gemessen (2026-08-31, HEAD "
    "ed0f460, in-process Mutationsprobe mit Rueckweg, kein Dateiedit): kein_p23_verkauf in beide "
    "Kegel zu haengen kippt genau diesen Test auf XPASS(strict) und laesst /fragen erreichbar "
    "(kein 500 in drei Phasen Baseline/Mutation/Rueckweg), reisst aber die volle Suite von 3 auf "
    "251 rote Tests, weil rund 20 Fremddateien den Pflichtkegel als eigene, nicht importierte "
    "Python-Liste ohne dieses Feld hartkodieren -- deshalb bleibt der Marker diese Runde stehen, "
    "waehrend statt des Kegels ein engerer Widerspruchs-Waechter geprueft wird, der nur bei "
    "gefuellten §23-Detailfeldern + nie beantwortetem Kreuz feuert."))
def test_kreuz_nie_beantwortet_bei_tatsaechlichem_verkauf_darf_nicht_bestaetigt_liefern(base):
    _neuer_fall(base, "b_nie_beantwortet", "rentner_gesamt")
    paare = [(f, w) for f, w in KEGEL_OHNE_VERKAUF if f != FLAG_ID] + P23_VERKAUF
    _antworten(base, "b_nie_beantwortet", paare)
    erg = _ergebnis(base, "b_nie_beantwortet")
    assert erg["grund"] != "bestaetigt", (
        f"Kreuz {FLAG_ID!r} nie beantwortet, aber die vier §23-Detailfelder bejahen einen Verkauf "
        f"(Gewinn 50.000 EUR) -- erwartet: KEINE Zahl (Sperre oder offen). Tatsaechlich: "
        f"grund={erg['grund']!r}, zahl_cent={erg['zahl_cent']!r}. Dieselbe Person mit "
        f"wahrheitsgemaess {FLAG_ID}=False sperrt korrekt -- unbeantwortet wird wie 'kein "
        f"Verkauf' behandelt, das ist fail-open auf dem Screening-Kreuz.")


def test_verkauf_bejaht_liefert_konkreten_sperrgrund_mit_klartext(base):
    _neuer_fall(base, "r1_mit_verkauf", "rentner_gesamt")
    _antworten(base, "r1_mit_verkauf", KEGEL_MIT_VERKAUF + P23_VERKAUF)
    mit_verkauf = _ergebnis(base, "r1_mit_verkauf")

    assert mit_verkauf["grund"] == ERWARTETER_SPERRGRUND, (
        f"§23-Verkauf ohne jede Befreiungs-Abfrage lieferte grund={mit_verkauf['grund']!r} "
        f"(erwartet der KONKRETE Sperrgrund {ERWARTETER_SPERRGRUND!r}, den die fremd_arten-"
        f"Mitgliedschaft von {FLAG_ID!r} produziert -- nicht bloss irgendeine Sperre). "
        f"zahl_cent={mit_verkauf['zahl_cent']!r}. Vault-Referenz: backlog/taxgraph/"
        f"p23-sonstige-kreuz-kopplung-rentner-reachable.md, dort identisch Δ=2.100.000 Cent "
        f"(21.000,00 €) bei HEAD c745e59 gemessen.")

    klartext_katalog = SPERRGRUND_KLARTEXT.get(ERWARTETER_SPERRGRUND)
    assert klartext_katalog, (
        f"Sperrgrund {ERWARTETER_SPERRGRUND!r} hat keinen oder einen leeren Klartext-Eintrag in "
        f"SPERRGRUND_KLARTEXT -- fuer den Nutzer waere die Sperre dann ein rohes Maschinenwort.")
    assert mit_verkauf.get("klartext") == klartext_katalog, (
        f"Der Katalog-Klartext erreicht den Browser nicht 1:1 im /ergebnis-Response. "
        f"Katalog={klartext_katalog!r} Response={mit_verkauf.get('klartext')!r}.")
    assert "verkauf" in klartext_katalog.lower() or "veräußer" in klartext_katalog.lower(), (
        f"Der Klartext zu {ERWARTETER_SPERRGRUND!r} nennt keinen privaten Verkauf -- ein Nutzer, "
        f"der ueber {FLAG_ID!r} hier landet, sieht nur die vier alten Gruende (Renten/sonstige "
        f"Einkuenfte, Vermietung, Kapitalertraege, Gewinn aus einem Betrieb) und seinen eigenen "
        f"nicht. Text: {klartext_katalog!r}")
