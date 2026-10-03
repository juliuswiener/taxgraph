#!/usr/bin/env python3
"""Erzeugt die E2E-Fixtures `rust/fixtures/e2e/<fall>.json` (Fall-Datei) und `<fall>.xml`.

WARUM ES DIESE DATEI GIBT (2026-10-01). `rust/bescheid/tests/einreichung_e2e.rs` faehrt eine
Fall-Datei durch `bescheid::deklaration::einreichungs_xml` und verlangt das ELSTER-XML, das
Python fuer denselben Fall baut — Byte fuer Byte. Beides entsteht hier, ueber den Nutzerpfad:
  * `korpus_faelle.lege_an` — `api.fall_anlegen` + `api.event`, der Pflicht-Kegel kommt aus
    `tests/_kegel.py::kegel_fuer`. Ableitungen (`_leite_ab`, `_rechne_ab`) laufen wie im Betrieb.
  * `api.einreichen` — der echte Endpunkt bis zum XML. Ein Ersatz fuer `checkest_gate.validate`
    greift das XML ab und wirft `RuntimeError`; `einreichen` antwortet 503 und bindet keinen
    Snapshot. Die ERiC-Pruefung gehoert nicht zur Fixture.

ZIEL IST EINE FRISCHE TEMP-WURZEL: das Skript setzt `$TAXGRAPH_DATEN` selbst, vor dem Import.
Die echte Fallliste (~/.local/share/taxgraph/faelle) bleibt unberuehrt, gleich was die Umgebung
traegt. Die Temp-Wurzel wird am Ende geloescht.

BYTE-GLEICH REPRODUZIERBAR: die Uhr des Stores (`store._now`) zaehlt je Fall ab einem festen
Zeitpunkt in Sekunden, damit `ts` und `event_id` bei jedem Lauf gleich ausfallen. Zwei Laeufe
hintereinander: `git diff -- rust/fixtures/e2e` bleibt leer.

Hersteller-ID: die gesperrte Test-ID 74931, nie die echte — die stuende sonst im Repo.

Aufruf (aus dem Repo-Wurzelverzeichnis):

    python3 tools/parity/e2e_faelle.py

SICHERHEIT: alle Werte sind ERFUNDEN (keine echten Steuerdaten). Ausgegeben werden nur fall_id,
Statuscode und Dateigroessen.
"""
from __future__ import annotations

import datetime
import itertools
import os
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
ZIEL = os.path.join(ROOT, "rust", "fixtures", "e2e")

WURZEL = tempfile.mkdtemp(prefix="e2e-faelle-")
os.environ["TAXGRAPH_DATEN"] = WURZEL      # VOR dem Import: `api_constants.FAELLE`

from korpus_faelle import VZ, lege_an     # noqa: E402  (setzt sys.path und TAXGRAPH_NO_AUTH)
import api as API                          # noqa: E402
import checkest_gate as CE                 # noqa: E402  (unter elster/, den Pfad setzt api)
import store as ST                         # noqa: E402

TEST_HERSTELLER_ID = "74931"
UHR_START = datetime.datetime(2026, 1, 15, 9, 0, tzinfo=datetime.timezone.utc)
ABGEGRIFFEN = "e2e_faelle: XML abgegriffen, keine ERiC-Pruefung"

# Alle Werte ERFUNDEN. Steuernummer: Praefix 9181 passt zum Empfaenger BY (sonst wirft der Writer).
STAMMDATEN = {
    "stammdaten_nachname": "Muster", "stammdaten_vorname": "Erika",
    "stammdaten_geburtsdatum": "12.03.1985",
    "stammdaten_strasse": "Musterstraße", "stammdaten_hausnummer": "7",
    "stammdaten_plz": "80331", "stammdaten_wohnort": "München",
    "stammdaten_keine_bankverbindung": True,
    "stammdaten_art_est_erklaerung": True,
    "stammdaten_steuernummer": "9181081508155",
    "veranlagung": "einzel",
    "kist_konfession": "keine",
}

# Angestellte, Anlage N, sonst keine Einkuenfte.
ARBEITNEHMER = dict(STAMMDATEN, **{
    "steuerklasse": "1",
    "bruttoarbeitslohn": 4_850_000,
    "p36_lohnsteuer": 720_000,
    "vor_an_anteil_rv": 451_050, "vor_ag_anteil_rv": 451_050, "vor_rv_ausserhalb_lstb": 0,
    "kein_gewinn": True, "kein_kap": True, "kein_vuv": True, "kein_sonstige": True,
})

# Rentner im Folgejahr (Beginn 2023, Freibetrag fixiert), Anlage R.
RENTNER = dict(STAMMDATEN, **{
    "stammdaten_geburtsdatum": "07.09.1957",
    "rentner_renten_art": "gesetzliche_rente",
    "rentner_jahresrente": 1_980_000,
    "rentner_renten_beginn_jahr": 2023,
    "rentner_alter_bei_rentenbeginn": 65,
    "rentner_rentenfreibetrag": 346_500,
    "rentner_grad_der_behinderung": 0,
    "rentner_hilflos_blind_taubblind": False,
    "rentner_hinterbliebenenbezuege": False,
    "rentner_pflegegrad": 0,
    "rentner_gepflegter_hilflos": False,
    # Renten sind sonstige Einkuenfte (§ 22): `kein_sonstige=True` sperrte mit flag_konsistenz_offen.
    "kein_gewinn": True, "kein_kap": True, "kein_vuv": True, "kein_sonstige": False,
})

# Die Angestellte plus drei Ring-Werte (`_mit_ring_werten`): Verpflegungskuerzung (Anlage N),
# Summenzeilen Anlage V, Handwerkerleistung aus zwei Instanzen. Die Bedingungsfelder daneben
# verlangt der K2-Guard (`_an_gesamt_sperrgrund`), sonst 409.
GESAMT = dict(ARBEITNEHMER, **{
    "tage_ueber_8h_eintaegig": 20,
    "vpf_monate_am_ort": 1,
    "vpf_mittagessen_gestellt_anzahl": 5,
    "kein_vuv": False,
    "vv_einnahmen": 960_000,
    "vv_nebenkosten_umgelegt": 180_000,
    "vv_gebaeude_afa": 300_000,
    "vv_schuldzinsen": 250_000,
    "vv_erhaltungsaufwand": 80_000,
    "vv_sonstige_wk": 40_000,
    "hh_handwerker_betrag": 60_000,
    "hh_handwerker_betrag__2": 45_000,
    "hh_rechnung_unbar": True,
    "hh_handwerker_keine_foerderung": True,
    "hh_in_eu_ewr": True,
})

# Die Angestellte mit Hausnummerzusatz: E0101207 neben E0101206, und der Zusatz in <AbsStr> ("Musterstraße 7a").
HAUSNUMMER_ZUSATZ = dict(ARBEITNEHMER, stammdaten_hausnummerzusatz="a")

FAELLE: list[tuple[str, str, dict]] = [
    ("arbeitnehmer", "gesamt", ARBEITNEHMER),
    ("rentner", "rentner_gesamt", RENTNER),
    ("gesamt", "gesamt", GESAMT),
    ("hausnummer_zusatz", "gesamt", HAUSNUMMER_ZUSATZ),
]


def _uhr_ab_start() -> None:
    """`store._now` zaehlt ab `UHR_START` in Sekunden: gleiche `ts`/`event_id` je Lauf."""
    takt = itertools.count()
    ST._now = lambda: (UHR_START + datetime.timedelta(seconds=next(takt))).isoformat()


def baue(name: str, scheibe: str, gesetzt: dict) -> None:
    fall_id = f"e2e-{name}"
    _uhr_ab_start()
    ergebnis = lege_an(fall_id, scheibe, gesetzt)
    if ergebnis != "neu":
        sys.exit(f"{fall_id}: {ergebnis}")
    with open(os.path.join(API.FAELLE, f"{fall_id}.json"), "rb") as f:
        fall_datei = f.read()
    ablage: list = []

    def abgreifen(xml: str, datenart: str):
        """Haelt das XML fest, das `einreichen` an ERiC gaebe."""
        ablage.append((xml, datenart))
        raise RuntimeError(ABGEGRIFFEN)
    CE.validate = abgreifen
    st, antwort = API.einreichen(fall_id, {})
    if (st, antwort.get("detail")) != (503, ABGEGRIFFEN) or len(ablage) != 1:
        sys.exit(f"{fall_id}: einreichen {st} {antwort.get('grund')} "
                 f"{antwort.get('unvollstaendig') or antwort.get('detail') or ''}")
    xml, datenart = ablage[0]
    if datenart != f"ESt_{VZ}":
        sys.exit(f"{fall_id}: Datenart {datenart}, erwartet ESt_{VZ}")
    for endung, inhalt in (("json", fall_datei), ("xml", xml.encode("utf-8"))):
        with open(os.path.join(ZIEL, f"{name}.{endung}"), "wb") as f:
            f.write(inhalt)
    print(f"  {fall_id:18s} {scheibe:15s} json={len(fall_datei)} xml={len(xml.encode())} Bytes")


def main() -> int:
    # Vor jedem Python-Tor: welcher Baum rechnet?
    print(f"api:   {API.__file__}\nstore: {ST.__file__}\nZiel:  {ZIEL}")
    os.environ["ELSTER_HERSTELLER_ID"] = TEST_HERSTELLER_ID
    os.makedirs(ZIEL, exist_ok=True)
    try:
        for name, scheibe, gesetzt in FAELLE:
            baue(name, scheibe, gesetzt)
    finally:
        shutil.rmtree(WURZEL)
    print(f"{len(FAELLE)} Faelle, VZ {VZ}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
