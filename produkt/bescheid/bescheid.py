"""Rechenkern des Steuerbescheids — Fassade über die vier Sachgebiets-Module.

api.py sagt seit dem ersten Tag: "Keine Steuerlogik, keine zweite Wahrheit hier." Am 2026-08-18
sind die 26 Funktionen, die dem widersprachen, aus der Endpunkt-Schicht hierher gezogen
(3579 → 1147 Zeilen dort). Am 2026-08-19 ist diese Datei selbst aufgeteilt worden: 2610 Zeilen
in einem Modul waren zwar richtig einsortiert, aber nicht mehr navigierbar.

DIE VIER TEILE, und warum der Schnitt dort liegt — gemessen, nicht nach Gefühl:

    bescheid_einkuenfte    10 Fn    Gewinn, Kapital, § 23, DBA, § 35
    bescheid_abzuege        8 Fn    Kinder, Sonderausgaben, agB, zwei geteilte Helfer
    bescheid_deklaration    2 Fn    Sperrgründe und Ring-Werte
    bescheid_zweige         6 Fn    Dispatcher und die Rechenkerne

Die Aufrufrichtung ist ZYKLENFREI: `zweige` und `deklaration` bauen auf `einkuenfte` und
`abzuege`, nie umgekehrt (22 Kreuz-Aufrufe, alle in eine Richtung). Deshalb reichen zwei
Import-Blöcke in den beiden oberen Modulen, und keines muss ein anderes zur Laufzeit nachladen.

WARUM DIESE FASSADE BLEIBT: `import bescheid` ist die Schnittstelle, die api.py und mehrere
Tests benutzen. Der Schnitt ist eine innere Angelegenheit dieses Pakets; ihn nach aussen
durchzureichen hiesse, 26 Import-Zeilen in api.py auf vier Module zu verteilen und jeden
künftigen Umzug dort nachzuziehen.

KEIN STAR-IMPORT: `from X import *` bindet Werte für alle Namen auf einmal und unsichtbar —
untersagt und geprüft in tests/test_split_naht_gate.py. Deshalb stehen die Namen hier einzeln.
"""
from __future__ import annotations

# Blatt-Module: bauen auf niemanden aus diesem Paket.
from bescheid_abzuege import (  # noqa: F401
    _abs3_eligible,
    _kind_behinderten_pb_daten,
    _kind_kv_pv_summe,
    _kinderbetreuung_summe,
    _oepnv_eur,
    _p33b_kind_pauschbetraege,
    _schulgeld_summe,
    _shared_steuer_sonder_agb,
)
from bescheid_einkuenfte import (  # noqa: F401
    _gewinn_partner_anteil,
    _gwg_sofortabzug_summe,
    _laufender_gewinn,
    _laufender_gewinn_partner,
    _p20_kapitaleinkuenfte,
    _p23_ansonsten_einkuenfte,
    _p35_gezahlte_gewst,
    _p35_partner_anteile,
    _p35_summen,
    _shared_dba_sonstige,
)

# Bauen auf den beiden darüber auf.
from bescheid_deklaration import (  # noqa: F401
    _an_gesamt_sperrgrund,
    _mit_ring_werten,
    # Gehört zu _an_gesamt_sperrgrund wie die Übersetzung zum Wort: der Sperrgrund ist ein
    # Maschinenwort, und wer ihn liefert, muss auch den Satz dazu liefern können.
    sperrgrund_klartext,
)
from bescheid_zweige import (  # noqa: F401
    _abschlusszahlung_cent,
    _bescheid_fn,
    _zweig_abziehbarer_betrag,
    _zweig_festzusetzende_est,
    _zweig_festzusetzende_est_gesamt,
    _zweig_festzusetzende_est_rentner,
)

__all__ = [
    # Die Naht: was der Rest von api.py aus dem Kern ruft (tests/test_bescheid_grenze.py).
    "_bescheid_fn", "_an_gesamt_sperrgrund", "sperrgrund_klartext", "_abschlusszahlung_cent",
    "_mit_ring_werten",
    # Alles Übrige, damit `import bescheid` weiterhin dasselbe hergibt wie vor dem Schnitt.
    "_abs3_eligible", "_gewinn_partner_anteil", "_gwg_sofortabzug_summe",
    "_kind_behinderten_pb_daten", "_kind_kv_pv_summe", "_kinderbetreuung_summe",
    "_laufender_gewinn", "_laufender_gewinn_partner", "_oepnv_eur", "_p20_kapitaleinkuenfte",
    "_p23_ansonsten_einkuenfte", "_p33b_kind_pauschbetraege", "_p35_gezahlte_gewst",
    "_p35_partner_anteile", "_p35_summen", "_schulgeld_summe", "_shared_dba_sonstige",
    "_shared_steuer_sonder_agb", "_zweig_abziehbarer_betrag", "_zweig_festzusetzende_est",
    "_zweig_festzusetzende_est_gesamt", "_zweig_festzusetzende_est_rentner",
]
