"""Die Frage nach der EXISTENZ kommt vor der Frage nach den MERKMALEN.

GEMESSEN 2026-08-21 im echten Nutzerlauf. Julius bekam als erste Frage zu § 35a:

    "Wurden die Handwerkerleistungen NICHT öffentlich gefördert (zum Beispiel durch einen
     KfW-Zuschuss oder ein zinsverbilligtes Darlehen)?"

Er hatte keine Handwerkerleistungen. Die Frage, die das geklärt hätte, existiert — sie stand
nur an zweiter Stelle:

    hh_hat_aufwendungen: "Hattest du dieses Jahr Kosten für Handwerker, Haushaltshilfe oder
                          haushaltsnahe Dienstleistungen?"

URSACHE: `gate_gewicht()` liefert für alle vier Gates dieser Regel denselben Wert (10) — sie
schalten dieselben Felder ab, also sind sie gleich wichtig. Den Gleichstand brach
`naechste_fragen` bis dahin ALPHABETISCH: `hh_handwerker_` liegt vor `hh_hat_`. Die Reihenfolge
der Fragen hing damit am Buchstaben.

WARUM DAS MEHR IST ALS EINE UNSCHÖNHEIT: wer auf die Merkmalsfrage mit „nein" antwortet, schliesst
die Regel aus (relevanz()) — und die Eingangsfrage kommt dann NIE. Bei Julius war das Ergebnis
zufällig richtig (er hatte wirklich keine), aber aus dem falschen Grund. Umgekehrt verliert
jemand mit öffentlich geförderten Handwerkerleistungen die Regel, ohne je gefragt worden zu sein,
ob er überhaupt welche hatte.

BEHOBEN über `eingangsfrage: true` in der Bindung — DEKLARIERT, nicht am Feldnamen geraten. Eine
Heuristik über „hat_"/„kein_" wäre genau der Fehler, der bei der Frage-Polarität schon einmal
zwei Feldern das Gegenteil der Nutzerantwort entlockt hat (s. `frage_invertiert`).

NULL LLM.
"""
from __future__ import annotations

import collections
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/traverser", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import store as ST      # noqa: E402
import traverser as TR  # noqa: E402

BINDUNG = TR.lade_bindung()
GEWICHT = TR.gate_gewicht(BINDUNG)


def _echte_gates_je_regel() -> dict[str, list[str]]:
    """Regel -> ihre echten Gates (Gewicht > 0 UND nicht `gate: false`).

    `gate: false` heisst „Deklaration, keine Rechen-Voraussetzung" — ein solches Feld schaltet
    nichts ab und konkurriert deshalb nicht um den ersten Platz, auch wenn gate_gewicht() ihm
    heute noch einen Wert gibt.
    """
    je_regel: dict[str, list[str]] = collections.defaultdict(list)
    for fid, b in BINDUNG.items():
        if b.get("askable") and GEWICHT.get(fid, 0) > 0 and b.get("gate") is not False:
            je_regel[b["quelle"]["regel_id"]].append(fid)
    return je_regel


def _gleichstaende() -> dict[str, list[str]]:
    """Regeln, in denen MEHRERE echte Gates dasselbe Gewicht tragen — nur dort entscheidet
    überhaupt etwas anderes als das Gewicht, und nur dort ist die Reihenfolge eine Frage."""
    out = {}
    for rid, gates in _echte_gates_je_regel().items():
        zaehl = collections.Counter(GEWICHT[f] for f in gates)
        strittig = [f for f in gates if zaehl[GEWICHT[f]] > 1]
        if len(strittig) > 1:
            out[rid] = sorted(strittig)
    return out


# Regeln mit mehreren gleichgewichtigen Gates, in denen es KEINE Existenzfrage gibt: dort fragt
# die Software direkt nach Merkmalen eines Sachverhalts, den sie nie erhoben hat. Das ist keine
# Reihenfolge-Frage mehr, sondern eine fehlende Frage — der Eintrag hier ist eine SCHULD und die
# Arbeitsliste für die Screening-Flags (Backlog screening-flags-ausgabenseite).
#
# Ein Eintrag verschwindet, sobald die Regel eine Eingangsfrage bekommt; test_schuld_ist_noch_offen
# wird rot, wenn er unnötig geworden ist. Genau so hat sich die SCHULD-Liste in
# test_gate_polaritaet.py selbst abgeräumt.
OHNE_EINGANGSFRAGE: dict[str, str] = {
    "p16_4_freibetrag":
        "§ 16 Abs. 4: alle vier Gates fragen nach Merkmalen des Betriebsverkaufs (Alter 55, "
        "Erstmaligkeit) — die Existenzfrage ('hast du einen Betrieb verkauft?') fehlt. Der "
        "Block hängt aber an kein_gewinn, wird also für den Arbeitnehmer schon abgeschaltet.",
    "p21_2_verbilligte_vermietung_wk":
        "§ 21 Abs. 2: beide Gates fragen nach Merkmalen der Vermietung. Hängt an kein_vuv, "
        "für Nicht-Vermieter also bereits abgeschaltet.",
    "p33_1_2_agb_abzug":
        "§ 33: 'Waren die Ausgaben notwendig und der Höhe nach angemessen?' setzt Ausgaben "
        "voraus, die nie erfragt wurden. KEIN Screening-Flag darüber — hier fehlt die "
        "Eingangsfrage wirklich.",
    # NICHT HIER: p35c_sanierung_ermaessigung. Die Regel hat nur EIN echtes Gate (das zweite,
    # p35c_bereits_ermaessigung_frueher, trägt `gate: false`), also gar keinen Gleichstand — dieser
    # Test greift dort nicht. Julius' Fund vom 2026-08-21 ("Hast du für dieses Gebäude in früheren
    # Jahren schon einmal eine Steuerermäßigung bekommen?" an jemanden ohne Gebäude) ist deshalb
    # KEIN Reihenfolge-Problem, sondern eine schlicht fehlende Existenzfrage. Sie gehört zu den
    # Screening-Flags (Backlog screening-flags-ausgabenseite), nicht hierher — ein Eintrag an
    # dieser Stelle würde ihn als „gelöst, sobald sortiert" ausweisen, und das wäre er nicht.
    "p6_2_gwg_sofortabzug":
        "§ 6 Abs. 2: alle drei Gates fragen nach Merkmalen eines Geräts, das nie erfragt wurde. "
        "Die Existenzfrage läge bei den Arbeitsmitteln, nicht hier.",
    "p9_1_3_nr5_doppelte_haushaltsfuehrung":
        "§ 9 Abs. 1 S. 3 Nr. 5: 'Hast du die Zweitwohnung nur aus beruflichen Gründen?' setzt "
        "eine Zweitwohnung voraus. Die Reihenfolge stimmt heute zufällig (dhf_beruflich_ liegt "
        "alphabetisch vorn), die Existenzfrage fehlt trotzdem.",
}


@pytest.mark.parametrize("regel", sorted(_gleichstaende()))
def test_regel_mit_gleichstand_hat_eine_eingangsfrage(regel):
    """Wo mehrere Gates gleich schwer wiegen, muss eine von ihnen die Existenzfrage sein —
    sonst entscheidet der Feldname, welche Frage der Nutzer zuerst sieht."""
    if regel in OHNE_EINGANGSFRAGE:
        pytest.skip(f"bekannte Lücke: {OHNE_EINGANGSFRAGE[regel]}")
    gates = _gleichstaende()[regel]
    markiert = [f for f in gates if BINDUNG[f].get("eingangsfrage")]
    assert markiert, (
        f"{regel} hat {len(gates)} gleichgewichtige Gates und keine davon ist als Eingangsfrage "
        f"markiert: {gates}\nDamit entscheidet der Feldname, welche Frage zuerst kommt. Trägt "
        f"eine davon die Existenzfrage ('gab es das überhaupt?'), dann `eingangsfrage: true` "
        f"setzen. Gibt es keine, gehört die Regel nach OHNE_EINGANGSFRAGE — mit Begründung, "
        f"denn dann FEHLT die Frage und das ist der teurere Befund.")


def test_hoechstens_eine_eingangsfrage_je_regel():
    """Zwei Eingangsfragen wären keine Ordnung, sondern derselbe Gleichstand eine Ebene tiefer."""
    je_regel = collections.Counter(
        b["quelle"]["regel_id"] for b in BINDUNG.values() if b.get("eingangsfrage"))
    mehrfach = {r: n for r, n in je_regel.items() if n > 1}
    assert not mehrfach, f"mehrere Eingangsfragen je Regel: {mehrfach}"


def test_eingangsfrage_ist_ein_gate_und_wird_gefragt():
    """`eingangsfrage` an einem Feld, das gar nicht vorgezogen werden kann, ist tote Deklaration."""
    schaden = []
    for fid, b in sorted(BINDUNG.items()):
        if not b.get("eingangsfrage"):
            continue
        if not b.get("askable"):
            schaden.append(f"{fid}: nicht askable")
        # gate_gewicht 0 ist seit 2026-08-26 KEIN Schaden mehr: `_nach_themen` zieht jede
        # Eingangsfrage in ihrem Thema nach vorn, auch ein Betragsfeld. Vorher galt die
        # Deklaration dort als tot — und genau daran scheiterten die zwei Faelle aus dem
        # E2E-Durchgang: „Hattest du groessere aussergewoehnliche Ausgaben?" (Position 3 von 3)
        # und „Wie viel hast du fuer die Betreuung deines Kindes gezahlt?" (Position 10 von 10)
        # sind beide Betragsfelder und beide die Existenzfrage ihres Themas.
        #
        # Was weiterhin Schaden ist: die Frage steht in der Queue ueberhaupt nicht vorn. Das
        # pruefen wir jetzt direkt statt ueber das Gewicht als Stellvertreter.
        elif b.get("gate") is False and GEWICHT.get(fid, 0) > 0:
            schaden.append(f"{fid}: `gate: false` — Deklaration, kann keine Regel eröffnen")
        else:
            s_ = ST.leerer_store(2025, fall_id="eingang")
            q = TR.naechste_fragen(s_, BINDUNG)
            thema = [f for f in q
                     if (BINDUNG[f].get("quelle") or {}).get("regel_id")
                     == b["quelle"]["regel_id"]]
            if thema and thema[0] != fid:
                schaden.append(f"{fid}: steht in seinem Thema nicht vorn (dort: {thema[0]})")
    assert not schaden, "eingangsfrage an ungeeigneten Feldern:\n  " + "\n  ".join(schaden)


@pytest.mark.parametrize("regel", sorted(_gleichstaende()))
def test_die_eingangsfrage_kommt_wirklich_zuerst(regel):
    """Die Naht: Deklaration in der Bindung UND Wirkung in der Queue. Ohne diesen Test könnte
    jemand `eingangsfrage` setzen, ohne dass naechste_fragen sie je berücksichtigt."""
    if regel in OHNE_EINGANGSFRAGE:
        pytest.skip("keine Eingangsfrage vorhanden")
    gates = _gleichstaende()[regel]
    eingang = [f for f in gates if BINDUNG[f].get("eingangsfrage")]
    if not eingang:
        pytest.skip("keine Eingangsfrage markiert")
    s = ST.leerer_store(2025, fall_id="eingangsfrage")
    reihenfolge = TR.naechste_fragen(s, BINDUNG)
    platz = {f: i for i, f in enumerate(reihenfolge)}
    zuerst, andere = eingang[0], [f for f in gates if f != eingang[0]]
    fehlend = [f for f in andere if f not in platz]
    assert zuerst in platz, f"{zuerst} steht gar nicht in der Fragenliste"
    spaeter = [f for f in andere if f in platz and platz[f] < platz[zuerst]]
    assert not spaeter, (
        f"{regel}: die Merkmalsfragen {spaeter} kommen VOR der Eingangsfrage {zuerst} "
        f"(Platz {platz[zuerst]}). Genau das war der Befund vom 2026-08-21."
        + (f"\n(nicht in der Liste, deshalb nicht geprüft: {fehlend})" if fehlend else ""))


@pytest.mark.parametrize("regel", sorted(OHNE_EINGANGSFRAGE))
def test_schuld_ist_noch_offen(regel):
    """Ein Eintrag in OHNE_EINGANGSFRAGE ist eine Schuld, kein Freibrief: bekommt die Regel eine
    Eingangsfrage, muss der Eintrag raus — sonst bleibt eine geschlossene Lücke für immer als
    Dauerausnahme stehen (dieselbe Mechanik wie SCHULD in test_gate_polaritaet.py)."""
    gleich = _gleichstaende()
    if regel not in gleich:
        pytest.fail(f"{regel} hat keinen Gleichstand mehr — Eintrag aus OHNE_EINGANGSFRAGE entfernen.")
    markiert = [f for f in gleich[regel] if BINDUNG[f].get("eingangsfrage")]
    assert not markiert, (
        f"{regel} hat jetzt eine Eingangsfrage ({markiert}) — Eintrag aus OHNE_EINGANGSFRAGE "
        f"entfernen, die Lücke ist geschlossen.")


def test_der_gefundene_fall_namentlich():
    """§ 35a namentlich, damit ein Rückbau auffällt und nicht bloss der Sweep grün bleibt."""
    assert BINDUNG["hh_hat_aufwendungen"].get("eingangsfrage") is True
    s = ST.leerer_store(2025, fall_id="p35a")
    r = TR.naechste_fragen(s, BINDUNG)
    platz = {f: i for i, f in enumerate(r)}
    assert platz["hh_hat_aufwendungen"] < platz["hh_handwerker_keine_foerderung"], (
        "Die Förder-Detailfrage kommt wieder vor der Frage, ob es überhaupt Handwerkerkosten gab.")


# ---------------------------------------------------------------- ein Thema am Stück

def _wechsel(felder):
    """Wie oft wechselt die Queue das Thema?"""
    regeln = [(BINDUNG[f].get("quelle") or {}).get("regel_id") or "" for f in felder]
    return sum(1 for a, b in zip(regeln, regeln[1:]) if a != b), len(set(regeln))


def test_die_queue_springt_nicht_zwischen_themen_hin_und_her():
    """Julius, 2026-08-25: „abwechselnd fragen zu kindern, behinderung, arbeitsort, dann wieder
    kinder ist schwer nachvollziehbar."

    GEMESSEN vorher: 175 Wechsel bei 62 Themen — fast dreimal so viele Sprünge wie nötig, weil
    Gates nach Gewicht und Slots nach Unsicherheits-Beitrag stehen und beides quer zum Thema läuft.

    Nachher: **61 Wechsel bei 62 Themen** — das theoretische Minimum, jedes Thema genau einmal.
    Deshalb steht hier eine exakte Schranke und keine Ratsche: mehr als ein Wechsel je Thema
    bedeutet, dass ein Thema wieder auseinandergerissen wird.

    Ein Zwischenstand, der grün ausgesehen hätte: Gates und Slots getrennt zu gruppieren ergab 84
    Wechsel und liess jedes Thema ZWEIMAL erscheinen — die Zweitwohnung mit zehn Fragen vorn und
    drei weiteren viel später. Eine Schranke von 100 hätte das durchgelassen."""
    s = ST.leerer_store(2025, fall_id="reihenfolge")
    q = TR.naechste_fragen(s, BINDUNG)
    wechsel, themen = _wechsel(q)
    assert wechsel == themen - 1, (
        f"{wechsel} Themenwechsel bei {themen} Themen — {wechsel - themen + 1} Thema/Themen "
        f"kommen mehrfach vor, die Queue springt wieder hin und her.")
    assert wechsel >= themen - 1, (
        f"{wechsel} Wechsel bei {themen} Themen — weniger als ein Wechsel je Thema ist unmöglich; "
        f"die Messung stimmt nicht.")


def test_ein_thema_folgt_der_reihenfolge_des_vordrucks():
    """Innerhalb eines Themas stehen die Felder mit ELSTER-Kennzahl in der Reihenfolge des
    amtlichen Vordrucks — die Nummerierung bildet sie ab und kostet nichts.

    ANLASS, Julius' E2E-Durchgang am 2026-08-27: der Kinderfreibetrags-Block begann mit VIER
    Fragen zum ANDEREN Elternteil (Geburtsdatum, Verhältnis, Name, Zeitraum), während der Vorname
    des eigenen Kindes auf Platz 12 von 13 stand. Sortiert hatte bis dahin der
    Unsicherheits-Beitrag, und der ist quer zu allem, was ein Mensch erwartet.

    Geprüft wird am Kind-Block namentlich, weil er der gefundene Fall ist und mit 13 Feldern der
    grösste — und zusätzlich über alle Themen, damit die Regel nicht auf einen Einzelfall
    zusammenschrumpft."""
    s = ST.leerer_store(2025, fall_id="vordruck")
    for fid, e in BINDUNG.items():
        if e.get("screening"):
            ST.append_event(s, feld_id=fid,
                            wert=(False if fid == "kein_kind"
                                  else fid.startswith(("kein_", "keine_"))),
                            zustand="bestaetigt",
                            herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                      "haftung": "nutzer"},
                            schreiber="ui:laie",
                            signal={"signal_1": None, "signal_2": "klick"}, bindung=BINDUNG)
    q = TR.naechste_fragen(s, BINDUNG)

    kind = [f for f in q
            if (BINDUNG[f].get("quelle") or {}).get("regel_id") == "p32_6_kinderfreibetraege"]
    assert len(kind) >= 10, f"Nur {len(kind)} Kind-Felder — Messung prüfen."
    assert kind[0] == "kind_vorname", (
        f"Der Kind-Block beginnt mit {kind[0]!r} statt mit dem Vornamen — im Vordruck steht er "
        f"als erstes (E0500107).")

    # Dieselben Ausnahmen wie im Traverser, und zwar aus demselben Grund: Eingangsfragen und
    # echte Gates stehen vorn, WEIL ihre Antwort andere Fragen streicht. Der Vordruck ordnet die
    # Felder, bei denen es nur um Reihenfolge geht. Ohne diese Ausnahme meldet der Test die
    # Eingangsfrage des Betreuungs-Blocks (E0506105, „Wie viel hast du für die Betreuung
    # gezahlt?") als Verstoss — sie steht zu Recht vor den kleineren Kennzahlen.
    def _sortierbar(f):
        return (BINDUNG[f].get("elster_kz")
                and not BINDUNG[f].get("eingangsfrage")
                and GEWICHT.get(f, 0) == 0)

    verdreht = []
    je_thema: dict[str, list[str]] = {}
    for f in q:
        je_thema.setdefault((BINDUNG[f].get("quelle") or {}).get("regel_id") or "", []).append(f)
    for regel, felder in je_thema.items():
        for formal in (True, False):
            kz = [BINDUNG[f]["elster_kz"] for f in felder if _sortierbar(f)
                  and (("geltungsbedingung" in (BINDUNG[f].get("quelle") or {})) is formal)]
            if len(kz) > 1 and kz != sorted(kz):
                verdreht.append(f"{regel} ({'formales Gate' if formal else 'Wertfeld'}): {kz[:6]}")
    assert not verdreht, ("Kennzahlen nicht in Vordruck-Reihenfolge:\n  " + "\n  ".join(verdreht))


def test_zusammenhaengende_themen_bleiben_zusammen():
    """Der Fall, an dem es Julius auffiel, namentlich: die Zweitwohnung. Ihre dreizehn Fragen
    standen über die Queue verteilt; jetzt stehen sie an einem Stück, mit der Eingangsfrage
    vorneweg."""
    s = ST.leerer_store(2025, fall_id="dhf-block")
    q = TR.naechste_fragen(s, BINDUNG)
    plaetze = [i for i, f in enumerate(q) if f.startswith("dhf_")]
    assert len(plaetze) >= 10, f"Nur {len(plaetze)} dhf-Fragen — Messung prüfen."
    spanne = plaetze[-1] - plaetze[0] + 1
    assert spanne == len(plaetze), (
        f"Die {len(plaetze)} Zweitwohnungs-Fragen sind über {spanne} Plätze verteilt — dazwischen "
        f"stehen {spanne - len(plaetze)} fremde Fragen.")
# Die Existenzfrage steht seit 2026-08-26 in der Ankreuzliste (`keine_zweitwohnung`), nicht
    # mehr im Block selbst — `dhf_eigener_hausstand` fragt nach dem Hauptwohnsitz und taugte nie
    # als Eingang. Geprueft wird deshalb nur noch, dass der Block zusammenhaengt.
    assert all(q[i].startswith("dhf_") for i in range(plaetze[0], plaetze[-1] + 1))
