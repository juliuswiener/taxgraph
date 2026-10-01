#!/usr/bin/env python3
"""Erzeugt Parity-Korpus-Faelle fuer die Zweige, die der reale Bestand kaum traegt.

WARUM ES DIESE DATEI GIBT (2026-10-01). `reale_faelle` in
`rust/parity/tests/bescheid_zweige_paritaet.rs` faehrt jede Falldatei unter
`faelle_verzeichnis()` in 24 Varianten. Von 4608 Zeilen rechneten 2928 NICHT, sondern
brachen mit `KeyError` ab (`festzusetzende_est_gesamt`: 384x 'veranlagung', 312x
'arbeitstage', 42x 'oepnv_kosten_jahr'). Grund: 122 der 192 Bestandsdateien tragen keines
der vier `EP_FELDER`, 6 weitere drei von vier. Der `KeyError` ist ABSICHT
(`bescheid_zweige.py:213` liest `slots[k]`, nicht `.get`) — auf dem Nutzerpfad kann er
nicht auftreten, weil `_feste_zahl` (api.py) den vollen Kegel VOR dem Ring prueft und sonst
`None` liefert. Das Parity-Orakel ruft `_bescheid_fn` darunter auf und umgeht diese Pruefung.

Folge: die Zeile ist gruen (Python und Rust werfen dieselbe Klasse) und rechnet nie — genau
das Ticket `parity-lauf-gruen-ohne-dass-die-zeile-rechnet`.

GEBAUT WIRD UEBER DEN NUTZERPFAD, nicht am Modul vorbei:
  * `api.fall_anlegen`  — derselbe Endpunkt wie POST /fall
  * `api.event`         — DER einzige Schreib-Endpunkt (duenne Huelle ueber store.append_event);
                          Auflage T (Typ-Konformitaet) und Zwei-Signal greifen hier wie im Betrieb
  * `tests/_kegel.py::kegel_fuer` — der Kegel-Bauer des Repos, KEINE handgeschriebene Feldliste.
    Ein neues Kegel-Mitglied erscheint damit von selbst in jedem erzeugten Fall.

Damit traegt jeder Fall den vollen Pflicht-Kegel und stirbt nicht am KeyError.

ZIEL IST NUR `$TAXGRAPH_DATEN/faelle`, und die Variable ist Pflicht. Ohne sie fiele
`api.FAELLE` auf die XDG-Vorgabe zurueck (~/.local/share/taxgraph/faelle) — das ist die
Fallliste, die das Produkt dem Nutzer zeigt. Genau dort lagen am 2026-10-01 die sechs Faelle,
bis sie verschoben wurden. Eine vorhandene Datei wird NIE ueberschrieben — `fall_anlegen`
antwortet 409, dieses Skript meldet das und macht weiter.

Kein Audit-Eintrag: `fall_anlegen` protokolliert nur mit angemeldetem Nutzer, unter
`TAXGRAPH_NO_AUTH=1` gibt es keinen, und `event` protokolliert nicht (gemessen: 0 Zeilen).

Reproduzierbar im INHALT, nicht byte-gleich: `event_id` und `ts` entstehen beim Schreiben.
Zwei Laeufe ergeben dieselben Felder, Werte und Zustaende, aber andere sha256-Summen.

Aufruf (aus dem Repo-Wurzelverzeichnis):

    TAXGRAPH_DATEN=<eigenes Verzeichnis> python3 tools/parity/korpus_faelle.py
    TAXGRAPH_DATEN=<eigenes Verzeichnis> python3 tools/parity/korpus_faelle.py --probe

SICHERHEIT: erzeugte Faelle tragen ERFUNDENE Werte (keine echten Steuerdaten). Ausgegeben
werden nur fall_id, Scheibe, Statuscode und Zaehlwerte — keine Werte, keine Betraege.
"""
from __future__ import annotations

import argparse
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
for _sub in ("produkt/haut", "produkt/store", "produkt/mapping", "produkt/traverser",
             "produkt/engine", "tests"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

os.environ.setdefault("TAXGRAPH_NO_AUTH", "1")   # wie tests/conftest.py, sonst 401 auf /fall

# VOR dem Import: `api_constants.FAELLE` wird beim Import aus `$TAXGRAPH_DATEN` bestimmt.
# ponytail: prueft nur "gesetzt", nicht "zeigt woanders hin als die echte Fallliste" — ein
# ausdrueckliches TAXGRAPH_DATEN=~/.local/share/taxgraph geht durch. Ausbau: realpath gegen
# `api_constants._daten_wurzel()` ohne die Variable vergleichen.
if not os.environ.get("TAXGRAPH_DATEN", "").strip():
    sys.exit("TAXGRAPH_DATEN fehlt — ohne die Variable schriebe dieses Skript in die echte "
             "Fallliste (~/.local/share/taxgraph/faelle). Setz ein eigenes Verzeichnis.")

import api as API                    # noqa: E402
from _kegel import kegel_fuer        # noqa: E402

VZ = 2025

# ---- Der eine Laien-Schreibvorgang, wortgleich zu tests/test_ring_regression_kampagne.py ----

def _laie(feld_id: str, wert) -> dict:
    return {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{feld_id}"}}


def lege_an(fall_id: str, scheibe: str, gesetzt: dict) -> str:
    """Ein Fall ueber die echten Endpunkte. Gibt 'neu' | 'vorhanden' | 'ABWEISUNG: ...'."""
    try:
        st, _ = API.fall_anlegen({"scheibe": scheibe, "veranlagungszeitraum": VZ,
                                  "fall_id": fall_id})
    except API.ApiError as e:
        if e.status == 409:
            return "vorhanden"
        return f"ABWEISUNG: {e.status} {e.detail}"
    if st != 201:
        return f"ABWEISUNG: {st}"
    for feld_id, wert in kegel_fuer(scheibe, dict(gesetzt)):
        try:
            st, _ = API.event(fall_id, _laie(feld_id, wert))
        except API.ApiError as e:
            return f"ABWEISUNG bei {feld_id}: {e.status} {e.detail}"
        if st != 201:
            return f"ABWEISUNG bei {feld_id}: {st}"
    return "neu"


# ---- Die Faelle: je einer fuer einen Zweig, den der Bestand kaum traegt ----
#
# Alle Zahlen sind ERFUNDEN. Die Zweig-Felder sind bewusst so gewaehlt, dass sie die
# Vergleichszahl BEWEGEN (nachgemessen, s. Bericht), nicht nur im Snapshot stehen.

# Gemeinsamer Rumpf: Angestellter, kein Gewinn/Kapital/VuV, sonstige Einkuenfte moeglich.
RUMPF = {
    "veranlagung": "einzel",
    "bruttoarbeitslohn": 6_000_000,
    "kein_gewinn": True, "kein_kap": True, "kein_vuv": True, "kein_sonstige": True,
    "kist_konfession": "keine",
}

# § 23 — Veraeusserungsgewinn ueber der Freigrenze (5.000.000 - 3.000.000 - 100.000 = 1.900.000).
# `kein_p23_verkauf=False` ist die Bedingung, nicht die Zahl: ohne sie greift der Screening-Flag
# als "kein privater Verkauf" und der Zweig laeuft gar nicht erst.
P23 = dict(RUMPF, **{
    "kein_p23_verkauf": False,
    "kein_sonstige": False,
    "p23_veraeusserungspreis": 5_000_000,
    "p23_anschaffung_herstellungskosten": 3_000_000,
    "p23_werbungskosten": 100_000,
    "p23_veraeusserungs_typ": "grundstueck",
    "p23_anzahl_verkaeufe": 1,
})

# § 34c DBA-Anrechnung — Staat Frankreich (Methode Anrechnung, kein Kapital-Sachverhalt,
# sonst sperrt dba_kapital_offen). 3.000 EUR gezahlt auf 40.000 EUR auslaendische Einkuenfte.
DBA = dict(RUMPF, **{
    "dba_gezahlte_auslaendische_steuer": 300_000,
    "dba_auslaendische_einkuenfte": 4_000_000,
    "dba_staat": "Frankreich",
    "dba_einkunftsart": "zinsen",
    "dba_methode": "dba_anrechnung",
})

# § 31 Kinder — zwei Kinder mit KV/PV-Beitraegen. Die Betraege liegen AUSSERHALB des Kegels und
# bewegen die Zahl trotzdem in ALLEN sechs Parity-Varianten, auch bei werte="kegel": der Ring
# liest Snapshot und Store, die `_bescheid_fn` beim Bau bekommt, nicht nur das `werte`-Dict.
KINDER = dict(RUMPF, **{
    "kein_kind": False,
    "fam_anzahl_kinder": 2,
    "kind_idnr": "12345678901",
    "kind_kindschaftsverhaeltnis_a": "1",
    "kind_unter_14_haushaltszugehoerig": True,
    "kind_kv": 120_000,
    "kind_pv": 30_000,
})

# § 22 Nr. 1 S. 3 aa — ERSTJahr (Beginn == VZ) auf der Halb-Prozent-Kohorte 2025 (83,5 %).
# Nur der Erstjahres-Zweig laeuft durch `_renten_stpfl` (Prozent x Jahresrente); das Folgejahr
# rechnet mit dem fixierten EURO-Freibetrag und beruehrt die Kohortentabelle nicht.
RENTE_ERSTJAHR = {
    "veranlagung": "einzel",
    "rentner_renten_art": "gesetzliche_rente",
    "rentner_jahresrente": 2_400_000,
    "rentner_renten_beginn_jahr": 2025,
    "rentner_alter_bei_rentenbeginn": 65,
    "rentner_rentenfreibetrag": 0,
    "rentner_grad_der_behinderung": 0,
    "rentner_hilflos_blind_taubblind": False,
    "rentner_hinterbliebenenbezuege": False,
    "rentner_pflegegrad": 0,
    "rentner_gepflegter_hilflos": False,
    "kein_gewinn": True, "kein_kap": True, "kein_vuv": True, "kein_sonstige": True,
    "kist_konfession": "keine",
}

# Dasselbe im Rentner-Ring, damit `festzusetzende_est_rentner` den Zweig sieht.
RENTE_ERSTJAHR_DBA = dict(RENTE_ERSTJAHR, **{
    "dba_gezahlte_auslaendische_steuer": 300_000,
    "dba_auslaendische_einkuenfte": 1_000_000,
    "dba_staat": "Frankreich",
    "dba_einkunftsart": "zinsen",
    "dba_methode": "dba_anrechnung",
})

# § 22 aa FOLGEJAHR auf der Halb-Prozent-Kohorte 2023 (82,5 %) — Beginn < VZ, Freibetrag fixiert.
RENTE_FOLGEJAHR = dict(RENTE_ERSTJAHR, **{
    "rentner_renten_beginn_jahr": 2023,
    "rentner_rentenfreibetrag": 300_000,
})

FAELLE: list[tuple[str, str, dict]] = [
    ("parity-korpus-kind-gesamt", "gesamt", KINDER),
    ("parity-korpus-p23-gesamt", "gesamt", P23),
    ("parity-korpus-dba-gesamt", "gesamt", DBA),
    ("parity-korpus-rente-erstjahr", "rentner_gesamt", RENTE_ERSTJAHR),
    ("parity-korpus-rente-folgejahr", "rentner_gesamt", RENTE_FOLGEJAHR),
    ("parity-korpus-rente-dba", "rentner_gesamt", RENTE_ERSTJAHR_DBA),
]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--probe", action="store_true",
                    help="nur auflisten, was angelegt wuerde (kein Schreibvorgang)")
    args = ap.parse_args()

    print(f"Zielverzeichnis: {API.FAELLE}")
    if args.probe:
        for fall_id, scheibe, gesetzt in FAELLE:
            print(f"  wuerde anlegen: {fall_id:34s} scheibe={scheibe:16s} "
                  f"kegel={len(kegel_fuer(scheibe, dict(gesetzt)))} Felder")
        return 0

    neu = vorhanden = 0
    for fall_id, scheibe, gesetzt in FAELLE:
        ergebnis = lege_an(fall_id, scheibe, gesetzt)
        print(f"  {fall_id:34s} {scheibe:16s} {ergebnis}")
        if ergebnis == "neu":
            neu += 1
        elif ergebnis == "vorhanden":
            vorhanden += 1
    print(f"\n{neu} neu angelegt, {vorhanden} bereits vorhanden, {len(FAELLE)} gesamt")
    return 0


if __name__ == "__main__":
    sys.exit(main())
