"""Eine CSV-Zeile mit unlesbarem Betrag verschwand still (Vault
backlog/taxgraph/kontoauszug-zeile-mit-unlesbarem-betrag-verschwindet-still.md, gemessen 2026-10-02).

Vorher lieferte `_eur_cent_signed` für `abc` und `1,2,3` den Betrag 0; `uebernehme_kontoauszug`
übersprang jeden Betrag >= 0; der CSV-Zweig in `api.kontoauszug` zählte nichts nach `verworfen`.
Die Oberfläche zeigt nur `uebernommen`, `transaktionen` und `hinweis` (`app.js`
`kontoauszugHochladen`) — der Nutzer sah also nicht, dass eine Ausgabe fehlt.

Verlangt ist nur, was in jeder Lösung gilt: beide Zeilen zählen in `verworfen`, und der `hinweis`
nennt den Betrag als Grund. Den Wortlaut legt der Test nicht fest.

Die Kontrolle daneben MUSS grün bleiben: sie belegt, dass der Upload selbst funktioniert und der
Test den Betrag misst, nicht die Route. Rust-Gegenstück: rust/api/tests/offene_defekte.rs
`kontoauszug_unlesbarer_betrag_steht_in_verworfen` (ignoriert, Route 501) und die Regel selbst in
rust/eingang/src/kontoauszug.rs `tests::csv_unlesbarer_betrag_zaehlt_in_verworfen`."""
from __future__ import annotations

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


def test_unlesbarer_betrag_steht_in_verworfen_mit_grund(base):
    b = _hochladen(base, "ka-unlesbar", KOPF + LESBAR + UNLESBAR)
    assert b["uebernommen"] == 1, b
    assert b["verworfen"] == 2, f"abc und 1,2,3 fehlen in verworfen: {b}"
    assert "Betrag" in b.get("hinweis", ""), f"der Hinweis nennt den Betrag nicht als Grund: {b}"


def test_betrag_mit_4301_ziffern_steht_in_verworfen(base):
    """int() liest höchstens 4300 Ziffern; vorher warf der Parser ValueError (gemessen: 500)."""
    b = _hochladen(base, "ka-lang", KOPF + LESBAR + f"18.03.2025;{'1' * 4301};Ruerup-Rente Basisrente\n")
    assert (b["uebernommen"], b["verworfen"]) == (1, 1), b
