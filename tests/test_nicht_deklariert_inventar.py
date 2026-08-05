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
  kap_gewinn_sonstige,
  kap_gewinn_sonstige_partner,
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
#   kap_gewinn_sonstige             — MODELL-MISMATCH (4-Topf vs XSD-Summe), braucht Kz-Struktur
#   kap_gewinn_sonstige_partner     — selbe Struktur wie Person A
#   vv_entgelt_quote_prozent        — "XSD-E-Nr Sektions-Lookup-Nachtrag" = Kz-Mapping offen
OFFEN_MANUELL = frozenset({
    "kap_gewinn_sonstige",
    "kap_gewinn_sonstige_partner",
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

        # (1) EXPLIZITES Feld schlaegt jede Heuristik. `kz_status: endgueltig` in der Bindung
        # heisst: am XSD belegt, dass es KEIN Kz gibt (nicht "noch nicht gesucht").
        # Eingefuehrt 2026-08-05, weil das Substring-Matching unten an der Prosa scheitert:
        # ein Grund, der "ENDGUELTIG kein E10-Kz (belegt, nicht Backlog)" sagt, enthaelt
        # das Wort "Backlog" und wurde als OFFEN gezaehlt — die Klassifikation las das
        # Gegenteil der Aussage. Neue Felder tragen `kz_status`, Altbestand laeuft weiter
        # ueber die Heuristik.
        status = v.get("kz_status")
        if status == "endgueltig":
            endgueltig.append((fid, typ, grund[:80]))
            continue
        if status == "offen":
            offen.append((fid, typ, grund[:80]))
            continue

        # (2) Manuelle Ausnahmen
        if fid in OFFEN_MANUELL:
            offen.append((fid, typ, grund[:80]))
            continue
        if fid in ENDGUELTIG_MANUELL:
            endgueltig.append((fid, typ, grund[:80]))
            continue

        # (3) Heuristik ueber Marker-Woerter (Altbestand)
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
    # 2026-08-05: 38 -> 36. Die zwei § 36-Felder (p36_lohnsteuer, p36_vorauszahlungen) sind
    # von OFFEN nach ENDGUELTIG gewandert — am E10-2025.xsd belegt, dass es dafuer kein
    # Erklaerungs-Kz gibt (E10 erklaert Besteuerungsgrundlagen, die Anrechnung rechnet das FA).
    # Sie tragen jetzt `kz_status: endgueltig` in der Bindung. Das ist KEINE Kz-Arbeit, sondern
    # eine korrigierte Einordnung: die Zahl war vorher zu hoch, nicht die Lage besser geworden.
    # 2026-08-05 Block 1: 36 -> 34. kist_gezahlt -> E0107601, kist_erstattet -> E0107602
    # gebunden (E10/SA/KiSt/Gezahlt/Sum | Erstattet). Das IST erledigte Kz-Arbeit,
    # anders als der 38->36-Schritt davor (der war nur eine korrigierte Einordnung).
    # 2026-08-05 Block 2: 34 -> 30. berufsausbildung_aufwendungen -> E0108202,
    # p22_nr3_einkuenfte -> E0305301 (Einkuenfte, NICHT E0305101 Einnahmen),
    # gewst_messbetrag -> E0801606, gewst_hebesatz -> E0801705.
    # vv_entgelt_quote_prozent und verlustvortrag_bestand bleiben OFFEN: dort ist das
    # Kz zwar da, passt aber semantisch/typmaessig nicht (Kuerzungs- statt Entgeltquote;
    # Ja-Feld statt Betrag) — kz_status: offen, Begruendung in der Bindung.
    # 2026-08-05 Block 3: 30 -> 26. p33a_unterhalt_aufwendungen -> E0120103,
    # p33a_unterhalt_kv_pv -> E0124401 (darin enthaltene Teilmenge, nicht additiv),
    # p35c_sanierungsaufwendungen -> E0241901, p35c_energieberater -> E0242001
    # (GETRENNTE Kz — der Energieberater hatte faelschlich dasselbe Kz notiert).
    # 2026-08-05 Block 4: 26 -> 21. kinderbetreuungskosten -> E0506105,
    # realsplitting_unterhaltsleistungen -> E0304601, realsplitting_empfaenger_kv_pv ->
    # E0300717 (SO/Unt_Leist, NICHT die § 33a-Kz aus ESt1A_U — die waren faelschlich
    # notiert), dba_auslaendische_einkuenfte -> E0601401,
    # dba_gezahlte_auslaendische_steuer -> E0601901.
    # 2026-08 Feldsplit: basis_kv_pv (+_partner) -> basis_kv + basis_pv (+_partner).
    # Alte Felder fallen aus OFFEN raus (-2). Kz-Bindung Schritt 3: alle 4 neuen
    # Felder haben Kz via VERZWEIGUNG/PARTNER_VERZWEIGUNG -> nicht mehr OFFEN.
    # Netto: 23-4=19. weitere_vorsorgeaufwendungen(_partner) bleibt OFFEN (10 Abs. 1 Nr. 3a).
    OFFEN_SOLL = 18  # 19 - 1: p33a_andere_einkuenfte_bezuege in ENDGUELTIG gewandert (durch dev-1s Durchgangstests)
    # === Themenblock-Gruppierung ===
    # OFFEN-Felder in Themenblöcke gruppiert
    bloecke = {
        "§36 Anrechnung (LSt, VZ)": ["p36_lohnsteuer", "p36_vorauszahlungen"],
        "KV/PV-Vorsorge §10": ["basis_kv", "basis_pv", "basis_kv_partner", "basis_pv_partner",
                                "weitere_vorsorgeaufwendungen", "weitere_vorsorgeaufwendungen_partner"],
        "Unterhalt §33a (Rest)": ["p33a_andere_einkuenfte_bezuege",
                                  "p33a_ausbildung_anzahl_kinder"],
        "EÜR/Gewinn §§13-18": ["betriebseinnahmen", "afa_jahresbetrag",
                                "einkuenfte_gewinn", "gewinnanteil",
                                "verguetung_taetigkeit", "verguetung_darlehen",
                                "verguetung_ueberlassung"],
        "Kapital §20 Modell-Mismatch": ["kap_gewinn_sonstige", "kap_gewinn_sonstige_partner"],
        "§23 private Veräußerung": ["p23_veraeusserungspreis",
                                    "p23_anschaffung_herstellungskosten",
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