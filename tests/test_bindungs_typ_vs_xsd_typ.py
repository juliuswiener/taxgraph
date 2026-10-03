"""Gate: Bindungs-`typ` vs XSD-Kz-Typ.

Jedes gebundene Kz muss in GENAU EINEN Prüfzweig fallen — ein Feld, das durch
alle Zweige fällt (unbekannter typ), ist ein FEHLER, kein Skip:

  typ=bool   -> XSD MUSS ein Ja-Typ sein (Ja1 / JaX / JaNein12 / Ja2)
  typ=cent/int -> XSD darf KEIN Ja-Typ sein; traegt das Kz eine enumeration oder
                ein pattern, muss JEDER Wert der Bindungsschranke dazu passen
  typ=text   -> XSD-Facetten: xs:enumeration -> beispielwert MUSS ein
                Enum-Wert sein; xs:pattern -> beispielwert MUSS alle
                Patterns matchen; sonst Freitext ok

E60xx-Kz (E77/EÜR, andere Datenart) werden separat mit E77-Schema geprüft,
wenn verfügbar. Kz ohne lokales Schema werden in einer SICHTBAREN Liste
gemeldet, nie still übersprungen.

Fängt aktuell (gemessen, HEAD d9107c9):
  - E0205508 (bool an Ganzzahl gebunden)
  - E0500807/E0500808 (text "leibliches Kind" an Enum {1,2,3} gebunden)
  - E0500601/E0500805 (text "01.01.2025 - 31.12.2025" an TT.MM-TT.MM-Pattern)
"""
from __future__ import annotations

import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))
sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))

import est_mapping as EM  # noqa: E402
import xsd_verify as X  # noqa: E402
import traverser as TR  # noqa: E402

pytest.importorskip("yaml")

_SCHEMA_2025 = X._find_schema(2025)
requires_real_schema = pytest.mark.skipif(
    _SCHEMA_2025 is None,
    reason="lokales ERiC-E10-2025.xsd nicht gefunden ($ERIC_DIR/~/02_Software/eric)")

_E77_SCHEMA_2025 = X._find_schema(2025, "E77-{jahr}.xsd")


def _deklariere_einzeln(feld_id: str, kz: str, wert, bindung: dict, vz: int = 2025) -> dict:
    """`EM.deklariere` fuer genau diesen einen Wert, wie ihn der Store ablegt.

    Ein Feld des Pflegeblocks bekommt den Pflegegrad 3 dazu: `_pflegeblock` laesst den Block als
    Ganzes fallen, wenn weder Pflegegrad 2..4 noch Merkzeichen H dasteht. Allein kam der Wert nie
    in der Kz an, der Pruefer sah ihn nicht — und `rentner_pflege_weitere_personen` (E0106603,
    Schema `.{0,1}`) lief mit Bereich 0..20 unbemerkt durch."""
    snap = {feld_id: {"wert": wert, "zustand": "bestaetigt"}}
    if feld_id != "rentner_pflegegrad" and kz in EM.PFLEGE_KZ:
        snap["rentner_pflegegrad"] = {"wert": 3, "zustand": "bestaetigt"}
    return EM.deklariere(snap, bindung, vz=vz)


@requires_real_schema
def test_pflegeblock_ist_die_gruppe_aus_dem_xsd():
    """Drift-Waechter fuer `est_mapping.PFLEGE_KZ` (Entscheidung pflegegrad-kodierung-elster,
    Punkt 4: die Kz-Menge des Blocks wird eingefroren, mit ponytail: und Fundstelle).

    Die Menge wird LIVE aus dem E10-2025.xsd abgeleitet: alle Kz, deren Schema-Pfad durch
    `AgB/Pflege_PB/Einz` fuehrt. Verglichen wird gegen die eingefrorene Konstante — kommt ein
    Feld des Blocks in die Bindung oder ein Kz ins Schema, faellt es hier auf.

    Zwei Ausnahmen stehen namentlich: E0161901 ("weitere an der Pflege beteiligte Personen",
    maxOccurs 9 in derselben Gruppe) ist bewusst NICHT in der Menge — kein Bindungsfeld, der
    Mapper schreibt es nie. Umgekehrt darf kein Kz der Menge ausserhalb der Gruppe liegen.
    """
    import elster_xml as EX
    pfade = EX.kz_pfade(2025)
    in_gruppe = {kz for kz, p in pfade.items() if "Pflege_PB" in p}
    eingefroren = set(EM.PFLEGE_KZ)
    assert in_gruppe - eingefroren == {"E0161901"}, (
        f"Kz der Gruppe AgB/Pflege_PB/Einz, die nicht in PFLEGE_KZ stehen: "
        f"{sorted(in_gruppe - eingefroren)}")
    assert eingefroren - in_gruppe == set(), (
        f"PFLEGE_KZ enthaelt Kz ausserhalb der Gruppe: {sorted(eingefroren - in_gruppe)}")
    # Die sieben Kz sind genau die gebundenen Felder des Blocks in bindung_rentner.yaml.
    bindung = TR.lade_bindung()
    gebunden = {b["elster_kz"] for b in bindung.values()
                if b.get("elster_kz") and b["elster_kz"] in in_gruppe}
    assert gebunden == eingefroren, (
        f"Bindung und PFLEGE_KZ weichen ab: nur in der Bindung {sorted(gebunden - eingefroren)}, "
        f"nur in PFLEGE_KZ {sorted(eingefroren - gebunden)}")


def test_ist_ja_typ_erkennung():
    """Unit-Test: die 4 Ja-Typen + Nicht-Ja-Typen."""
    assert X.ist_ja_typ("Ja1BaseCType")
    assert X.ist_ja_typ("Ja1BaseCType_RABE")
    assert X.ist_ja_typ("JaXBaseCType")
    assert X.ist_ja_typ("JaXBaseCType_RABE")
    assert X.ist_ja_typ("JaNein12BaseCType")
    assert X.ist_ja_typ("JaNein12BaseCType_RABE")
    assert X.ist_ja_typ("Ja2BaseCType")
    assert X.ist_ja_typ("Ja2BaseCType_RABE")
    assert not X.ist_ja_typ("GanzzahlNichtNegOhneFuehrNull_MaxVK12_CType")
    assert not X.ist_ja_typ("GanzzahlNichtNegOhneFuehrNull_MaxVK12_CType_RABE")
    assert not X.ist_ja_typ("StringBaseWithAliasCType")
    assert not X.ist_ja_typ("Enum_Kind_K_Verh_K_Verh_A_E0500807_CType")
    assert not X.ist_ja_typ("xs:string")


@requires_real_schema
def test_bindungs_typ_vs_xsd_typ():
    """JEDES gebundene Kz in genau einem Prüfzweig — kein stiller Durchfall."""
    bindung = TR.lade_bindung()

    # E10- und E77-Typ-Facetten (E77 optional)
    e10_meta = X._resolve_kz_meta(_SCHEMA_2025, "E10")
    e77_meta: dict[str, dict] = {}
    if _E77_SCHEMA_2025 is not None:
        e77_meta = X._resolve_kz_meta(_E77_SCHEMA_2025, "E77")

    def _schema_fuer_kz(kz: str) -> dict[str, dict]:
        return e77_meta if kz[1:3] == "60" else e10_meta

    mismatches: list[str] = []
    ungeprueft: list[str] = []

    for feld_id, b in sorted(bindung.items()):
        kz = b.get("elster_kz")
        if not kz:
            continue
        typ = b.get("typ")

        meta = _schema_fuer_kz(kz).get(kz)
        if meta is None:
            ungeprueft.append(f"{feld_id} (Kz {kz}): Kz nicht im Schema")
            continue

        # GENAU EIN Zweig — ein neuer typ fällt durch und wird gemeldet
        if typ == "bool":
            if not meta["is_ja"]:
                mismatches.append(
                    f"{feld_id}: typ=bool, Kz {kz}, XSD type={meta['type_name']} "
                    f"(kein Ja-Typ)")
        elif typ in ("cent", "int"):
            if meta["is_ja"]:
                mismatches.append(
                    f"{feld_id}: typ={typ}, Kz {kz}, XSD type={meta['type_name']} "
                    f"(Ja-Typ, aber Betrag)")
            # Schranke der Bindung gegen die XSD-enum (Vault: pflegegrad-kodierung-elster, Punkt 5).
            # Durch diese Luecke lief `rentner_pflegegrad`: die Bindung nimmt 1..5 an, das Schema
            # kennt nur {"2","3","4"} — und der Mapper schrieb den Wert unveraendert hinaus. ERiC
            # weist dann die GANZE Erklaerung ab (rc=610001002).
            # Geprueft wird JEDER Wert der Bindungsschranke, nicht der Beispielwert: der ist genau
            # einer und lag mit Pflegegrad 3 IN der enum, der Defekt blieb unsichtbar.
            # Zulaessig ist zweierlei: der Wert kommt nicht in der Kz an (der Mapper laesst ihn
            # weg) ODER er kommt als gueltiger enum-Schluessel an (der Mapper bildet ihn ab).
            bereich = b.get("bereich") or {}
            if meta["enums"] and "min" in bereich and "max" in bereich:
                for w in range(bereich["min"], bereich["max"] + 1):
                    d = _deklariere_einzeln(feld_id, kz, w, bindung)
                    if kz in d["deklaration"] and str(d["deklaration"][kz]) not in meta["enums"]:
                        mismatches.append(
                            f"{feld_id}: typ={typ}, Kz {kz}, Bindung erlaubt {w}, XSD erlaubt nur "
                            f"{meta['enums']} — deklariert als {d['deklaration'][kz]!r}")
            # Dieselbe Schranke gegen das XSD-pattern. 77 gebundene Zahl-Kz tragen eins, und bis
            # 2026-10-01 sah dieser Zweig KEINES davon an — er las nur `meta["enums"]`. Ein Pruefer,
            # der 77 Felder nicht ansieht, meldet trotzdem „bestanden" und sieht aus wie ein
            # bestandener Pruefer.
            #
            # Geprueft wird die MENGE, DIE DIE BINDUNG DURCHLAESST — `enum_werte`, wo sie steht,
            # sonst der Bereich; nicht der beispielwert. Der ist genau einer, und beim Pflegegrad
            # lag er mit 3 IN der XSD-enum, waehrend 1, 2, 5 hindurchliefen.
            #
            # Ein stellenzahlbegrenzendes Muster ist KEINE Werteaufzaehlung: `.{1,3}` gegen einen
            # Bereich bis 366 ist eine Obergrenze, kein Widerspruch. Der Zweig fragt deshalb nach
            # dem Wert, nicht nach der Form des Musters — neun solche Kz bleiben gruen.
            #
            # Und er urteilt nur ueber Werte, die wirklich in der Kz ankommen: was `deklariere`
            # weglaesst (0 auf einem Kz ohne Null), kann das Schema nicht verletzen.
            durchgelassen = b.get("enum_werte")
            if durchgelassen is None and "min" in bereich and "max" in bereich:
                durchgelassen = [str(w) for w in range(bereich["min"], bereich["max"] + 1)]
            if meta["patterns"] and durchgelassen:
                for w in durchgelassen:
                    d = _deklariere_einzeln(feld_id, kz, int(w) if w.lstrip("-").isdigit() else w,
                                            bindung)
                    if kz not in d["deklaration"]:
                        continue
                    v = str(d["deklaration"][kz])
                    if not all(re.fullmatch(p, v) for p in meta["patterns"]):
                        mismatches.append(
                            f"{feld_id}: typ={typ}, Kz {kz}, Bindung erlaubt {w}, XSD pattern="
                            f"{meta['patterns']} — deklariert als {v!r}")
        elif typ == "enum" and feld_id in EM.WERTEKODIERUNG:
            # Klasse i (est_mapping.WERTEKODIERUNG): enum_werte sind Laien-Vokabular, KEIN
            # 1:1-Passthrough — geprüft wird die ÜBERSETZUNG (die amtlichen Codes), nicht die
            # Laien-Werte selbst gegen die XSD-enum.
            if not meta["enums"]:
                mismatches.append(
                    f"{feld_id}: typ=enum (Wertekodierung), Kz {kz}, XSD keine enumeration")
            else:
                for laie, code in EM.WERTEKODIERUNG[feld_id]["code"].items():
                    if code not in meta["enums"]:
                        mismatches.append(
                            f"{feld_id}: Wertekodierung {laie!r} -> {code!r}, Kz {kz}, "
                            f"XSD erlaubt nur {meta['enums']}")
        elif typ == "enum":
            enum_werte = b.get("enum_werte", [])
            if not meta["enums"]:
                mismatches.append(
                    f"{feld_id}: typ=enum, Kz {kz}, XSD keine enumeration — "
                    f"enum_werte {enum_werte} können nicht gemappt werden")
            else:
                for ev in enum_werte:
                    if str(ev) not in meta["enums"]:
                        mismatches.append(
                            f"{feld_id}: typ=enum, Kz {kz}, enum_werte enthalten {ev!r}, "
                            f"XSD erlaubt nur {meta['enums']}")
        elif typ == "datum":
            beispiel = b.get("beispielwert")
            if meta["patterns"]:
                if beispiel is None:
                    ungeprueft.append(f"{feld_id} (Kz {kz}): datum an Pattern "
                                      f"{meta['patterns']} gebunden, aber kein beispielwert")
                elif not all(re.fullmatch(p, str(beispiel)) for p in meta["patterns"]):
                    mismatches.append(
                        f"{feld_id}: typ=datum, Kz {kz}, XSD pattern={meta['patterns']}, "
                        f"beispielwert {beispiel!r} matcht nicht")
            # kein Pattern -> Datumsfeld ohne Format-Facette, ok
        elif typ == "text":
            beispiel = b.get("beispielwert")
            if meta["enums"]:
                if beispiel is None:
                    ungeprueft.append(f"{feld_id} (Kz {kz}): text an Enum "
                                      f"{meta['enums']} gebunden, aber kein beispielwert")
                elif str(beispiel) not in meta["enums"]:
                    mismatches.append(
                        f"{feld_id}: typ=text, Kz {kz}, XSD enum={meta['enums']}, "
                        f"beispielwert {beispiel!r} nicht in enum")
            elif meta["patterns"]:
                if beispiel is None:
                    ungeprueft.append(f"{feld_id} (Kz {kz}): text an Pattern "
                                      f"{meta['patterns']} gebunden, aber kein beispielwert")
                elif not all(re.fullmatch(p, str(beispiel)) for p in meta["patterns"]):
                    mismatches.append(
                        f"{feld_id}: typ=text, Kz {kz}, XSD pattern={meta['patterns']}, "
                        f"beispielwert {beispiel!r} matcht nicht")
            # weder enum noch pattern -> Freitext, ok
        else:
            mismatches.append(f"{feld_id}: unbekannter typ {typ!r} — "
                              f"fällt durch alle Prüfzweige")

    if ungeprueft:
        print("--- Nicht prüfbar (sichtbar, kein stiller Skip) ---")
        for u in ungeprueft:
            print(f"  {u}")

    assert not mismatches, (
        "Bindungs-Typ ↔ XSD-Typ Mismatches:\n" + "\n".join(mismatches))


# `E0106603` (Anzahl weiterer Pflegepersonen) wie in E10-2025.xsd:1380 und E10-2024.xsd:1444:
# hoechstens EIN Zeichen, also 0..9. Abschrift, damit der Test ohne lokales Schema laeuft;
# `test_pflegepersonen_muster_steht_im_schema` haelt sie gegen das Schema.
_PFLEGEPERSONEN_MUSTER = ".{0,1}"


def test_pflegepersonen_bereich_haelt_das_xsd_muster_ein():
    """Hermetisch (kein Schema noetig): jeder Wert, den die Bindung fuer E0106603 durchlaesst,
    kommt in der Kz an und passt zum Schema-Muster. Bereich 0..20 liess 10..20 durch; ELSTER
    nimmt zweistellige Werte nicht an (xmllint gegen E10-2025.xsd: Pattern verletzt)."""
    bindung = TR.lade_bindung()
    b = bindung["rentner_pflege_weitere_personen"]
    assert b["elster_kz"] == "E0106603"
    werte = range(b["bereich"]["min"], b["bereich"]["max"] + 1)
    angekommen, abweichend = 0, []
    for w in werte:
        d = _deklariere_einzeln("rentner_pflege_weitere_personen", "E0106603", w, bindung)
        # KONTROLLE: der Wert kommt an. Faellt er weg, sieht der Test nichts und bleibt gruen.
        assert "E0106603" in d["deklaration"], f"{w} kam in E0106603 nicht an"
        angekommen += 1
        if not re.fullmatch(_PFLEGEPERSONEN_MUSTER, str(d["deklaration"]["E0106603"])):
            abweichend.append(w)
    assert angekommen == len(werte)
    assert not abweichend, (
        f"Bereich {b['bereich']['min']}..{b['bereich']['max']} laesst {abweichend} durch; "
        f"E0106603 erlaubt nur das Muster {_PFLEGEPERSONEN_MUSTER}")


@requires_real_schema
@pytest.mark.parametrize("jahr", [2024, 2025])
def test_pflegepersonen_muster_steht_im_schema(jahr):
    schema = X._find_schema(jahr)
    if schema is None:
        pytest.skip(f"lokales E10-{jahr}.xsd nicht gefunden")
    assert X._resolve_kz_meta(schema, "E10")["E0106603"]["patterns"] == [_PFLEGEPERSONEN_MUSTER]


# Die 17 Felder der Decision textfeld-format-aus-xsd-beim-speichern: 9 ohne `muster`, 8 Zeiträume.
_FORMAT_FELDER = {
    "kind_idnr", "p33a_person_idnr", "rentner_gepflegter_idnr", "stammdaten_bic", "stammdaten_plz",
    "stammdaten_hausnummer", "dhf_bestanden_bis", "p33a_unterstuetzungszeitraum", "p33a_zahlungszeitraum",
    "kind_wohnsitz_inland_zeitraum", "kind_kindschaftsverh_zeitraum_a", "kind_kindschaftsverh_zeitraum_b",
    "kind_anderer_elternteil_zeitraum", "kind_betreuung_zeitraum", "kind_betreuung_eigenanteil_zeitraum",
    "kind_betreuung_kein_gemeinsamer_haushalt_zeitraum", "kind_betreuung_haushaltszugehoerigkeit_zeitraum",
}


@requires_real_schema
@pytest.mark.parametrize("jahr", [2024, 2025])
def test_textfeld_mit_festem_format_speichert_wie_das_xsd(jahr):
    """AK1: ein Textfeld, dessen Kz im XSD ein Format-Pattern trägt, prüft beim Speichern genau
    dieses Pattern. Das Pattern kommt live aus dem Schema des Jahres, über alle Ableitungsschritte
    und mit Längen (tools/parity/xsd_muster.py). Speichern = store._pruefe_typ_konformitaet
    (Auflage T + F). Beide urteilen auf beispielwert und Abwandlungen gleich."""
    if X._find_schema(jahr) is None:
        pytest.skip(f"lokales E10-{jahr}.xsd nicht gefunden")
    sys.path.insert(0, os.path.join(ROOT, "tools", "parity"))
    import xsd_muster as XM   # erst hier: nur dieser Test braucht das Messwerkzeug
    bindung = TR.lade_bindung()
    sch = XM.schema(jahr)
    # Format-Pattern: ein Schritt mit Pattern, das weder Basis-Zeichensatz noch reine Länge ist.
    paare = [(f, kz) for f, kz in XM.auswahl(bindung, ["text"])
             if kz in sch and any(s["xsd"] and s["typ"] not in XM.BASIS
                                  and not all(XM.LAENGE.fullmatch(p) for p in s["xsd"])
                                  for s in sch[kz]["schritte"])]
    assert _FORMAT_FELDER <= {f for f, _ in paare}, sorted(_FORMAT_FELDER - {f for f, _ in paare})
    abweichend = []
    for fid, kz in paare:
        bw = bindung[fid]["beispielwert"]
        proben = {"beispielwert": bw, **XM.proben(bw), "Zeichen weg": bw[:-1],
                  "Ziffern->٣": re.sub("[0-9]", "٣", bw), "5-fach": bw * 5}
        for name, wert in proben.items():
            xsd_ok = not XM.verletzt(wert, sch[kz]["schritte"])
            if XM.speichern_ok(fid, wert, bindung) != xsd_ok:
                abweichend.append(f"{fid} (Kz {kz}) {name}: XSD {'nimmt an' if xsd_ok else 'weist ab'}, "
                                  "Speichern nicht")
    assert not abweichend, "Speichern urteilt anders als das XSD:\n" + "\n".join(abweichend)