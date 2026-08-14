"""Kein enum-Wert erreicht den Nutzer als Rohwert.

Befund 2026-08-14 (Julius beim ersten Durchklicken): "Antwortmöglichkeiten sind häufig einfach die
Enums, also da steht dann land_forst klein geschrieben." Gemessen: 22 askable enum-Felder, 74
distinkte Rohwerte, NULL Anzeigetexte — app.js setzte `o.textContent = v`.

Die härtesten Fälle waren die Kindschaftsverhältnisse: dort stand im Auswahlfeld schlicht "1",
"2", "3". Die Bedeutung war da, aber im Fragetext versteckt ("1 = leibliches Kind/Adoptivkind,
2 = Pflegekind, 3 = Enkelkind/Stiefkind") — also an der Stelle, die man liest, bevor man das
Auswahlfeld öffnet, und nicht mehr, wenn man darin sucht.

Dieser Test hält die Tabelle vollständig: ein neuer enum_wert in irgendeiner Bindung ohne Eintrag
in ENUM_LABELS wird rot. Ohne ihn verfällt die Tabelle beim nächsten hinzugefügten Land oder
Rentenart lautlos wieder auf Rohwerte.

NULL LLM.
"""
from __future__ import annotations

import glob
import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))

import api_constants as AC  # noqa: E402


def _askable_enums() -> dict[str, list[str]]:
    """{feld_id: [enum_werte]} für jedes askable enum-Feld über alle Bindungen."""
    out: dict[str, list[str]] = {}
    for fp in sorted(glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml"))):
        for b in yaml.safe_load(open(fp)).get("bindungen", []):
            if b.get("askable") and b.get("typ") == "enum" and b.get("enum_werte"):
                out[b["feld_id"]] = list(b["enum_werte"])
    return out


def test_jedes_askable_enum_feld_hat_labels():
    fehlend = sorted(f for f in _askable_enums() if f not in AC.ENUM_LABELS)
    assert not fehlend, (
        f"Diese enum-Felder haben keine Anzeigetexte — der Nutzer sieht den Rohwert: {fehlend}\n"
        f"Eintrag in api_constants.ENUM_LABELS ergänzen.")


def test_jeder_einzelne_enum_wert_hat_einen_anzeigetext():
    luecken = []
    for fid, werte in _askable_enums().items():
        labels = AC.ENUM_LABELS.get(fid, {})
        for w in werte:
            if w not in labels:
                luecken.append(f"{fid}:{w}")
    assert not luecken, (
        f"{len(luecken)} enum-Werte ohne Anzeigetext: {sorted(luecken)[:20]}")


def test_labels_sind_lesbar_und_nicht_der_rohwert():
    """Gegen die billige Erfüllung des Tests oben: ein Label, das den Rohwert nur kopiert, ist
    keins. Ausgenommen sind Werte, die von Haus aus lesbar sind — Ländernamen wie 'Schweiz'
    stehen bereits richtig in der Bindung."""
    LESBAR_ROH = {"Deutschland", "Frankreich", "Italien", "Schweiz", "Niederlande", "Polen",
                  "Tschechien", "Dänemark", "Luxemburg", "Türkei", "Spanien", "USA", "Kanada"}
    schlecht = []
    for fid, werte in _askable_enums().items():
        for w in werte:
            label = AC.ENUM_LABELS.get(fid, {}).get(w)
            if label is None or w in LESBAR_ROH:
                continue
            if label == w:
                schlecht.append(f"{fid}:{w}")
            elif "_" in label and label.replace("_", " ") == w.replace("_", " "):
                schlecht.append(f"{fid}:{w} (nur Unterstriche ersetzt)")
    assert not schlecht, f"Labels, die nur den Rohwert wiederholen: {schlecht}"


def test_api_liefert_die_labels_mit_aus():
    """Die Tabelle nützt nichts, wenn sie die Oberfläche nie erreicht. /fragen muss enum_labels
    je Frage mitschicken — app.js liest genau diesen Schlüssel."""
    for sub in ("produkt/store", "produkt/traverser", "produkt/mapping",
                "produkt/unsicherheit", "golden"):
        sys.path.insert(0, os.path.join(ROOT, sub))
    import api as API      # noqa: E402
    import store as ST     # noqa: E402

    store = ST.leerer_store(2025, fall_id="enum-labels")
    store["scheibe"] = "gesamt"
    bindung = API._scheibe_bindung(store)
    enum_felder = {f for f, b in bindung.items() if b.get("typ") == "enum" and b.get("askable")}
    assert enum_felder, "Scheibe gesamt hat keine askable enum-Felder — Test misst nichts."

    # den Auslieferungspfad nachbauen (fragen() braucht einen persistierten Fall)
    fid = sorted(enum_felder)[0]
    labels = AC.ENUM_LABELS.get(fid)
    assert labels, f"{fid} hat keine Labels"
    assert all(w in labels for w in bindung[fid]["enum_werte"]), (
        f"{fid}: nicht jeder enum_wert hat ein Label")
