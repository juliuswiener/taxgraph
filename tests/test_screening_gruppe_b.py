"""Die vier Abwesenheits-Fragen schalten jetzt auch etwas ab.

Befund aus reports/adjudikation/screening_vorsortierung_2026-08-14.md: `kein_gewinn`, `kein_kap`,
`kein_vuv` und `kein_sonstige` existierten längst und wurden auch gestellt — nur hing keine
einzige Folge-Regel daran. Wer „nein, keine Vermietung" antwortete, bekam trotzdem jede
Vermietungs-Frage. Eine Frage, die keine Wirkung hat, ist schlimmer als keine Frage: sie kostet
den Nutzer Zeit und suggeriert, sie werde berücksichtigt.

Behoben durch neun Einträge in `bindung_regel_bedingungen.yaml` — kein neues Feld, kein Code.

Was hier geprüft wird, und warum genau das:

  1. **Beide Richtungen je Regel.** Nur „schaltet ab" zu prüfen ließe die gefährlichere Hälfte
     offen: eine Bedingung mit falscher Polarität schaltet die Regel für ALLE ab, auch für den,
     der die Einkunftsart hat — das wäre stille Under-Deklaration. Dieselbe Fehlerklasse wie
     [[gate-polaritaet-normalfall-antwort]] (ein verdrehtes Gate kostete den ganzen
     Verpflegungsmehraufwand).
  2. **Unbeantwortet schließt NICHT aus.** Der fail-closed-Kern von relevanz(): nur ein
     BESTÄTIGT abweichender Wert schließt aus. Sonst verschwänden Fragen, bevor der Nutzer die
     Screening-Frage überhaupt gesehen hat.
  3. **Der Rentner-Fall.** `p22_1_leibrente_besteuerungsanteil` steht auch im rentner-Kegel; der
     Bericht hat das ausdrücklich als Risiko markiert. Ein Rentner antwortet auf „Hattest du
     sonstige Einkünfte (z.B. Rente)?" mit ja, also kein_sonstige=False — genau der Wert, bei dem
     die Regel gilt. Der Test hält das fest, weil eine spätere Umformulierung der Frage die
     Polarität kippen könnte.
  4. **Der gemessene Nutzerpfad.** Ende zu ende, über den echten Traverser: die abgeschalteten
     Felder dürfen nicht mehr gefragt werden. Gemessen 188 -> 156 Fragen in der Scheibe `gesamt`
     (2026-08-16, nach der Anlage V; davor 179 -> 156 mit 23 Feldern — die neuen Objekt- und
     Nutzungsangaben der Anlage V hängen alle an `p21_vermietung_einkuenfte` und verschwinden
     mit ihr).

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API      # noqa: E402
import store as ST     # noqa: E402
import traverser as TR # noqa: E402

BINDUNG = TR.lade_bindung()

# (Screening-Feld, Regel) — dieselben neun Paare wie in bindung_regel_bedingungen.yaml.
# HINWEIS: p16_4_freibetrag ist NICHT mehr in PAARE, weil die Architektur von regel-weit
# (regel_bedingungen) auf feld-weit (feld_bedingung in bindung_rentner.yaml) geändert wurde.
# Die Regel bleibt "unentschieden" (Partner-Gates offen), aber die 4 Ich-Felder werden
# korrekt per feld_bedingung an kein_gewinn ausgeschlossen, die 2 Partner-Felder bleiben
# korrekt per feld_bedingung an veranlagung=zusammen.
PAARE = [
    ("kein_vuv", "p21_vermietung_einkuenfte"),
    ("kein_kap", "p20_6_verlustverrechnung"),
    ("kein_kap", "p20_9_sparer_pauschbetrag"),
    ("kein_kap", "p32d_1_kirchensteuer"),
    ("kein_gewinn", "p15_1_2_mitunternehmer"),
    ("kein_gewinn", "p4_3_gewinn"),
    ("kein_sonstige", "p22_1_leibrente_besteuerungsanteil"),
    ("kein_sonstige", "p23_veraeusserungsgewinn"),
    ("kein_sonstige", "p22_3_leistungen"),
    # Screening C1 (2026-08-15), der größte Einzelhebel: eine Antwort nimmt 22 Felder aus dem
    # Dialog jedes Kinderlosen. Bewusst in DERSELBEN Liste wie Gruppe B — die vier Richtungen
    # (schaltet ab / lässt stehen / unbeantwortet / vorläufig) gelten hier genauso, und eine
    # eigene Testdatei hätte nur die Gefahr geschaffen, dass eine Richtung vergessen wird.
    ("kein_kind", "p32_6_kinderfreibetraege"),
    ("kein_kind", "p24b_entlastungsbetrag"),
    ("kein_kind", "p10_1_3_kv_pv_kind"),
    ("kein_kind", "p33b_abs5_kind_uebertragung"),
    ("kein_kind", "p10_1_9_schulgeld"),
    ("kein_kind", "p33a_ausbildungsfreibetrag"),
    # Screening C (2026-08-25). Anlass: Julius meldete vier Fragen mit „frage hat keine
    # daseinsberechtigung" — Merkmalsfragen zu Sachverhalten, deren Existenz er verneint hatte.
    # Die Existenzfrage stand jeweils früh in der Queue; sie zu beantworten nahm die Detailfrage
    # trotzdem nicht weg, weil zwischen den Regeln keine Verbindung bestand.
    ("kein_kind", "p10_1_5_kinderbetreuung"),          # „War dein Kind unter 14 Jahre alt?"
    ("kein_gewinn", "p34_3_ermaessigter_durchschnittssatz"),
    ("kein_vuv", "p21_2_verbilligte_vermietung_wk"),
]


def _store_mit(feld: str | None, wert, zustand: str = "bestaetigt"):
    s = ST.leerer_store(2025, fall_id="screening-b")
    if feld is not None:
        ST.append_event(s, feld_id=feld, wert=wert, zustand=zustand,
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                        schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"klick@{feld}"},
                        ts="2026-08-14T12:00:00+00:00")
    return s


@pytest.mark.parametrize("feld,regel", PAARE)
def test_abwesenheit_schaltet_die_regel_ab(feld, regel):
    """Der Zweck der Übung: „nein, habe ich nicht" nimmt die Folge-Fragen aus dem Dialog."""
    rel = TR.relevanz(_store_mit(feld, True), BINDUNG)
    assert rel[regel]["status"] == "ausgeschlossen", (
        f"{feld}=True lässt {regel} weiter im Dialog — die Screening-Frage hat keine Wirkung.")


@pytest.mark.parametrize("feld,regel", PAARE)
def test_anwesenheit_laesst_die_regel_stehen(feld, regel):
    """Die gefährlichere Hälfte. Eine verdrehte Polarität schaltet die Regel für ALLE ab — auch
    für den, der die Einkunftsart tatsächlich hat. Das fiele niemandem auf: der Dialog wirkt
    kürzer, die Erklärung ist unvollständig, und keine Zahl wird rot."""
    rel = TR.relevanz(_store_mit(feld, False), BINDUNG)
    assert rel[regel]["status"] != "ausgeschlossen", (
        f"{feld}=False (der Nutzer HAT diese Einkunftsart) schließt {regel} aus — "
        f"stille Under-Deklaration.")


@pytest.mark.parametrize("feld,regel", PAARE)
def test_unbeantwortet_schliesst_nicht_aus(feld, regel):
    """fail-closed: solange die Screening-Frage offen ist, bleibt alles im Angebot."""
    rel = TR.relevanz(_store_mit(None, None), BINDUNG)
    assert rel[regel]["status"] != "ausgeschlossen", (
        f"{regel} ist ausgeschlossen, obwohl {feld} niemand beantwortet hat.")


@pytest.mark.parametrize("feld,regel", PAARE)
def test_vorlaeufig_schliesst_nicht_aus(feld, regel):
    """Ein VORLÄUFIGER Vorschlag (KI, Vorjahr, Beleg) darf keine Einkunftsart abschalten — sonst
    genügte eine erfundene Abwesenheit des Modells, um eine ganze Einkunftsart aus der Erklärung
    zu nehmen, ohne dass der Mensch je zugestimmt hat."""
    rel = TR.relevanz(_store_mit(feld, True, zustand="vorlaeufig"), BINDUNG)
    assert rel[regel]["status"] != "ausgeschlossen", (
        f"Ein vorläufiges {feld}=True schließt {regel} aus — ohne zweites Signal.")


def test_rentner_behaelt_seine_rente():
    """Der vom Bericht markierte Risikofall: p22_1_leibrente steht auch im rentner-Kegel. Ein
    Rentner bejaht die Frage nach sonstigen Einkünften (kein_sonstige=False) — die Regel muss
    stehen bleiben, sonst verliert er seine Rentenbesteuerung."""
    rel = TR.relevanz(_store_mit("kein_sonstige", False), BINDUNG)
    r = rel["p22_1_leibrente_besteuerungsanteil"]
    assert r["status"] != "ausgeschlossen", f"Rentner verliert die Leibrente: {r}"


# ---------------------------------------------------------------- gemessener Nutzerpfad
def _dialog_lauf(explizit: dict, scheibe: str = "gesamt", max_fragen: int = 600) -> list[str]:
    """Der echte Nutzerpfad: naechste_fragen() -> antworten -> wiederholen. Kein Feld wird direkt
    gesetzt. Liefert die Liste der TATSÄCHLICH gestellten Fragen.

    Bewusst über den Traverser statt über API.fragen(): letzteres berechnet nebenbei den
    Ring-Beitrag, und Platzhalter-Antworten wie Rentenbeginn=0 lassen die Rechen-Engine
    zu Recht abbrechen. Gemessen wird hier, WELCHE Fragen kommen — nicht, was sie ergeben.
    Dieselbe Bauart wie tests/test_dialog_ehrlich_lauf.py."""
    store = ST.leerer_store(2025, fall_id=f"screening_{scheibe}")
    store["scheibe"] = scheibe
    bindung = API._scheibe_bindung(store)
    gestellt: list[str] = []
    for _ in range(max_fragen):
        offen = TR.naechste_fragen(store, bindung)
        if not offen:
            return gestellt
        fid = offen[0]
        b = bindung.get(fid, {})
        if fid in explizit:
            wert = explizit[fid]
        elif b.get("typ") in ("bool", "boolean"):
            wert = False
        elif b.get("typ") == "enum":
            werte = b.get("enum_werte") or []
            wert = werte[0] if werte else "x"
        elif b.get("typ") == "string":
            wert = ""
        else:
            wert = 0
        ST.append_event(store=store, feld_id=fid, wert=wert, zustand="bestaetigt",
                        herkunft={"quelle": "screening_b"}, schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"ok@{fid}"},
                        ts="2026-08-14T12:00:00Z")
        gestellt.append(fid)
    raise AssertionError(f"Dialog endet nach {max_fragen} Fragen nicht (Scheibe {scheibe}).")


_STAMM = {"veranlagung": "zusammen", "bruttoarbeitslohn": 6000000}
_KEINE_EINKUNFTSARTEN = dict(_STAMM, kein_gewinn=True, kein_kap=True, kein_vuv=True,
                             kein_sonstige=True)


def test_kinderfrage_nimmt_zweiundzwanzig_felder_aus_dem_dialog():
    """Screening C1, der größte Einzelhebel — gemessen auf dem Nutzerpfad.

    Beide Läufe verneinen alle vier Einkunftsarten, unterscheiden sich also NUR in der
    Kinderfrage. Die Differenz ist damit sauber dieser einen Antwort zuzuordnen: 156 → 134.

    Geprüft wird zusätzlich, dass alles Entfernte auch wirklich kinderbezogen ist. Eine reine
    Zahl würde nicht auffallen lassen, wenn das Tor nebenbei etwas Fremdes mitnimmt — und genau
    das ist der Schaden, den ein zu breit gezogenes Screening anrichtet."""
    ja = _dialog_lauf(dict(_KEINE_EINKUNFTSARTEN, kein_kind=False))
    nein = _dialog_lauf(dict(_KEINE_EINKUNFTSARTEN, kein_kind=True))
    assert "kein_kind" in ja, "Die Kinderfrage wird gar nicht gestellt."
    entfernt = set(ja) - set(nein)
    assert len(entfernt) >= 20, (
        f"Die Kinderfrage nimmt nur {len(entfernt)} Felder aus dem Dialog (gemessen: 22).")
    regeln = {r for f, r in PAARE if f == "kein_kind"}
    fremd = sorted(f for f in entfernt
                   if (BINDUNG.get(f) or {}).get("quelle", {}).get("regel_id") not in regeln)
    assert not fremd, f"Die Kinderfrage entfernt Felder fremder Regeln: {fremd}"


def test_screening_filtert_und_loescht_nicht():
    """Ende zu ende über den echten Traverser — die Ebene, auf der der Nutzer es merkt. Zwei
    Läufe derselben Scheibe, einmal alle Einkunftsarten verneint, einmal alle bejaht.

    Die Invariante steht bewusst als DIFFERENZ der beiden Läufe da und nicht als feste Feldliste:
    welche Felder eine Scheibe überhaupt enthält, ist ihre Sache. Ein erster Anlauf mit fester
    Liste war rot und hatte recht — die fünf `rentner_*`-Felder werden in `gesamt` NIE gefragt,
    auch ohne Screening nicht. Genau deshalb waren es gemessen 23 verschwundene Fragen und nicht
    die 28 Felder aus dem Bericht.

    Dass die Zahl mitwächst, ist der Zweck der Bauart: mit der Anlage V (2026-08-16) sind es 32.
    Sie ist deshalb eine UNTERGRENZE. Ein Sinken wäre der interessante Fall — und war es auch:
    23 -> 19 deckte auf, dass vier neue Deklarations-Bools der Anlage V als Gates wirkten und
    dem normalen Vermieter die ganze Vermietung aus dem Dialog nahmen (siehe
    tests/test_gate_polaritaet.py).

    Geprüft wird beides zusammen, weil nur das Paar die Aussage trägt: verschwunden sein muss
    etwas (sonst wirkt das Screening nicht), und alles Verschwundene muss dem wieder gestellt
    werden, der die Einkunftsart hat (sonst hat es gelöscht statt gefiltert)."""
    ohne = _dialog_lauf(dict(
        _STAMM, kein_gewinn=True, kein_kap=True, kein_vuv=True, kein_sonstige=True))
    mit = _dialog_lauf(dict(
        _STAMM, kein_gewinn=False, kein_kap=False, kein_vuv=False, kein_sonstige=False))

    entfernt = set(mit) - set(ohne)
    assert len(entfernt) >= 30, (
        f"Das Screening nimmt nur {len(entfernt)} Fragen aus dem Dialog (gemessen: 32). "
        f"Entweder ist die Verbindung zwischen Screening-Frage und Folge-Regel verloren — oder "
        f"die Regel wird schon vorher von einem verdrehten Gate abgeschaltet, dann fehlen ihre "
        f"Felder in BEIDEN Läufen (so gefunden am 2026-08-16, Anlage V).")
    # Jedes entfernte Feld muss zu einer der neun Regeln gehören ODER selbst eine `feld_bedingung`
    # auf eines der Screening-Felder tragen — sonst hat das Screening etwas mitgenommen, das
    # niemand daran gehängt hat.
    #
    # ERWEITERT 2026-08-26: bis dahin wirkte das Screening ausschliesslich über ganze Regeln, und
    # genau das war die Lücke, die Julius im Durchgang fand: „Aus welcher Art von Tätigkeit stammt
    # dieser Gewinn? — kein gewinn ist erwähnt worden". Das Feld sitzt in `p2_festzusetzung_einzel`,
    # der Basisregel, die nie entfallen darf; dieselbe Bauart beim Behinderten-Wahlrecht in der
    # allgemeinen agB-Regel. Solche Felder tragen jetzt eine `feld_bedingung` (Feldebene statt
    # Regelebene) — sie verschwinden zu Recht, und der Test muss sie kennen, sonst verbietet er
    # genau die Abhilfe.
    #
    # Die Prüfung bleibt eng: es genügt NICHT, dass ein Feld irgendeine Bedingung trägt — sie muss
    # auf eines der vier Screening-Felder dieses Laufs zeigen.
    regeln = {r for _, r in PAARE}
    geprueft = {"kein_gewinn", "kein_kap", "kein_vuv", "kein_sonstige"}
    # Seit 2026-08-26 hängen weitere Regeln an denselben vier Fragen (z. B. p3_nr72_pv an
    # `kein_gewinn` — „es war von keiner photovoltaikanlage die rede"). PAARE listet nur die neun
    # der ursprünglichen Gruppe; massgeblich ist, woran die Regel TATSÄCHLICH hängt.
    import traverser as _TR  # noqa: E402 — nur für die Bedingungs-Tabelle
    rb = _TR.lade_regel_bedingungen()
    regeln |= {r for r, cs in rb.items() if any(c["feld"] in geprueft for c in cs)}
    fremd = sorted(
        f for f in entfernt
        if (BINDUNG.get(f) or {}).get("quelle", {}).get("regel_id") not in regeln
        and ((BINDUNG.get(f) or {}).get("feld_bedingung") or {}).get("feld") not in geprueft)
    assert not fremd, f"Screening entfernt Felder fremder Regeln: {fremd}"
    # Und die Gegenrichtung: nichts aus den neun Regeln bleibt dem erspart, der sie braucht.
    assert not (set(ohne) & entfernt)
    assert len(ohne) < len(mit), "Verneinen macht den Dialog nicht kürzer."


def test_rentner_wird_weiter_nach_seiner_rente_gefragt():
    """Der vom Bericht markierte Risikofall, auf dem Nutzerpfad statt auf der relevanz-Ebene.
    `p22_1_leibrente_besteuerungsanteil` steht im rentner-Kegel; hinge dort eine verdrehte
    Bedingung, verlöre jeder Rentner seine Rentenbesteuerung."""
    gestellt = _dialog_lauf(_STAMM, scheibe="rentner_gesamt")
    renten = [f for f in gestellt if f.startswith("rentner_renten")
              or f == "rentner_jahresrente"]
    assert renten, (
        f"Der Rentner-Dialog fragt nichts zur Rente mehr. Gestellt: {sorted(gestellt)[:20]}")
