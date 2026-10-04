#!/usr/bin/env python3
"""Erzeugt rust/fixtures/eingang_orakel.json: Eingaben samt Antworten des PYTHON-Orakels
(`tools/parity/schritt8_oracle.py` ueber `produkt/eingang/*`). Der Rust-Test
`rust/eingang/tests/orakel_werte.rs` spielt dieselben Eingaben gegen die Crate `eingang` und
vergleicht -- hermetisch, ohne Python zur Laufzeit. Die Eingaben sind der feste Anteil dessen,
was `rust/parity/tests/eingang_paritaet.rs` (PARITY=1) zufaellig zieht, dazu systematische Reihen
an den Entscheidungsstellen (Schluesselwoerter, Betragsgrenzen, Rundung, Ziffernzahl).

Abschnitte (Schluessel -> Liste von Faellen mit `py` = Antwort des Orakels):
  csv, csv_gross, csv_roh, csv_dict, cent, pdf_zeilen, tsv, konto (CSV-Auszuege), konto_json,
  konto_stichwort, verwirf, buchungsfelder, buchungsfelder_daten, hinweis, ocr, ocr_konstanten, vast,
  vorjahr, edaten, beleg.

`ocr`: je Fall drei Shell-Skripte (pdftotext, pdftoppm, tesseract; nur Builtins), die Python hier auf
PATH legt und `rust/eingang/tests/ocr_pfade.rs` ebenso; `py` hat die Antwort beider Leser.

Neu erzeugen:   python3 tools/parity/extract_eingang_orakel.py
"""
from __future__ import annotations

import csv
import io
import itertools
import json
import os
import random
import sys
import tempfile
from decimal import Decimal, localcontext

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(ROOT, "tools", "parity"))
sys.path.insert(0, ROOT)

import schritt8_oracle as O  # noqa: E402

OUT = os.environ.get("EO_OUT_JSON") or os.path.join(ROOT, "rust", "fixtures", "eingang_orakel.json")
TS = "2026-09-29T00:00:00+00:00"
SEED = 20261004
H = O.HANDLER
M = O._module()
BINDUNG = M["bindung"]
KW, BW, VW = M["KW"], M["BW"], M["VW"]


class R:
    """Wie die Zufallsquelle der Paritaetstests: n(bis), wahl(liste), p(prozent)."""

    def __init__(self, seed: int):
        self.r = random.Random(seed)

    def n(self, bis: int) -> int:
        return self.r.randrange(max(bis, 1))

    def wahl(self, v):
        return v[self.n(len(v))]

    def p(self, prozent: int) -> bool:
        return self.n(100) < prozent


ZWECKE = [
    "Malermeister Huber Rechnung 12", "SANITÄR Müller", "Spende Rotes Kreuz", "Tierheim e.V.", "Miete Oktober",
    "Gebäudereinigung GmbH", "Minijob-Zentrale", "Rürup-Rente Beitrag", "Amazon",
    "Unicef IBAN DE89370400440532013000", "Stadtwerke Kto 12345678901", "Heizung Wartung", "Gartenpflege Schmidt",
    "", "Supermarkt", "Hilfswerk", "Saldo alt", "e. V. Verein", "Winterdienst", 'Klempner Notdienst "24h"',
    "Zahlung; Rest", "StNr 181/815/08155",
]
BETRAEGE = [
    "-480,00", "-1.234,56", "1.234,56", "-12.5", "abc", "", "  -5,00 € ", "+-3", "--7", "-1e3", "inf", "nan",
    "-0,005", "-0,015", "-99", "12", "-1_000,5", "-1.000.000,00", "-0,00", "−5,00", "-١٢,٠٠", "-2,675",
    "-𝟏𝟐,𝟓", "-١_٠٠٠,٥", "-1.234", "1e3", "1,2,3", "480,5", "480", "480.00", "-1200,00 €",
    "92233720368547758,07", "92233720368547758,08",
    # Grenzen von _eur_cent_signed: Nachkommastellen, Tausendergruppen, Vorzeichen, Leerraum
    "-0,1", "-0,10", "-0,100", "-1,234", "-.5", "-5.", "5.5", "-5,5", "-1.234,5", "-12.345,67", "-1234.56",
    "-1.2.3,4", "-12.3456", "+5,00", "- 5,00", "-5,00 EUR", "-5,00 eur", "5 €", "€ 5", "-5,00€",
    # deutsch mit Tausendergruppe: hoechstens zwei Nachkommastellen, sonst unlesbar
    "-1.234,567", "1.234,5678", "-12.345,678", "-1.234,000", "1.234,5", "1.234,50", "-123.456.789,01",
]


def csv_zelle(r: R, s: str) -> str:
    if ";" in s or '"' in s or r.p(15):
        return '"' + s.replace('"', '""') + '"'
    return s


def csv_auszug(r: R) -> str:
    koepfe = [
        ["Buchungstag", "Betrag", "Verwendungszweck"], ["datum", "betrag", "zweck"], [" DATE ", "Amount", "Description"],
        ["Buchungsdatum", "Umsatz", "Buchungstext", "Saldo"], ["Datum", "Betrag (EUR)", "Beschreibung"], ["x", "y"],
        ["Betrag", "betrag", "Zweck"], ["Datum", "Verwendungszweck"],
    ]
    kopf = r.wahl(koepfe)
    ende = r.wahl(["\n", "\r\n", "\n", "\n"])
    out = ";".join(csv_zelle(r, k) for k in kopf)
    for _ in range(r.n(8)):
        out += ende
        if r.p(8):
            continue
        zellen = ["01.03.2025", r.wahl(BETRAEGE), r.wahl(ZWECKE)]
        if r.p(10):
            zellen.pop()
        if r.p(10):
            zellen.append("extra")
        out += ";".join(csv_zelle(r, z) for z in zellen)
    if r.p(3):
        out += "a\rb"
    if r.p(3):
        out += '\n"offen;quote'
    if r.p(50):
        out += ende
    return out


def json_auszug(r: R):
    werte = [-48000, "-500", -12.7, True, "1,5", None, -5, " -7 ", [1], 0, "0", False, 0.0, -0.5, 5]
    liste = []
    for _ in range(r.n(6)):
        if r.p(3):
            liste.append("kein objekt")
            continue
        o = {}
        if r.p(90):
            o["betrag"] = r.wahl(werte)
        if r.p(90):
            o["verwendungszweck"] = r.wahl(ZWECKE) if r.p(70) else r.wahl([5, None, [], {}, 0, False, ["x"], 1.5, ""])
        if r.p(70):
            o["datum"] = "2025-03-01" if r.p(80) else 20250301
        liste.append(o)
    return liste


# Handgemachte CSV-Texte an den Zustaenden der Zeilenmaschine (`eingang::csv`, CPython `_csv.c`).
CSV_ECKEN = [
    "", "\n", "\r\n", "\r", "Datum;Betrag;Zweck", "Datum;Betrag;Zweck\n", "\nDatum;Betrag;Zweck\n01.03.2025;-1,00;Maler",
    "Datum;Betrag;Zweck\n\n01.03.2025;-1,00;Maler\n\n", "Datum;Betrag;Zweck\r\r01.03.2025;-1,00;Maler",
    "Datum;Betrag;Zweck\r\n\r\n01.03.2025;-1,00;Maler\r\n", "Datum;Betrag;Zweck\n\r01.03.2025;-1,00;Maler",
    'Datum;Betrag;Zweck\n01.03.2025;-1,00;"Mal;er"', 'Datum;Betrag;Zweck\n01.03.2025;-1,00;"Mal""er"',
    'Datum;Betrag;Zweck\n01.03.2025;-1,00;"Maler"x', 'Datum;Betrag;Zweck\n01.03.2025;-1,00;"Maler"\n',
    'Datum;Betrag;Zweck\n01.03.2025;-1,00;"Maler\nzwei Zeilen"', 'Datum;Betrag;Zweck\n01.03.2025;-1,00;"Maler\r\nzwei"\n',
    'Datum;Betrag;Zweck\n01.03.2025;-1,00;"offen', 'Datum;Betrag;Zweck\n01.03.2025;-1,00;"',
    'Datum;Betrag;Zweck\n"', 'Datum;Betrag;Zweck\n"\n', 'Datum;Betrag;Zweck\n01.03.2025;-1,00;Ma"ler',
    'Datum;Betrag;Zweck\n01.03.2025;-1,00;""', 'Datum;Betrag;Zweck\n01.03.2025;-1,00;""x', 'Datum;Betrag;Zweck\n;;',
    "Datum;Betrag;Zweck\n01.03.2025;-1,00;Maler;;;", "Datum;Betrag;Zweck\n01.03.2025", "Datum;Betrag;Zweck\n01.03.2025;",
    "Datum;Betrag;Zweck\n01.03.2025;-1,00;Ma\rler", "Datum;Betrag;Zweck\n01.03.2025;-1,00;Ma\x00ler",
    "Datum;Betrag;Zweck\n01.03.2025;-1,00;Maler\r", "Datum;Betrag;Zweck\n01.03.2025;-1,00;Maler\n\r",
    "﻿Datum;Betrag;Zweck\n01.03.2025;-1,00;Maler", "Datum;Betrag;Zweck\n 01.03.2025 ; -1,00 ; Maler ",
    "Datum;Betrag;Zweck\n01.03.2025;;Maler\n01.03.2025;  ;Maler\n01.03.2025;abc;Maler",
    "Datum;;Zweck\n01.03.2025;-1,00;Maler", "Datum;Betrag\n01.03.2025;-1,00", "Betrag;Zweck\n-1,00;Maler",
    "Datum;Betrag;Zweck;Datum\n01.03.2025;-1,00;Maler;02.03.2025",
]

# Grosse Felder an der Feldgrenze (`csv.field_size_limit` = 131072): (Praefix, Fuellzeichen, n, Suffix).
# Im Fixture nur als Spezifikation, die Antwort gekuerzt (Zeichenketten ueber 100 Zeichen -> Laenge + Kopf).
GROSS = [
    ("", "x", 131071, ";Betrag\n1;2"), ("", "x", 131072, ";Betrag\n1;2"), ("", "x", 131073, ";Betrag\n1;2"),
    ("Datum;Betrag;Zweck\n01.03.2025;-1,00;", "y", 131071, ""), ("Datum;Betrag;Zweck\n01.03.2025;-1,00;", "y", 131072, ""),
    ("Datum;Betrag;Zweck\n01.03.2025;-1,00;", "y", 131073, ""),
    ('Datum;Betrag;Zweck\n01.03.2025;-1,00;"', "y", 131071, '"'), ('Datum;Betrag;Zweck\n01.03.2025;-1,00;"', "y", 131072, '"'),
    ('Datum;Betrag;Zweck\n01.03.2025;-1,00;"', "y", 131073, '"'),
    ("Datum;Betrag;Zweck\n01.03.2025;-1,00;", "y", 131070, ";z\n"), ("Datum;Betrag;Zweck\n", "y", 131073, ";-1,00;Maler"),
    ("Datum;Betrag;Zweck\n", "\n", 3, "01.03.2025;-1,00;Maler"),
]


def kurz(v):
    """Lange Zeichenketten durch Laenge und Kopf ersetzen (nur die Gross-Faelle)."""
    if isinstance(v, str) and len(v) > 100:
        return {"len": len(v), "kopf": v[:20]}
    if isinstance(v, (list, tuple)):
        return [kurz(x) for x in v]
    if isinstance(v, dict):
        return {k: kurz(x) for k, x in v.items()}
    return v


# Schluesselwoerter je Kategorie aus der Python-Tabelle selbst; je Wort klein, GROSS und mit Umgebung.
STICHWORTE = [w for _kat, ws in KW._KEYWORDS for w in ws]


def konto(tx, llm=None):
    return H["eingang.konto"]({"tx": tx, "ts": TS, "llm": llm})


def tx(zweck, betrag=-48000, datum="01.03.2025"):
    return {"datum": datum, "betrag": betrag, "verwendungszweck": zweck}


def vast_zahlen() -> list:
    """Betragstexte der VaSt: Ziffernzahl, Rundung, Exponent, Unterstrich, Vorzeichen."""
    z = ["45000.00", "1,5", "-0.005", "0,015", "1e2", " 7 ", "", "abc", "NaN", "Infinity", "-inf", "inf", "1_000.5",
         "0.125", "12345678901234567890123456.785", "١٢٣٫٤", "٤٥٠٠٠.٠٠", "1E+2", "1e+2", ".5", "5.", "1..2", "+3", "--3",
         "sNaN", "0e999999", "1e-999999", "0", "0.00", "00.00", "-0", "+0", "0.004", "0.005", "0.006", "0.015", "0.025",
         "0.035", "0.0050", "0.00500001", "0.995", "0.994", "1.005", "1.015", "99.995", "99.994", "-99.995", "-0.015",
         "1_000", "1__2", "1_", "_1", "1_0.5_0", "1e1_0", "1.5e", "e5", "-", "+", ".", "1e", "1e+", "1e-", "1.e2", ".e2",
         "1.5E2", "1.5e-2", "15e-1", "150e-2", "1e0", "1e1", "1e2", "1e16", "1e17", "1e18", "1e19", "1e20", "1e21",
         "1 000", "1,000", "1.000,00", "1,000.00", "1,5,5", "5,", ",5", "9" * 17, "9" * 18, "9" * 19, "9" * 20,
         "9" * 17 + ".99", "9" * 18 + ".99", "9" * 19 + ".99", "1" + "0" * 16, "1" + "0" * 17, "1" + "0" * 18,
         "1" + "0" * 19, "92233720368547758.07", "92233720368547758.08", "92233720368547758.065", "92233720368547758.075",
         "-92233720368547758.07", "-92233720368547758.08", "-92233720368547758.075", "922337203685477580.7",
         "9223372036854775.807", "9223372036854775.8075", "0.0000000000000000000000000001", "1" * 27 + ".5", "1" * 28 + ".5",
         "1" * 29, "1" * 30, "0." + "1" * 27, "0." + "1" * 28, "0." + "1" * 29, "0." + "0" * 26 + "5",
         "0." + "0" * 27 + "5", "0." + "0" * 28 + "5", "0.5" + "0" * 30 + "1", "0.5" + "0" * 30,
         "9" * 26 + ".995", "9" * 27 + ".995", "9" * 28 + ".995", "0.00" + "9" * 25, "0.00" + "9" * 26, "0.00" + "9" * 27,
         "0.0049999999999999999999999999", "0.0050000000000000000000000001", "1e-2", "1e-3", "5e-3", "6e-3", "15e-3",
         "1e-4", "0.0e5", "00000000000000000000000000000001", "1" + "0" * 40 + "e-38", "1" + "0" * 40 + "e-39"]
    return z


# ---- PDF/OCR mit Stub-Programmen: pdftotext/pdftoppm/tesseract sind kleine Shell-Skripte (nur Builtins) in
# einem Verzeichnis, auf das PATH zeigt; dieselben Skripte laufen unter Python und unter Rust.
TSV_KOPF = "level\\tpage_num\\tblock_num\\tpar_num\\tline_num\\tword_num\\tleft\\ttop\\twidth\\theight\\tconf\\ttext\\n"
# pdftoppm: legt `<Praefix>-<Seite>.png` ab (Seite = Wert nach -f, sonst 1)
PPM_EINZEL = 'f=1; while [ $# -gt 0 ]; do case $1 in -f) f=$2;; esac; p=$1; shift; done; : > "$p-$f.png"'


def ppm_viele(n: int) -> str:
    """pdftoppm ohne -f/-l: n Seiten `<Praefix>-001.png` ..."""
    return ('for a; do p=$a; done; i=1; while [ $i -le %d ]; do : > "$p-$(printf %%03d $i).png"; i=$((i+1)); done' % n)


# tesseract: je Bild zwei Zeilen: Bildname (Konfidenz 96) und `zwei` (41)
TESS_ZWEI = ("printf '" + TSV_KOPF + "5\\t1\\t1\\t1\\t1\\t1\\t0\\t0\\t1\\t1\\t96\\t%s\\n5\\t1\\t1\\t1\\t2\\t1\\t0\\t0\\t1\\t1\\t41\\tzwei\\n' \"${1##*/}\"")
TESS_LEER = "printf '" + TSV_KOPF + "'"
# tesseract mit Konfidenz-Sonderfaellen: min je Zeile, 0 zaehlt, unlesbar (`abc`) und -1 zaehlen nicht,
# Zeile nur aus unlesbaren Werten = 0.0, Leerwort zaehlt mit Konfidenz mit, fremde level fallen weg,
# Gruppen in Reihenfolge des ersten Auftretens (Block 2 vor Block 1/Absatz 2)
# (level, block, absatz, zeile, text, conf)
_ZEILEN = [
    ("4", 1, 1, 1, "XX", "99"),
    ("5", 1, 1, 1, "Alpha", "90"), ("5", 1, 1, 1, "Beta", "55"),
    ("5", 1, 1, 2, "Null", "0"), ("5", 1, 1, 2, "Gamma", "80"),
    ("5", 1, 1, 3, "Kaputt", "abc"), ("5", 1, 1, 3, "Delta", "70"),
    ("5", 1, 1, 4, "Nur", "-1"),
    ("5", 1, 1, 5, " ", "30"), ("5", 1, 1, 5, "Eps", "99"),
    ("5", 2, 1, 1, "Zeta", "64.5"),
    ("5", 1, 2, 1, "Eta", "100"), ("5", 1, 2, 1, "", "40"),
    ("5", 1, 1, 1, "Theta", "20"),
]
TESS_KONF = ("printf '" + TSV_KOPF + "".join(
    f"{lv}\\t1\\t{b}\\t{par}\\t{z}\\t1\\t0\\t0\\t1\\t1\\t{conf}\\t{text}\\n" for lv, b, par, z, text, conf in _ZEILEN) + "'")
# tesseract, das seine Umgebung und den Bildnamen meldet (OMP_THREAD_LIMIT nur fuer tesseract)
TESS_OMP = ("printf '" + TSV_KOPF + "5\\t1\\t1\\t1\\t1\\t1\\t0\\t0\\t1\\t1\\t90\\t%s\\n5\\t1\\t1\\t1\\t2\\t1\\t0\\t0\\t1\\t1\\t90\\t%s\\n' "
            "\"${1##*/}\" \"tess-omp=${OMP_THREAD_LIMIT:-leer}\"")
# pdftoppm legt neben dem Bild eine Nicht-PNG-Datei ab, die zuerst sortiert
PPM_MIT_LOG = ('for a; do p=$a; done; : > "$p-000.log"; : > "$p-001.png"; : > "$p-002.png"')
# pdftoppm meldet OMP_THREAD_LIMIT im Dateinamen
PPM_OMP = 'f=1; while [ $# -gt 0 ]; do case $1 in -f) f=$2;; esac; p=$1; shift; done; : > "$p-omp${OMP_THREAD_LIMIT:-leer}-$f.png"'
# pdftotext, das OMP_THREAD_LIMIT als Seitentext meldet (plausibel: laenger als 20 Zeichen)
PDFTOTEXT_OMP = "printf 'pdftotext sieht OMP=%s\\n\\f' \"${OMP_THREAD_LIMIT:-leer}\""
PDFTOTEXT_OMP_KURZ = "printf 'x\\f'"


def pdftotext_seiten(seiten: list) -> str:
    """pdftotext, das die Seiten mit Seitenvorschub getrennt ausgibt (auch nach der letzten)."""
    def esc(s: str) -> str:
        return (s.replace("\\", "\\\\").replace("%", "%%").replace("'", "'\\''")
                .replace("\n", "\\n").replace("\x07", "\\007"))
    return "printf '" + "".join(esc(s) + "\\f" for s in seiten) + "'"


def ocr_lauf(stubs: list, pdf: str, beleg_datei=None) -> dict:
    """Beide Leser unter Python mit den Skripten `stubs` (pdftotext, pdftoppm, tesseract) auf PATH."""
    with tempfile.TemporaryDirectory() as td:
        for name, rumpf in zip(("pdftotext", "pdftoppm", "tesseract"), stubs):
            f = os.path.join(td, name)
            with open(f, "w") as fh:
                fh.write("#!/bin/sh\n" + rumpf + "\n")
            os.chmod(f, 0o755)
        alt = os.environ["PATH"]
        alt_omp = os.environ.get("OMP_THREAD_LIMIT")
        os.environ["PATH"] = td
        os.environ["OMP_THREAD_LIMIT"] = "7"     # tesseract setzt 1, alle anderen erben 7
        try:
            pfad = pdf
            if beleg_datei is not None:
                pfad = os.path.join(td, beleg_datei[0])
                with open(pfad, "w", encoding="utf-8", newline="") as fh:
                    fh.write(beleg_datei[1])
            konto = O._fang(lambda p: list(KW.lies_kontoauszug_pdf(p)), pfad) if beleg_datei is None else None
            beleg = O._fang(lambda p: list(BW.lies_beleg_text(p)), pfad)
        finally:
            os.environ["PATH"] = alt
            if alt_omp is None:
                os.environ.pop("OMP_THREAD_LIMIT", None)
            else:
                os.environ["OMP_THREAD_LIMIT"] = alt_omp
    return {"konto": konto, "beleg": beleg}


def csv_roh(text: str, trenner: str = ";"):
    """`csv.reader` bis zum ersten Fehler: (Datensaetze davor, Fehlertext oder None)."""
    rows, err = [], None
    try:
        for row in csv.reader(io.StringIO(text), delimiter=trenner):
            rows.append(row)
    except csv.Error as e:
        err = str(e)
    return rows, err


def csv_dict(text: str, trenner: str = ";"):
    """`csv.DictReader` wie `eingang::csv::dict_reader`: (Kopf, Zeilen ohne Ueberzaehliges, Fehler)."""
    rd = csv.DictReader(io.StringIO(text), delimiter=trenner)
    try:
        kopf = list(rd.fieldnames or [])
    except csv.Error as e:
        return [], [], str(e)
    rows, err = [], None
    try:
        for row in rd:
            rows.append({k: v for k, v in row.items() if k is not None})
    except csv.Error as e:
        err = str(e)
    return kopf, rows, err


def praez27_vs_28(s: str) -> bool:
    """Ergibt `_cent` mit 27 statt 28 Stellen Praezision ein anderes Ergebnis?"""
    def mit(prec: int):
        with localcontext() as c:
            c.prec = prec
            return int((Decimal(s) * 100).to_integral_value())
    return mit(27) != mit(28)


def vast_reihen() -> list:
    """Betragstexte an den Rundungs- und Ziffernschranken von `eingang::vast::cent`."""
    z = []
    for k in range(24, 32):
        for d in ("", "4", "5", "6", "50", "51"):
            z += ["0." + "9" * k + d, "9" * k + "." + d if d else "9" * k, "0." + "0" * 3 + "9" * k + d]
    # Uebertrag beim Runden auf 28 Stellen (alle Neunen), Ergebnis noch im Bereich
    z += ["0." + "9" * 28 + "5", "0." + "9" * 28 + "4", "0." + "9" * 28 + "6", "0.0" + "9" * 28 + "5", "0." + "9" * 27 + "5"]
    # Hat 27 statt 28 Stellen Praezision einen Einfluss? Doppelrundung: n,4999...95 (28 signifikante
    # Stellen) rundet bei 27 Stellen auf n,5 und von dort half-even auf die gerade Nachbarzahl.
    gefunden = []
    for n in (1, 3, 5, 7, 9):
        for k in (24, 25, 26):
            t = f"0.0{n}4" + "9" * k + "5"
            gefunden.append(t)
    assert any(praez27_vs_28(t) for t in gefunden), "kein Fall, der die Praezision 27/28 trennt"
    z += gefunden + ["0.0050000000000000000000000000001", "0.00499999999999999999999999995", "0.004999999999999999999999999951"]
    # 19/20/21 Stellen: Ziffernzahl plus Exponent
    for n in (17, 18, 19, 20, 21, 22):
        z += ["1" * n, "1" * n + ".5", "1" * n + "e-2", "1" * (n - 1) + "e1", "1e" + str(n - 2), "1." + "0" * 5 + "e" + str(n - 1)]
    z += ["1" + "0" * 17 + "." + "0" * 3, "92233720368547758.07", "92233720368547757.99", "92233720368547758.065", "9223372036854775.807"]
    return z


def main() -> None:
    r = R(SEED)
    out: dict = {"ts": TS, "seed": SEED}

    # ---- csv / parse_csv
    korpus = H["korpus"]({"dateien": ["tests/test_kontoauszug_writer.py", "tests/test_llm_deckel_und_wiederholung.py",
                                      "tests/test_kontoauszug_maskierung.py"]})
    erzeugt = [csv_auszug(r) for _ in range(250)]
    csvs = []
    for c in list(CSV_ECKEN) + list(korpus) + erzeugt:
        if c not in csvs:  # doppelte raus, Reihenfolge stabil
            csvs.append(c)
    py = H["eingang.csv"]({"texte": csvs})
    out["csv"] = [{"text": t, "py": p} for t, p in zip(csvs, py)]
    py_von = dict(zip(csvs, py))
    # Zeilenmaschine direkt: alle Folgen bis Laenge 5 ueber {a ; " CR LF}, dazu Handfaelle.
    alphabet = ["a", ";", '"', "\r", "\n"]
    roh_texte = ["".join(t) for n in range(0, 6) for t in itertools.product(alphabet, repeat=n)]
    roh_texte += [c for c in CSV_ECKEN if len(c) < 400] + [t for t in csvs if len(t) < 400 and t not in CSV_ECKEN][:60]
    roh_texte = list(dict.fromkeys(roh_texte))
    out["csv_roh"] = [[t, *csv_roh(t)] for t in roh_texte]
    out["csv_roh_tab"] = [[t, *csv_roh(t, "\t")] for t in ["a\tb\n", 'a\t"b\tc"\n', "\ta\n", "a\t\tb", 'a"b\tc', "a\rb"]]
    dict_texte = ["", "\n", "a;b\n1;2\n", "a;b\n1\n", "a;b\n1;2;3\n", "a;b\n\n1;2\n\n3;4\n", "a;b\r\n\r\n1;2\r\n", "a;a\n1;2\n",
                  "a;b;c\n1\n", "a;b\n;\n", "a;b\n1;\n", "a;b\n\"x;y\";2\n", "a;b\n1;2\n3\n4;5;6;7\n", "a;b\n1;2",
                  'a;b\n"offen', "a\rb;c\n1;2\n", "a;b\n1\r2;3\n", ";\n1;2\n", "a;b\n \n1;2\n", "x\n\n\n"]
    out["csv_dict"] = [[t, *csv_dict(t)] for t in dict_texte]
    out["csv_gross"] = []
    for pre, z, n, suf in GROSS:
        text = pre + z * n + suf
        out["csv_gross"].append({"praefix": pre, "fuell": z, "n": n, "suffix": suf,
                                 "py": kurz(H["eingang.csv"]({"texte": [text]})[0])})

    # ---- Uebernahme aus CSV (jeder dritte mit LLM-Rueckfall)
    llm_texte = ['{"kategorie": "spende"}', '{"kategorie": "handwerker"}', "unklar", '{"kategorie": "miete"}', "",
                 'x {"kategorie": "vorsorge"}']
    out["konto"] = []
    for i, t in enumerate(erzeugt):
        res = py_von[t]
        if "ok" not in res:
            continue
        txs = [{"datum": d["datum"], "betrag": d["betrag"], "verwendungszweck": d["verwendungszweck"]}
               for d in res["ok"][0]]
        llm = [r.wahl(llm_texte) for _ in range(r.n(6))] if i % 3 == 0 else None
        out["konto"].append({"tx": txs, "llm": llm, "py": konto(txs, llm)})

    # ---- Uebernahme aus JSON
    out["konto_json"] = []
    for _ in range(150):
        roh = json_auszug(r)
        out["konto_json"].append({"roh": roh, "py": konto(roh, None)})

    # ---- Schluesselwoerter: je Wort klein, GROSS, eingebettet; Betrag negativ und 0/positiv; LLM-Deckel
    out["konto_stichwort"] = []
    for w in STICHWORTE:
        for zweck in (w, w.upper(), f"Zahlung {w} Rechnung 7", f"X{w.title()}Y"):
            out["konto_stichwort"].append({"tx": [tx(zweck)], "llm": None, "py": konto([tx(zweck)], None)})
    for betrag in (0, 1, -1, 48000):
        out["konto_stichwort"].append({"tx": [tx("Maler Huber", betrag)], "llm": None,
                                       "py": konto([tx("Maler Huber", betrag)], None)})
    # Mehrere Buchungen derselben Kategorie (die erste gewinnt), gemischte Kategorien, LLM-Rueckfall am Deckel.
    gemischt = [tx("Maler"), tx("Spende"), tx("Maler 2"), tx("Rentenversicherung"), tx("Reinigung"), tx("Minijob"),
                tx("Supermarkt", -5), tx("Gehalt", 100000)]
    out["konto_stichwort"].append({"tx": gemischt, "llm": None, "py": konto(gemischt, None)})
    kategorien = [(kat, ws) for kat, ws in KW._KEYWORDS]
    for (k1, w1), (k2, w2) in itertools.combinations(kategorien, 2):
        for a, b in ((w1[0], w2[0]), (w2[-1], w1[-1])):
            for zweck in (f"{a} {b}", f"{b} {a}"):
                out["konto_stichwort"].append({"tx": [tx(zweck)], "llm": None, "py": konto([tx(zweck)], None)})
    unklar = [tx(f"Posten {i}", -100 - i) for i in range(56)]
    for n_llm in (0, 1, 49, 50, 51, 56):
        llm = ['{"kategorie": "spende"}'] * n_llm
        out["konto_stichwort"].append({"tx": unklar, "llm": llm, "py": konto(unklar, llm)})
    for llm in (['{"kategorie": "handwerker"}'], ['{"kategorie": "dienstleistung"}'], ['{"kategorie": "minijob"}'],
                ['{"kategorie": "spende"}'], ['{"kategorie": "vorsorge"}'], ['{"kategorie": "miete"}'], ["unklar"], [""]):
        t = [tx("Posten Quelle unbekannt")]
        out["konto_stichwort"].append({"tx": t, "llm": llm, "py": konto(t, llm)})

    # ---- Direkte Funktionen des Kontoauszugs: verwirf_unlesbare_betraege, pruefe_buchungsfelder, hinweis
    def fang(f, *args):
        try:
            return {"ok": f(*args)}
        except Exception as e:  # noqa: BLE001
            return {"err": type(e).__name__, "msg": str(e)}
    betraege_roh = [-500, "5", 10 ** 10, -(10 ** 10), 10 ** 10 - 1, -(10 ** 10) + 1, None, "x", [1], True, 1.9, "1_5", " 7 ", "",
                    0, "-0", 99999999999, "9999999999", "10000000000", "-9999999999", {"a": 1}, 1e10, 9999999999.0]
    out["verwirf"] = []
    for n0 in (0, 3):
        liste = [{"betrag": b, "verwendungszweck": f"z{i}"} for i, b in enumerate(betraege_roh)] + [{}, {"verwendungszweck": "ohne"}]
        out["verwirf"].append({"liste": liste, "n": n0, "py": fang(KW.verwirf_unlesbare_betraege, liste, n0)})
        for b in betraege_roh:
            e = [{"betrag": b}]
            out["verwirf"].append({"liste": e, "n": n0, "py": fang(KW.verwirf_unlesbare_betraege, e, n0)})
    out["verwirf"].append({"liste": [], "n": 4, "py": fang(KW.verwirf_unlesbare_betraege, [], 4)})
    out["verwirf"].append({"liste": ["kein objekt", 5, None], "n": 0, "py": fang(KW.verwirf_unlesbare_betraege, ["kein objekt", 5, None], 0)})
    zw = ["Maler", "", 0, None, False, 5, 1.5, [1], ["x"], {}, {"a": 1}, True, "0"]
    out["buchungsfelder"] = []
    for betrag in (-5, 0, 5, "-5", "0", -0.5, None, True, False):
        for z in zw:
            liste = [{"datum": "01.03.2025", "betrag": betrag, "verwendungszweck": z}]
            out["buchungsfelder"].append({"liste": liste, "py": fang(KW.pruefe_buchungsfelder, liste)})
    # Daten mit nicht haltbaren Zahlen: Rust bekommt dafuer die Kennzeichnung `#float`/`#int` (Marke `#`).
    nan, inf, gross = float("nan"), float("inf"), 2 ** 70
    daten = [("01.03.2025", "01.03.2025"), (nan, "#float"), (inf, "#float"), (-inf, "#float"), (gross, "#int"), (-gross, "#int"),
             ([nan], ["#float"]), ({"a": gross}, {"a": "#int"}), ([gross, nan], ["#int", "#float"]), ([[gross], {"k": [nan]}], [["#int"], {"k": ["#float"]}]),
             (2 ** 63, "#int"), (2 ** 63 - 1, 2 ** 63 - 1), (-(2 ** 63), -(2 ** 63)), (-(2 ** 63) - 1, "#int"), (1.5, 1.5), (None, None), ("text", "text")]
    out["buchungsfelder_daten"] = []
    for py_datum, rs_datum in daten:
        py_l = [{"datum": py_datum, "betrag": -5, "verwendungszweck": "Maler"}]
        rs_l = [{"datum": rs_datum, "betrag": -5, "verwendungszweck": "Maler"}]
        out["buchungsfelder_daten"].append({"rust": rs_l, "py": fang(KW.pruefe_buchungsfelder, py_l)})
    # Reihenfolge: erst alle Daten, dann alle Zwecke; Zahl, die die Akte nicht haelt, im Zweck
    py_l = [{"datum": "d", "betrag": -5, "verwendungszweck": 7}, {"datum": nan, "betrag": -5, "verwendungszweck": "x"}]
    rs_l = [{"datum": "d", "betrag": -5, "verwendungszweck": 7}, {"datum": "#float", "betrag": -5, "verwendungszweck": "x"}]
    out["buchungsfelder_daten"].append({"rust": rs_l, "py": fang(KW.pruefe_buchungsfelder, py_l)})
    py_l = [{"datum": "d", "betrag": -5, "verwendungszweck": nan}]
    rs_l = [{"datum": "d", "betrag": -5, "verwendungszweck": "#float"}]
    out["buchungsfelder_daten"].append({"rust": rs_l, "py": fang(KW.pruefe_buchungsfelder, py_l)})
    py_l = [{"datum": "d", "betrag": -5, "verwendungszweck": gross}]
    rs_l = [{"datum": "d", "betrag": -5, "verwendungszweck": "#int"}]
    out["buchungsfelder_daten"].append({"rust": rs_l, "py": fang(KW.pruefe_buchungsfelder, py_l)})
    out["hinweis"] = [{"n": n, "fmt": f, "py": KW.hinweis_verworfen(n, f)} for n in (0, 1, 2, 100) for f in ("pdf", "csv", "json", "", "PDF")]

    # ---- PDF/OCR (Stub-Programme)
    lang = "Zeile mit genug Text fuer eine Seite"
    seite = lambda n: f"{lang} {n}"      # noqa: E731
    out["ocr"] = []
    out["ocr_konstanten"] = {
        n: [m.PDFTOTEXT_ZEITLIMIT_S, m.PDFTOPPM_ZEITLIMIT_S, m.TESSERACT_ZEITLIMIT_S, m.OCR_SEITEN_HOECHSTZAHL]
        for n, m in (("kontoauszug", KW), ("beleg", BW))}

    def fall(name, stubs, pdf="x.pdf", beleg_datei=None):
        e = ocr_lauf(stubs, pdf, beleg_datei)
        out["ocr"].append({"name": name, "stubs": stubs, "datei": beleg_datei, "py": e})

    fall("voll 3 Seiten", ["exit 0", ppm_viele(3), TESS_ZWEI])
    fall("voll 40 Seiten", ["exit 0", ppm_viele(40), TESS_ZWEI])
    fall("voll 41 Seiten", ["exit 0", ppm_viele(41), TESS_ZWEI])
    fall("voll 0 Seiten", ["exit 0", ppm_viele(0), TESS_ZWEI])
    fall("voll weisse Seiten", ["exit 0", ppm_viele(2), TESS_LEER])
    fall("pdftotext Exit 3 ohne Text", ["exit 3", ppm_viele(2), TESS_ZWEI])
    fall("pdftotext Exit 3 mit Text", [pdftotext_seiten([seite(1)]) + "; exit 3", ppm_viele(2), TESS_ZWEI])
    for code in (1, 2, 4, 99, 255):
        fall(f"pdftotext Exit {code}", [pdftotext_seiten([seite(1)]) + f"; exit {code}", ppm_viele(2), TESS_ZWEI])
    fall("pdftotext Signal", ["kill -9 $$", ppm_viele(2), TESS_ZWEI])
    fall("Textlayer plausibel", [pdftotext_seiten([seite(1), seite(2), seite(3)]), ppm_viele(1), TESS_ZWEI])
    fall("Textlayer ohne Seitenvorschub", ["printf 'Zeile mit genug Text fuer eine Seite'", PPM_EINZEL, TESS_ZWEI])
    fall("Seite 2 implausibel", [pdftotext_seiten([seite(1), "x", seite(3)]), PPM_EINZEL, TESS_ZWEI])
    fall("Seite 1 und 3 implausibel", [pdftotext_seiten(["x", seite(2), "  y  "]), PPM_EINZEL, TESS_ZWEI])
    fall("alle Seiten implausibel", [pdftotext_seiten(["a", "b", "c"]), PPM_EINZEL, TESS_ZWEI])
    fall("Seiten mit Zeilenumbruechen", [pdftotext_seiten([seite(1) + "\n\nzweite Zeile\n\n", "x", seite(3) + "\n"]), PPM_EINZEL, TESS_ZWEI])
    fall("Seiten mit mehr Zeilen", [pdftotext_seiten(["a\nb\nc", seite(2) + "\nzweite\ndritte\n", "z"]), PPM_EINZEL, TESS_ZWEI])
    for n in (19, 20, 21):
        fall(f"Seite mit {n} Zeichen", [pdftotext_seiten(["a" * n, seite(2)]), PPM_EINZEL, TESS_ZWEI])
        fall(f"Seite mit {n} Zeichen und Leerraum", [pdftotext_seiten(["  " + "a" * n + " \n"]), PPM_EINZEL, TESS_ZWEI])
    fall("Seite mit 20 Zeichen nach strip (innen Leerzeichen)", [pdftotext_seiten(["a" + " " * 18 + "b"]), PPM_EINZEL, TESS_ZWEI])
    for n_schlecht, n_gut in ((40, 0), (41, 0), (40, 3), (41, 4), (39, 1), (45, 0)):
        seiten = ["x"] * n_schlecht + [seite(i) for i in range(n_gut)]
        fall(f"{n_schlecht} implausibel, {n_gut} plausibel", [pdftotext_seiten(seiten), PPM_EINZEL, TESS_ZWEI])
    fall("BEL im Textlayer", [pdftotext_seiten(["Datum\x07Betrag und noch mehr Text dazu"]), PPM_EINZEL, TESS_ZWEI])
    fall("nur BEL", ["printf '\\007\\007'", ppm_viele(1), TESS_ZWEI])
    fall("BEL und Leerraum", ["printf ' \\007 \\n'", ppm_viele(1), TESS_ZWEI])
    fall("pdftoppm Exit 1", ["exit 0", "exit 1", TESS_ZWEI])
    fall("tesseract Exit 1", ["exit 0", ppm_viele(2), "exit 1"])
    fall("tesseract Exit 1 (Einzelseite)", [pdftotext_seiten(["x"]), PPM_EINZEL, "exit 1"])
    fall("kein Bild (Einzelseite)", [pdftotext_seiten(["x"]), "exit 0", TESS_ZWEI])
    fall("tesseract kaputtes TSV", ["exit 0", ppm_viele(1), "printf 'a\\n\"b\\n'"])
    for name, inhalt in (("beleg.txt", "Zeile eins\r\nZeile zwei\rZeile drei\nEnde"), ("beleg.TXT", "A\r\nB"), ("beleg.Txt", "A\rB"),
                         ("beleg.txt", ""), ("beleg.txt", "\r"), ("beleg.txt", "\r\r\n\n"), ("beleg.txt.pdf", "A\r\nB"),
                         ("beleg.pdf", "A\r\nB")):
        fall(f"Beleg-Datei {name!r} {inhalt!r}", ["exit 0", PPM_EINZEL, "printf 'tesseract:%s\\n' \"$1\"" if False else TESS_ZWEI], None, (name, inhalt))
    fall("Voll-Scan mit Konfidenz-Sonderfaellen", ["exit 0", ppm_viele(1), TESS_KONF])
    fall("Voll-Scan: zwei Seiten mit Konfidenz-Sonderfaellen", ["exit 0", ppm_viele(2), TESS_KONF])
    fall("Einzelseite mit Konfidenz-Sonderfaellen", [pdftotext_seiten(["x"]), PPM_EINZEL, TESS_KONF])
    fall("gemischt: Versatz bei mehrzeiligen Seiten", [pdftotext_seiten(["kurz", seite(2) + "\nzweite\n\n\n", "kurz", seite(4) + "\n", "kurz"]), PPM_EINZEL, TESS_KONF])
    fall("gemischt: Seite 1 plausibel mit Leerzeilen am Ende", [pdftotext_seiten([seite(1) + "\n\n\n", "kurz"]), PPM_EINZEL, TESS_KONF])
    fall("gemischt: Seite 10 per OCR", [pdftotext_seiten([seite(i) for i in range(1, 10)] + ["kurz"]), PPM_EINZEL, TESS_ZWEI])
    fall("pdftoppm legt Nicht-PNG-Datei ab", ["exit 0", PPM_MIT_LOG, TESS_ZWEI])
    fall("OMP_THREAD_LIMIT: pdftotext sieht den geerbten Wert", [PDFTOTEXT_OMP, PPM_OMP, TESS_OMP])
    fall("OMP_THREAD_LIMIT: pdftoppm im Voll-Scan, tesseract", ["exit 0", PPM_OMP, TESS_OMP])
    fall("OMP_THREAD_LIMIT: pdftoppm je Einzelseite, tesseract", [PDFTOTEXT_OMP_KURZ, PPM_OMP, TESS_OMP])
    fall("Beleg ohne Textlayer: tesseract-Text", ["exit 0", PPM_EINZEL, "printf 'Zeile eins\\r\\nZeile zwei\\rdrei\\n'"])
    fall("Beleg ohne Textlayer: nur Zeilenende", ["exit 0", PPM_EINZEL, "printf '\\r\\n\\r'"])

    # ---- Betrag-Parser
    betraege = list(BETRAEGE)
    betraege += [f"{r.wahl(['', '-', '+', ' -'])}{r.n(100000)}{r.wahl([',00', '.5', ',5', ',055', '', '.000,99', 'e2', '_5'])}"
                 for _ in range(150)]
    out["cent"] = [{"wert": w, "py": p} for w, p in zip(betraege, H["eingang.cent"]({"werte": betraege}))]

    # ---- PDF-Zeilen, tsv
    zeilen = [
        "01.03.2025 Maler Huber -480,00 EUR", "01.03.2025 Saldo -1,00", "Tagessaldo 1.234,56",
        "02.03.2025 Miete -800,00 -1.200,00", "03.03.2025 Spende 50,00", "04.03.2025 ohne Betrag", "Zwischensumme -5,00",
        "05.03.2025\tKlempner  -120,00 €", "1.3.2025 x -1,00", "06.03.2025 Sollzinsen 3,50% -45,00", "  ",
        "07.03.2025 Rechnungssumme -99,99 eur", "08.03.2025 X -9,99\x0c09.03.2025 Y -1,00",
        "09.03.2025 KONTOSTAND -3,00", "09.03.2025 Zwischensumme alt -3,00", "09.03.2025 Saldovortrag -3,00",
        "09.03.2025 Saldo-Anpassung -3,00", "09.03.2025 Zwischensummen -3,00", "09.03.2025 Buchung +12,00 -7,50",
        "09.03.2025 Buchung +12,00", "09.03.2025 Buchung 1,5 -2,00", "09.03.2025 -12.345,67", "10.03.25 Kurzjahr -5,00",
        "11.03.2025 Euro -5,00 €", "11.03.2025 Euro -5,00 EUR  ", "11.03.2025 Euro -5,00 eur\t", "11.03.2025 Wert 9,99-",
        "Saldo -5,00 12,00", "Saldo 12,00 -5,00", "Zwischensumme 1,00 -2,00 3,00", "Saldo 1,00 2,00", "Kontostand",
        "12.03.2025 Kontostand neu 5,00", "13.03.2025 Buchung -5,00 EUR EUR", "14.03.2025 Euro€ -5,00", "15.03.2025 \u20ac -5,00 \u20ac",
        "16.03.2025 A  B\t\tC -5,00   EUR  ", "20.03.2025 Gross -92.233.720.368.547.758,08", "21.03.2025 Gross -999.999.999.999.999.999.999,99",
        "22.03.2025 Max -92.233.720.368.547.758,07", "23.03.2025 Gross 92.233.720.368.547.758,08 Zweck", "17.03.2025 -1.234,56", "18.03.2025 +1.234,56 Gutschrift", "19.03.2025 a -5,00 b",
    ]
    out["pdf_zeilen"] = []
    for _ in range(300):
        text = r.wahl(["\n", "\r\n", "\x0b", " "]).join(r.wahl(zeilen) for _ in range(r.n(10)))
        conf = {str(r.n(10)): r.n(100) / 100.0 for _ in range(r.n(4))}
        out["pdf_zeilen"].append({"text": text, "conf": conf})
    # Schwellen: Confidence genau 0.6, knapp darunter und darueber, Zeile ohne Eintrag.
    for c in (0.0, 0.59, 0.6, 0.61, 1.0):
        out["pdf_zeilen"].append({"text": "01.03.2025 Maler -480,00\n02.03.2025 Spende -50,00", "conf": {"0": c, "1": 1.0 - c}})
    out["pdf_zeilen"].append({"text": "", "conf": {}})
    for f in out["pdf_zeilen"]:
        f["py"] = H["eingang.pdf_zeilen"]({"faelle": [f]})[0]

    tsvs = []
    for _ in range(150):
        t = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n"
        for _ in range(r.n(12)):
            t += "{}\t1\t{}\t1\t{}\t1\t0\t0\t1\t1\t{}\t{}\n".format(
                r.wahl(["5", "4", "5"]), r.n(2), r.n(3), r.wahl(["96.5", "-1", "80", "x", "59.9", "0", "100", "55.55"]),
                r.wahl(["01.03.2025", "Maler", "-480,00", " ", '"q', "EUR", "", "a b"]))
        tsvs.append(t)
    tsvs += ["", "level\ttext\n", "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n"]
    out["tsv"] = [{"text": t, "py": p} for t, p in zip(tsvs, H["eingang.tsv"]({"texte": tsvs}))]

    # ---- VaSt
    zahlen = list(dict.fromkeys(vast_zahlen() + vast_reihen()))
    zahlen_n = zahlen + [None]
    lstb_namen = ["BruttoArbLohn", "LSteuer", "ArbnKiSteuer", "ArbnAnteilArblVers", "ArbnAnteilKrankVers", "Soli",
                  "ArbnAnteilPflegVers"]
    kurzwerte = ["45000.00", "1,5", "-0.005", "0,015", "7", "", "abc", "Infinity", "0.125", "0", "-3.00", "12345678901234567890123456.785",
            None, "9.99", "100", "1e2"]
    lstb = [{r.wahl(lstb_namen): r.wahl(kurzwerte) or "" for _ in range(r.n(7))} for _ in range(200)]
    lstb += [{n: "100.00"} for n in lstb_namen] + [{"ArbnAnteilArblVers": "5.00", "LSteuer": "8200.00"}, {}]
    lstb += [{"ArbnAnteilArblVers": a, "ArbnAnteilKrankVers": b} for a, b in
             [("1.00", "2.00"), ("", "2.00"), ("1.00", ""), ("", ""), ("-1.00", "1.00"), ("0", "0")]]
    lersl = [[{"Betrag": r.wahl(kurzwerte), "Art": r.wahl(["ALG", " Krankengeld ", "", "ALG", None, "Zuschuss"])}
              for _ in range(r.n(4))] for _ in range(200)]
    halb = "50000000000000000.00"
    lersl += [[{"Betrag": "1.00", "Art": "A"}, {"Betrag": "-1.00", "Art": "B"}], [{"Betrag": "1.00", "Art": "B"}, {"Betrag": "1.00", "Art": "A"},
              {"Betrag": "1.00", "Art": "B"}], [{"Betrag": halb, "Art": "ALG"}] * 2, [{"Betrag": halb, "Art": "ALG"}] * 2 +
              [{"Betrag": "-" + halb, "Art": "ALG"}], [{"Betrag": halb, "Art": "ALG"}, {"Betrag": "-" + halb, "Art": "ALG"},
              {"Betrag": "1.00", "Art": "ALG"}], [{"Betrag": "-5.00", "Art": "ALG"}], [{"Betrag": "0.00", "Art": "ALG"}], [],
              [{"Betrag": "5.00"}], [{"Betrag": "5.00", "Art": " "}], [{"Betrag": None, "Art": "ALG"}, {"Betrag": "3.00", "Art": "ALG"}]]
    py = H["eingang.vast"]({"werte": zahlen_n, "lstb": lstb, "lersl": lersl})
    out["vast"] = {"werte": zahlen_n, "lstb": lstb, "lersl": lersl, "py": py}

    # ---- Vorjahr
    flags = sorted(VW.uebertragbare_felder(BINDUNG))
    vorjahr_werte = [4_000_000, "40000", True, 1.5, None, 0, "verheiratet", -5, "Maier\u0000", "01.01-31.122"]
    out["vorjahr"] = []
    for _ in range(120):
        felder = {}
        for _ in range(r.n(10)):
            fid = r.wahl(flags) if r.p(90) else "verlustvortrag_bestand"
            felder[fid] = {"wert": r.wahl(vorjahr_werte), "zustand": r.wahl(["bestaetigt", "vorlaeufig", "bestaetigt"])}
        vorbelegt = sorted({r.wahl(flags) for _ in range(r.n(3))})
        req = {"felder": felder, "vz": 2025, "ts": TS, "vorbelegt": vorbelegt}
        out["vorjahr"].append({"req": req, "py": H["eingang.vorjahr"](req)})
    # Wert ab der Betragsgrenze (Magnitude): keine der fuenf Pruef-Abweisungen, die Uebernahme bricht ab.
    for fid in [f for f in flags if BINDUNG[f].get("typ") == "cent"][:8]:
        if True:
            for w in (10 ** 10, 10 ** 10 - 1):
                req = {"felder": {fid: {"wert": w, "zustand": "bestaetigt"}}, "vz": 2025, "ts": TS, "vorbelegt": []}
                out["vorjahr"].append({"req": req, "py": H["eingang.vorjahr"](req)})
    # Referenzwert Verlustvortrag: bestaetigt, vorlaeufig, fehlt.
    for z in ("bestaetigt", "vorlaeufig", None):
        felder = {"verlustvortrag_bestand": {"wert": 5000, **({"zustand": z} if z else {})}}
        req = {"felder": felder, "vz": 2025, "ts": TS, "vorbelegt": []}
        out["vorjahr"].append({"req": req, "py": H["eingang.vorjahr"](req)})

    # ---- eDaten
    felder = sorted(BINDUNG)
    out["edaten"] = []
    for _ in range(150):
        saetze = [{"feld_id": r.wahl(felder) if r.p(85) else "unbekannt_xyz",
                   "wert": r.wahl([123_456, "123", True, 2, "ja"]), "kategorie": r.wahl(["LStB/LSteuer", None])}
                  for _ in range(r.n(6))]
        req = {"saetze": saetze, "ts": TS, "mit_bindung": r.p(50)}
        out["edaten"].append({"req": req, "py": H["eingang.edaten"](req)})
    # Eigene Angaben haben Vorrang: vorbelegte Felder werden nicht ueberschrieben (Python-Orakel mit Vorbelegung).
    def edaten_vorbelegt(req):
        ST = M["ST"]
        store = ST.leerer_store(2025)
        for fid in req["vorbelegt"]:
            ST.append_event(store, feld_id=fid, wert=1, zustand="vorlaeufig",
                            herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                            schreiber="ui:laie", signal={"signal_1": None, "signal_2": None}, ts=req["ts"])
        r = O._fang(M["EW"].uebernehme_edaten, store, req["saetze"], ts=req["ts"],
                    bindung=BINDUNG if req.get("mit_bindung") else None)
        return {"r": r, "events": store["events"]}
    for _ in range(60):
        saetze = [{"feld_id": r.wahl(felder), "wert": r.wahl([123_456, "123", 2]), "kategorie": "LStB/LSteuer"}
                  for _ in range(1 + r.n(4))]
        vorbelegt = sorted({s_["feld_id"] for s_ in saetze if r.p(50)})
        req = {"saetze": saetze, "ts": TS, "mit_bindung": r.p(50), "vorbelegt": vorbelegt}
        out["edaten"].append({"req": req, "py": edaten_vorbelegt(req)})
    # Ein Feld, das schon aktiv ist (eigene Angabe), und ein doppelter Satz im selben Aufruf.
    for fid in felder[:1]:
        req = {"saetze": [{"feld_id": fid, "wert": 5, "kategorie": None}] * 2, "ts": TS, "mit_bindung": True}
        out["edaten"].append({"req": req, "py": H["eingang.edaten"](req)})
        req = {"saetze": [{"feld_id": fid, "wert": 5, "kategorie": "x"}], "ts": TS, "mit_bindung": False}
        out["edaten"].append({"req": req, "py": H["eingang.edaten"](req)})

    # ---- Beleg
    texte = []
    fixd = os.path.join(ROOT, "tests", "fixtures")
    for n in sorted(os.listdir(fixd)):
        if n.endswith(".txt"):
            texte.append(open(os.path.join(fixd, n), encoding="utf-8").read())
    anker = []
    for typ in BW.BELEG_TYPEN:
        for modus, wert in BW.beleg_felder(BINDUNG, typ).values():
            anker.append(f"Nr. {wert}" if modus == "nr" else wert)
    koepfe = ["Lohnsteuerbescheinigung 2025", "Zuwendungsbestätigung", "Rechnung Handwerker", "Haushaltsnahe Dienstleistung",
              "Minijob Haushaltsscheck", "Handwerker Dienstleistung", "Rechnung", "GELDZUWENDUNG", "Haushaltsscheck",
              "Dienstleistung", "haushaltsnah", "HANDWERKER", "Lohnsteuerbescheinigung Zuwendungsbestätigung"]
    for k in koepfe:
        texte.append(k)
    for _ in range(150):
        t = r.wahl(koepfe)
        for _ in range(r.n(10)):
            a = r.wahl(anker)
            a = a.upper() if r.p(20) else a
            t += "\n{}{} {}".format(r.wahl(["", "  ", "3. "]), a,
                                     r.wahl(["45.000,00", "1.234,56 EUR", "12,00 3,50", "ohne Betrag", "7,5", "100,00"]))
        texte.append(t)
    out["beleg"] = []
    for i, t in enumerate(texte):
        conf = {a.replace("Nr. ", "", 1): 0.42 for a in anker[:3]} if i % 4 == 0 else {}
        req = {"text": t, "conf": conf, "schreibe": i % 2 == 0, "ref": "upload-1", "ts": TS}
        out["beleg"].append({"req": req, "py": H["eingang.beleg"](req)})
    # Ein Slot, der den Typ-Tag nur ENTHAELT (`außerhalb Lohnsteuerbescheinigung`, Feld
    # `vor_rv_ausserhalb_lstb`), gehoert nicht zum Typ: `startswith`, nicht `in`.
    for t in ("Lohnsteuerbescheinigung 2025\nNr. 3 45.000,00\naußerhalb Lohnsteuerbescheinigung 1.234,56",
              "Lohnsteuerbescheinigung 2025\naußerhalb Lohnsteuerbescheinigung: 100,00\nNr. 4 7,00",
              "AUSSERHALB LOHNSTEUERBESCHEINIGUNG 9,99\nLohnsteuerbescheinigung\nNr. 1 12,00"):
        for schreibe in (False, True):
            req = {"text": t, "conf": {}, "schreibe": schreibe, "ref": "upload-1", "ts": TS}
            out["beleg"].append({"req": req, "py": H["eingang.beleg"](req)})
    # Ueberlange Zeile: die Backtracking-Grenze von fancy-regex (Rust) schlaegt zu, Python-`re` nicht.
    # Das Muster des Ankers `Nr. 3` ist in beiden Faellen kein Treffer; Rust muss den Laufzeitfehler als
    # "passt nicht" lesen (Luecke statt Rate-Wert). Der Text steht als Rezept im Fixture (400 KB sparen).
    for einheit, n in ((" 3.5", 1_000), (" 3.5", 100_000), ("Nr  ", 100_000)):
        rezept = ["Lohnsteuerbescheinigung 2025\n", einheit, n, " 45,00\nNr. 3 45.000,00"]
        req = {"text_aus": rezept, "conf": {}, "schreibe": False, "ref": "upload-1", "ts": TS}
        roh = {k: v for k, v in req.items() if k != "text_aus"}
        roh["text"] = rezept[0] + einheit * n + rezept[3]
        out["beleg"].append({"req": req, "py": H["eingang.beleg"](roh)})

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        f.write("\n")
    print(OUT, os.path.getsize(OUT), "Byte;", {k: (len(v) if isinstance(v, list) else "-") for k, v in out.items()})


if __name__ == "__main__":
    main()
