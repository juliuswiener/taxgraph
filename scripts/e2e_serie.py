#!/usr/bin/env python3
"""e2e-Serientreiber: fährt Lebenslagen über die Haut-HTTP-Endpunkte, bis der Traverser
nichts mehr fragt, und hält /preflight, /ergebnis, /deklaration fest.

Grund für diese Datei im Repo (statt /tmp): am 2026-08-28 lief schon einmal eine e2e-Serie,
der Treiber lag in /tmp, /tmp wurde geleert, der Treiber war unwiederbringlich weg — nur die
Ergebnisse überlebten in Notizen (s. Auftrag 2026-09-07).

Jede Antwort trägt "origin":
  "person" — folgt schlüssig aus der Lebenslage-Beschreibung
  "schema" — aus dem Feldtyp geraten, weil die Lebenslage dazu nichts hergibt

stdlib-only (urllib), keine neuen Abhängigkeiten. Erwartet einen laufenden Server unter BASE.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import secrets
import sys
import urllib.error
import urllib.request

BASE = "http://127.0.0.1:8000"
MAX_ROUNDS = 400  # eine Runde = eine Frage seit 2026-09-07 (s. Kommentar in fahre_fall); 324 offene
                  # Fragen wurden zu Fallbeginn gemessen, 400 laesst Luft ohne einen echten Haenger zu verdecken
OUT_DIR = "/tmp/e2e_2026-09-07"

_INSTANZ_RE = re.compile(r"^(.*)__(\d+)$")


def _req(method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(BASE + path, data=data, method=method,
                                  headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        raw = e.read()
        try:
            return e.code, json.loads(raw)
        except Exception:
            return e.code, {"roh": raw.decode("utf-8", "replace")}


def _laie(feld_id, wert):
    return {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{feld_id}"}}


def _basis(feld_id):
    """Instanz-Suffix (__2, __3, ...) abstreifen — Übersteuerungen gelten je Basis-Feld."""
    m = _INSTANZ_RE.match(feld_id)
    return m.group(1) if m else feld_id


def _instanz_feld_id(basis, i):
    """Instanz i eines Feldes: `basis` für die erste, `basis__i` ab der zweiten.
    Dieselbe Konvention wie traverser.instanz_feld_id / app.js instanzFeldId."""
    return basis if i <= 1 else f"{basis}__{i}"


def _schreibe(fall_id, feld_id, wert):
    """Ein Event schreiben — und wie die echte Oberfläche mit Auflage B umgehen.

    `store.append_event` weist ein zweites Event auf dasselbe Feld ohne `ersetzt` ab
    (fail-closed, Auflage B). app.js holt sich das `ersetzt` vorab über /warum, wenn das Feld
    laut /stand schon belegt ist (schreibeInstanzen, ~Z.1193). Hier derselbe Effekt mit einer
    Anfrage weniger im Normalfall: erst schreiben, und NUR bei genau dieser Abweisung das
    `ersetzt` nachholen und einmal wiederholen. Kein blindes Wiederholen — jeder andere Fehler
    bleibt stehen und wird gemeldet."""
    st, b = _req("POST", f"/fall/{fall_id}/event", _laie(feld_id, wert))
    if st == 422 and "ersetzt" in json.dumps(b, ensure_ascii=False):
        st_w, b_w = _req("GET", f"/fall/{fall_id}/feld/{feld_id}/warum")
        ev_id = ((b_w or {}).get("justification") or {}).get("event_id") if st_w == 200 else None
        if ev_id:
            ev = _laie(feld_id, wert)
            ev["ersetzt"] = ev_id
            ev["signal"]["signal_1"] = ev_id
            return _req("POST", f"/fall/{fall_id}/event", ev)
    return st, b


def _entscheide(frage, overrides):
    """Liefert (wert, origin, begruendung) für eine gestellte Frage."""
    basis = _basis(frage["feld_id"])
    if basis in overrides:
        wert, begr = overrides[basis]
        gueltig = frage["typ"] != "enum" or not frage.get("enum_werte") or wert in frage["enum_werte"]
        if gueltig:
            return wert, "person", begr
        # Override passt nicht zu den erlaubten enum-Werten dieser Frage -> unten auf schema zurückfallen

    typ = frage["typ"]
    if typ == "bool":
        return False, "schema", "Feldtyp bool, Lebenslage sagt dazu nichts -> false"
    if typ in ("cent", "int"):
        wert = 0
        bereich = frage.get("bereich")
        if bereich and bereich.get("min", 0) > wert:
            wert = bereich["min"]
        return wert, "schema", "Feldtyp Betrag, Lebenslage sagt dazu nichts -> 0 (bzw. bereich.min)"
    if typ == "enum":
        werte = frage.get("enum_werte") or [None]
        return werte[0], "schema", "Feldtyp enum, Lebenslage sagt dazu nichts -> erster Wert"
    # text/datum: die Auflage nennt dafür keinen literalen Default -> der Beispielwert der API selbst
    return frage.get("beispielwert"), "schema", "Feldtyp text/datum, Lebenslage sagt dazu nichts -> beispielwert der API"


LEBENSLAGEN = {
    "arbeitslos_zwei_kinder": {
        "beschreibung": "arbeitslos, zwei Kinder",
        "overrides": {
            "kein_kind": (False, "zwei Kinder laut Lebenslage"),
            "fam_anzahl_kinder": (2, "zwei Kinder laut Lebenslage"),
            "keine_lohnersatzleistungen": (False, "arbeitslos -> bezieht Arbeitslosengeld (Lohnersatzleistung)"),
            "bruttoarbeitslohn": (0, "arbeitslos -> kein Arbeitslohn"),
        },
    },
    "alleinstehend_vermietung": {
        "beschreibung": "alleinstehend, Einkünfte aus Vermietung",
        "overrides": {
            "veranlagung": ("einzel", "alleinstehend -> Einzelveranlagung"),
            "kein_vuv": (False, "Einkünfte aus Vermietung laut Lebenslage"),
            "vv_anzahl_objekte": (1, "Vermietungseinkünfte vorhanden -> mind. ein Objekt"),
        },
    },
    "verheiratet_handwerker": {
        "beschreibung": "verheiratet, Handwerker (selbständig)",
        "overrides": {
            "veranlagung": ("zusammen", "verheiratet -> Zusammenveranlagung"),
            "kein_gewinn": (False, "Handwerker selbständig -> Gewinneinkünfte"),
            "gewinn_betriebsart": ("selbstaendig", "Lebenslage nennt explizit '(selbständig)'"),
        },
    },
    "rentner_behinderung": {
        "beschreibung": "Rentner mit Behinderung (Grad der Behinderung)",
        "overrides": {
            "rentner_anzahl_renten": (1, "Rentner -> mind. eine Rente"),
            "rentner_grad_der_behinderung": (50, "Rentner mit Behinderung (Grad der Behinderung) laut Lebenslage"),
            "keine_behinderung_pflege": (False, "Behinderung laut Lebenslage"),
        },
    },
}


def fahre_fall(lage_key, overrides, fall_id, scheibe="gesamt", vz=2025):
    lauf = {"lage": lage_key, "fall_id": fall_id, "antworten": [], "fehler": [], "runden": 0}
    st, b = _req("POST", "/fall", {"scheibe": scheibe, "veranlagungszeitraum": vz, "fall_id": fall_id})
    if st != 201:
        lauf["fehler"].append({"schritt": "POST /fall", "status": st, "body": b})
        return lauf

    # EINE Runde ist jetzt EINE Frage — nicht mehr ein ganzer /fragen-Stapel. Fund 2026-09-07:
    # die alte Fassung holte /fragen einmal pro Runde und beantwortete dann ALLE offenen Fragen
    # dieser Liste in einer Schleife, ohne zwischendurch neu zu fragen. Die echte Oberfläche tut
    # das Gegenteil (app.js: eine Frage zeigen, schreiben, sofort /fragen neu holen — laut
    # Kommentar dort eine bewusst durchgehaltene Regel, "nie zwei Aufforderungen gleichzeitig").
    # Die alte Schleife konnte dadurch ein Feld ansprechen, das der Server zwischen Listenabruf
    # und dieser Antwort schon selbst berechnet hatte (`ableitung` in der Bindung, z.B.
    # `geburtsjahr` aus `stammdaten_geburtsdatum`) -> 422 "hat schon ein aktives Event". Das war
    # ein Treiber-Artefakt, kein Produktfehler (gemessen 2026-09-07, s. Vault). MAX_ROUNDS zählt
    # deshalb jetzt Einzelfragen, nicht Fragebögen — hochgesetzt, weil eine Scheibe zu Fallbeginn
    # bis zu 324 offene Fragen zeigt (gemessen) und jede davon jetzt ein eigener Schritt ist.
    beantwortet = set()
    fehlgeschlagen = set()
    for runde in range(1, MAX_ROUNDS + 1):
        st, b = _req("GET", f"/fall/{fall_id}/fragen")
        if st != 200:
            lauf["fehler"].append({"schritt": f"GET /fragen runde {runde}", "status": st, "body": b})
            break
        offen = [f for f in b["fragen"]
                 if f["feld_id"] not in beantwortet and f["feld_id"] not in fehlgeschlagen]
        lauf["runden"] = runde
        if not offen:
            break
        frage = offen[0]   # nur die erste — wie die Oberfläche (AKTUELL = fragen[0]), dann neu holen
        wert, origin, begruendung = _entscheide(frage, overrides)
        # ALLE Instanzen der Karte auf einmal, wie app.js `schreibeInstanzen()` beim Absenden:
        # der Traverser führt nur das Basisfeld und legt die Zahl als `instanz_anzahl` daneben,
        # die Oberfläche baut daraus N Eingabefelder. Wer hier nur Instanz 1 schreibt, lässt bei
        # "2 Kinder" die zweite Reihe leer — gemessen 2026-09-08: 22 preflight-Widersprüche
        # "Angegeben hast du 2, ausgefüllt sind 1", und der Fall wird nie abgabefähig.
        # Jede Instanz bekommt DENSELBEN Wert (beide Kinder heißen "Anna"). Für einen Treiber,
        # dessen Werte ohnehin zu ~98 % aus dem Feldtyp geraten sind ("schema"), ist das ehrlich
        # genug; wer fachlich unterscheidbare Instanzen braucht, muss die Lebenslage erweitern.
        anzahl = frage.get("instanz_anzahl") or 1
        st, eb = None, None
        for i in range(1, anzahl + 1):
            fid_i = _instanz_feld_id(frage["feld_id"], i)
            st, eb = _schreibe(fall_id, fid_i, wert)
            if st != 201:
                break
        eintrag = {"feld_id": frage["feld_id"], "typ": frage["typ"], "wert": wert,
                   "origin": origin, "begruendung": begruendung, "status": st}
        if anzahl > 1:
            eintrag["instanzen"] = anzahl
        if st == 201:
            beantwortet.add(frage["feld_id"])
        else:
            fehlgeschlagen.add(frage["feld_id"])
            eintrag["fehler_body"] = eb
        lauf["antworten"].append(eintrag)
    else:
        lauf["fehler"].append({"schritt": "rundenlimit", "hinweis": f"MAX_ROUNDS={MAX_ROUNDS} erreicht"})

    for ep in ("preflight", "ergebnis", "deklaration"):
        st, b = _req("GET", f"/fall/{fall_id}/{ep}")
        lauf[ep] = {"status": st, "body": b}

    return lauf


def _zusammenfassung(lauf):
    n = len(lauf["antworten"])
    n_person = sum(1 for a in lauf["antworten"] if a["origin"] == "person")
    n_schema = n - n_person
    n_fehl = sum(1 for a in lauf["antworten"] if a["status"] != 201)
    zahl = lauf.get("ergebnis", {}).get("body", {}).get("zahl_cent")
    grund = lauf.get("ergebnis", {}).get("body", {}).get("grund")
    pf_status = lauf.get("preflight", {}).get("status")
    pf_body = lauf.get("preflight", {}).get("body", {})
    # `pf_body["status"]`, NICHT `pf_body["abgabefaehig"]`: der Schlüssel `abgabefaehig` steht in
    # keiner preflight-Antwort (die Antwort trägt fall_id/status/items). `.get()` lieferte dafür
    # stumm None, und die Zusammenfassung las sich wie "nicht abgabefähig", während in Wahrheit
    # nur die falsche Frage gestellt war — gemessen 2026-09-08 an vier Läufen, von denen einer
    # in Wirklichkeit GREEN war. Ein fehlender Schlüssel darf nicht wie ein Befund aussehen.
    pf_urteil = pf_body.get("status")
    n_items = len(pf_body.get("items") or ())
    return (f"{lauf['lage']}: {lauf['runden']} Runden, {n} Fragen ({n_person} person / {n_schema} schema, "
            f"{n_fehl} fehlgeschlagen), ergebnis.zahl_cent={zahl} grund={grund}, "
            f"preflight http={pf_status} urteil={pf_urteil} ({n_items} Einwände)")


def selftest():
    """Wegwerf-Fall gegen die ep-Scheibe: prüft nur die Event-Form/Status-Codes, keine fachliche Aussage."""
    fall_id = f"selftest-{secrets.token_hex(4)}"
    fehler = []

    st, b = _req("POST", "/fall", {"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": fall_id})
    if st != 201 or b.get("fall_id") != fall_id:
        fehler.append(f"POST /fall: status={st} body={b}")

    st, b = _req("GET", f"/fall/{fall_id}/fragen")
    if st != 200 or not isinstance(b.get("fragen"), list) or not b["fragen"]:
        fehler.append(f"GET /fragen: status={st} body={b}")
    else:
        frage = b["fragen"][0]
        wert, origin, _ = _entscheide(frage, {})
        st, eb = _req("POST", f"/fall/{fall_id}/event", _laie(frage["feld_id"], wert))
        if st != 201:
            fehler.append(f"POST /event: status={st} body={eb}")
        elif eb.get("feld_id") != frage["feld_id"] or eb.get("zustand") != "bestaetigt" or "event_id" not in eb:
            fehler.append(f"POST /event Antwortform unerwartet: {eb}")

    st, b = _req("GET", f"/fall/{fall_id}/stand")
    if st != 200 or "felder" not in b:
        fehler.append(f"GET /stand: status={st} body={b}")

    if fehler:
        print("SELFTEST FEHLGESCHLAGEN:")
        for f in fehler:
            print(f" - {f}")
        return 1
    print(f"SELFTEST OK (fall_id={fall_id})")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--selftest", action="store_true", help="Event-Form gegen einen Wegwerf-Fall prüfen, sonst nichts tun")
    ap.add_argument("--lage", choices=sorted(LEBENSLAGEN), help="nur eine Lebenslage fahren (Default: alle vier)")
    ap.add_argument("--out-dir", default=OUT_DIR)
    args = ap.parse_args()

    if args.selftest:
        return selftest()

    try:
        with urllib.request.urlopen(BASE + "/", timeout=10) as r:
            st = r.status
    except urllib.error.HTTPError as e:
        st = e.code
    if st != 200:
        print(f"Server antwortet nicht wie erwartet auf GET / (status={st}) — Abbruch.", file=sys.stderr)
        return 1

    os.makedirs(args.out_dir, exist_ok=True)
    lagen = [args.lage] if args.lage else sorted(LEBENSLAGEN)
    for lage_key in lagen:
        cfg = LEBENSLAGEN[lage_key]
        fall_id = f"e2e-{lage_key.replace('_', '-')}-{secrets.token_hex(3)}"
        print(f"--- {lage_key} ({cfg['beschreibung']}) fall_id={fall_id} ---")
        lauf = fahre_fall(lage_key, cfg["overrides"], fall_id)
        out_path = os.path.join(args.out_dir, f"{lage_key}.json")
        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(lauf, f, ensure_ascii=False, indent=2)
        print(_zusammenfassung(lauf))
        print(f"  -> {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
