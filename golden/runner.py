"""Golden-corpus runner: check the Catala § 32a formalisation against the
curated cases in golden/cases/, and verify each case's citation anchor against
the frozen source text (hard gate).

For every case:
  1. the `zitatanker` must occur (after normalisation) in the referenced
     sources/ document;
  2. the Catala-computed tarifliche ESt must equal `erwartung.tarifliche_est`.

Exit code 0 only if all cases pass. Requires the assembled Catala package
(bash oracle/gettsim/assemble_catala.sh) and PyYAML.

Run: python golden/runner.py   (or: make golden)
"""

from __future__ import annotations

import glob
import os
import re
import sys

import yaml  # noqa: F401

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)

from yamlstrict import load_str  # noqa: E402


def load_yaml_fh(fh):
    """Strikt laden: doppelte Schluessel sind ein Fehler, kein Ueberschreiben."""
    with fh:
        return load_str(fh.read(), herkunft=getattr(fh, "name", "<yaml>"))
_CAT = os.path.join(ROOT, "oracle", "gettsim", "_catala")
sys.path.insert(0, os.path.join(_CAT, "rt"))
sys.path.insert(0, _CAT)

from pkg import Einkommensteuertarif as E  # noqa: E402  (Catala-generated)
from pkg import Entfernungspauschale as EP  # noqa: E402
from pkg import Arbeitszimmer_homeoffice as AZ  # noqa: E402
from pkg import Haushaltsnahe as HN  # noqa: E402  (§ 35a Abs. 1-3, charge29-Promotion)
from pkg import SpendenAbzug as SA  # noqa: E402  (§ 10b Abs. 1, charge29-Promotion)
from pkg import ZumutbareBelastung as ZB  # noqa: E402  (§ 33 Abs. 3 zumutbare Belastung)
from pkg import AgbAbzug as AG  # noqa: E402  (§ 33 Abs. 1 agB-Abzug)
from pkg import Kirchensteuerabzug as KI  # noqa: E402  (§ 10 Abs. 1 Nr. 4 KiSt)
from pkg import Altersentlastungsbetrag as AE  # noqa: E402  (§ 24a, charge30)
from pkg import Entlastungsbetrag as EB  # noqa: E402  (§ 24b Alleinerziehende, charge30)
from pkg import Familienleistungsausgleich as FL  # noqa: E402  (§ 31 Günstigerprüfung, charge30)
from pkg import VerbilligteVermietungWk as VV  # noqa: E402  (§ 21 Abs. 2 verbilligte Vermietung WK-Kürzung)
from pkg import KrankenPflegeVorsorge as KP  # noqa: E402  (§ 10 Abs. 1 Nr. 3/3a KV/PV-Vorsorge)
from pkg import Berufsausbildungsaufwendungen as BA  # noqa: E402  (§ 10 Abs. 1 Nr. 7 Berufsausbildung, Tier-1)
from pkg import BetriebsFreibetrag as BF  # noqa: E402  (§ 16 Abs. 4 Betriebsveräußerungs-Freibetrag, 2-I)
from pkg import EuerGewinn as EG  # noqa: E402  (§ 4 Abs. 3 Einnahmenüberschussrechnung, 2-II)
from pkg import Verlustvortrag as VL  # noqa: E402  (§ 10d Abs. 2 Verlustvortrag-Abzug)
from pkg import MitunternehmerEinkuenfte as ME  # noqa: E402  (§ 15 Abs. 1 S. 1 Nr. 2 Mitunternehmer, #2-Front)
from pkg import ErmaessigterDurchschnittssatz as ED  # noqa: E402  (§ 34 Abs. 3 ermäßigter Durchschnittssatz, Stufe-2a)
from catala_runtime import Money, Decimal, Bool, Integer  # noqa: E402


def _az_params(year: int) -> dict:
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "arbeitszimmer_homeoffice.yaml"), encoding="utf-8"))
    return {k: p[k]["wert"] for k in
            ("jahrespauschale", "tagespauschale_pro_tag", "tagespauschale_hoechstbetrag")}


def catala_raumkosten(s: dict) -> int:
    r = _az_params(s["veranlagungszeitraum"])
    out = AZ.raumkostenabzug(AZ.RaumkostenabzugIn(
        arbeitszimmer_vorhanden_in=Bool(s.get("arbeitszimmer_vorhanden", False)),
        ist_mittelpunkt_in=Bool(s.get("ist_mittelpunkt", False)),
        tatsaechliche_aufwendungen_in=Money(f"{int(s.get('tatsaechliche_aufwendungen', 0))}.00"),
        jahrespauschale_gewaehlt_in=Bool(s.get("jahrespauschale_gewaehlt", False)),
        monate_ohne_mittelpunkt_in=int(s.get("monate_ohne_mittelpunkt", 0)),
        homeoffice_tage_in=int(s.get("homeoffice_tage", 0)),
        jahrespauschale_in=Money(f"{int(r['jahrespauschale'])}.00"),
        tagespauschale_pro_tag_in=Money(f"{int(r['tagespauschale_pro_tag'])}.00"),
        tagespauschale_hoechstbetrag_in=Money(f"{int(r['tagespauschale_hoechstbetrag'])}.00")))
    return int(out.abzug_gesamt) // 100


def _ep_saetze(year: int) -> dict:
    """Read the Entfernungspauschale rates for a VZ from params/."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "entfernungspauschale.yaml"), encoding="utf-8"))
    return {k: p[k]["wert"] for k in
            ("satz_bis_20_km", "satz_ab_21_km", "staffelgrenze_km", "hoechstbetrag_ohne_kfz")}


def catala_grundfreibetrag(year: int) -> int:
    """§ 32a Abs. 1 S. 2 Nr. 1 EStG — Grundfreibetrag des VZ, EURO (aus params/)."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "einkommensteuertarif_p32a.yaml"), encoding="utf-8"))
    return int(p["grundfreibetrag"]["wert"])


def catala_arbeitnehmer_pauschbetrag(year: int) -> int:
    """§ 9a S. 1 Nr. 1a EStG — Arbeitnehmer-Pauschbetrag des VZ, EURO (aus params/)."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "arbeitnehmerpauschbetrag.yaml"), encoding="utf-8"))
    return int(p["wert"]["wert"])


def catala_entfernungspauschale(s: dict) -> int:
    year = s["veranlagungszeitraum"]
    r = _ep_saetze(year)
    out = EP.berechnung(EP.BerechnungIn(
        entfernung_km_roh_in=Decimal(str(s["entfernung_km_roh"])),
        arbeitstage_in=int(s["arbeitstage"]),
        eigenes_oder_ueberlassenes_kfz_in=Bool(s.get("eigenes_oder_ueberlassenes_kfz", False)),
        oepnv_kosten_jahr_in=Money(f"{int(s.get('oepnv_kosten_jahr', 0))}.00"),
        satz_bis_20_km_in=Money(f"{r['satz_bis_20_km']:.2f}"),
        satz_ab_21_km_in=Money(f"{r['satz_ab_21_km']:.2f}"),
        staffelgrenze_km_in=int(r["staffelgrenze_km"]),
        hoechstbetrag_in=Money(f"{int(r['hoechstbetrag_ohne_kfz'])}.00")))
    return int(out.abziehbarer_betrag) // 100


def catala_ep_ab_21km(s: dict) -> int:
    """§ 9 Abs. 1 S. 3 Nr. 4 S. 2 EStG — der ab dem 21. VOLLEN Entfernungskilometer erhöhte
    Teil der Entfernungspauschale (Bemessungsbasis der § 101-Mobilitätsprämie), EURO int.

    Deterministisch aus DENSELBEN params wie catala_entfernungspauschale (single source of
    truth für die Sätze, KEIN neues Input-Feld → kein Denormalisierungs-Drift). KONSERVATIV
    abgerundet und gekappt am tatsächlich abziehbaren Rest (abziehbarer_betrag − Anteil ≤ 20 km)
    → nie mehr ab-21km als real berücksichtigt. Für eine PRÄMIE (Auszahlung) ist Unter-Ansatz
    die K2-sichere Richtung (Über-Ansatz = Über-Förderung = Fiskus-Verlust = Under-tax-Analog)."""
    year = s["veranlagungszeitraum"]
    r = _ep_saetze(year)
    km_voll = int(Decimal(str(s["entfernung_km_roh"])))                # volle Entfernungs-km (§ 9 Abs.1 S.3 Nr.4 S.4)
    grenze = int(r["staffelgrenze_km"])                               # 20
    arbeitstage = int(s.get("arbeitstage", 0))
    satz_ab21_ct = int(Decimal(str(r["satz_ab_21_km"])) * 100)        # €/km ab 21 in Cent
    satz_bis20_ct = int(Decimal(str(r["satz_bis_20_km"])) * 100)      # €/km bis 20 in Cent
    ab21_roh = arbeitstage * max(0, km_voll - grenze) * satz_ab21_ct // 100   # abrunden → konservativ
    ep_bis20 = arbeitstage * min(km_voll, grenze) * satz_bis20_ct // 100
    gesamt = catala_entfernungspauschale(s)                           # tatsächlich abziehbar (inkl. Höchstbetrag)
    return max(0, min(ab21_roh, gesamt - ep_bis20))


def _p101_bemessungsgrundlage(s: dict) -> int:
    """§ 101 S. 1-3 EStG — Bemessungsgrundlage der Mobilitätsprämie, EURO int.

      Basis  = berücksichtigte Entfernungspauschale ab dem 21. km (entfernungspauschale_ab_21km,
               vgl. catala_ep_ab_21km).
      S. 3   = bei Einkünften aus nichtselbständiger Arbeit (ist_arbeitnehmer=True) NUR SOWEIT
               diese ab-21km-EP zusammen mit den übrigen Werbungskosten den Arbeitnehmer-
               Pauschbetrag (§ 9a S. 1 Nr. 1a) übersteigt → Basis = min(ab21,
               max(0, werbungskosten_gesamt − arbeitnehmer_pauschbetrag)). Bei Nicht-AN
               (Betriebsausgaben, ist_arbeitnehmer=False) entfällt S. 3.
      S. 2   = begrenzt auf den Betrag, um den das zvE den Grundfreibetrag unterschreitet
               (max(0, GFB − zvE)); bei Zusammenveranlagung übergibt der Caller gemeinsames
               zvE + VERDOPPELTEN Grundfreibetrag (§ 101 S. 2 Hs. 2)."""
    ep_ab_21 = int(s.get("entfernungspauschale_ab_21km", 0))
    zvE = int(s.get("zu_versteuerndes_einkommen", 0))
    gfb = int(s.get("grundfreibetrag", 0))
    if s.get("ist_arbeitnehmer", False):                              # § 101 S. 3 (AN-Pauschbetrag-soweit)
        wk_gesamt = int(s.get("werbungskosten_gesamt", 0))
        an_pausch = int(s.get("arbeitnehmer_pauschbetrag", 0))
        ep_ab_21 = min(ep_ab_21, max(0, wk_gesamt - an_pausch))
    unterschreitung = max(0, gfb - zvE)                              # § 101 S. 2
    return min(ep_ab_21, unterschreitung)


def catala_p101_mobilitaetspraemie(s: dict) -> int:
    """§ 101 S. 4 EStG — Mobilitätsprämie: 14 % der Bemessungsgrundlage. EURO int (abgerundet)."""
    return _p101_bemessungsgrundlage(s) * 14 // 100


def catala_p101_mobilitaetspraemie_cent(s: dict) -> int:
    """§ 101 S. 4 EStG — Mobilitätsprämie = 14 % der Bemessungsgrundlage, CENT-exakt.

    bemessungsgrundlage(EURO) × 14 = CENT ohne Rundungsschnitt (14 %/100 × 100 ct/€ = ×14,
    derselbe Einheiten-Trick wie catala_kist). Prämie=Auszahlung → cent-exakt statt euro-Floor
    (kein Unter-Ansatz zu Lasten des Steuerpflichtigen). Ring-Naht-Variante (mobilitaetspraemie_cent)."""
    return _p101_bemessungsgrundlage(s) * 14


def catala_werbungskosten_n(s: dict) -> int:
    """§ 9 Werbungskosten Anlage N — ROH-Summe in EURO, OHNE § 9a-Arbeitnehmer-Pauschbetrag.
    Den Pauschbetrag-Guenstiger wendet der Tarif `festzusetzende_est_einzel` intern an
    (handverifiziert: ESt(WK 0)==ESt(WK 1230)); ein § 9a hier waere doppelter Abzug.

    Stufe 1b: Entfernungspauschale (Catala-Modul) + doppelte Haushaltsfuehrung (Python-Andockung
    _dhf_abzug, Registry-Transkription p9_1_3_nr5) + Verpflegung (§ 9 Abs. 4a) + Übernachtung
    (§ 9 Abs. 1 Nr. 5a, _uebernachtung_abzug, Registry-Transkription p9_1_3_nr5a_*) + Arbeitsmittel
    (§ 9 Abs. 1 Nr. 6/7 i.V.m. § 6 Abs. 2 GWG, Level-1 = Sofortabzug ≤ 800 EUR via catala_p6_2_gwg;
    der mehrjährige § 7-AfA-Zweig > 800 ist ungebunden -> die Haut sperrt ihn fail-closed).
    Alle EURO. Kein § 9a (der sitzt im Tarif)."""
    wk = 0
    if "entfernung_km_roh" in s:
        wk += catala_entfernungspauschale(s)
    if "unterkunftskosten_monat" in s:
        wk += _dhf_abzug(s, s["veranlagungszeitraum"])
    if any(k in s for k in ("tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig")):
        wk += _verpflegung_abzug(s, s["veranlagungszeitraum"])
    if "uebernachtung_kosten_monat" in s:
        wk += _uebernachtung_abzug(s, s["veranlagungszeitraum"])
    # Arbeitsmittel (§ 9 Abs. 1 S. 3 Nr. 7 i.V.m. § 6 Abs. 2): Level-1 GWG-Sofortabzug ≤ 800 EUR.
    # Die Haut lässt nur AK ≤ 800 mit ausgeübtem Wahlrecht bis hierher (Guard sperrt > 800 = § 7-AfA
    # sowie abgelehntes Wahlrecht). catala_p6_2_gwg gibt bei > 800 defensiv 0 (over-tax-safe).
    if "am_anschaffungskosten" in s:
        wk += catala_p6_2_gwg({"gwg_anschaffungskosten_netto": s["am_anschaffungskosten"]})
    return wk


def _dhf_params(year: int) -> dict:
    """Kappungsgrenzen doppelte Haushaltsfuehrung aus params/<vz> (§ 9 Abs. 1 S. 3 Nr. 5)."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "dhf_p9_1_nr5.yaml"), encoding="utf-8"))
    return {k: p[k]["wert"] for k in ("cap_monat_inland", "cap_monat_ausland")}


def _dhf_abzug(s: dict, year: int) -> int:
    """§ 9 Abs. 1 S. 3 Nr. 5 EStG — abziehbare Unterkunftskosten der doppelten Haushaltsfuehrung,
    EURO. WOERTLICHE Transkription des Registry-Rechenwegs von p9_1_3_nr5_doppelte_haushaltsfuehrung
    (hinweis): 'Monatsmiete 1.400 EUR wird auf 1.000 EUR gekappt; 1.000 x 12 = 12.000,00 EUR' /
    'Kappung wirkt je Monat, nicht auf das Jahr: 1.000 x 6 = 6.000,00 EUR' / 'Auslandsunterkunft:
    Kappung bei 2.000 EUR je Monat'. Grenzen aus params/<vz> (nie hardcoden). KOPPLUNG: bei
    Aenderung der Registry-Regel p9_1_3_nr5 diese Formel nachziehen (Konsistenz-Gate haelt sie fest)."""
    cap = _dhf_params(year)
    grenze = cap["cap_monat_inland"] if s.get("im_inland", True) else cap["cap_monat_ausland"]
    return min(int(s.get("unterkunftskosten_monat", 0)), grenze) * int(s.get("monate", 0))


def _verpflegung_params(year: int) -> dict:
    """Verpflegungspauschalen (Inland) aus params/<vz> (§ 9 Abs. 4a S. 3 Nr. 1-3)."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "verpflegung_p9_4a.yaml"), encoding="utf-8"))
    return {k: p[k]["wert"] for k in ("pauschale_24h", "pauschale_an_abreise", "pauschale_ab_8h")}


def _verpflegung_pauschale(s: dict, year: int) -> int:
    """§ 9 Abs. 4a S. 3 EStG — Verpflegungspauschale JE Reise-Tag, EURO. WOERTLICHE Transkription
    der Registry-Staffel p9_4a_verpflegungsmehraufwand: An-/Abreisetag (Nr. 2) -> 14; sonst voller
    Tag ab 24 h (Nr. 1) -> 28; eintaegig > 8 h (Nr. 3) -> 14; sonst 0. Saetze aus params/<vz>.
    Der Jahres-WK-Abzug (_verpflegung_abzug) summiert diese Pauschale ueber die Tage je Kategorie
    (Tage-Bindung dev-2). KOPPLUNG: bei Aenderung der Registry-Regel p9_4a diese Staffel nachziehen."""
    p = _verpflegung_params(year)
    if s.get("an_oder_abreisetag"):
        return p["pauschale_an_abreise"]
    stunden = int(s.get("abwesenheit_stunden", 0))
    if stunden >= 24:
        return p["pauschale_24h"]
    if stunden > 8:
        return p["pauschale_ab_8h"]
    return 0


def _verpflegung_abzug(s: dict, year: int) -> int:
    """§ 9 Abs. 4a S. 3 EStG — Jahres-Verpflegungspauschale, EURO = Summe der Tage je Kategorie mal
    ihrer Pauschale (Registry-Transkription p9_4a, Sätze aus params/<vz>):
        tage_24h × 28 (voller Tag) + tage_an_abreise × 14 + tage_ueber_8h_eintaegig × 14.
    LÜCKE (bewusst, NICHT still — Haut-Guard/Annahme): die 3-Monats-Frist (S. 6, Pauschale nur die
    ersten drei Monate je Einsatzort) und die Mahlzeitenkürzung (S. 8) sind REDUKTIONEN, hier NICHT
    modelliert — ein voller Σ ohne sie überschätzt den Abzug. KOPPLUNG: bei Registry-Änderung p9_4a
    diese Formel + _verpflegung_pauschale nachziehen."""
    p = _verpflegung_params(year)
    return (int(s.get("tage_24h", 0)) * p["pauschale_24h"]
            + int(s.get("tage_an_abreise", 0)) * p["pauschale_an_abreise"]
            + int(s.get("tage_ueber_8h_eintaegig", 0)) * p["pauschale_ab_8h"])


def _uebernachtung_abzug(s: dict, year: int) -> int:
    """§ 9 Abs. 1 S. 3 Nr. 5a EStG — abziehbare Übernachtungskosten bei Auswärtstätigkeit, EURO.
    WÖRTLICHE Transkription der Registry-Regeln p9_1_3_nr5a_uebernachtung_vor_48 / _nach_48:
    Zeitraum VOR Ablauf der 48 Monate (uebernachtung_monate_bisher < 48) → tatsächliche Kosten
    ungekappt (Satz 1-3): kosten × monate. NACH Ablauf (>= 48) → Kappung auf den Nr.-5-Betrag
    (Satz 4 via Verweis, 1.000 EUR/Monat Inland aus params/<vz>/dhf_p9_1_nr5.yaml):
    min(kosten, 1.000) × monate. Der Ring-Guard stellt sicher, dass der Zeitraum NICHT die
    48-Monats-Schwelle überspannt (p9_1_3_nr5a_uebernachtung: zeitraum_ohne_schwellenuebertritt)
    und Inland ist (Ausland-2.000er-Grenze zurückgestellt). KOPPLUNG: bei Änderung der Registry-
    Regeln p9_1_3_nr5a_* diese Formel nachziehen."""
    kosten = int(s.get("uebernachtung_kosten_monat", 0))
    monate = int(s.get("uebernachtung_monate", 0))
    bisher = int(s.get("uebernachtung_monate_bisher", 0))
    if bisher >= 48:
        cap = _dhf_params(year)["cap_monat_inland"]
        return min(kosten, cap) * monate
    return kosten * monate


def catala_vermietung_einkuenfte(s: dict) -> int:
    """§ 21 Abs. 1 EStG — Einkünfte aus Vermietung und Verpachtung (Überschuss der Einnahmen über die
    Werbungskosten, § 2 Abs. 2 Nr. 2), EURO. Weg A (Python-Andockung — kein callable Catala-Scope im
    assemblierten pkg): einnahmen − (gebaeude_afa + schuldzinsen + erhaltungsaufwand +
    sonstige_werbungskosten). NEGATIV möglich (Vermietungsverlust). WÖRTLICHE Transkription des
    Registry-Rechenwegs p21_vermietung_einkuenfte. KOPPLUNG: bei Registry-Änderung nachziehen; das
    Konsistenz-Gate hält runner↔registry fest. Die tarifliche Verrechnung macht catala_gesamt
    (einkuenfte_vermietung-Input); dieser Accessor liefert nur die Überschuss-Größe."""
    return (int(s.get("einnahmen", 0))
            - (int(s.get("gebaeude_afa", 0)) + int(s.get("schuldzinsen", 0))
               + int(s.get("erhaltungsaufwand", 0)) + int(s.get("sonstige_werbungskosten", 0))))


def catala_einkuenfte_nichtselbststaendig(s: dict) -> int:
    """§ 19 i.V.m. § 2 Abs. 2 Nr. 2, § 9a S. 1 Nr. 1 EStG — Einkünfte aus nichtselbständiger Arbeit
    (Bruttolohn − Werbungskosten, mindestens Arbeitnehmer-Pauschbetrag 1230), EURO. Liest die fertige
    Größe summe_der_einkuenfte des einzel-Tarifs (§ 9a-Günstiger IM Scope) — KEIN § 9a-Doppelabzug.
    Für die § 2-Abs.-3-Summierung im Gesamt-Scope (kombiniert §19+§21): dieser Wert geht als
    einkuenfte_nichtselbststaendig in catala_gesamt. sonderausgaben=0 hier — der § 10c-Floor gilt EINMAL
    auf Personen-Ebene und wird von catala_gesamt (_sonderausgaben_final) gebildet, nicht je Einkunftsart."""
    out = E.festzusetzende_est_einzel(E.FestzusetzendeEstEinzelIn(
        bruttoarbeitslohn_in=Money(f"{int(s.get('bruttoarbeitslohn', 0))}.00"),
        werbungskosten_in=Money(f"{int(s.get('werbungskosten', 0))}.00"),
        sonderausgaben_in=Money("0.00"),
        veranlagungszeitraum_in=VZ_ENUM[s["veranlagungszeitraum"]]))
    return int(out.summe_der_einkuenfte) // 100


def catala_p35a_haushaltsnahe(s: dict) -> int:
    """§ 35a Abs. 1-5 EStG. EURO rein, EURO raus. Antrag implizit (Aufwand-Eingabe).
    EU/EWR (Abs.4) gatet Abs.1-3. Rechnung+unbar (Abs.5 S.3) gatet NUR Abs.2/3."""
    minijob = int(s.get("hh_minijob_aufwendungen", 0))
    dienstleistungen = int(s.get("hh_dienstleistungen", 0))
    handwerker = int(s.get("hh_handwerker_arbeitskosten", 0))
    eu_ewr = s.get("hh_in_eu_ewr", {}).get("wert") is True
    rechnung = s.get("hh_rechnung_unbar", {}).get("wert") is True
    if not eu_ewr:                       # Abs.4: gilt für alle drei
        return 0
    if not rechnung:                     # Abs.5 S.3: nur Abs.2/3, Minijob unberührt
        dienstleistungen = 0
        handwerker = 0
    ermessigung = 0
    if minijob > 0:
        ermessigung += min(minijob * 20 // 100, 510)
    if dienstleistungen > 0:
        ermessigung += min(dienstleistungen * 20 // 100, 4000)
    if handwerker > 0:
        ermessigung += min(handwerker * 20 // 100, 1200)
    if s.get("p35a_mitveranlagung", {}).get("wert") is True:
        ermessigung = ermessigung // 2
    return ermessigung


def catala_p3_nr72_photovoltaik(s: dict) -> int:
    """§ 3 Nr. 72 EStG — steuerfreie Einnahmen aus Gebäude-Photovoltaik. EURO rein, EURO raus.

    Rückgabe = der Betrag, der von den Gewinneinkünften ABZUZIEHEN ist (0 = keine Befreiung).

    Zwei kumulative Leistungsgrenzen (S. 1), beide „bis zu"/„höchstens", also einschließend:
      - 30 kWp je Wohn- oder Gewerbeeinheit
      - insgesamt 100 kWp pro Steuerpflichtigem oder Mitunternehmerschaft

    Die Befreiung ist eine Freigrenze, kein Freibetrag: wird eine Grenze gerissen, sind die
    Einnahmen VOLL steuerpflichtig (kein anteiliger Abzug bis zur Grenze).

    Anker: "die Einnahmen und Entnahmen im Zusammenhang mit dem Betrieb von auf, an oder in
    Gebäuden (einschließlich Nebengebäuden) vorhandenen Photovoltaikanlagen, wenn die
    installierte Bruttoleistung laut Marktstammdatenregister bis zu 30 Kilowatt (peak) je
    Wohn- oder Gewerbeeinheit und insgesamt höchstens 100 Kilowatt (peak) pro
    Steuerpflichtigem oder Mitunternehmerschaft beträgt"
    (sources/gesetze-im-internet/estg_p3_2026-07-30.txt)
    """
    einnahmen = int(s.get("pv_einnahmen", 0))
    if einnahmen <= 0:
        return 0
    if s.get("pv_auf_gebaeude", {}).get("wert") is not True:
        return 0                                    # S. 1: nur Anlagen auf/an/in Gebäuden
    leistung = int(s.get("pv_bruttoleistung_kwp", 0))
    einheiten = int(s.get("pv_anzahl_einheiten", 0))
    if leistung <= 0 or einheiten <= 0:
        return 0                                    # ohne Leistungsangabe keine Befreiung
    if leistung > 30 * einheiten:                   # S. 1: 30 kWp je Einheit
        return 0
    if leistung > 100:                              # S. 1: insgesamt höchstens 100 kWp
        return 0
    return einnahmen


def catala_p10b_spenden(s: dict) -> int:
    """§ 10b Abs. 1 S. 1 EStG — abziehbare Zuwendungen (Spenden), EURO (module SpendenAbzug, charge29-
    Promotion): min(zuwendungen; 20 % des Gesamtbetrags der Einkünfte). GdE = est_einzel-Ergebnis VOR
    Sonderausgaben (keine Zirkularität). Roh-Wert speist als Sonderausgabe § 2 Abs. 4 (p32a
    sonderausgaben-Input, additiv); die 4-‰-Umsatz-Alternative + Großspenden-Vortrag sind eigene Nachträge."""
    r = SA.spenden_abzug(SA.SpendenAbzugIn(
        zuwendungen_in=Money(f"{int(s.get('zuwendungen', 0))}.00"),
        gesamtbetrag_der_einkuenfte_in=Money(f"{int(s.get('gesamtbetrag_der_einkuenfte', 0))}.00")))
    return int(r.spenden_abzug) // 100


def _zumutbar_money(s: dict):
    """§ 33 Abs. 3 EStG zumutbare Belastung als Money (Tranchen-Methode post-2017-BFH, je Stufe nur auf ihren
    GdE-Anteil; Sätze 1-7 % nach Kinderzahl/Splitting). NUR über die Regel — die Staffelung ist Steuerlogik,
    NIE Frontend-Rechnung. anzahl_kinder ist Catala-Integer (Python-int bräche den >=-Vergleich im Scope)."""
    return ZB.zumutbare_belastung(ZB.ZumutbareBelastungIn(
        gesamtbetrag_der_einkuenfte_in=Money(f"{int(s.get('gesamtbetrag_der_einkuenfte', 0))}.00"),
        anzahl_kinder_in=Integer(int(s.get("anzahl_kinder", 0))),
        splitting_in=bool(s.get("splitting", False)))).zumutbare_belastung


def catala_p33_zumutbar(s: dict) -> int:
    """§ 33 Abs. 3 EStG zumutbare Belastung, EURO (module ZumutbareBelastung). Reine Exposition der Zwischen-
    größe (Tests/Transparenz) — die agB-Verrechnung macht catala_p33_agb (Money-genau, EIN Rundungsschritt)."""
    return int(_zumutbar_money(s)) // 100


def catala_p33_agb(s: dict) -> int:
    """§ 33 Abs. 1 EStG — abziehbare außergewöhnliche Belastung, EURO (module AgbAbzug): agB-Aufwendungen minus
    zumutbare Belastung (§ 33 Abs. 3), min. 0. Kettet ZumutbareBelastung → AgbAbzug INTERN mit Money-Präzision
    (der fraktionale Tranchen-zumutbar bleibt Money bis zum finalen abzug_agb → EIN Euro-Rundungsschritt). Roh-
    Wert speist § 2 Abs. 4 aussergewoehnliche_belastungen (p32a). anzahl_kinder/splitting → zumutbar-Staffel."""
    r = AG.agb_abzug(AG.AgbAbzugIn(
        aussergewoehnliche_belastungen_in=Money(f"{int(s.get('aussergewoehnliche_belastungen', 0))}.00"),
        zumutbare_belastung_in=_zumutbar_money(s)))
    return int(r.abzug_agb) // 100


def catala_p10_kist(s: dict) -> int:
    """§ 10 Abs. 1 Nr. 4 EStG — abziehbare Kirchensteuer, EURO (module Kirchensteuerabzug): gezahlte minus
    erstattete KiSt. Der Erstattungsüberhang (erstattet > gezahlt, § 10 Abs. 4b) ist ein NICHT materialisierter
    Nachtrag (fehlt die GdE-Hinzurechnung) → die Scheibe sperrt diesen Fall fail-closed (erstattungsueberhang_
    offen), dieser Accessor sieht nur den regulären Fall. Roh-Wert speist § 2 Abs. 4 sonderausgaben (additiv)."""
    r = KI.kirchensteuerabzug(KI.KirchensteuerabzugIn(
        gezahlte_kirchensteuer_in=Money(f"{int(s.get('gezahlte_kirchensteuer', 0))}.00"),
        erstattete_kirchensteuer_in=Money(f"{int(s.get('erstattete_kirchensteuer', 0))}.00")))
    return int(r.abziehbare_kirchensteuer) // 100


# Kirchensteuer-Hebesätze — Landes-KiStG-Beschluss (KEINE EStG-Konstante): 8 % Bayern +
# Baden-Württemberg, 9 % übrige Länder. Amtlich: sources/kirchensteuer/kist_hebesatz_2026-07-22.txt
# (Art. 8 BayKirchStG + Erzbistum München; FinMin-NRW-Erlass 18.04.2024). VZ 2024-2026.
_KIST_BY_BW = ("bayern", "baden_wuerttemberg")                    # 8 %
_KIST_KONFESSION_STEUERERHEBEND = ("evangelisch", "roemisch-katholisch")


def catala_kist(s: dict) -> int:
    """§ 51a EStG i.V.m. Landes-KiStG — Kirchensteuer-Festsetzung auf die Maßstabsteuer, CENT.

    Pipeline-verifiziert (snapshot p51a_kirchensteuer, catala_a): int-kodiert konfession
    0=keine/3=andere → satz 0 ; sonst bundesland Bayern/Baden-Württemberg → 8 % ; sonst 9 %.
    Hier laien-enum-domäniert (Store-Werte). est_mit_fb = § 51a-Bemessungsgrundlage (veranlagte
    ESt mit Kinderfreibetrag = SolZ-§3-Abs.2-Zwilling, ohne § 32d-Abgeltung-Kapital — die
    Abgeltung-KiSt e/(4+k) ist ein eigener Nachtrag), EURO. EURO × Prozent-int = CENT, exakt
    (ganzzahlige EURO-Basis × ganzzahliger Hebesatz → kein Rundungsschnitt)."""
    if str(s.get("konfession", "keine")) not in _KIST_KONFESSION_STEUERERHEBEND:
        return 0
    satz = 8 if str(s.get("bundesland", "")) in _KIST_BY_BW else 9
    return int(s.get("est_mit_fb", 0)) * satz


def catala_p36_abschlusszahlung(s: dict) -> int:
    """§ 36 Abs. 2+4 EStG — Abschlusszahlung (+) / Erstattung (−), CENT.

    Festzusetzende ESt (§ 36 Abs. 1) minus anrechenbare Beträge: durch Steuerabzug erhobene ESt/LSt
    (§ 36 Abs. 2 S. 1 Nr. 2) — auf volle Euro AUFgerundet (§ 36 Abs. 3 S. 1) — minus geleistete
    ESt-Vorauszahlungen (§ 36 Abs. 2 S. 1 Nr. 1, § 37). Überschuss zuungunsten (+) = Abschlusszahlung,
    zugunsten (−) = Erstattung (§ 36 Abs. 4). MVP-AN: nur LSt + VZ — keine KapESt darüber, keine
    Forschungszulage (Nr. 3), kein § 32c (Nr. 4). Snapshot p36_2_anrechnung (verified_bedingt).
    Nur die LSt wird aufgerundet (Abzugsbetrag Abs. 2 Nr. 2); die festgesetzte ESt ist bereits volle
    Euro (§ 32a) und die Vorauszahlungen werden in vollen Euro festgesetzt (§ 37)."""
    lst_cent = int(s.get("lohnsteuer_cent", 0))
    lst_aufgerundet = -(-lst_cent // 100) * 100          # ceil auf volle Euro (§ 36 Abs. 3 S. 1)
    return (int(s.get("festzusetzende_est_cent", 0))
            - lst_aufgerundet - int(s.get("vorauszahlungen_cent", 0)))


def _altersentlastung_kohorte(folgejahr: int):
    """§ 24a S. 5 Kohorten-Staffel (prozentsatz % + hoechstbetrag EURO) je maßgebendem Folgejahr (Jahr NACH der
    Vollendung des 64. Lj), lebenslang fix. Außerhalb der Tabelle geklemmt: vor 2005 → Höchststaffel, ab 2058 → 0."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", "kohorten", "altersentlastungsbetrag_p24a.yaml"), encoding="utf-8"))
    k = p["kohorten"]
    j = min(max(folgejahr, min(k)), max(k))
    return k[j]["prozentsatz"], k[j]["hoechstbetrag"]


def catala_p24a_altersentlastung(s: dict) -> int:
    """§ 24a EStG Altersentlastungsbetrag, EURO (module Altersentlastungsbetrag): min(prozentsatz × (Arbeitslohn
    + positive Summe der übrigen Einkünfte); Höchstbetrag). prozentsatz/Höchstbetrag = Kohorten-Lookup nach dem
    maßgebenden Folgejahr = geburtsjahr + 65 (Jahr nach Vollendung des 64. Lj), lebenslang fix. Leibrenten +
    Versorgungsbezüge sind NICHT Bemessungsgrundlage (§ 24a S. 2) — der Aufrufer speist nur Arbeitslohn + positive
    übrige Einkünfte. prozentsatz als PROZENT (13.2, das Modul teilt /100). Roh → p32a altersentlastungsbetrag
    (§ 2 Abs. 3). geburtsjahr ≤ 0 (unbekannt/nicht erfasst) → Betrag 0 (fail-safe, kein Phantom-Abzug). § 24a S. 3
    (64+-Gate): der Betrag wird erst gewährt, wenn das 64. Lebensjahr VOR Beginn des VZ vollendet ist = maßgebendes
    Folgejahr (geburtsjahr + 65) ≤ VZ; geburtsjahr + 65 > VZ → noch nicht berechtigt → 0 (Jan-1-Tagesgenauigkeit =
    geburtsjahr-only-Näherung, over-tax-safe: 1961-01-01-Geborene erst VZ2026 statt 2025). VZ absent → Gate inaktiv."""
    vz = int(s.get("veranlagungszeitraum", 0))
    geburtsjahr = int(s.get("geburtsjahr", 0))
    if geburtsjahr <= 0 or (vz > 0 and geburtsjahr + 65 > vz):
        return 0
    prozent, hoechst = _altersentlastung_kohorte(geburtsjahr + 65)
    r = AE.altersentlastungsbetrag(AE.AltersentlastungsbetragIn(
        arbeitslohn_in=Money(f"{int(s.get('arbeitslohn', 0))}.00"),
        positive_andere_einkuenfte_in=Money(f"{int(s.get('positive_andere_einkuenfte', 0))}.00"),
        prozentsatz_in=Decimal(str(prozent)),
        hoechstbetrag_in=Money(f"{int(hoechst)}.00")))
    return int(r.altersentlastungsbetrag) // 100


def catala_p24b_entlastung(s: dict) -> int:
    """§ 24b EStG Entlastungsbetrag für Alleinerziehende, EURO (module Entlastungsbetrag): Grundbetrag +
    Erhöhung je weiterem Kind, anteilig um monate_ohne_voraussetzung gekürzt. Nur bei alleinstehend (§ 24b
    Abs. 3) UND mind. 1 berücksichtigungsfähigem Kind. alleinstehend_in ist Catala-Bool (Python-bool bricht
    not_). Roh → p32a entlastungsbetrag_alleinerziehende (§ 2 Abs. 3)."""
    return int(EB.entlastungsbetrag(EB.EntlastungsbetragIn(
        alleinstehend_in=Bool(bool(s.get("alleinstehend", False))),
        anzahl_kinder_in=Integer(int(s.get("anzahl_kinder", 0))),
        monate_ohne_voraussetzung_in=Integer(int(s.get("monate_ohne_voraussetzung", 0))))).entlastungsbetrag) // 100


def catala_p31_familienleistung(s: dict) -> int:
    """§ 31 EStG Familienleistungsausgleich (Günstigerprüfung), EURO (module Familienleistungsausgleich): das
    Kind wird über Kindergeld ODER den Kinderfreibetrag (§ 32 Abs. 6) entlastet — je nachdem, was günstiger ist.
    est_ohne_freibetraege (Tarif OHNE Freibetrag) + est_mit_freibetraegen (MIT) + kindergeld → est_nach_familien-
    ausgleich: Freibetrag-Günstiger → est_mit + Hinzurechnung Kindergeld (§ 31 S. 4); sonst est_ohne (Kindergeld
    bleibt). Der Aufrufer rechnet est_ohne/est_mit (zwei catala_gesamt-Läufe) + leitet kindergeld/freibetrag aus
    params ab. Alle Eingaben EURO."""
    return int(FL.familienleistungsausgleich(FL.FamilienleistungsausgleichIn(
        est_ohne_freibetraege_in=Money(f"{int(s.get('est_ohne_freibetraege', 0))}.00"),
        est_mit_freibetraegen_in=Money(f"{int(s.get('est_mit_freibetraegen', 0))}.00"),
        kindergeld_in=Money(f"{int(s.get('kindergeld', 0))}.00"))).est_nach_familienausgleich) // 100


def catala_p21_2_verbilligt(s: dict) -> int:
    """§ 21 Abs. 2 EStG — abziehbare Werbungskosten bei verbilligter Wohnraumvermietung, EURO (module
    VerbilligteVermietungWk): entgelt_quote ≥ 66 % der ortsüblichen Marktmiete → volle WK; < 66 % → WK ×
    (quote/100). Der 50–66-%-Totalüberschussprognose-Korridor wird konservativ anteilig gekürzt (SAFE-Richtung,
    über- nie unterbesteuert; Voll-WK-bei-positiver-Prognose = benannter Nachtrag). entgelt_quote als PROZENT-
    Decimal (50, das Modul vergleicht ≥ 66 und dividiert /100). Der Tatbestand (Wohnzwecke, auf Dauer) wird VOM
    AUFRUFER gegated (nicht Scope-Input); werbungskosten = die JE OBJEKT summierten Detail-WK."""
    r = VV.verbilligte_vermietung_wk(VV.VerbilligteVermietungWkIn(
        werbungskosten_in=Money(f"{int(s.get('werbungskosten', 0))}.00"),
        entgelt_quote_prozent_in=Decimal(str(int(s.get("entgelt_quote_prozent", 100))))))
    return int(r.abziehbare_werbungskosten) // 100


def catala_p10_kv_pv(s: dict) -> int:
    """§ 10 Abs. 1 Nr. 3/3a EStG — abziehbare Kranken-/Pflegeversicherungs-Vorsorge, EURO (module
    KrankenPflegeVorsorge): Höchstbetrag 1900 (mit Anspruch auf Zuschuss/Beihilfe, z.B. Beamte/AN mit AG-Zuschuss)
    / 2800 (ohne). abziehbar = max(basis_kv_pv; min(basis_kv_pv + weitere; HB)) — der § 10 Abs. 4 S. 4 BASIS-
    DURCHBRUCH: die Basis-KV/PV (Existenzminimum) ist IMMER voll abziehbar, auch über dem Höchstbetrag; nur die
    weiteren Vorsorgeaufwendungen (Haftpflicht etc.) werden gedeckelt. ⚠ basis_kv_pv + weitere GETRENNT (NICHT
    vorsummieren — sonst würde die Basis auf den HB gedeckelt = Über-Besteuerung bei hohen Basis-Beiträgen).
    Roh → sonderausgaben (§ 2 Abs. 4, additiv, EIGENER Abs.4-HB getrennt von Abs.3-Basisvorsorge). mit_anspruch_
    auf_zuschuss ist Catala-Bool (Python-bool bricht not_)."""
    r = KP.kranken_pflege_vorsorge(KP.KrankenPflegeVorsorgeIn(
        basis_kv_pv_in=Money(f"{int(s.get('basis_kv_pv', 0))}.00"),
        weitere_vorsorgeaufwendungen_in=Money(f"{int(s.get('weitere_vorsorgeaufwendungen', 0))}.00"),
        mit_anspruch_auf_zuschuss_in=Bool(bool(s.get("mit_anspruch_auf_zuschuss", False)))))
    return int(r.abziehbare_kv_pv_vorsorge) // 100


def catala_p10_1_7_berufsausbildung(s: dict) -> int:
    """§ 10 Abs. 1 Nr. 7 EStG — Aufwendungen für die eigene Berufsausbildung, EURO (module
    Berufsausbildungsaufwendungen): abziehbare Sonderausgaben = min(aufwendungen, 6000) — Höchstbetrag je Person
    (§ 10 Abs. 1 Nr. 7 S. 1). Roh → sonderausgaben (§ 2 Abs. 4, additiv, wie § 10b/KV-PV/KiSt Person-A). Read-Key
    berufsausbildung_aufwendungen (feld_id 1:1 dev-2-Binding) → Modul-Slot aufwendungen. Satz 2 (je-Person
    Ehegatten) = Person-B-Nachtrag wie A.2; Satz 3/4 (auswärtige Unterbringung / § 4/§ 9-Verweise) = Backlog."""
    r = BA.berufsausbildung(BA.BerufsausbildungIn(
        aufwendungen_in=Money(f"{int(s.get('berufsausbildung_aufwendungen', 0))}.00")))
    return int(r.abziehbare_sonderausgaben) // 100


def catala_p16_4_freibetrag(s: dict) -> int:
    """§ 16 Abs. 4 EStG — Freibetrag für den Betriebsveräußerungs-/-aufgabegewinn, EURO (module BetriebsFreibetrag):
    freibetrag = 45000 − max(0, veräußerungsgewinn − 136000), 0 bei gewinn ≤ 0 (voll abgeschmolzen ab 181000). ROHER
    Freibetrag (ungecappt) — der Aufrufer floort den steuerbaren Rest bei 0 (max(0, vg − fb); § 16 Abs. 4 „soweit
    nicht übersteigt", FB > vg erzeugt keinen Verlust). Read-Key rentner_veraeusserungsgewinn (feld_id 1:1
    dev-2-Binding, NICHT bare veraeusserungsgewinn = § 8b-KStG-Kollision) → Modul-Slot veraeusserungsgewinn_in.
    Alters-55/Behinderung-Gate (§ 16 Abs. 4 S. 1) + einmal-je-Leben (S. 2) = fail-closed in api.py (_sperrgrund),
    Felder rentner_alter_55_oder_berufsunfaehig + rentner_freibetrag_erstmalig."""
    r = BF.betriebs_freibetrag(BF.BetriebsFreibetragIn(
        veraeusserungsgewinn_in=Money(f"{int(s.get('rentner_veraeusserungsgewinn', 0))}.00")))
    return int(r.freibetrag) // 100


def catala_euer_gewinn(s: dict) -> int:
    """§ 4 Abs. 3 EStG — Gewinn aus Einnahmenüberschussrechnung, EURO (module EuerGewinn): gewinn =
    betriebseinnahmen − betriebsausgaben. Kann NEGATIV sein (Verlustjahr — § 4 Abs. 3 lässt den Verlust zu,
    der im § 2 Abs. 3-Ausgleich andere Einkünfte mindert). betriebsausgaben = AGGREGAT: der Aufrufer summiert
    sonstige_betriebsausgaben + afa_jahresbetrag (+ GWG-Σ = Stufe-2b-Nachtrag) und übergibt die Summe. Accessor
    nimmt EUROS (die //100-Umrechnung liegt im slot_fn — wie catala_p16_4_freibetrag/catala_p10_1_7). Die 7
    EÜR-Geltungsbedingungen (§ 4 Abs. 3 Buchführung/durchlaufende Posten, § 12 Nr. 1/3, § 15 Abs. 2/§ 18 Abs. 1
    Einkunftsart) sind Netz-Eingabe-Annahmen; die § 13-LuF-Grenze fängt der luf_euer_offen-Guard der Haut."""
    r = EG.euer_gewinn(EG.EuerGewinnIn(
        betriebseinnahmen_in=Money(f"{int(s.get('betriebseinnahmen', 0))}.00"),
        betriebsausgaben_in=Money(f"{int(s.get('betriebsausgaben', 0))}.00")))
    return int(r.gewinn) // 100


def catala_mitunternehmer_einkuenfte(s: dict) -> int:
    """§ 15 Abs. 1 S. 1 Nr. 2 EStG — Einkünfte als Mitunternehmer, EURO (module MitunternehmerEinkuenfte):
    gewinnanteil + verguetung_taetigkeit + verguetung_darlehen + verguetung_ueberlassung (Gewinnanteil +
    3 Sondervergütungen: Tätigkeit/Darlehen/Überlassung, reine Addition, § 15 Abs. 1 S. 1 Nr. 2 S. 1). gewinnanteil =
    der § 15a-ausgleichsfähige Anteil (Feststellungsbescheid, § 15a Abs. 4) → KANN NEGATIV sein (Verlust-Anteil,
    § 15 Abs. 3 S. 2), roh summiert — die § 15a-Verlustbeschränkung liegt IM Input (wie gewst_messbetrag/
    verlustvortrag_bestand FA-festgestellt). Sondervergütungen ≥ 0 (Zuflüsse). Accessor nimmt EUROS (slot_fn //100,
    p10_1_7/euer-Konvention). Naht → einkuenfte_gewinn (§ 15 gewerblich, Anlage G)."""
    r = ME.mitunternehmer_einkuenfte(ME.MitunternehmerEinkuenfteIn(
        gewinnanteil_in=Money(f"{int(s.get('gewinnanteil', 0))}.00"),
        verguetung_taetigkeit_in=Money(f"{int(s.get('verguetung_taetigkeit', 0))}.00"),
        verguetung_darlehen_in=Money(f"{int(s.get('verguetung_darlehen', 0))}.00"),
        verguetung_ueberlassung_in=Money(f"{int(s.get('verguetung_ueberlassung', 0))}.00")))
    return int(r.einkuenfte_mitunternehmer) // 100


def catala_p6_2_gwg(s: dict) -> int:
    """§ 6 Abs. 2 EStG — Sofortabzug geringwertiger Wirtschaftsgüter (GWG) EINES Assets, EURO (module
    GwgSofortabzug): sofortabzug = (anschaffungskosten_netto ≤ 800) ? netto : 0. PER ASSET (der Aufrufer Σ-t
    über alle GWG-Instanzen, EM.instanzen(gwg)). > 800 netto → 0 (kein GWG, muss über AfA/afa_jahresbetrag).
    Read-Key gwg_anschaffungskosten_netto (feld_id 1:1 dev-2-Binding, netto = § 9b-bereinigt = Deklarations-
    Annahme). Accessor nimmt EUROS (die //100-Umrechnung liegt im slot_fn — wie catala_p16_4/euer_gewinn). LAZY
    Modul-Import: erst bei erstem Aufruf (nach dev-2s p6_2-Promotion/-Export), damit runner.py auch vorher lädt."""
    from pkg import GwgSofortabzug as GW  # noqa: E402  (lazy — § 6 Abs. 2 GWG-Sofortabzug, 2-III)
    r = GW.gwg_sofortabzug(GW.GwgSofortabzugIn(
        anschaffungskosten_netto_in=Money(f"{int(s.get('gwg_anschaffungskosten_netto', 0))}.00")))
    return int(r.sofortabzug) // 100


def catala_p7_linear_afa(s: dict) -> int:
    """§ 7 Abs. 1 EStG — lineare AfA für ein Wirtschaftsgut > 800 EUR, EURO (Pure-Python).
    Anschaffungskosten in Cent, Nutzungsdauer in Jahren (integer). AfA-Betrag =
    anschaffungskosten_cent // (nutzungsdauer * 100) — Euro-gleichmäßig über die Laufzeit
    verteilt, floor (konservativ/over-tax-safe). Nutzungsdauer ≤ 0 → Abzug 0."""
    ak_cent = int(s.get("anschaffungskosten_cent", 0))
    nd = int(s.get("nutzungsdauer", 0))
    if nd <= 0 or ak_cent <= 0:
        return 0
    nd_cent = nd * 100
    return ak_cent // nd_cent


# -- Kapital § 20 / § 32d (Weg A — kein callable Catala-Scope im pkg). EURO. --

def _sparer_pauschbetrag(year: int) -> int:
    """§ 20 Abs. 9 S. 1 EStG Sparer-Pauschbetrag (je Person, 1000) aus params/<vz>."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "sparer_pauschbetrag_p20_9.yaml"), encoding="utf-8"))
    return p["wert"]["wert"]


def _abgeltungssatz(year: int) -> int:
    """§ 32d Abs. 1 S. 1 EStG Abgeltungsteuersatz in Prozent (25) aus params/<vz>."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "abgeltungssatz_p32d.yaml"), encoding="utf-8"))
    return p["wert"]["wert"]


def catala_sparer_pb(s: dict) -> int:
    """§ 20 Abs. 9 EStG — Kapitalerträge nach Sparer-Pauschbetrag, EURO. WÖRTLICHE Transkription des
    Registry-Rechenwegs p20_9_sparer_pauschbetrag: einkuenfte_nach_sparer_pb = max(0, kapitalertraege −
    pausch); pausch = Sparer-Pauschbetrag (bei Zusammenveranlagung verdoppelt, § 20 Abs. 9 S. 3). Wert
    aus params/<vz> (nie hardcoden). KOPPLUNG: bei Registry-Änderung p20_9 nachziehen; Konsistenz-Gate."""
    year = s["veranlagungszeitraum"]
    pausch = _sparer_pauschbetrag(year) * (2 if s.get("zusammenveranlagung") else 1)
    return max(0, int(s.get("kapitalertraege", 0)) - pausch)


def catala_kapital_verrechnung(s: dict) -> int:
    """§ 20 Abs. 6 EStG — verrechnete Kapitaleinkünfte (zwei GETRENNTE Verlust-Töpfe, je Boden 0), EURO.
    WÖRTLICHE Transkription p20_6_verlustverrechnung: saldo_aktien = max(0, gewinn_aktien − verlust_aktien);
    saldo_sonstige = max(0, gewinn_sonstige − verlust_sonstige); verrechnete = saldo_aktien + saldo_sonstige.
    KEINE topfübergreifende Verrechnung (Aktienverlust mindert nicht den sonstigen Gewinn). Verlustvortrag
    § 10d = benannter GAP. KOPPLUNG: bei Registry-Änderung p20_6 nachziehen; Konsistenz-Gate."""
    saldo_aktien = max(0, int(s.get("gewinn_aktien", 0)) - int(s.get("verlust_aktien", 0)))
    saldo_sonstige = max(0, int(s.get("gewinn_sonstige", 0)) - int(s.get("verlust_sonstige", 0)))
    return saldo_aktien + saldo_sonstige


def catala_kapital_steuer(s: dict) -> int:
    """§ 32d Abs. 1 EStG — Kapital-Steuer via Günstigerprüfung (Abs. 6), EURO. WÖRTLICHE Transkription
    p32d_1_abgeltung: abgeltung = satz% × kapitaleinkuenfte; guenstiger_delta = est_regulaer_mit_kap −
    est_regulaer_ohne_kap; kapital_steuer = min(abgeltung, guenstiger_delta). Die zwei est-Größen liefert
    der AUFRUFER (§ 2-Integration, zwei catala_gesamt-Läufe mit/ohne Kapital im zvE) — hier NICHT gerechnet
    (Registry-hinweis). Satz aus params/<vz>. KOPPLUNG: bei Registry-Änderung p32d_1 nachziehen; Konsistenz-Gate."""
    kap = int(s.get("kapitaleinkuenfte", 0))
    abgeltung = kap * _abgeltungssatz(s["veranlagungszeitraum"]) // 100
    delta = int(s.get("est_regulaer_mit_kap", 0)) - int(s.get("est_regulaer_ohne_kap", 0))
    return min(abgeltung, delta)


# -- Rente § 22 (Weg A — kein callable Catala-Scope). EURO. --------------------

class RentenfreibetragFixierungOffen(Exception):
    """§ 22 Nr. 1 S. 3 aa — aa-Folgejahr OHNE fixierten Rentenfreibetrag: fail-closed, kein Bescheid
    (ab dem 2. Jahr ist der Freibetrag in EURO fix; %×erhoehte Jahresrente wuerde unterbesteuern, K2)."""


AA_RENTEN_ARTEN = frozenset({"gesetzliche_rente", "berufsstaendische_versorgung", "private_basisrente"})
BB_RENTEN_ARTEN = frozenset({"private_leibrente", "sonstige_leibrente"})


def _rente_besteuerungsanteil(jahr: int) -> float:
    """§ 22 Nr. 1 S. 3 aa — Besteuerungsanteil je Rentenbeginn-Kohorte (params/kohorten, VZ-agnostisch)."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", "kohorten", "rente_besteuerungsanteil_p22.yaml"), encoding="utf-8"))
    return p["kohorten"][jahr]["besteuerungsanteil_prozent"]


def _rente_ertragsanteil(alter: int) -> float:
    """§ 22 Nr. 1 S. 3 a bb — Ertragsanteil je Alter bei Rentenbeginn (params/kohorten, VZ-agnostisch)."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", "kohorten", "rente_ertragsanteil_p22.yaml"), encoding="utf-8"))
    return p["kohorten"][alter]["ertragsanteil_prozent"]


def _renten_wk_pb(year: int) -> int:
    """§ 9a S. 1 Nr. 3 EStG Werbungskosten-Pauschbetrag für Renten (102) aus params/<vz>."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "renten_werbungskostenpauschbetrag_p9a.yaml"), encoding="utf-8"))
    return p["wert"]["wert"]


def _renten_stpfl(jahresrente: int, prozent: float) -> int:
    """§ 22 Nr. 1 S. 3 — steuerpflichtiger Anteil = (prozent/100) × jahresrente, EURO. WÖRTLICHE
    Transkription p22_1_leibrente_besteuerungsanteil-Arithmetik (Division /100 in decimal, Cent-Schnitt
    zuletzt). Integer-genau für Prozentwerte mit ≤ 1 Nachkommastelle (aa- und bb-Tabellen). KOPPLUNG:
    Konsistenz-Gate bindet diese Multiplikation an die p22_1-test_seeds (prozent-agnostisch)."""
    return jahresrente * round(prozent * 10) // 1000


def catala_renten_einkuenfte(s: dict) -> int:
    """§ 22 Nr. 1 EStG — steuerpflichtige Renten-Einkünfte, EURO → einkuenfte_sonstige (catala_gesamt).
    4 Zweige (Instructor-Ruling, K2 Rentenfreibetrag-Fixierung):
      bb (private_leibrente/sonstige_leibrente): jahresrente × Ertragsanteil%(alter) − WK-PB (exakt, alle Jahre).
      aa Erstjahr (renten_beginn_jahr == VZ): jahresrente × Besteuerungsanteil%(Kohorte) − WK-PB.
      aa Folgejahr MIT rentenfreibetrag: (jahresrente − rentenfreibetrag) − WK-PB (Freibetrag EUR-fix ab Jahr 2).
      aa Folgejahr OHNE rentenfreibetrag: RentenfreibetragFixierungOffen (fail-closed, kein Rate-Bescheid).
    Prozente aus params/kohorten (dev-2, VZ-agnostisch), WK-PB aus params/<vz>. sonstige renten_art = GAP."""
    year = s["veranlagungszeitraum"]
    art = s["renten_art"]
    rente = int(s.get("jahresrente", 0))
    wk_pb = _renten_wk_pb(year)
    if art in BB_RENTEN_ARTEN:
        stpfl = _renten_stpfl(rente, _rente_ertragsanteil(int(s["alter_bei_rentenbeginn"])))
    elif art in AA_RENTEN_ARTEN:
        beginn = int(s["renten_beginn_jahr"])
        if beginn == year:
            stpfl = _renten_stpfl(rente, _rente_besteuerungsanteil(beginn))
        elif beginn < year and s.get("rentenfreibetrag") is not None:
            stpfl = rente - int(s["rentenfreibetrag"])
        else:
            raise RentenfreibetragFixierungOffen(f"aa-Folgejahr {beginn}<{year} ohne fixierten Rentenfreibetrag")
    else:
        raise ValueError(f"renten_art {art!r} nicht ring-fähig (MVP: aa+bb; sonstige = GAP)")
    return max(0, stpfl - wk_pb)


# -- § 33b Pauschbeträge (Behinderung / Pflege / Hinterbliebene), Weg A, EURO. ---

def _p33b_params(year: int) -> dict:
    """§ 33b Pauschbetrag-Tabellen aus params/<vz> (2021er-Reform-Fassung)."""
    return load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "behinderten_pauschbetrag_p33b.yaml"), encoding="utf-8"))


def catala_behinderten_pb(s: dict) -> int:
    """§ 33b Abs. 2/3 EStG — Behinderten-Pauschbetrag, EURO. WÖRTLICHE Transkription p33b_behinderten_
    pauschbetrag: Blinde/Taubblinde/Hilflose → Höchstbetrag (Abs. 3 S. 3, ersetzt die Staffel); sonst GdB <
    20 → 0; sonst GdB-Staffel (Abs. 3 S. 2, Tier-Floor gdb//10*10). Werte aus params/<vz>. Konsistenz-Gate."""
    p = _p33b_params(s["veranlagungszeitraum"])
    if s.get("ist_hilflos_blind_taubblind"):
        return p["blind_hilflos_taubblind"]
    gdb = int(s.get("grad_der_behinderung", 0))
    if gdb < 20:
        return 0
    return p["gdb_staffel"][min(100, (gdb // 10) * 10)]


def catala_pflege_pb(s: dict) -> int:
    """§ 33b Abs. 6 EStG — Pflege-Pauschbetrag, EURO. WÖRTLICHE Transkription p33b_pflege_pauschbetrag:
    ist_hilflos hat VORRANG (→ 1.800, unabhängig vom Pflegegrad); sonst Pflegegrad-Staffel (PG2 600, PG3
    1.100, PG4/5 1.800, sonst 0). Werte aus params/<vz>. Konsistenz-Gate."""
    p = _p33b_params(s["veranlagungszeitraum"])
    if s.get("ist_hilflos"):
        return p["pflege_hilflos"]
    return p["pflege_staffel"].get(int(s.get("pflegegrad", 0)), 0)


def catala_hinterbliebenen_pb(s: dict) -> int:
    """§ 33b Abs. 4 EStG — Hinterbliebenen-Pauschbetrag (370), EURO. Wert aus params/<vz>. Konsistenz-Gate."""
    p = _p33b_params(s["veranlagungszeitraum"])
    return p["hinterbliebenen"] if s.get("hat_hinterbliebenenbezuege") else 0


# -- §33a EStG (Unterhalt Abs.1 + Ausbildungsfreibetrag Abs.2). EURO. --------------
# Pure-Python. p33a_unterhalt: Hand-geschriebenes Catala-Modul (rules/estg/p33a_unterhalt/),
# aber NICHT in clerk.toml → kein Python-Build → Pure-Python (wie solzg/p34c).
# p33a_ausbildungsfreibetrag: verified_bedingt Snapshot, A==B (1200€ Festbetrag).

_GFB_33A = {2024: 11784, 2025: 12096, 2026: 12348}


def catala_p33a_unterhalt(s: dict) -> int:
    """§33a Abs.1 EStG — Unterhaltsabzug, EURO. Höchstbetrag = GFB + kv_pv − max(0, andere_einkuenfte − 624),
    return min(aufwendungen, Höchstbetrag). Schonbetrag 624€ fassungskonstant, GFB driftet per VZ.
    5 pipeline-seeds fidel (10k/0/0→10000, 15k/0/0→12348 GFB-Deckel, etc.). Accessor nimmt EUROS."""
    year = s["veranlagungszeitraum"]
    assert year in _GFB_33A, f"§33a GFB VZ {year}: nur 2024-2026"
    gfb = _GFB_33A[year]
    aufw = int(s["aufwendungen"])
    kv_pv = int(s.get("kv_pv_beitraege", 0))
    andere = int(s.get("andere_einkuenfte_bezuege", 0))
    anrechnung = max(0, andere - 624)
    hoechstbetrag = max(0, gfb + kv_pv - anrechnung)
    return min(aufw, hoechstbetrag)


def catala_p33a_ausbildungsfreibetrag(s: dict) -> int:
    """§33a Abs.2 EStG — Ausbildungsfreibetrag, 1200€ je Kind, EURO. Per-Kind-Komposition:
    der Snapshot (1200€ per-unit) bleibt fidel; der Ring multipliziert mit anzahl_kinder.
    Haelftelung Eltern (Abs.2 S.4) + Monats-Zwoelftelung (Abs.3) = Stufe-2-Backlog."""
    return int(s.get("anzahl_kinder", 0)) * 1200


def catala_p10_1_5_kinderbetreuung(s: dict) -> int:
    """§10 Abs.1 Nr.5 EStG — Kinderbetreuungskosten, 80% capped 4800€ je Kind, EURO.
    Multi-Kind-Komposition: anzahl_kinder * min(aufwand_pro_kind * 0.8, 4800).
    Accessor nimmt EUROS. Seeds: 6000/1→4800, 10000/2→9600, 1000/1→800."""
    anzahl = int(s.get("anzahl_kinder", 0))
    aufw = int(s.get("aufwendungen", 0))
    if anzahl <= 0:
        return 0
    # Pro-Kind-Betrachtung (Schnitt): wir nehmen an, dass 'aufwendungen' die SUMME sind
    # und verteilen sie gleichmäßig (Worst-Case für Deckelung wenn ungleich, aber ELSTER
    # fragt oft Summe ab). Gesetz sagt: höchstens 4800 je Kind.
    # Falls 'aufwendungen' pro Kind gemeint ist, müsste das Wiring das regeln.
    # Hier: aufwendungen / anzahl = aufwand_pro_kind.
    aufwand_pro_kind = aufw // anzahl
    abzug_pro_kind = min(int(aufwand_pro_kind * 0.8), 4800)
    return abzug_pro_kind * anzahl


# -- §10 Abs.1a Nr.1 EStG Realsplitting (Unterhalt Ex-Ehegatte). EURO. -----------
# Pure-Python aus verified_bedingt Snapshot (p10_1a_realsplitting.json).
# 4 pipeline-seeds fidel. Abzug = min(unterhaltsleistungen, 13.805 + kv_pv_beitraege).
def catala_p10_1a_realsplitting(s: dict) -> int:
    """§10 Abs.1a Nr.1 EStG — Realsplitting, Unterhalt Ex-Ehegatte bis 13.805€ + KV/PV.
    Accessor nimmt EUROS. Seeds: 10k/0→10k, 15k/0→13805, 15k/2k→15k, 20k/0→13805."""
    unterhaltsleistungen = int(s.get("unterhaltsleistungen", 0))
    kv_pv_beitraege = int(s.get("kv_pv_beitraege", 0))
    deckel = 13805 + kv_pv_beitraege
    return min(unterhaltsleistungen, deckel)


# -- §32b EStG Progressionsvorbehalt (Abs.1 Nr.1 Lohnersatz). EURO. -----------------
# Pure-Python aus verified_bedingt Snapshot (p32b_progressionsvorbehalt.json).
# 3 pipeline-seeds fidel. Besonderer Steuersatz = est_auf_erhoehte / erhoehte, applied
# to zvE. Post-Engine-Wrapper (NICHT tarif_modifiziert, Scheibe-Isolation).

def catala_p32b_1(s: dict) -> int:
    """§32b Abs.1 Nr.1/2 EStG — Progressionsvorbehalt, EURO (cent-floor via integer arith).
    erhoehte_bemessung = zvE + progressionseinkuenfte; besonderer_steuersatz =
    est_auf_erhoehte_bemessung / erhoehte_bemessung (0 wenn 0); ret = satz * zvE.
    Accessor nimmt EUROS. Seeds: 30000/10000/7209→5406, 30000/0/4217→4217, 10000/5000/435→290."""
    zvE = int(s["zu_versteuerndes_einkommen"])
    pe = int(s["progressionseinkuenfte"])
    est_erhoeht = int(s["est_auf_erhoehte_bemessung"])
    erhoehte = zvE + pe
    if erhoehte <= 0:
        return 0
    return est_erhoeht * zvE // erhoehte


def _kindergeld(year: int) -> int:
    """Monatliches Kindergeld je Kind aus params/<vz> (§ 66 EStG): 250/255/259."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "kindergeld_p66.yaml"), encoding="utf-8"))
    return p["kindergeld_monatlich_je_kind"]["wert"]


def _kinderfreibetrag(year: int, veranlagung) -> int:
    """§ 32 Abs. 6 EStG Kinderfreibetrag JE KIND (sächliches Existenzminimum + BEA-Freibetrag je Elternteil),
    EURO, aus params/<vz>. Einzelveranlagung = ein Elternteil-Anteil; Zusammenveranlagung = verdoppelt (Abs. 2).
    Basis der § 31-Günstigerprüfung (Kinderfreibetrag-Wirkung vs. Kindergeld)."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "kinderfreibetrag_p32.yaml"), encoding="utf-8"))
    je_elternteil = p["kinderfreibetrag_je_elternteil"]["wert"] + p["bea_freibetrag_je_elternteil"]["wert"]
    return je_elternteil * (2 if veranlagung == "zusammen" else 1)


def _vorsorge_hb(year: int) -> int:
    """Vorsorge-Hoechstbetrag aus params/<vz> (§ 10 Abs. 3): 27566/29344/30826.
    Deckel-Eingabe von p10_1_2_altersvorsorge; ueber _vorsorge_abzug in den
    Sonderausgaben-Pfad von catala_gesamt verdrahtet."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "vorsorge_hoechstbetrag_p10.yaml"), encoding="utf-8"))
    return p["hoechstbeitrag"]["wert"]


def _vorsorge_abzug(s: dict, year: int) -> int:
    """§ 10 Abs. 1 Nr. 2, Abs. 3: abziehbare Altersvorsorge (p10_1_2, MVP 100 %-
    Ansatz). Die Gesamtbeitraege (inkl. AG-Anteil) werden auf den Hoechstbeitrag zur
    knappschaftl. RV aus params/<vz> gedeckelt, dann um den steuerfreien AG-Anteil
    gekuerzt - Kuerzung NACH dem Cap (Instructor-Semantik msg 1197). Nur aktiv, wenn
    ein Vorsorge-Beitrag gesetzt ist; sonst 0 (Rueckwaertskompatibilitaet)."""
    beitraege = int(s.get("vorsorge_gesamtbeitraege_inkl_ag", 0))
    if not beitraege:
        return 0
    ag = int(s.get("vorsorge_ag_anteil_steuerfrei", 0))
    return max(0, min(beitraege, _vorsorge_hb(year)) - ag)


def _sonderausgabenpauschbetrag(year: int) -> int:
    """§ 10c Satz 1 EStG Sonderausgaben-Pauschbetrag (je Person, 36) aus params/<vz>."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "sonderausgabenpauschbetrag.yaml"), encoding="utf-8"))
    return p["wert"]["wert"]


def _sonderausgaben_final(actual_sa: int, year: int, veranlagung) -> int:
    """§ 10c EStG Guenstigervergleich: mindestens der Sonderausgaben-Pauschbetrag (36 je Person,
    bei Zusammenveranlagung verdoppelt, § 10c Satz 2) - Wert aus params/<vz> (nie hardcoden).
    Der einzel/zusammen-Tarif buendelt § 10c intern (Engine-Groesse sonderausgaben_pauschbetrag);
    der Gesamt-Scope nimmt die FINALEN Sonderausgaben (Vertrag: Aufrufer bildet den § 10c-Floor,
    festzusetzende_est_gesamt.py subtrahiert nur). KOPPLUNG: das Konsistenz-Gate haelt diesen Wert
    an der Engine-Groesse fest."""
    pausch = _sonderausgabenpauschbetrag(year) * (2 if veranlagung == "zusammen" else 1)
    return max(int(actual_sa), pausch)


# -- GewSt-Kette (Paket 4, §§ 6-11/35) - Python-Andockung analog _vorsorge_abzug.
# Macht den GewSt-Steuermessbetrag (p11) + § 35-Anrechnung (p35_1) produktiv, ohne
# p35_1 strukturell zu beruehren. Rechnung in ganzen CENT (exakt); die catala-Regeln
# p7/p8/p9_*/p10a/p11 sind separat via clerk/snapshot verifiziert. Erwartungen =
# dev-2-Hand-Ketten (reports/review/2026-07-16-gewst-ketten-golden.md), unabhaengige
# Zweitrechnung.

def _gewst_hinzurechnung_p8(s: dict) -> int:
    """§ 8 Nr. 1: Viertel der um 200.000 geminderten Summe der Finanzierungsanteile
    (a/b/c 100 %, d 1/5, e 1/2, f 1/4). Ganze Euro."""
    summe = (int(s.get("gewst_entgelte_schulden", 0))
             + int(s.get("gewst_renten", 0))
             + int(s.get("gewst_stille", 0))
             + int(s.get("gewst_miet_beweglich", 0)) // 5
             + int(s.get("gewst_miet_unbeweglich", 0)) // 2
             + int(s.get("gewst_rechte", 0)) // 4)
    return max(0, summe - 200000) // 4


def _gewst_kuerzung_p9(s: dict, year: int) -> int:
    """§ 9 Nr. 1 S. 1 (VZ-Split: EZ 2024 = 1,2 % Einheitswert; EZ 2025+ =
    tatsaechliche Grundsteuer) + Nr. 2 Gewinnanteile + Nr. 2a Schachtel. Ganze Euro."""
    if year <= 2024:
        grund = int(s.get("gewst_einheitswert", 0)) * 12 // 1000  # 1,2 %
    else:
        grund = int(s.get("gewst_grundsteuer", 0))
    return (grund + int(s.get("gewst_gewinnanteile_mitunternehmer", 0))
            + int(s.get("gewst_schachteldividenden", 0)))


def _gewst_messbetrag_cent(s: dict) -> int:
    """§ 7 -> § 10a -> § 11: Steuermessbetrag in CENT. Gewerbeertrag = Gewinn +
    § 8 - § 9; optional § 10a-Verlustabzug (1-Mio-Sockel + 60 %); dann abrunden auf
    volle 100 Euro, Freibetrag 24.500 (natuerl. Person/PersG), Messzahl 3,5 %."""
    year = int(s["veranlagungszeitraum"])
    ge = (int(s.get("gewinn_gewerbebetrieb", 0)) + _gewst_hinzurechnung_p8(s)
          - _gewst_kuerzung_p9(s, year))
    fehlbetrag = int(s.get("fehlbetrag_bestand", 0))
    if fehlbetrag:
        kapazitaet = 1000000 + max(0, ge - 1000000) * 60 // 100
        ge -= min(fehlbetrag, kapazitaet)
    abgerundet = (ge // 100) * 100 if ge > 0 else 0
    nach_fb = max(0, abgerundet - 24500)
    return nach_fb * 35 // 10   # * 3,5 % in Cent (nach_fb Vielfaches von 100 -> exakt)


def _gewst_p35_anrechnung_cent(s: dict) -> int:
    """§ 35 EStG (p35_1): min(4x Messbetrag, tatsaechlich zu zahlende GewSt =
    Messbetrag x Hebesatz). Deckel 3 (Ermaessigungshoechstbetrag) = ESt-Kontext,
    ausserhalb der reinen GewSt-Kette (dev-2). Hebesatz in Prozent. CENT."""
    mb = _gewst_messbetrag_cent(s)
    vierfach = mb * 4
    gewst = mb * int(s["gewst_hebesatz"]) // 100
    return min(vierfach, gewst)


def catala_gewst(s: dict) -> int:
    """Dispatch der GewSt-Kette: Steuermessbetrag ODER § 35-Anrechnung, in CENT."""
    if s.get("gewst_output") == "p35_anrechnung":
        return _gewst_p35_anrechnung_cent(s)
    return _gewst_messbetrag_cent(s)


# --- KStG Nenner B (Kapitalgesellschaft, Paket 5, K1-K5 + § 23 + SolZ + GewSt) -----
# Alle Zwischengroessen in CENT (Klasse-5, Cent-Schnitt zuletzt, // = floor). Kette:
# einkommen_slot (§ 8 Abs.1/3) - § 8b-netto + § 10-Addback = einkommen_vor_spenden
# -> - § 9-Spenden = GdE (§ 10d-Basis) -> - § 10d-Verlust (70%) = zvE (§ 24-FB=0)
# -> KSt 15 % (§ 23) + SolZ 5,5 % + GewSt (Slot = einkommen_vor_spenden).

def _kst_8b_netto_cent(s: dict) -> int:
    """§ 8b: Bezuege (Abs.1) + Veraeusserung (Abs.2) 100 % ausser Ansatz MINUS 5 %
    Pauschale (Abs.5/3) = 95 % netto. Streubesitz < 10 % (Abs.4): Dividende voll
    steuerpflichtig (steuerfrei=0, Pauschale faellt mit). 95 % NIE als Konstante. CENT."""
    div = int(s.get("dividende_bezuege", 0)) * 100
    steuerfrei = div if int(s.get("beteiligung_prozent", 0)) >= 10 else 0
    veraeuss = int(s.get("veraeusserungsgewinn", 0)) * 100
    ausser = steuerfrei + veraeuss
    return ausser - ausser * 5 // 100


def _kst_massgebliches_einkommen_cent(s: dict) -> int:
    """§ 8 Abs.1-Slot + vGA (Abs.3 S.2) - verd. Einlage (Abs.3 S.3), - § 8b-netto,
    + § 10-Addback (Nr.2/3). = maßgebliches Einkommen § 8a Abs.1 S.2 (PRE-§ 4h/§ 9/§ 10d):
    Basis fuer die § 4h-EBITDA-Herleitung (N3) UND fuer den § 4h-Add-back. CENT."""
    slot = (int(s.get("gewinn_estg", 0)) + int(s.get("verdeckte_gewinnausschuettung", 0))
            - int(s.get("verdeckte_einlage", 0))) * 100
    addback = (int(s.get("personensteuern", 0)) + int(s.get("geldstrafen", 0))) * 100
    return slot - _kst_8b_netto_cent(s) + addback


def _kst_nichtabziehbare_zinsen_cent(s: dict) -> int:
    """§ 4h/§ 8a Zinsschranke-Add-back (N1-N3). ebitda_basis = maßgebliches Einkommen +
    Zinsaufwand + AfA - Zinsertrag (§ 8a: Einkommen statt Gewinn). Deckel = 30 % ebitda_basis
    + EBITDA-Vortrag; zinsaufwand_eff = Zinsaufwand + Zinsvortrag (nur Abzugstest). N2-Ausnahme:
    Freigrenze nettozins < 3 Mio ODER Konzern-lit-b ODER Escape-lit-c -> voller Abzug.
    ADD-BACK = gebuchter Zinsaufwand - abziehbar_final, UNGEKLEMMT (Vortrags-Verbrauch senkt
    das Einkommen, § 4h Abs.1 S.6; der Zinsvortrag war nie im gewinn_estg abgezogen -> NICHT
    im Add-back-Grundbetrag). CENT.

    S.-7-LATENZ (benannter Nachtrag, Backlog): greift eine Abs.-2-Ausnahme (Freigrenze/
    Konzern/Escape), gibt dieser Zweig zinsaufwand_eff INKL. Zinsvortrag frei -> bei
    zins_vortrag_bestand > 0 waere der Add-back negativ (Einkommen sinkt um den Vortrag),
    was § 4h Abs.1 S.7 ("Absatz 2 findet keine Anwendung, soweit Zinsaufwendungen aufgrund
    eines Zinsvortrags erhoeht wurden") anteilig ausschliesst. KEIN Golden kombiniert Ausnahme
    UND Zinsvortrag > 0 (S.-7-Anteiligkeit nicht modelliert, N2-Bedingung
    freigrenze_zinsvortrag_anteiligkeit_s7_nachtrag). Ein echter Tripwire braucht
    Negativ-zvE-Handling -> Backlog, hier NICHT gebaut."""
    zinsauf = int(s.get("zinsaufwand", 0)) * 100
    zinsert = int(s.get("zinsertrag", 0)) * 100
    afa = int(s.get("abschreibungen", 0)) * 100
    zvortrag = int(s.get("zins_vortrag_bestand", 0)) * 100
    evortrag = int(s.get("ebitda_vortrag_bestand", 0)) * 100
    ebitda_basis = _kst_massgebliches_einkommen_cent(s) + zinsauf + afa - zinsert
    verr_ebitda = ebitda_basis * 30 // 100
    zinsauf_eff = zinsauf + zvortrag
    deckel = verr_ebitda + evortrag
    abziehbar_kern = zinsert + min(zinsauf_eff - zinsert, deckel)
    nettozins = zinsauf - zinsert
    ausnahme = (nettozins < 3000000 * 100
                or bool(s.get("keine_konzern_oder_nahestehende_b"))
                or bool(s.get("eigenkapital_escape_c")))
    abziehbar_final = zinsauf_eff if ausnahme else abziehbar_kern
    return zinsauf - abziehbar_final


def _kst_verlustbestand_nach_8c_8d_cent(s: dict) -> int:
    """§ 8c (N4) / § 8d (N5) auf den Verlustbestand VOR § 10d. Schaedlicher Erwerb -> 0;
    § 8d-Suspension (schaedlich UND Antrag UND Fortfuehrungs-Voraussetzungen) -> voller
    Bestand erhalten (fortfuehrungsgebunden). CENT."""
    bestand = int(s.get("verlustvortrag_bestand", 0)) * 100
    schaedlich = bool(s.get("schaedlicher_erwerb"))
    nach_8c = 0 if schaedlich else bestand
    suspension = (schaedlich and bool(s.get("antrag_8d"))
                  and bool(s.get("fortfuehrungs_voraussetzungen")))
    return bestand if suspension else nach_8c


def _kst_einkommen_vor_spenden_cent(s: dict) -> int:
    """einkommen_vor_spenden = maßgebliches Einkommen + § 4h-Add-back (nichtabziehbare
    Zinsen erhoehen das Einkommen, N6). Zugleich GewSt-Slot (§ 7 GewStG) UND § 9-Abs.2-S.1-
    Hoechstbetragsbasis. CENT. (Ohne § 4h-Inputs = maßgebliches Einkommen unveraendert.)"""
    return _kst_massgebliches_einkommen_cent(s) + _kst_nichtabziehbare_zinsen_cent(s)


def _kst_spendenabzug_cent(s: dict) -> int:
    """§ 9 Abs.1 Nr.2: min(Zuwendungen, max(20 % Einkommen, 4 Promille (Umsatz+Loehne))).
    20/100 und 4/1000 exakt. Basis = einkommen_vor_spenden (Abs.2 S.1). CENT."""
    evs = _kst_einkommen_vor_spenden_cent(s)
    grenze_a = evs * 20 // 100
    grenze_b = (int(s.get("umsaetze", 0)) + int(s.get("loehne_gehaelter", 0))) * 100 * 4 // 1000
    return min(int(s.get("zuwendungen", 0)) * 100, max(grenze_a, grenze_b))


def _kst_zve_cent(s: dict) -> int:
    """§ 9-Spenden -> § 10d-Verlust (Sockel 1 Mio + 70 %, KapGes ohne Splitting) -> zvE.
    § 10d-Basis (GdE) = Einkommen NACH Spenden (R 7.1 KStR). § 24-Freibetrag = 0
    (KapGes-Ausschluss § 24 S.2 Nr.1). CENT."""
    evs = _kst_einkommen_vor_spenden_cent(s)
    gde = evs - _kst_spendenabzug_cent(s)
    sockel = 1000000 * 100
    hoechst = sockel + max(0, gde - sockel) * 70 // 100
    verlust = min(_kst_verlustbestand_nach_8c_8d_cent(s), hoechst)
    return max(0, gde - verlust)


def _kst_gewst_cent(s: dict) -> int:
    """GewSt KapGes: Gewerbeertrag = einkommen_vor_spenden (mit § 8b/§ 10, OHNE § 9/§ 10d;
    § 8/§ 9 GewStG-Anpassungen = 0, § 10a-Vortrag = 0). KEIN 24.500-Freibetrag (nur
    natuerl. Person/PersG, § 11). floor auf 100 Euro, Messzahl 3,5 %, x Hebesatz. CENT.
    (§ 8 Nr.9 / § 9 Nr.5 GewStG-Spendenkorrektur = benannter Nachtrag, nicht modelliert.)"""
    ge_euro = _kst_einkommen_vor_spenden_cent(s) // 100
    abger = (ge_euro // 100) * 100 if ge_euro > 0 else 0
    messbetrag_cent = abger * 35 // 10
    return messbetrag_cent * int(s["gewst_hebesatz"]) // 100


def catala_kst_nenner_b(s: dict) -> int:
    """Nenner B (KapGes) in CENT: KSt (§ 23, 15 % des zvE) + SolZ (5,5 %) + GewSt."""
    kst = _kst_zve_cent(s) * 15 // 100
    solz = kst * 55 // 1000
    return kst + solz + _kst_gewst_cent(s)


# -- Solidaritaetszuschlag § 3, § 4 SolzG 1995 (natuerliche Person). CENT. -------------

_SOLZ_FREIGRENZE = {
    # VZ → (einzel, zusammen). Wert-Provenance: params/<vz>/solidaritaetszuschlag_solzg.yaml,
    # gegengeprueft mit GETTSIM 1.2.1. Nur die Freigrenze driftet; Satz (5,5%)/Milderung (11,9%)
    # fassungskonstant. VZ 2024: Inflationsausgleichsgesetz; 2025/2026: SteFeG.
    2024: (18130, 36260),
    2025: (19950, 39900),
    2026: (20350, 40700),
}


def catala_solz(s: dict) -> int:
    """§3, §4 SolzG 1995 Solidaritaetszuschlag fuer natuerliche Personen, CENT.
    bemessungsgrundlage (EURO) = KiFB-fiktive ESt (§3 Abs.2: IMMER mit §32 Abs.6-
    Freibetraegen, unabhaengig von §31-Guenstigerpruefung). kapital_steuer (EURO) =
    §32d-Abgeltung (§3 Abs.3 S.2: 5,5% OHNE Freigrenze). splitting = Zusammenveranlagung
    (§32a Abs.5/6 → doppelte Freigrenze). veranlagungszeitraum = int-VZ (2024-2026)."""
    year = s["veranlagungszeitraum"]
    basis = int(s["bemessungsgrundlage"])              # EURO, KiFB-fiktiv
    kap_est = int(s.get("kapital_steuer", 0))           # EURO, §32d-Abgeltung
    basis_main = max(0, basis - kap_est)                # §3 Abs.3 S.1
    splitting = s["splitting"]

    assert year in _SOLZ_FREIGRENZE, f"SolZ VZ {year}: nur 2024-2026"
    fg_einzel, fg_zusammen = _SOLZ_FREIGRENZE[year]
    freigrenze = fg_zusammen if splitting else fg_einzel

    # §4 S.1+2 + S.3 (Cent-Floor). Ganzzahlige Cent-Rechnung = decimal-Truncate-Äquivalent.
    if basis_main <= freigrenze:
        solz_main_cent = 0
    else:
        regel_cent = basis_main * 55 // 10             # 5,5 % in Cent (floor)
        milderung_cent = (basis_main - freigrenze) * 119 // 10   # 11,9 % (floor)
        solz_main_cent = min(regel_cent, milderung_cent)

    # §3 Abs.3 S.2: Kapital-SolZ 5,5 % OHNE Freigrenze (Floor auf Cent)
    solz_kap_cent = kap_est * 55 // 10
    return solz_main_cent + solz_kap_cent


VZ_ENUM = {
    2024: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2024, None),
    2025: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2025, None),
    2026: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2026, None),
}

_UMLAUT = str.maketrans({"ä": "ae", "ö": "oe", "ü": "ue", "ß": "ss",
                         "Ä": "ae", "Ö": "oe", "Ü": "ue"})


def normalize(text: str) -> str:
    return re.sub(r"\s+", " ", text.translate(_UMLAUT).lower()).strip()


def _gesamt_out(s: dict):
    """Baut den FestzusetzendeEstGesamt(-Zusammen)-Scope-Output aus dem Sachverhalt (EINE Wahrheit für
    catala_gesamt UND den GdE-Zwilling catala_gesamt_gde). Reine Andockstellen-Money-Übersetzung."""
    year = s["veranlagungszeitraum"]
    def m(k):
        return Money(f"{int(s.get(k, 0))}.00")
    # § 31 S. 4 Hinzurechnung: ist `kinder_ganzjaehrig` gesetzt, wird der Jahres-
    # Kindergeld-Betrag aus params/<vz> abgeleitet (Monatswert x 12 x Kinder),
    # sonst gilt der direkte sachverhalt-Wert. So ist die Groesse params-geankert.
    hinzu_kg = int(s.get("hinzurechnung_kindergeld", 0))
    if s.get("kinder_ganzjaehrig"):
        hinzu_kg = _kindergeld(year) * 12 * int(s["kinder_ganzjaehrig"])
    # § 10 Sonderausgaben: die abziehbare Altersvorsorge (p10_1_2, auf den params-HB
    # gedeckelt) wird zum sonderausgaben-Wert addiert. § 10c-Floor (Sonderausgaben-
    # Pauschbetrag 36/72): der Gesamt-Scope buendelt § 10c NICHT selbst (anders als der
    # einzel/zusammen-Tarif), also bildet der Aufrufer den Guenstigervergleich. Bindet nur,
    # wenn die tatsaechlichen SA (inkl. Vorsorge) unter dem Pauschbetrag liegen - die
    # Bestandsfaelle mit SA >= Pauschbetrag rechnen unveraendert.
    sonder = _sonderausgaben_final(
        int(s.get("sonderausgaben", 0)) + _vorsorge_abzug(s, year), year, s.get("veranlagung"))
    scope = (E.festzusetzende_est_gesamt_zusammen if s.get("veranlagung") == "zusammen"
             else E.festzusetzende_est_gesamt)
    cls = (E.FestzusetzendeEstGesamtZusammenIn if s.get("veranlagung") == "zusammen"
           else E.FestzusetzendeEstGesamtIn)
    out = scope(cls(
        einkuenfte_nichtselbststaendig_in=m("einkuenfte_nichtselbststaendig"),
        einkuenfte_kapitalvermoegen_in=m("einkuenfte_kapitalvermoegen"),
        einkuenfte_vermietung_in=m("einkuenfte_vermietung"),
        einkuenfte_sonstige_in=m("einkuenfte_sonstige"),
        einkuenfte_gewinn_in=m("einkuenfte_gewinn"),
        altersentlastungsbetrag_in=m("altersentlastungsbetrag"),
        entlastungsbetrag_alleinerziehende_in=m("entlastungsbetrag_alleinerziehende"),
        sonderausgaben_in=Money(f"{sonder}.00"),
        aussergewoehnliche_belastungen_in=m("aussergewoehnliche_belastungen"),
        freibetraege_kinder_in=m("freibetraege_kinder"),
        sonstige_abzuege_vom_einkommen_in=m("sonstige_abzuege_vom_einkommen"),
        anzurechnende_auslaendische_steuern_in=m("anzurechnende_auslaendische_steuern"),
        steuerermaessigungen_in=m("steuerermaessigungen"),
        steuer_kapital_gesondert_in=m("steuer_kapital_gesondert"),
        hinzurechnung_kindergeld_in=Money(f"{hinzu_kg}.00"),
        hinzurechnung_zulage_in=m("hinzurechnung_zulage"),
        tarif_modifiziert_in=Bool(s.get("tarif_modifiziert", False)),
        tarifliche_est_modifiziert_in=m("tarifliche_est_modifiziert"),
        veranlagungszeitraum_in=VZ_ENUM[year]))
    return out


def catala_gesamt(s: dict) -> int:
    """Verallgemeinerte § 2-Veranlagung → festzusetzende Einkommensteuer, EURO."""
    return int(_gesamt_out(s).festzusetzende_est) // 100


def catala_gesamt_gde(s: dict) -> int:
    """§ 2 Abs. 3 Gesamtbetrag der Einkünfte (ECHT: alle Einkunftsarten − § 24a/§ 24b), EURO — die Basis
    für die § 10b-20%-Deckelung und die § 33-zumutbar-Staffel im gefalteten gesamt-Ring (statt der früheren
    §19-only-GdE der Sonder-Scheiben). Steht VOR den Abzügen fest (§ 2 Abs. 3 vor Abs. 4) → keine Zirkularität.
    Sonderausgaben/agB/Ermäßigungen im Sachverhalt beeinflussen den GdE NICHT (erst Einkommen/zvE)."""
    return int(_gesamt_out(s).gesamtbetrag_der_einkuenfte) // 100


def catala_p10d_2(s: dict) -> int:
    """§ 10d Abs. 2 EStG — Verlustvortrag-Abzug, EURO (module Verlustvortrag): verlustabzug = GdE ≤ 0 ? 0 :
    min(verlustvortrag_bestand, min(GdE, sockel + max(0, GdE − sockel) × 0.70)); sockel = 1 Mio (einzel) / 2 Mio
    (zusammen). Der min(GdE)-Cap (§ 10d Abs. 2 „bis zu einem GdE von 1 Mio unbeschränkt" = 100 % nur bis zur GdE-
    Höhe, man kann nicht mehr abziehen als GdE) + der GdE ≤ 0-Floor sind in der GEFIXTEN Regel (sha 294cdd6a; die
    alte cap-lose Version über-abzog für GdE < sockel = gesetzwidrig). Read-Keys: gesamtbetrag_einkuenfte (= der
    GdE-Zwilling catala_gesamt_gde), verlustvortrag_bestand (Feststellungsbescheid), zusammenveranlagung. Accessor
    nimmt EUROS (die //100-Umrechnung liegt im slot_fn). Der Abzug mindert den GdE (§ 2 Abs. 4-Vorstufe, § 10d
    Abs. 2 „vorrangig vor Sonderausgaben, agB, sonstigen Abzugsbeträgen") → Naht via sonstige_abzuege_vom_einkommen."""
    r = VL.verlustvortrag_abzug(VL.VerlustvortragAbzugIn(
        gesamtbetrag_einkuenfte_in=Money(f"{int(s.get('gesamtbetrag_einkuenfte', 0))}.00"),
        verlustvortrag_bestand_in=Money(f"{int(s.get('verlustvortrag_bestand', 0))}.00"),
        zusammenveranlagung_in=Bool(bool(s.get('zusammenveranlagung', False)))))
    return int(r.verlustabzug) // 100


# -- DBA § 34c Abs. 1 EStG (Anrechnung ausländischer Steuer). EURO. --------------
# Pure-Python (p34c_1 steht in clerk.toml NICHT — nur p34_3; promoted+inert via
# pipeline, 4 test_seeds verified fidel). EURO-floor-Idiom = ganzzahlige Division.
# Q1-invariant (dev-2-Fragetext): auslaendische_einkuenfte_staat = TEILMENGE der
# bereits im zvE enthaltenen Welteinkünfte, NIE additiv zur GdE — reiner Zähler.

# -- §23 EStG private Veräußerungsgeschäfte. EURO. ----------------------------
# Pure-Python (3 promoted+inert Snapshots: p23_veraeusserungsgewinn, p23_freigrenze,
# p23_3_verlusttopf). Multi-Instanz wie §21 (pro Veräußerung ein p23-Veraeusserungspreis/
# Anschaffungs/WK-Tripel), Σ über Instanzen im Ring. Freigrenze 1000€ (§23 Abs.3 S.5):
# Gesamtgewinn ≥ 1000 → voll steuerpflichtig; < 1000 → 0 (Wächter 999→0/1000→1000).

def catala_p23_veraeusserungsgewinn(s: dict) -> int:
    """§23 Abs.3 EStG — Veräußerungsgewinn EINES Geschäfts, EURO.
    veraeusserungspreis − anschaffungs_herstellungskosten − werbungskosten.
    KANN NEGATIV sein (Verlust). 3 test_seeds pipeline-verified. Accessor nimmt EUROS."""
    return (int(s["veraeusserungspreis"])
            - int(s["anschaffungs_herstellungskosten"])
            - int(s["werbungskosten"]))


def catala_p23_freigrenze(s: dict) -> int:
    """§23 Abs.3 S.5 EStG — Freigrenze 1000€, EURO. GESAMTgewinn ≥ 1000 → VOLL
    steuerpflichtig; < 1000 → 0 (Freigrenze, KEIN Freibetrag). Wächter: 999→0, 1000→1000.
    4 test_seeds pipeline-verified."""
    gesamt = int(s["gesamtgewinn"])
    return gesamt if gesamt >= 1000 else 0


def catala_p23_verlusttopf(s: dict) -> int:
    """§23 Abs.3 S.7 EStG — Verlusttopf same-year, EURO. anzusetzende_einkuenfte =
    max(0, gewinn_pvg − verlust_pvg). Verluste mindern nur Gewinne, nie NEGATIV
    in die §2-Einkünfte. 4 test_seeds pipeline-verified. Mehrjahr-Verlustvor-/rücktrag
    (§23 Abs.3 S.8) = Stufe-2-Backlog (nicht in Stufe-1)."""
    return max(0, int(s["gewinn_pvg"]) - int(s["verlust_pvg"]))


def catala_p34c_1(s: dict) -> int:
    """§34c Abs.1 EStG — Anrechnung ausländischer Steuer, Single-Country, EURO.
    anrechnung = min(gezahlte_auslaendische_steuer, deutsche_est_inkl_ausl * ausl / zvE).
    zve ≤ 0 or ausl ≤ 0 → 0. 4 test_seeds pipeline-verified (3000→3000, 10000→8000-
    Deckel, ausl=0→0, zve=60000-ausl=30000-30000→5000)."""
    gezahlt = int(s["gezahlte_auslaendische_steuer"])
    est = int(s["deutsche_est_inkl_ausl"])
    zve = int(s["zu_versteuerndes_einkommen"])
    ausl = int(s["auslaendische_einkuenfte_staat"])
    if ausl <= 0 or zve <= 0:
        return 0
    hoechstbetrag = est * ausl // zve
    return min(gezahlt, hoechstbetrag)


def catala_gesamt_tarifliche(s: dict) -> int:
    """Die tarifliche ESt (§ 32a auf das zvE) des gesamt-Scopes, EURO — Output-Feld tarifliche_est des
    FestzusetzendeEstGesamt(-Zusammen)-Scopes. UNABHÄNGIG von steuerermaessigungen (nur zvE-abhängig, empirisch
    == catala_est(zvE) = §32a) → kein Zirkel bei der § 35-Anrechnung. Dient als „geminderte tarifliche Steuer"
    (§ 35 Abs. 1 S. 4, MVP ohne DBA/§ 34c/§ 32d-ausländische Steuern) für den § 35 Abs. 1 S. 2-Ermäßigungshöchstbetrag."""
    return int(_gesamt_out(s).tarifliche_est) // 100


def catala_gesamt_zve(s: dict) -> int:
    """§ 2 Abs. 5 zu versteuerndes Einkommen des gesamt-Scopes, EURO — Output-Feld zu_versteuerndes_einkommen des
    FestzusetzendeEstGesamt(-Zusammen)-Scopes. Basis für die § 34 Abs. 1-Fünftelregelung (verbleibendes zvE = zvE − ao,
    ao = außerordentliche Einkünfte = § 16-vg-netto). Tarif-unabhängig (steht vor § 32a) → kein Zirkel mit tarif_modifiziert."""
    return int(_gesamt_out(s).zu_versteuerndes_einkommen) // 100


def catala_ermaessigter_durchschnittssatz(s: dict) -> int:
    """§ 34 Abs. 3 EStG — ermäßigter Durchschnittssatz auf den VÄ-Gewinn (≤ 5 Mio), EURO (module Ermaessigter-
    Durchschnittssatz): est_ao = min(ao, 5Mio) × max(0.56 × Durchschnittssatz; 0.14). Durchschnittssatz =
    est_gesamt_zzgl_progression / bemessungsgrundlage_durchschnitt (§ 34 Abs. 3 S. 2: tarifliche ESt aufs volle zvE zzgl.
    Progressionsvorbehalt / volles zvE). Modul liefert NUR est_ao (den ≤5Mio-Teil); est_rest (verbleibendes zvE am
    Normaltarif, S. 3) + der >5Mio-Excess (fail-closed abs3_ueber_5mio_offen, Stufe-2b) sind Ring-Naht. Accessor nimmt
    EUROS (slot_fn //100). MVP: est_gesamt = grundtarif(zvE) OHNE § 32b-Progressionszuschlag (nicht im Ring)."""
    r = ED.ermaessigter_durchschnittssatz(ED.ErmaessigterDurchschnittssatzIn(
        ao_einkuenfte_in=Money(f"{int(s.get('ao_einkuenfte', 0))}.00"),
        est_gesamt_zzgl_progression_in=Money(f"{int(s.get('est_gesamt_zzgl_progression', 0))}.00"),
        bemessungsgrundlage_durchschnitt_in=Money(f"{int(s.get('bemessungsgrundlage_durchschnitt', 0))}.00")))
    return int(r.est_ao) // 100


def _p35c_ermaessigung_cent(s: dict) -> int:
    """§ 35c Abs. 1 EStG energetische Sanierung: satz 7 % (6 % im uebernaechsten
    Foerderjahr), hoechst 14.000 (12.000 uebernaechst); Ermaessigung = min(satz x
    Aufwand, hoechst). CENT (Python-Accessor analog GewSt/KSt; die § 2-Verrechnung
    mit der tariflichen ESt ist NICHT diese Regel)."""
    uebernaechst = bool(s.get("ist_uebernaechstes_foerderjahr"))
    satz = 6 if uebernaechst else 7
    hoechst = (12000 if uebernaechst else 14000) * 100
    aufwand = int(s.get("sanierungsaufwendungen", 0)) * 100
    return min(aufwand * satz // 100, hoechst)


def _kfz_nutzungswert_monat_cent(s: dict) -> int:
    """§ 6 Abs. 1 Nr. 4 S. 2 EStG 1 %-Regel: nutzungswert_monat = BLP / 100 / Teiler
    (Teiler 1 voll, 2 halb, 4 viertel bei E/Hybrid). MONATSwert in CENT."""
    blp = int(s.get("bruttolistenpreis", 0)) * 100
    teiler = int(s.get("bruchteils_teiler", 1))
    return blp // 100 // teiler


def catala_est(sachverhalt: dict) -> int:
    # § 35c energetische Sanierung + § 6 Abs. 1 Nr. 4 Kfz-Nutzungswert (Paket 9, amtliche
    # Goldens): eigenstaendige Python-Accessoren, VZ-agnostisch -> VOR dem year-Read.
    if "sanierungsaufwendungen" in sachverhalt:
        return _p35c_ermaessigung_cent(sachverhalt)
    if "bruttolistenpreis" in sachverhalt:
        return _kfz_nutzungswert_monat_cent(sachverhalt)
    year = sachverhalt["veranlagungszeitraum"]
    veranlagung = sachverhalt.get("veranlagung")
    # Verallgemeinerte § 2-Veranlagung (Gesamtfall).
    if sachverhalt.get("gesamtfall"):
        return catala_gesamt(sachverhalt)
    # GewSt-Kette (§§ 6-11/35): Steuermessbetrag oder § 35-Anrechnung, in CENT.
    if sachverhalt.get("gewerbesteuer"):
        return catala_gewst(sachverhalt)
    # KStG Nenner B (Kapitalgesellschaft): KSt + SolZ + GewSt, in CENT.
    if sachverhalt.get("koerperschaft"):
        return catala_kst_nenner_b(sachverhalt)
    # Entfernungspauschale (§ 9): abziehbarer Betrag.
    if "entfernung_km_roh" in sachverhalt:
        return catala_entfernungspauschale(sachverhalt)
    # Arbeitszimmer/Homeoffice (§ 4 Abs. 5 Nr. 6b/6c): abzug_gesamt.
    if "arbeitszimmer_vorhanden" in sachverhalt:
        return catala_raumkosten(sachverhalt)
    # § 26b Zusammenveranlagung (Splitting): Bruttolohn BEIDER Ehegatten -> festzusetzende ESt.
    if "bruttoarbeitslohn_a" in sachverhalt:
        return catala_est_zusammen(sachverhalt)
    # End-to-end Arbeitnehmerfall (Bruttolohn -> festzusetzende ESt).
    if "bruttoarbeitslohn" in sachverhalt:
        out = E.festzusetzende_est_einzel(E.FestzusetzendeEstEinzelIn(
            bruttoarbeitslohn_in=Money(f"{int(sachverhalt['bruttoarbeitslohn'])}.00"),
            werbungskosten_in=Money(f"{int(sachverhalt.get('werbungskosten', 0))}.00"),
            sonderausgaben_in=Money(f"{int(sachverhalt.get('sonderausgaben', 0))}.00"),
            veranlagungszeitraum_in=VZ_ENUM[year]))
        return int(out.festzusetzende_est) // 100
    # § 34 Abs. 1 Fuenftelregelung (ausserordentliche Einkuenfte), Kernfall verbleibendes zvE >= 0.
    if "ausserordentliche_einkuenfte" in sachverhalt:
        return catala_fuenftel(sachverhalt)
    # Tariff-level case (zvE -> tarifliche ESt).
    m = Money(f"{int(sachverhalt['zu_versteuerndes_einkommen'])}.00")
    if veranlagung == "einzel":
        out = E.grundtarif(E.GrundtarifIn(
            zu_versteuerndes_einkommen_in=m, veranlagungszeitraum_in=VZ_ENUM[year]))
    elif veranlagung == "zusammen":
        out = E.splittingtarif(E.SplittingtarifIn(
            zu_versteuerndes_einkommen_gemeinsam_in=m, veranlagungszeitraum_in=VZ_ENUM[year]))
    else:
        raise ValueError(f"unknown veranlagung: {veranlagung}")
    return int(out.tarifliche_steuer) // 100


def catala_est_einzel_zve(s: dict) -> int:
    """§ 2 Abs. 5 zu versteuerndes Einkommen des reinen-AN-einzel-Scopes (festzusetzende_est_einzel),
    EURO. SELBE Eingaben wie catala_est(einzel) → reproduziert dessen internes zvE (Bemessungsgröße
    der § 101-Grundfreibetrags-Unterschreitung, S. 2). Kein neuer Rechenpfad, nur Output-Feld-Lesen."""
    year = s["veranlagungszeitraum"]
    out = E.festzusetzende_est_einzel(E.FestzusetzendeEstEinzelIn(
        bruttoarbeitslohn_in=Money(f"{int(s['bruttoarbeitslohn'])}.00"),
        werbungskosten_in=Money(f"{int(s.get('werbungskosten', 0))}.00"),
        sonderausgaben_in=Money(f"{int(s.get('sonderausgaben', 0))}.00"),
        veranlagungszeitraum_in=VZ_ENUM[year]))
    return int(out.zu_versteuerndes_einkommen) // 100


def catala_fuenftel(s: dict) -> int:
    """§ 34 Abs. 1 Fuenftelregelung (ausserordentliche Einkuenfte), Kernfall verbleibendes zvE >= 0.

    ORCHESTRIERT bestehende Scopes (catala_gesamt-Muster, KEIN neuer Rechenpfad, keine Nachbildung
    der Tariflogik): der § 32a-Tarif kommt aus dem Catala-Scope Einkommensteuertarif, der Faktor 5
    ist die exakte Struktur-Konstante der Regel p34_fuenftel_ao_est (dort als est_ao = 5 x Differenz
    verifiziert). Glue = Subtraktion (verbleibendes zvE), 1/5-Aufteilung und Summe.

        verbleibendes_zve = zvE - ao
        est1 = Tarif(verbleibendes_zve)
        est2 = Tarif(verbleibendes_zve + ao/5)
        est_ao = 5 * (est2 - est1)                 # p34_fuenftel_ao_est
        tarifliche_est = est1 + est_ao

    NEGATIVFALL § 34 Abs. 1 S. 3 (verbleibendes zvE negativ, zvE positiv) ist modelliert: est_ao =
    5 x Tarif(zvE/5), tarifliche ESt = est_ao (Grundbetrag 0). Regel p34_1_s3_fuenftel_negativ
    (Paket 10c Block 8). H 34.2 Bsp 2 (Erwartung 12.010) als Golden verankert.
    """
    year = s["veranlagungszeitraum"]
    ver = s.get("veranlagung")
    zve = int(s["zu_versteuerndes_einkommen"])
    ao = int(s["ausserordentliche_einkuenfte"])
    verbleibendes_zve = zve - ao

    def _tarif_cent(x: int) -> int:
        m = Money(f"{int(x)}.00")
        if ver == "einzel":
            out = E.grundtarif(E.GrundtarifIn(
                zu_versteuerndes_einkommen_in=m, veranlagungszeitraum_in=VZ_ENUM[year]))
        elif ver == "zusammen":
            out = E.splittingtarif(E.SplittingtarifIn(
                zu_versteuerndes_einkommen_gemeinsam_in=m, veranlagungszeitraum_in=VZ_ENUM[year]))
        else:
            raise ValueError(f"unknown veranlagung: {ver}")
        return int(out.tarifliche_steuer)

    # § 34 Abs. 1 S. 3: verbleibendes zvE negativ UND zvE positiv -> est_ao = 5 x Tarif(zvE/5);
    # die tarifliche ESt IST das est_ao komplett (Grundbetrag 0, kein est1-Summand). Regel
    # p34_1_s3_fuenftel_negativ (Sonderpfad-Schwester, faithful-Andockung-Artefakt nicht_echt).
    if verbleibendes_zve < 0:
        if zve <= 0:
            raise ValueError("§ 34 Abs. 1 S. 3 setzt ein positives zvE voraus.")
        return (5 * _tarif_cent(zve // 5)) // 100

    est1 = _tarif_cent(verbleibendes_zve)
    est2 = _tarif_cent(verbleibendes_zve + ao // 5)
    est_ao = 5 * (est2 - est1)
    return (est1 + est_ao) // 100


def catala_est_zusammen(s: dict) -> int:
    """§ 26b Zusammenveranlagung (Splitting) — festzusetzende ESt, EURO. Roh-Bruttolohn + Roh-WK
    PRO Person; der § 9a-Arbeitnehmer-Pauschbetrag (1230) je Ehegatte UND der Splittingtarif werden
    vom Catala-Scope festzusetzende_est_zusammen INTERN gerechnet (handverifiziert: WK 500/500 ==
    WK 0/0, Pauschbetrag greift; Splitting-Vorteil 60000+20000 -> 13838 vs 2x einzel 15251). Kein
    § 9a-Nachbau in der Haut (Doktrin wie einzel/EP/dHf). MVP: Person B ohne gesonderte WK (wk_b=0)."""
    year = s["veranlagungszeitraum"]
    out = E.festzusetzende_est_zusammen(E.FestzusetzendeEstZusammenIn(
        bruttoarbeitslohn_a_in=Money(f"{int(s.get('bruttoarbeitslohn_a', 0))}.00"),
        bruttoarbeitslohn_b_in=Money(f"{int(s.get('bruttoarbeitslohn_b', 0))}.00"),
        werbungskosten_a_in=Money(f"{int(s.get('werbungskosten_a', 0))}.00"),
        werbungskosten_b_in=Money(f"{int(s.get('werbungskosten_b', 0))}.00"),
        sonderausgaben_gemeinsam_in=Money(f"{int(s.get('sonderausgaben_gemeinsam', 0))}.00"),
        veranlagungszeitraum_in=VZ_ENUM[year]))
    return int(out.festzusetzende_est) // 100


def main() -> int:
    cases = sorted(glob.glob(os.path.join(ROOT, "golden", "cases", "*.yaml")))
    if not cases:
        print("no golden cases found")
        return 1

    source_cache: dict[str, str] = {}
    failures = []

    for path in cases:
        c = load_yaml_fh(open(path, encoding="utf-8"))
        cid = c["id"]
        s = c["sachverhalt"]
        erw = c["erwartung"]
        exp = erw.get("tarifliche_est",
                      erw.get("festzusetzende_est",
                              erw.get("abziehbarer_betrag",
                                      erw.get("abzug_gesamt",
                                              erw.get("gewst_cent",
                                                      erw.get("nenner_b_cent",
                                                              erw.get("sanierung_ermaessigung_cent",
                                                                      erw.get("nutzungswert_monat_cent"))))))))
        q = c["quelle"]

        # 1. citation-anchor gate
        src_path = os.path.join(ROOT, q["datei"])
        if src_path not in source_cache:
            source_cache[src_path] = normalize(open(src_path, encoding="utf-8").read())
        anchor_ok = normalize(q["zitatanker"]) in source_cache[src_path]

        # 2. value check against Catala
        got = catala_est(s)
        value_ok = got == exp

        if anchor_ok and value_ok:
            print(f"OK       {cid}  (est={got})")
        else:
            reason = []
            if not anchor_ok:
                reason.append("Zitatanker nicht im Quelltext")
            if not value_ok:
                reason.append(f"Wert Catala={got} != erwartet={exp}")
            print(f"FAIL     {cid}  -> {'; '.join(reason)}")
            failures.append(cid)

    print(f"\n{len(cases) - len(failures)}/{len(cases)} Faelle bestanden.")
    return 0 if not failures else 1


# -- §35c EStG energetische Sanierungsmassnahmen + Energieberater ---------------
# Pure-Python aus verified_bedingt Snapshots (p35c_sanierung_ermaessigung.json,
# p35c_energieberater_ermaessigung.json). Beide sind Teile einer gemeinsamen Integration
# via Jahresdeckel in rules/estg/p35c/energetische_massnahmen.catala_en.

def catala_p35c_sanierung(s: dict) -> int:
    """§35c Abs.1 EStG — Sanierungsermaessigung, 7%/7%/6% Staffel mit Jahresdeckel.
    ist_uebernaechstes_foerderjahr=False: 7% bis 14.000 EUR
                                 True:  6% bis 12.000 EUR
    Accessor nimmt EUROS, gibt EUROS. Input-Werte: sanierungsaufwendungen (EUR int),
    ist_uebernaechstes_foerderjahr (bool). Seeds aus Snapshot:
    - Jahr 1-2: min(20000 * 0.07, 14000) = 1400
    - Jahr 3: min(20000 * 0.06, 12000) = 1200
    - Brutto-Cap: min(200000 * 0.07, 14000) = 14000
    """
    aufw = int(s.get("sanierungsaufwendungen", 0))
    ist_uebernachst = bool(s.get("ist_uebernaechstes_foerderjahr", False))

    satz = 0.06 if ist_uebernachst else 0.07
    hoechstbetrag = 12000 if ist_uebernachst else 14000

    roh = int(aufw * satz)
    return min(roh, hoechstbetrag)


def catala_p35c_energieberater(s: dict) -> int:
    """§35c Abs.1 S.4 EStG — Energieberater-Sondersatz, 50% (kein Jahrestal).
    Im Abschlussjahr der energetischen Massnahme, NICHT auf 3 Jahre verteilt.
    Accessor nimmt EUROS, gibt EUROS. Input: energieberater_aufwendungen (EUR int).
    Seeds aus Snapshot:
    - 1000 EUR -> 500 EUR
    - 2000 EUR -> 1000 EUR
    - 0 EUR -> 0 EUR
    """
    aufwand = int(s.get("energieberater_aufwendungen", 0))
    return int(aufwand * 0.50)


def catala_p35c_jahresdeckel(s: dict) -> int:
    """§35c Jahresdeckel (Kombination Sanierung + Energieberater), EURO — Muster catala
    P35cJahresdeckel (rules/estg/p35c/energetische_massnahmen.catala_en): min(s+e, HB),
    HB = 12.000 im Abschlussjahr (ist_uebernaechstes_foerderjahr) sonst 14.000.
    sanierung_ermaessigung ist intern schon gedeckelt → Kombination subsumiert das."""
    sanierung = int(s.get("sanierung_ermaessigung", 0))
    energieberater = int(s.get("energieberater_ermaessigung", 0))
    hb = 12000 if s.get("ist_uebernaechstes_foerderjahr") else 14000
    summe = sanierung + energieberater
    return hb if summe > hb else summe


def catala_p22_nr3_einkuenfte(betrag_cent: int) -> int:
    """§ 22 Nr. 3 S. 2 EStG — Freigrenze für sonstige Einkünfte aus Leistungen, CENT.
    < 25600 Cent (256 €) → 0 (steuerfrei), ≥ 25600 Cent → voller Betrag.
    Stufe-1: reine Freigrenze, kein Verlustrestriktion S. 3/4 (Stufe-2-Backlog)."""
    return 0 if betrag_cent < 25600 else betrag_cent


if __name__ == "__main__":
    sys.exit(main())
