"""Eine CSV-Zeile mit unlesbarem Betrag verschwindet still (Vault
tickets/kontoauszug-zeile-mit-unlesbarem-betrag-verschwindet-still.md, gemessen 2026-10-02).

`_eur_cent_signed` liefert für `abc` und `1,2,3` den Betrag 0; `uebernehme_kontoauszug` überspringt
jeden Betrag >= 0; der CSV-Zweig in `api.kontoauszug` zählt nichts nach `verworfen`. Die Antwort
nennt die Zeile nur in `transaktionen`, ein `hinweis` entsteht nicht. Die Oberfläche zeigt nur
`uebernommen`, `transaktionen` und `hinweis` (`app.js` `kontoauszugHochladen`) — der Nutzer sieht
also nicht, dass eine Ausgabe fehlt.

Verlangt ist nur, was in jeder Lösung gilt: beide Zeilen zählen in `verworfen`, und der `hinweis`
nennt den Betrag als Grund. Den Wortlaut legt der Test nicht fest.

xfail(strict=True, raises=AssertionError) wie tests/test_luf_gewinn_kz_fehlt.py. Die Kontrolle
daneben MUSS grün bleiben: sie belegt, dass der Upload selbst funktioniert und der rote Test den
Betrag misst, nicht die Route. Rust-Gegenstück: rust/api/tests/offene_defekte.rs
`kontoauszug_unlesbarer_betrag_steht_in_verworfen`."""
from __future__ import annotations

import pytest

from test_paket_b_e2e_http import _req, base  # noqa: F401 — Fixture und HTTP-Helfer der Suite

KOPF = "datum;betrag;verwendungszweck\n"
# Dieselbe Rürup-Zeile wie test_kontoauszug_csv_vorsorge_vorschlag: deterministisch → vor_rv_ausserhalb_lstb.
LESBAR = "15.03.2025;-1200,00;Ruerup-Rente Jahresbeitrag Basisrente\n"
UNLESBAR = ("16.03.2025;abc;Ruerup-Rente Nachzahlung Basisrente\n"
            "17.03.2025;1,2,3;Ruerup-Rente Sonderzahlung Basisrente\n")


def _hochladen(base, fall_id, csv):
    _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id})
    st, b = _req(base, "POST", f"/fall/{fall_id}/kontoauszug", {"format": "csv", "inhalt": csv})
    assert st == 200
    return b


def test_kontrolle_lesbare_zeile_wird_uebernommen(base):
    b = _hochladen(base, "ka-kontrolle", KOPF + LESBAR)
    assert (b["transaktionen"], b["uebernommen"], b["verworfen"]) == (1, 1, 0)


@pytest.mark.xfail(strict=True, raises=AssertionError,
                   reason="Ticket kontoauszug-zeile-mit-unlesbarem-betrag-verschwindet-still")
def test_unlesbarer_betrag_steht_in_verworfen_mit_grund(base):
    b = _hochladen(base, "ka-unlesbar", KOPF + LESBAR + UNLESBAR)
    assert b["uebernommen"] == 1, b
    assert b["verworfen"] == 2, f"abc und 1,2,3 fehlen in verworfen: {b}"
    assert "Betrag" in b.get("hinweis", ""), f"der Hinweis nennt den Betrag nicht als Grund: {b}"
