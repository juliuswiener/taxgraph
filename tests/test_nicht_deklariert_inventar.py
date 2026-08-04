"""Inventar: alle Betragsfelder ohne elster_kz — ENDGUELTIG vs OFFEN.

Klassifiziert jeden elster_kz_grund in zwei Kategorien:
  ENDGUELTIG — fachlich kein Kz nötig. Der Wert geht in ein bestehendes Kz ein
               (Ring-Input, Aggregat-Summand, Kohorten-Schlüssel, Instanz-Reuse,
               andere Datenart, etc.). Kz-Arbeit würde das XSD nicht ändern.
  OFFEN — Kz-Arbeit steht aus. Der Wert hat kein bestehendes Kz und braucht
          eine eigenständige Kz-Bindung, XSD-Review oder Struktur-Entscheidung.

OFFEN_MARKER (10 Kerne, narrow):
  backlog, tbd, deferred, offen, eigene runde, folgeticket, null-mvp,
  kandidat, recon, mvp

Zusätzlich manuell klassifizierte Felder (6):
  berufsausbildung_aufwendungen, kap_gewinn_sonstige,
  kap_gewinn_sonstige_partner, kist_erstattet, kist_gezahlt,
  vv_entgelt_quote_prozent

ENDGUELTIG (8, trotz "Vordruck-Mapping open"-Marker):
  vpf_* (Kürzungsrechnung/Fristen-Reduktion/Satz 8/10/11):
    gehen in den Verpflegungs-Tage-Betrag (E0205409/E0205302/E0205201) ein.
    7 Felder: vpf_fruehstuecke_gestellt_anzahl, vpf_mittagessen_gestellt_anzahl,
    vpf_abendessen_gestellt_anzahl, vpf_mahlzeiten_gezahltes_entgelt,
    vpf_steuerfreie_erstattung_betrag, vpf_tage_24h_nach_drei_monaten,
    vpf_tage_an_abreise_nach_drei_monaten, vpf_tage_ueber_8h_nach_drei_monaten.

Stand: 2026-08-05, 115 Betragsfelder.

Ein Feld ohne elster_kz UND ohne elster_kz_grund ist ein harter Fehler.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))

import traverser as TR  # noqa: E402

# OFFEN_KERNE — Wort-Bestandteile, die Kz-Arbeit signalisieren.
# Ein Grund, der KEINEN dieser Kerne enthält, gilt als ENDGUELTIG.
OFFEN_KERNE = frozenset({
    "backlog", "tbd", "deferred", "offen", "eigene runde", "folgeticket",
    "null-mvp", "kandidat", "recon", "mvp",
})

# Manuelle Ausnahmen: Felder, die trotz fehlendem OFFEN_KERN Kz-Arbeit brauchen.
# (weil Marker-Sprache die echte Situation nicht trifft, z.B. "Modell-Mismatch")
# Jedes einzeln begründet:
#   berufsausbildung_aufwendungen   — "Kein XSD-Feld-Label" = echtes Mapping-Problem
#   kap_gewinn_sonstige             — MODELL-MISMATCH (4-Topf vs XSD-Summe), braucht Kz-Struktur
#   kap_gewinn_sonstige_partner     — selbe Struktur wie Person A
#   kist_erstattet                  — Vordruck-Form-Kz, Spalten-Kontext, Cross-Ref nötig
#   kist_gezahlt                    — selbe Struktur
#   vv_entgelt_quote_prozent        — "XSD-E-Nr Sektions-Lookup-Nachtrag" = Kz-Mapping offen
OFFEN_MANUELL = frozenset({
    "berufsausbildung_aufwendungen",
    "kap_gewinn_sonstige",
    "kap_gewinn_sonstige_partner",
    "kist_erstattet",
    "kist_gezahlt",
    "vv_entgelt_quote_prozent",
})

# Manuelle Ausnahme: Feld, das trotz OFFEN_KERN-Match ENDGUELTIG ist.
#   kap_kapitalertraege_partner — matcht "recon" nur wegen "Schema-Recon" (abgeschlossene
#     Analyse aus dem Kz-Instanz-Recon), NICHT "Kz-Recon" (offene Arbeit). Ist PARTNER_INSTANZ
#     (Kz-E1900701-Reuse Person A im person_b-Bucket) → ENDGUELTIG.
ENDGUELTIG_MANUELL = frozenset({
    "kap_kapitalertraege_partner",
})


def test_nicht_deklariert_inventar():
    """Alle 115 Betragsfelder: elster_kz=None -> Grund prüfen, klassifizieren."""
    bindung = TR.lade_bindung()

    offen: list[tuple[str, str, str]] = []
    endgueltig: list[tuple[str, str, str]] = []
    ohne_grund: list[tuple[str, str]] = []

    for fid, v in sorted(bindung.items()):
        typ = v.get("typ", "")
        if typ not in ("cent", "int", "euro"):
            continue
        kz = v.get("elster_kz")
        if kz:
            continue
        grund = v.get("elster_kz_grund", "") or ""

        if not grund:
            ohne_grund.append((fid, typ))
            continue

        # Manuelle Ausnahmen haben Vorrang
        if fid in OFFEN_MANUELL:
            offen.append((fid, typ, grund[:80]))
            continue
        if fid in ENDGUELTIG_MANUELL:
            endgueltig.append((fid, typ, grund[:80]))
            continue

        # Automatische Klassifikation
        grund_lower = grund.lower()
        if any(k in grund_lower for k in OFFEN_KERNE):
            offen.append((fid, typ, grund[:80]))
        else:
            endgueltig.append((fid, typ, grund[:80]))

    # === Harder Fehler: Felder ohne Grund ===
    assert not ohne_grund, (
        f"{len(ohne_grund)} Betragsfelder ohne elster_kz UND ohne elster_kz_grund: "
        + ", ".join(f"{f} ({t})" for f, t in ohne_grund))

    # === IST-Stand festschreiben (Ratsche) ===
    OFFEN_SOLL = 38  # 2026-08-05, 115 Betragsfelder. Bei Kz-Arbeit nachziehen.

    # === Themenblock-Gruppierung ===
    # OFFEN-Felder in Themenblöcke gruppiert
    bloecke = {
        "§36 Anrechnung (LSt, VZ)": ["p36_lohnsteuer", "p36_vorauszahlungen"],
        "KV/PV-Vorsorge §10": ["basis_kv_pv", "basis_kv_pv_partner",
                                "weitere_vorsorgeaufwendungen", "weitere_vorsorgeaufwendungen_partner"],
        "Unterhalt §33a": ["p33a_unterhalt_aufwendungen", "p33a_unterhalt_kv_pv",
                           "p33a_andere_einkuenfte_bezuege", "p33a_ausbildung_anzahl_kinder"],
        "Realsplitting §10 Abs.1a": ["realsplitting_unterhaltsleistungen",
                                      "realsplitting_empfaenger_kv_pv"],
        "DBA/AUS §34c": ["dba_auslaendische_einkuenfte", "dba_gezahlte_auslaendische_steuer"],
        "GewSt / §35": ["gewst_messbetrag", "gewst_hebesatz"],
        "EÜR/Gewinn §§13-18": ["betriebseinnahmen", "afa_jahresbetrag",
                                "einkuenfte_gewinn", "gewinnanteil",
                                "verguetung_taetigkeit", "verguetung_darlehen",
                                "verguetung_ueberlassung"],
        "§35c Sanierung": ["p35c_sanierungsaufwendungen", "p35c_energieberater_aufwendungen"],
        "KiSt §51a": ["kist_gezahlt", "kist_erstattet"],
        "Kapital §20 Modell-Mismatch": ["kap_gewinn_sonstige", "kap_gewinn_sonstige_partner"],
        "§22 Nr.3 / §23": ["p22_nr3_einkuenfte",
                           "p23_veraeusserungspreis", "p23_anschaffung_herstellungskosten",
                           "p23_werbungskosten"],
        "§32b Progressionsvorbehalt": ["p32b_progressionseinkuenfte"],
        "§10d Verlustvortrag": ["verlustvortrag_bestand"],
        "Berufsausbildung §10 Abs.1 Nr.7": ["berufsausbildung_aufwendungen"],
        "Kinderbetreuung §10 Abs.1 Nr.5": ["kinderbetreuungskosten"],
        "§21 VV Quote": ["vv_entgelt_quote_prozent"],
    }

    print(f"\n  Betragsfelder ohne elster_kz: {len(offen) + len(endgueltig)}")
    print(f"  ENDGUELTIG (kein Kz nötig):  {len(endgueltig)}")
    print(f"  OFFEN (Kz-Arbeit ausstehend): {len(offen)}")

    # Gruppiert ausgeben
    print(f"\n  --- OFFEN nach Themenblöcken ({len(offen)} Felder) ---")
    for blocktitel, fids in bloecke.items():
        vorhanden = [(fid, typ, grund) for fid, typ, grund in offen if fid in fids]
        if vorhanden:
            print(f"  [{blocktitel}] ({len(vorhanden)}):")
            for fid, typ, grund in sorted(vorhanden):
                print(f"    {fid:45}  {grund[:80]}")

    # Restliche OFFEN-Felder (nicht in Blöcken)
    rest = [(fid, typ, grund) for fid, typ, grund in offen
            if not any(fid in fids for fids in bloecke.values())]
    if rest:
        print(f"  [Rest / nicht zugeordnet] ({len(rest)}):")
        for fid, typ, grund in rest:
            print(f"    {fid:45}  {grund[:80]}")

    # Gate: OFFEN-Zahl darf nur sinken
    assert len(offen) == OFFEN_SOLL, (
        f"OFFEN-Zahl geändert: {OFFEN_SOLL} -> {len(offen)}. "
        f"Bei Kz-Arbeit OFFEN_SOLL nachziehen. "
        f"Differenz: {[f for f, _, _ in offen if f not in {f3 for f3, _, _ in endgueltig}][:10]}")