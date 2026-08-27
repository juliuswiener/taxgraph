"""Traverser — der Regel-Graph in zwei Leserichtungen (K1, Task #11). READ-ONLY, NULL LLM.

Rückwärts = Interview (`relevanz`, `naechste_fragen`), vorwärts = Beweis (`justification`,
`trace_ergebnis`). Reine Ableitung über Bindungstabelle + rules.yaml + Store; keine Catala-
Introspektion (Grenze, s. KONZEPT.md). Die EINZIGE Sicht, die Paket B (Haut) liest; geschrieben
wird ausschließlich über `store.append_event` (API.md).
"""
from __future__ import annotations

import functools
import glob
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
ROOT = os.path.dirname(PRODUKT)


# ---------------------------------------------------------------- Loader

def _yaml():
    import yaml
    return yaml


@functools.lru_cache(maxsize=1)
def lade_bindung() -> dict:
    """feld_id -> Bindungs-Eintrag (über alle bindung_*.yaml). Pro Prozess gecacht (statischer
    Repo-Content, ändert sich nie zur Laufzeit) — war ungecacht ~161ms/Call, Hotpath in JEDEM
    api.py-Handler."""
    yaml = _yaml()
    out = {}
    for f in glob.glob(os.path.join(PRODUKT, "bindung", "bindung_*.yaml")):
        d = yaml.safe_load(open(f))
        for b in d.get("bindungen", []):
            out[b["feld_id"]] = b
    return out


def lade_rules() -> dict:
    yaml = _yaml()
    doc = yaml.safe_load(open(os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")))
    return {r["rule_id"]: r for r in doc["regeln"]}


def lade_guenstiger() -> dict:
    yaml = _yaml()
    return yaml.safe_load(open(os.path.join(HERE, "guenstiger_liste.yaml")))


@functools.lru_cache(maxsize=1)
def lade_regel_bedingungen() -> dict:
    """regel_id -> Liste strukturierter Ob-Bedingungen ({regel_id, feld, wert, grund}) über alle
    bindung_*.yaml (schema.json $defs/regel_bedingung). Regel-weit, unabhängig von den eigenen
    Gate-Feldern der Regel (relevanz() prüft `feld` gegen den Store, nicht gegen die Regel selbst)."""
    yaml = _yaml()
    out = {}
    for f in glob.glob(os.path.join(PRODUKT, "bindung", "bindung_*.yaml")):
        d = yaml.safe_load(open(f))
        for rb in d.get("regel_bedingungen", []):
            out.setdefault(rb["regel_id"], []).append(rb)
    return out


@functools.lru_cache(maxsize=1)
def lade_themen_zuerst() -> list[str]:
    """Regeln, die den Fragebogen eröffnen — in dieser Reihenfolge.

    Deklariert in bindung_regel_bedingungen.yaml, nicht hier: welche Themen den Einstieg bilden,
    ist eine fachliche Entscheidung und keine Eigenschaft des Sortierverfahrens.
    """
    yaml = _yaml()
    out = []
    for f in glob.glob(os.path.join(PRODUKT, "bindung", "bindung_*.yaml")):
        d = yaml.safe_load(open(f)) or {}
        for e in (d.get("themen_zuerst") or []):
            if e["regel_id"] not in out:
                out.append(e["regel_id"])
    return out


@functools.lru_cache(maxsize=1)
def lade_instanz_gruppen() -> dict:
    """gruppe -> {gruppe, anzahl_feld, etikett, max, grund} über alle bindung_*.yaml
    (schema.json $defs/instanz_gruppe).

    Sagt, WOHER die Zahl der Instanzen kommt. Ohne Eintrag verhält sich eine Gruppe wie eine
    einzige Instanz — das ist der Zustand aller Gruppen ausser `kind` (2026-08-25), und ehrlicher
    als eine geratene Anzahl.

    GECACHT, wie `lade_bindung` daneben — und das ist hier keine Mikro-Optimierung: `/fragen` ruft
    `instanz_anzahl()` für JEDE Frage der Queue auf, und 69 Felder tragen eine `instanz_gruppe`.
    Ohne Cache parste ein einziger Fragen-Aufruf sämtliche bindung_*.yaml 69-mal von Platte.
    GEMESSEN 2026-08-25, am Tag des Baus: `/fragen` 24,7 s — und zwar bei jedem Aufruf, also nach
    jeder beantworteten Frage. 147 UI-Tests liefen daran in den Timeout.
    """
    yaml = _yaml()
    out = {}
    for f in glob.glob(os.path.join(PRODUKT, "bindung", "bindung_*.yaml")):
        d = yaml.safe_load(open(f))
        for g in d.get("instanz_gruppen", []):
            out[g["gruppe"]] = g
    return out


def instanz_anzahl(store: dict, bindung: dict, feld_id: str) -> tuple[int, str]:
    """Wie viele Eingabefelder braucht `feld_id` — und wie heisst eine Instanz?

    (1, "") für jedes Feld ohne Instanz-Achse oder ohne gepflegte Gruppe. Sonst die BESTÄTIGTE
    Zahl aus dem Zählfeld, begrenzt auf `max`.

    Nur BESTÄTIGT zählt: ein vorläufiger Vorschlag der KI („2 Kinder") darf nicht darüber
    entscheiden, wie viele Felder der Nutzer ausfüllen soll — er hat ihn ja noch nicht gesehen.
    Solange die Zahl fehlt, bleibt es bei einem Feld, und das Feld verhält sich wie bisher.
    """
    b = bindung.get(feld_id) or {}
    gruppe = b.get("instanz_gruppe")
    if not gruppe:
        return 1, ""
    g = lade_instanz_gruppen().get(gruppe)
    if not g:
        return 1, ""
    ev = _aktive_events(store).get(g["anzahl_feld"])
    if _unbeantwortet(ev):
        return 1, g["etikett"]
    n = ev.get("wert")
    if not isinstance(n, int) or isinstance(n, bool) or n < 1:
        return 1, g["etikett"]
    return min(n, int(g["max"])), g["etikett"]


def _aktive_events(store: dict) -> dict:
    """feld_id -> aktuell aktives Event (nicht durch ein späteres ersetzt)."""
    ersetzt = {e["ersetzt"] for e in store.get("events", []) if e.get("ersetzt")}
    aktiv = {}
    for e in store.get("events", []):
        if e["event_id"] in ersetzt:
            continue
        aktiv[e["feld_id"]] = e
    return aktiv


def _unbeantwortet(ev) -> bool:
    return ev is None or ev.get("zustand") == "vorlaeufig"


# ---------------------------------------------------------------- (a) RÜCKWÄRTS

def _regel_ids(bindung: dict) -> set:
    return {b["quelle"]["regel_id"] for b in bindung.values()}


def relevanz(store: dict, bindung: dict) -> dict:
    """Je Regel: status (ausgeschlossen|relevant|unentschieden) + offene Gates + offene Annahmen.

    Gate = askable bool-Geltungsbedingung. `false` (bestätigt) -> ausgeschlossen; offen/vorlaeufig ->
    unentschieden. Nicht-askable (berechnete) Geltungsbedingungen sind KEIN Gate, werden aber als
    `annahmen_offen` geführt (nie still als erfüllt, Auflage 1-Zusatz).

    `gate: false` (Bindungs-Schema) nimmt ein askables Feld aus den Gates heraus: es ist DEKLARATION,
    keine Rechen-Voraussetzung. Der Vordruck verlangt die Angabe ("wurde als Ferienwohnung
    genutzt?"), die Regel gilt in beide Richtungen. Ohne diese Unterscheidung nahm ein "nein" des
    Normalfalls dem Vermieter die ganze Anlage V aus dem Dialog (gemessen 2026-08-16). Die
    Geltungsbedingung erscheint dann als offene Annahme — nie still als erfüllt.

    Zusätzlich `regel_bedingungen` (lade_regel_bedingungen, schema.json $defs/regel_bedingung):
    strukturierte Ob-Bedingung AUSSERHALB der eigenen Regel-Felder (z.B. p2_festzusetzung_zusammen
    gilt nur bei veranlagung=="zusammen" — ein Feld, das selbst zu einer ANDEREN Regel gehört, kann
    ein bool-Gate strukturell nie sein). Bestätigt UND abweichend -> ausgeschlossen; unbeantwortet/
    vorläufig schließt NICHT aus (fail-closed, wie die Gates)."""
    aktiv = _aktive_events(store)
    bedingungen = lade_regel_bedingungen()
    out = {}
    for rid in _regel_ids(bindung):
        gates, annahmen = [], []
        for fid, b in bindung.items():
            q = b["quelle"]
            if q["regel_id"] != rid or "geltungsbedingung" not in q:
                continue
            if b.get("askable") and b.get("gate", True):
                gates.append(fid)
            else:
                annahmen.append(q["geltungsbedingung"])
        status, offen = "relevant", []
        for cond in bedingungen.get(rid, []):
            ev = aktiv.get(cond["feld"])
            if not _unbeantwortet(ev) and ev.get("wert") != cond["wert"]:
                status = "ausgeschlossen"
        if status != "ausgeschlossen":
            for fid in gates:
                ev = aktiv.get(fid)
                if _unbeantwortet(ev):
                    offen.append(fid)
                elif ev.get("wert") is False:
                    status = "ausgeschlossen"
                    break
            if status != "ausgeschlossen":
                status = "unentschieden" if offen else "relevant"
        out[rid] = {"status": status, "gates_offen": sorted(offen),
                    "annahmen_offen": sorted(annahmen)}
    return out


def gate_gewicht(bindung: dict) -> dict:
    """feld_id -> Zahl der askable Felder, die die Antwort auf dieses Gate abschalten kann.

    Das ist die Reihenfolge-Information, die im Graphen ohnehin steckt: ein Gate, dessen „nein"
    ganze Regeln streicht, erspart dem Nutzer mehr Fragen als eines, das nur sich selbst betrifft
    — also gehört es nach vorn. Zwei Quellen, beide aus der Bindung, keine Handliste:

      (a) eigenes bool-Gate der Regel — ein bestätigtes False schließt sie aus (relevanz(), s.o.),
          und alle ÜBRIGEN askable Felder derselben Regel entfallen mit;
      (b) das Feld steht als Ob-Bedingung einer FREMDEN Regel in regel_bedingungen — dann zählen
          deren askable Felder.

    Gemessen (Scheibe gesamt, 2026-08-14): veranlagung 38, vpf-Gates 13, hh_hat_aufwendungen 10.
    Vor dieser Sortierung standen die Gates alphabetisch, veranlagung damit auf Frage 203 von 243
    — die Antwort, die über 38 Partner-Felder entscheidet, kam fast zuletzt.

    Wächst von selbst mit: jeder neue regel_bedingungen-Eintrag (Screening-Modell) gibt seinem
    Gate automatisch Gewicht, ohne dass hier etwas nachgetragen werden muss."""
    bedingungen = lade_regel_bedingungen()
    felder_je_regel: dict[str, list[str]] = {}
    for fid, b in bindung.items():
        if b.get("askable"):
            felder_je_regel.setdefault(b["quelle"]["regel_id"], []).append(fid)

    gewicht: dict[str, int] = {}
    for fid, b in bindung.items():
        if not b.get("askable"):
            continue
        q = b["quelle"]
        n = 0
        if "geltungsbedingung" in q and b.get("typ") in ("bool", "boolean"):
            n += sum(1 for f in felder_je_regel.get(q["regel_id"], []) if f != fid)
        for rid, conds in bedingungen.items():
            if any(c["feld"] == fid for c in conds):
                n += len(felder_je_regel.get(rid, []))
        gewicht[fid] = n
    return gewicht


def naechste_fragen(store: dict, bindung: dict, beitrag: dict | None = None) -> list[str]:
    """Geordnete Interview-Queue: unbeantwortete askable Felder nicht-ausgeschlossener Regeln.
    Gating-Bedingungen zuerst (streichen ganze Regeln), dann Slots nach Unsicherheits-Beitrag
    (aus intervall.py, wenn übergeben), sonst deterministisch feld_id-sortiert.

    Die Gates untereinander stehen nach gate_gewicht() — wer viel abschaltet, kommt zuerst; bei
    gleichem Gewicht alphabetisch, damit die Reihenfolge deterministisch bleibt.

    Günstiger-sicher by construction: ALLE unbeantworteten askable Felder nicht-ausgeschlossener
    Regeln kommen in die Queue — kein Zweig wird anhand eines vorläufigen Siegers weggeschnitten."""
    rel = relevanz(store, bindung)
    aktiv = _aktive_events(store)
    kand = [fid for fid, b in bindung.items()
            if b.get("askable") and _unbeantwortet(aktiv.get(fid))
            and rel[b["quelle"]["regel_id"]]["status"] != "ausgeschlossen"
            and not _feld_ausgeschlossen(b, aktiv)]
    gw = gate_gewicht(bindung)
    # „Gate" heißt hier: die Antwort streicht andere Fragen — nicht: das Feld trägt technisch eine
    # geltungsbedingung. Beides fällt auseinander, und zwar beim wichtigsten Feld überhaupt:
    # `veranlagung` hat KEINE geltungsbedingung (es wirkt über regel_bedingungen auf
    # p2_festzusetzung_zusammen) und landete deshalb bei den Slots — alphabetisch auf Frage 203 von
    # 243, obwohl seine Antwort 38 Partner-Felder entscheidet. Wer abschaltet, kommt nach vorn.
    # BEI GLEICHEM GEWICHT ENTSCHIED BISHER DER BUCHSTABE — und das ist keine Ordnung, sondern
    # Zufall. Gemessen 2026-08-21 im echten Nutzerlauf: § 35a führt vier Gates mit je Gewicht 10;
    # `hh_handwerker_keine_foerderung` steht alphabetisch vor `hh_hat_aufwendungen`. Julius bekam
    # deshalb "Wurden die Handwerkerleistungen NICHT öffentlich gefördert?", bevor überhaupt
    # gefragt war, ob er Handwerkerkosten hatte — eine Frage nach dem Merkmal eines Sachverhalts,
    # dessen Existenz niemand erhoben hatte. Antwortet er darauf "nein", ist die Regel
    # ausgeschlossen und die Eingangsfrage kommt nie: das Ergebnis stimmt zufällig, die Begründung
    # nicht. In ZEHN Regeln entscheidet so der Feldname (tests/test_eingangsfrage_zuerst.py misst es).
    #
    # `eingangsfrage: true` in der Bindung bricht den Gleichstand: die Frage nach der EXISTENZ
    # kommt vor jeder Frage nach MERKMALEN. Deklariert statt geraten — eine Heuristik über den
    # Feldnamen ("hat_", "kein_") ist genau der Fehler, der bei der Frage-Polarität schon einmal
    # zwei Feldern das Gegenteil der Nutzerantwort entlockt hat (s. `frage_invertiert`).
    ist_gate = lambda f: "geltungsbedingung" in bindung[f]["quelle"] or gw.get(f, 0) > 0
    gates = sorted((f for f in kand if ist_gate(f)),
                   key=lambda f: (-gw.get(f, 0), not bindung[f].get("eingangsfrage"), f))
    slots = [f for f in kand if not ist_gate(f)]
    if beitrag:
        slots.sort(key=lambda f: (-beitrag.get(f, 0), f))
    else:
        slots.sort()
    return _nach_themen(gates + slots, bindung, _angefangene_themen(store, bindung))


def _angefangene_themen(store: dict, bindung: dict) -> list[str]:
    """Themen mit mindestens einer BEANTWORTETEN Frage — das zuletzt angefasste zuerst.

    Nur BESTÄTIGT zählt, wie überall sonst: ein vorläufiger KI-Vorschlag ist keine Antwort des
    Nutzers und darf ein Thema nicht für angefangen erklären, das er noch nie gesehen hat.

    Die Reihenfolge der Events IST die Reihenfolge der Antworten (der Log ist append-only), also
    kostet die Auskunft einen Durchlauf über die Events — dieselbe Grössenordnung wie
    `_aktive_events` nebenan, und nichts, was je Feld oder je Thema wiederholt würde.
    """
    zuletzt: dict[str, int] = {}
    for i, e in enumerate(store.get("events", [])):
        if e.get("zustand") != "bestaetigt":
            continue
        t = ((bindung.get(e["feld_id"]) or {}).get("quelle") or {}).get("regel_id")
        if t:
            zuletzt[t] = i
    return sorted(zuletzt, key=lambda t: -zuletzt[t])


def _nach_themen(felder: list[str], bindung: dict,
                 angefangen: list[str] | None = None) -> list[str]:
    """Hält Fragen desselben Themas beisammen, ohne die Rangfolge davor umzuwerfen.

    Julius, 2026-08-25: „wichtig auch dass die fragen in einer für den user sinnvollen reihenfolge
    kommt. abwechselnd fragen zu kindern, behinderung, arbeitsort, dann wieder kinder ist schwer
    nachvollziehbar."

    GEMESSEN vor dem Eingriff: **175 Themenwechsel bei 62 Themen** — fast dreimal so viele Sprünge
    wie nötig; in den ersten 24 Fragen allein 14. Die Ursache ist keine Nachlässigkeit, sondern
    das Ordnungsprinzip: Gates stehen nach Gewicht, Slots nach Unsicherheits-Beitrag, und beides
    ist quer zum Thema. Zwei Vermietungsfragen können durch eine Kinderfrage getrennt sein, weil
    die eben mehr Spanne trägt.

    DIE RANGFOLGE BLEIBT, sie wird nur auf die Themenebene gehoben: ein Thema steht dort, wo sein
    BESTES Feld nach der bisherigen Sortierung stand, und innerhalb des Themas gilt weiterhin die
    bisherige Reihenfolge. Damit steht das wichtigste Thema weiterhin vorn — `veranlagung` trägt
    das höchste Gate-Gewicht und zieht sein Thema mit an den Anfang —, aber niemand wird mehr
    zwischen zwei Themen hin- und hergeschickt.

    Stabil in beide Richtungen: `dict` hält die Einfügereihenfolge, also entscheidet allein die
    Position des ersten Feldes je Thema. Gleiche Eingabe, gleiche Ausgabe.

    ÜBER GATES UND SLOTS HINWEG, und das ist sicher: Weil `gates` vor `slots` steht, beginnt jedes
    Thema weiterhin mit seinen Gate-Fragen. Ein Thema kann seine Detailfragen also nicht vor sein
    eigenes Gate ziehen — und beantwortet der Nutzer das Gate mit „nein", ist die Regel
    ausgeschlossen und ihre Detailfragen stehen in der nächsten Queue gar nicht mehr. Getrennt
    gruppiert (erst alle Gates, dann alle Slots) erschien dagegen jedes Thema ZWEIMAL: die
    Zweitwohnung mit zehn Fragen vorn und drei weiteren viel später — genau das Hin und Her, um
    das es hier geht.
    """
    nach_thema: dict[str, list[str]] = {}
    for f in felder:
        nach_thema.setdefault((bindung[f].get("quelle") or {}).get("regel_id") or "", []).append(f)
    # Die Eingangsfrage steht in ihrem Thema IMMER vorn — auch wenn sie kein Gate ist.
    #
    # `eingangsfrage` brach bisher nur den Gleichstand zwischen gleich schweren GATES. Ein Thema,
    # dessen Existenzfrage ein Betragsfeld ist, ging deshalb leer aus: gemessen im E2E-Durchgang
    # am 2026-08-26 stand „Hattest du grössere aussergewöhnliche Ausgaben?" an Position 3 von 3
    # (hinter „Waren die Ausgaben notwendig?" und „Konntest du sie nicht vermeiden?"), und „Wie
    # viel hast du für die Betreuung deines Kindes gezahlt?" als LETZTE von zehn Betreuungsfragen.
    # Beide Male fragte der Fragebogen nach Merkmalen einer Sache, die er nie erhoben hatte.
    # EINMAL rechnen, nicht je Thema: `gate_gewicht` laeuft ueber die ganze Bindung, und die
    # Schleife darunter hat 76 Durchgaenge. Je Thema gerufen kostete das ~300 ms pro
    # /fragen-Aufruf — gemessen 2026-08-27, als vier UI-Tests daran in den Timeout liefen.
    gw_einmal = gate_gewicht(bindung)
    for thema, gruppe in nach_thema.items():
        eingang = [f for f in gruppe if bindung[f].get("eingangsfrage")]
        if eingang:
            gruppe = eingang + [f for f in gruppe if f not in eingang]
        nach_thema[thema] = _nach_vordruck(gruppe, bindung, gw_einmal)
    return [f for thema in _themen_folge(nach_thema, bindung, angefangen or [])
            for f in nach_thema[thema]]


def _feld_ausgeschlossen(eintrag: dict, aktiv: dict) -> bool:
    """Fällt DIESES Feld weg, obwohl seine Regel für alle gilt?

    `regel_bedingungen` schaltet ganze Regeln ab. Das reicht nicht, wo ein Spezialfeld in einer
    ALLGEMEINEN Regel sitzt — dann hängt am selben Regel-Schalter beides, das für jeden Geltende
    und das Besondere. Julius, 2026-08-26, zweimal an einem Abend:

      „Möchtest du für deine behinderungsbedingten Aufwendungen deinen Behinderten-Pauschbetrag
       nutzen…? … in der checkliste war keine behinderung angehakt."
      „Aus welcher Art von Tätigkeit stammt dieser Gewinn? … kein gewinn ist erwähnt worden"

    Beide Male hatte er die Existenzfrage BESTÄTIGT verneint. Das Wahlrecht steht in
    `p33_1_2_agb_abzug` (außergewöhnliche Belastungen — hat fast jeder), die Betriebsart in
    `p2_festzusetzung_einzel` (die Basisregel schlechthin). Beide Regeln durften nicht entfallen,
    also blieb das Spezialfeld stehen.

    FAIL-CLOSED wie die Regel-Bedingung daneben: ausgeschlossen wird nur bei einer BESTÄTIGTEN
    abweichenden Antwort. Schweigen schliesst nichts aus, ein vorläufiger KI-Vorschlag auch nicht
    — sonst nähme ein Vorschlag dem Nutzer eine Frage weg, die er nie gesehen hat.
    """
    bed = eintrag.get("feld_bedingung")
    if not bed:
        return False
    ev = aktiv.get(bed["feld"])
    if _unbeantwortet(ev):
        return False
    # `wert_nicht` statt `wert`, wo die Existenzfrage ein AUSWAHLFELD ist: `kist_konfession` hat
    # drei Werte, bei denen die Frage gilt (evangelisch, römisch-katholisch, andere) und einen,
    # bei dem sie entfällt („keine"). Ein einzelner `wert` könnte das nicht ausdrücken, und die
    # Folge stand im E2E-Durchgang am 2026-08-26 in der Queue: „Wie viel Kirchensteuer wurde von
    # deinem Arbeitgeber einbehalten?" auf Platz 51, „Gehörst du einer Kirche an?" auf Platz 75.
    if "wert_nicht" in bed:
        return ev.get("wert") == bed["wert_nicht"]
    return ev.get("wert") != bed["wert"]


def _nach_vordruck(gruppe: list[str], bindung: dict, gw: dict) -> list[str]:
    """Innerhalb eines Themas: die Felder mit ELSTER-Kennzahl in der Reihenfolge des Vordrucks.

    Die Kennzahlen sind durchnummeriert, wie das amtliche Formular sie führt — E0500107 (Vorname
    des Kindes) vor E0500406 (IdNr) vor E0500701 (Geburtsdatum) vor E0501103 (Name des anderen
    Elternteils). Diese Ordnung ist da und kostet nichts; ohne sie entschied der
    Unsicherheits-Beitrag, und der ist quer zu allem, was ein Mensch erwartet.

    GEMESSEN 2026-08-27 im Kinderfreibetrags-Block, wie Julius ihn im Durchgang sah: die ersten
    vier Fragen galten dem ANDEREN Elternteil (Geburtsdatum, Verhältnis, Name, Zeitraum), der
    Vorname des eigenen Kindes stand auf Platz 12 von 13.

    NUR DIE FELDER MIT KENNZAHL TAUSCHEN UNTEREINANDER DIE PLÄTZE. Alles andere — Gates,
    Eingangsfragen, Bedingungsfelder, die neuen Zählfelder — bleibt, wo es steht. Sonst würde eine
    Formular-Nummerierung darüber entscheiden, wann eine Frage kommt, die ganze Regeln abschaltet.
    """
    # UMSORTIERT WIRD NUR INNERHALB EINER KLASSE, nie über die Grenze hinweg. Die Queue trennt
    # Felder, die andere Fragen streichen, von solchen, die nur einen Wert liefern — diese
    # Trennung ist der Grund, warum ein Gate früh kommt, und eine Formular-Nummerierung darf sie
    # nicht aufheben. Gemessen 2026-08-27: über die Grenze hinweg sortiert, rutschten Gates hinter
    # Wertfelder und drei Bestandstests wurden zu Recht rot.
    #
    # Eingangsfragen bleiben ganz aussen vor: sie stehen vorn, weil sie die Existenz erheben.
    #
    # Die Klasse „formales Gate" (trägt eine geltungsbedingung, streicht aber nichts — Gewicht 0)
    # ist der gefundene Fall: ALLE dreizehn Felder des Kind-Blocks gehören ihr an, und untereinander
    # ist die Vordruck-Ordnung genau das, was ein Mensch erwartet.
    def _klasse(f):
        if bindung[f].get("eingangsfrage"):
            return None                                   # nie umsortieren
        if gw.get(f, 0) > 0:
            return None                                   # echtes Gate: Gewichtsordnung gilt
        return "formal" if "geltungsbedingung" in (bindung[f].get("quelle") or {}) else "wert"

    aus = list(gruppe)
    for klasse in ("formal", "wert"):
        mit = [f for f in gruppe if bindung[f].get("elster_kz") and _klasse(f) == klasse]
        if len(mit) < 2:
            continue
        sortiert = iter(sorted(mit, key=lambda f: bindung[f]["elster_kz"]))
        kandidaten = set(mit)
        aus = [next(sortiert) if f in kandidaten else f for f in aus]
    return aus


def _themen_folge(nach_thema: dict[str, list[str]], bindung: dict,
                  angefangen: list[str] | None = None) -> list[str]:
    """Ein Thema folgt dem Thema, das seine Voraussetzung erhebt.

    Julius, 2026-08-26: „hier wird immernoch ohne anlass nach einem übernachtungsort gefragt".
    Die Regelbedingung dafür gibt es und sie ist geprüft — `p9_1_3_nr5a_uebernachtung_nach_48`
    hängt an `vpf_auswaertige_taetigkeit`. Nur stand die Voraussetzung auf Platz 38 und die
    Übernachtungsfragen auf 109: **71 Fragen dazwischen.** Wer die Voraussetzung überspringt
    (nicht verneint), bekommt sie alle — fail-closed ist richtig, aber 71 Fragen später sieht
    niemand mehr den Zusammenhang.

    GEMESSEN, vor dem Eingriff: **28 von 30 abhängigen Themen** standen mehr als 20 Fragen von
    ihrer Bedingungsfrage entfernt, das weiteste (`p10_1_9_schulgeld` an `kein_kind`) um 293.
    Die meisten davon hängen an `screening`-Feldern und sind in der Praxis entschärft, weil die
    Ankreuzliste sie vorab erhebt — Julius' Fall gerade nicht.

    KEINE Topologie-Sortierung, mit Absicht: das Verfahren setzt jedes Thema EINMAL, direkt hinter
    das Thema seiner ersten Bedingung, sofern jenes schon steht. Ein Ring von Bedingungen kann
    damit keine Endlosschleife und keine Auslassung erzeugen — im schlimmsten Fall bleibt ein
    Thema an seinem alten Platz, und das ist der heutige Zustand.
    """
    bedingt = lade_regel_bedingungen()
    quelle: dict[str, set[str]] = {t: set() for t in nach_thema}

    def _merke(thema: str, feld: str | None) -> None:
        qt = (bindung.get(feld or "", {}).get("quelle") or {}).get("regel_id")
        if qt and qt != thema and qt in nach_thema:
            quelle[thema].add(qt)

    for thema, felder_des_themas in nach_thema.items():
        for c in bedingt.get(thema, []):
            _merke(thema, c["feld"])
        # Auch eine ABLEITUNG erzeugt eine Abhängigkeit: steht die Quelle hinter dem Feld, das aus
        # ihr berechnet wird, greift die Ableitung im echten Ablauf NIE — der Nutzer beantwortet
        # die Frage, bevor die Angabe da ist, aus der sie folgt.
        #
        # Gefunden im E2E-Durchgang am 2026-08-26, unmittelbar nachdem die Ableitung gebaut war:
        # „Ist dein Kind wegen einer Behinderung außerstande, für sich selbst zu sorgen?" stand auf
        # Platz 25, „Wann ist dein Kind geboren?" auf Platz 44. Die Ableitung war damit wirkungslos,
        # und zwar auf eine Art, die kein Einzeltest zeigt: jede Prüfung setzt das Geburtsdatum
        # selbst und misst danach.
        gruppen = lade_instanz_gruppen()
        for fid in felder_des_themas:
            eintrag = bindung.get(fid) or {}
            # Eine ABLEITUNG braucht ihre Quelle vorher: steht sie dahinter, greift die Ableitung
            # im echten Ablauf nie. Gemessen 2026-08-26: „Ist dein Kind wegen einer Behinderung
            # ausserstande…" auf Platz 25, „Wann ist dein Kind geboren?" auf Platz 44.
            if eintrag.get("ableitung"):
                _merke(thema, eintrag["ableitung"]["aus"])
            # Dasselbe für die Instanz-Achse: das ZÄHLFELD muss vor den Feldern stehen, die aus ihm
            # entstehen. Es liegt in einer eigenen Pseudo-Regel (damit es kein Gate der Rechenregel
            # wird), also in einem anderen Thema. Gemessen 2026-08-27: `vv_anzahl_objekte` auf
            # Platz 91, während die Objektfragen vorher begannen — der Nutzer hätte die Adresse
            # eingetragen, bevor ihn jemand nach der Zahl der Objekte gefragt hat.
            gr = eintrag.get("instanz_gruppe")
            zaehl = (gruppen.get(gr) or {}).get("anzahl_feld") if gr else None
            if zaehl and zaehl not in felder_des_themas:
                _merke(thema, zaehl)

    # Der Einstieg steht fest (s. lade_themen_zuerst): Stammdaten zuerst, dann der Rest nach
    # Gewicht. Ohne das eröffnete der Fragebogen mit „Hattest du Kosten für Handwerker?", während
    # Name und Anschrift auf Platz 50 standen.
    vorne = [t for t in lade_themen_zuerst() if t in nach_thema]

    # EIN ANGEFANGENES THEMA WIRD ZU ENDE GEFRAGT, bevor ein neues beginnt.
    #
    # Der Rest dieser Funktion ordnet THEMEN; welches Thema wo steht, entscheidet die Position
    # seines BESTEN Feldes in `gates + slots`. Genau dieses Feld beantwortet der Nutzer als erstes
    # — danach hat das Thema nur noch seine schwächeren Felder, fällt auf deren Rang zurück und
    # ein fremdes Thema steht vorn. `naechste_fragen` läuft nach JEDER Antwort neu, also wiederholt
    # sich das bei jedem Feld.
    #
    # GEMESSEN 2026-08-27 an einem simulierten Volldurchgang (288 Fragen, Kopf der Queue
    # beantworten, neu rechnen): **43 Abbrüche mitten im Thema, davon 43 aus genau diesem Grund und
    # 0, weil eine Frage noch nicht in der Queue stand.** Die abgebrochenen Fragen standen jedes Mal
    # direkt hinter der beantworteten und rutschten dann auf Platz 44, 91, 126, 165. Julius' Fälle
    # aus dem echten Durchgang: § 35a in zwei Stücken (#47-63 und #122), Kranken-/Pflege-
    # versicherung in drei, Arbeitsmittel in drei, Kirchensteuer von zwei Fragen zerteilt.
    #
    # WARUM DIE EINZELMESSUNG DAS NICHT SIEHT: die Bestandstests rufen `naechste_fragen` EINMAL auf
    # leerem Store und finden 61 Wechsel bei 62 Themen — das Minimum. Das ist auch richtig; die
    # Ordnung ist erst kaputt, wenn man sie fortschreibt. Der Fehler lebt zwischen zwei Aufrufen.
    #
    # Die Rangfolge selbst bleibt unangetastet: sie entscheidet weiterhin, welches Thema als
    # nächstes ANFÄNGT. Sie darf nur nicht mehr entscheiden, ob ein bereits begonnenes weiterläuft.
    laufend = [t for t in (angefangen or []) if t in nach_thema and t not in vorne]
    rest = [t for t in nach_thema if t not in vorne and t not in laufend]
    wunsch = vorne + laufend + rest

    # Jedes Thema kommt hinter ALLE seine Quellen. Mehrere sind der Normalfall: § 35a führt drei
    # Instanz-Gruppen (Handwerker, Dienstleistung, Minijob) mit je eigenem Zählfeld, also drei
    # Vorgänger — mit nur EINER Abhängigkeit je Thema (so war es bis 2026-08-27) kamen zwei der
    # drei Zählfragen hinter ihren eigenen Feldern.
    #
    # KEINE echte Topologie-Sortierung, und das ist Absicht: bei einem Ring von Bedingungen bricht
    # die Schleife nach `len(wunsch)` Runden ab und hängt den Rest in der Wunschreihenfolge an.
    # Schlimmstenfalls steht ein Thema wie vorher — nie eine Endlosschleife, nie ein verlorenes.
    folge: list[str] = []
    offen = list(wunsch)
    for _runde in range(len(wunsch) + 1):
        if not offen:
            break
        gesetzt = [t for t in offen if quelle[t] <= set(folge)]
        if not gesetzt:                      # Ring: Rest unverändert anhängen
            break
        folge += gesetzt
        offen = [t for t in offen if t not in gesetzt]
    return folge + offen


# ---------------------------------------------------------------- (b) VORWÄRTS

def justification(store: dict, feld_id: str, bindung: dict) -> dict | None:
    """Rekursions-Blatt: das Justification-Objekt eines Felds aus Store-Event + Bindungstabelle.
    None, wenn das Feld (noch) kein Event hat."""
    ev = _aktive_events(store).get(feld_id)
    if ev is None:
        return None
    b = bindung.get(feld_id, {})
    q = b.get("quelle", {})
    return {
        "feld_id": feld_id,
        "wert": ev["wert"],
        "zustand": ev["zustand"],
        "herkunft": ev["herkunft"],
        "event_id": ev["event_id"],
        "signal": ev.get("signal"),
        "regel_id": q.get("regel_id"),
        "signatur_slot": q.get("signatur_slot"),
        "geltungsbedingung": q.get("geltungsbedingung"),
        "anker_ref": b.get("anker_ref"),
    }


def trace_ergebnis(store: dict, bindung: dict, snapshot_id: str | None = None) -> dict:
    """Vorwärts-Trace: je beteiligter Regel (deren Felder belegt sind) die Justifications ihrer
    Felder. Regel/Slot/Feld/Event-EXAKT; per-Cent-Attribution ist benannter Nachtrag (KONZEPT.md)."""
    aktiv = _aktive_events(store)
    out = {"basis_snapshot": snapshot_id, "regeln": {}}
    for fid in aktiv:
        b = bindung.get(fid)
        if not b:
            continue
        rid = b["quelle"]["regel_id"]
        out["regeln"].setdefault(rid, []).append(justification(store, fid, bindung))
    for rid in out["regeln"]:
        out["regeln"][rid].sort(key=lambda j: j["feld_id"])
    return out
