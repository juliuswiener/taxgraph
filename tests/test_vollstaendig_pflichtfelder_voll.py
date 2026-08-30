"""Alle-Kz-Felder-Sweep: jedes im Fall beantwortete Feld mit elster_kz einzeln weglassen und
gegen ECHTES ERiC pruefen, ob `pflichtfelder_vollstaendig` das Fehlen sieht.

Warum es das gibt
------------------
Audit 2026-08-28 (vault: audits/taxgraph-vollstaendig-deckt-nichts.md,
audits/taxgraph-vollstaendig-luegt.md; Regal-Eintrag
backlog/taxgraph/vollstaendig-blind-fuer-fehlende-pflichtfelder.md): die Hauptschleife in
est_mapping.deklariere() (est_mapping.py:501) laeuft nur ueber Felder, die im Snapshot DA sind --
ein fehlendes Pflichtfeld kann per Konstruktion nie hineingeraten. Gemessen: von 19 Feldern mit
elster_kz im Durchstich-Fall (_fall_einzel) lehnt ECHTES ERiC 12 beim Fehlen ab (rc != CE.RC_OK).
Bei den restlichen 6 stoert sich ERiC selbst nicht am Fehlen (rc==CE.RC_OK) -- das ist kein
Blindspot, sondern eine echte Nicht-Pflicht.

Julius-Entscheidung 2026-08-30: DIESE Pruefung sitzt bewusst NICHT unter `eingaben_konsistent`.
`eingaben_konsistent` beantwortet eine andere Frage ("sind die VORHANDENEN Eingaben stimmig?")
-- ein fehlendes Feld macht die vorhandenen nicht unstimmig, gehoert also nicht zu dieser Aussage
(genau deshalb hiess der Schluessel vorher `vollstaendig` und wurde umbenannt, s. Commit
"eingaben_konsistent statt vollstaendig"). Die gepflegte Pflichtfeld-Liste (est_mapping.py:
PFLICHTFELDER) traegt stattdessen einen EIGENEN Schluessel, `pflichtfelder_vollstaendig`, der
genau diese Aussage macht und sonst nichts.

Ergaenzt test_checkest_durchstich.py: der dortige Durchstich prueft EINEN vollstaendigen Fall
gegen ERiC. Dieser Sweep prueft das GEGENTEIL -- jeweils EIN Feld absichtlich weggelassen --
und haelt `pflichtfelder_vollstaendig` gegen das amtliche Urteil.

Bauart: HARTES Gate, keine Ratschen-Obergrenze
------------------------------------------------
Anders als test_produkt_xml_erreicht_die_amtliche_pruefung() (Ratsche mit Toleranzwert) ist
dies fail-closed: rot, sobald IRGENDEIN Feld `pflichtfelder_vollstaendig=True` UND ein von ERiC
abgelehntes rc zeigt. Der Defekt sitzt in der Pruef-LOGIK (bzw. in der gepflegten Liste selbst),
nicht in einzelnen Feldern -- er verschwindet nur durch einen Fix an est_mapping.PFLICHTFELDER,
nie durch weitere Testfaelle.

Ueberspringt sauber, wenn ERiC oder die Hersteller-ID fehlen (credential-freies CI, gleiches
Muster wie test_checkest_durchstich.braucht_eric) -- meldet den Grund, statt falsch gruen zu
laufen oder zu fehlen.
"""
from __future__ import annotations

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/import", "produkt/mapping", "produkt/store",
             "produkt/traverser", "elster"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import checkest_gate as CE   # noqa: E402
import elster_xml as EX      # noqa: E402
import est_mapping           # noqa: E402
import store as ST           # noqa: E402
import traverser as TR       # noqa: E402

from test_checkest_durchstich import _ABSENDER, _HID, _fall_einzel, braucht_eric  # noqa: E402


def _ohne_feld(feld_id: str, felder_basis: dict, bindung: dict):
    """Wie _pruefe() in test_checkest_durchstich, aber mit GENAU EINEM Feld entfernt."""
    felder = dict(felder_basis)
    felder.pop(feld_id, None)
    dekl = est_mapping.deklariere(felder, bindung)
    try:
        xml = EX.erzeuge_xml(dekl, vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
    except EX.XmlFehler as exc:
        return dekl["pflichtfelder_vollstaendig"], "WRITER_ABBRUCH", str(exc)
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split()) for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    return dekl["pflichtfelder_vollstaendig"], rc, (texte[0] if texte else "")


@braucht_eric
def test_vollstaendig_sieht_kein_fehlendes_pflichtfeld():
    """Fail-closed: rot, solange ein Feld pflichtfelder_vollstaendig=True zeigt, obwohl ECHTES ERiC
    sein Fehlen ablehnt. Audit 2026-08-28, Regal-Eintrag
    backlog/taxgraph/vollstaendig-blind-fuer-fehlende-pflichtfelder.md. Liest bewusst
    `pflichtfelder_vollstaendig`, NICHT `eingaben_konsistent` (Julius-Entscheidung 2026-08-30):
    Konsistenz und Vollstaendigkeit sind zwei verschiedene Aussagen, s. Modul-Docstring oben.
    """
    store = _fall_einzel()
    bindung = TR.lade_bindung()
    felder, _sid = ST.materialisiere(store)

    kz_felder = sorted(f for f in felder if bindung.get(f, {}).get("elster_kz"))
    assert kz_felder, "kein einziges Feld mit elster_kz im Fall -- Fixtur kaputt, Test sagt nichts"

    luecken = []
    for feld_id in kz_felder:
        pflicht_vollstaendig, rc, text = _ohne_feld(feld_id, felder, bindung)
        if rc == "WRITER_ABBRUCH":
            continue                        # Writer selbst faengt es fail-closed ab -- kein Blindspot
        if rc == CE.RC_OK:
            continue                        # ELSTER stoert sich nicht am Fehlen -- kein Blindspot
        if pflicht_vollstaendig:
            kz = bindung[feld_id]["elster_kz"]
            luecken.append(f"{feld_id} ({kz}): pflichtfelder_vollstaendig=True, aber ERiC rc={rc} -> {text[:120]}")

    assert not luecken, (
        f"{len(luecken)} von {len(kz_felder)} Pflichtfeldern: pflichtfelder_vollstaendig=True, "
        f"obwohl ERiC das Fehlen ablehnt -- die gepflegte Liste est_mapping.PFLICHTFELDER deckt "
        f"diese Faelle noch nicht ab.\n"
        + "\n".join(luecken))
