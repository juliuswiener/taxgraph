"""Paritaets-Orakel fuer `rust/auth`, `rust/llm`, `rust/eingang` (Schritt 8).

Beantwortet `{"fn": "auth.<name>" | "llm.<name>" | "eingang.<name>", ...}` ueber die Python-Referenz
(`produkt/auth/auth.py`, `produkt/haut/{llm_client,api_llm,pii_filter}.py`, `produkt/eingang/*`).
Nur reine Funktionen und der LLM-Client gegen einen LOKALEN Fake-Server — nie ein echter Anbieter.
Audit schreibt in ein Temp-Verzeichnis, der Fluss-Mitschnitt bleibt aus.

Antwort: `{"ok": <ergebnis>}` oder `{"err": "<Ausnahmeklasse>", "msg": "<str(exc)>"}`.
"""
from __future__ import annotations

import ast
import copy
import os
import sys
import tempfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_M: dict = {}


def _module() -> dict:
    if not _M:
        tmp = tempfile.mkdtemp(prefix="schritt8-orakel-")
        os.environ["TAXGRAPH_AUDIT_DIR"] = tmp
        os.environ.pop("TAXGRAPH_FLOW", None)
        os.environ.pop("TAXGRAPH_KI_DEBUG", None)
        for sub in ("produkt/store", "produkt/traverser", "produkt/haut", "produkt/eingang", "produkt/auth"):
            p = os.path.join(ROOT, sub)
            if p not in sys.path:
                sys.path.insert(0, p)
        import audit  # noqa: E402
        audit.AUDIT_DIR = tmp
        import auth as AU  # noqa: E402
        import api_llm as AL  # noqa: E402
        import llm_client as LC  # noqa: E402
        import pii_filter as PF  # noqa: E402
        import kontoauszug_writer as KW  # noqa: E402
        import beleg_writer as BW  # noqa: E402
        import vorjahr_writer as VW  # noqa: E402
        import elster_writer as EW  # noqa: E402
        import vast_mapping as VM  # noqa: E402
        import store as ST  # noqa: E402
        import traverser as TR  # noqa: E402
        _M.update(AU=AU, AL=AL, LC=LC, PF=PF, KW=KW, BW=BW, VW=VW, EW=EW, VM=VM, ST=ST, TR=TR, tmp=tmp,
                  bindung=TR.lade_bindung())
    return _M


def _fang(f, *a, **kw):
    try:
        return {"ok": f(*a, **kw)}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "msg": str(exc)}


# ------------------------------------------------------------------ Fixtures aus den Python-Tests

def _korpus(req: dict) -> list:
    """Alle String-Konstanten der genannten Testdateien (AST), dedupliziert, in Fundreihenfolge."""
    out, gesehen = [], set()
    for rel in req["dateien"]:
        with open(os.path.join(ROOT, rel), encoding="utf-8") as f:
            baum = ast.parse(f.read())
        for knoten in ast.walk(baum):
            if isinstance(knoten, ast.Constant) and isinstance(knoten.value, str) and knoten.value not in gesehen:
                gesehen.add(knoten.value)
                out.append(knoten.value)
    return out


# ------------------------------------------------------------------ auth

def _auth_validiere(req: dict) -> dict:
    AU = _module()["AU"]
    return {"namen": [bool(AU._USER_RE.fullmatch(n)) for n in req["namen"]],
            "pw": [bool(AU._PW_RE.fullmatch(p)) for p in req["pw"]]}


def _auth_token(req: dict):
    AU = _module()["AU"]
    AU._JWT_SECRET = req["secret"]
    if "sub" in req:
        return AU._create_token(req["sub"])
    return AU.verify_token(req["token"])


def _auth_encode(req: dict) -> str:
    """Beliebige Payload signieren (Negativfaelle: abgelaufen, aud, sub als Zahl, ...)."""
    import jwt  # noqa: E402
    alg = req.get("alg", "HS256")
    return jwt.encode(req["payload"], None if alg == "none" else req["secret"], algorithm=alg)


def _auth_bcrypt(req: dict):
    AU = _module()["AU"]
    if "hash" in req:
        return AU._check_pw(req["pw"], req["hash"])
    return AU._hash_pw(req["pw"])


def _auth_handler(req: dict):
    """register/login/logout gegen eine Nutzerdatei; AuthError → {status, msg}."""
    AU = _module()["AU"]
    AU.USER_STORE = req["datei"]
    AU._JWT_SECRET = req["secret"]
    f = {"register": AU.register, "login": AU.login, "logout": AU.logout}[req["aktion"]]
    try:
        status, body = f(req["body"])
        return {"status": status, "body": body}
    except AU.AuthError as e:
        return {"status": e.status, "msg": str(e)}
    except Exception as e:  # noqa: BLE001 -- server.py antwortet darauf 500
        return {"status": 500, "msg": str(e)}


# ------------------------------------------------------------------ llm

def _llm_filtere(req: dict) -> list:
    PF = _module()["PF"]
    return [list(PF.filtere(t)) for t in req["texte"]]


def _llm_art9(req: dict) -> list:
    PF = _module()["PF"]
    return [PF.ist_besondere_kategorie(f, q) for f, q in req["paare"]]


def _llm_maskiere(req: dict) -> list:
    KW = _module()["KW"]
    return [KW.maskiere(t) for t in req["texte"]]


def _llm_parse(req: dict) -> list:
    m = _module()
    AL, PF = m["AL"], m["PF"]
    gef = PF.filtere(req["freitext"])[0]
    erlaubt = set(req["erlaubt"])
    out = []
    for t in req["texte"]:
        je, getroffen = AL._zuordnung_parse(t, erlaubt, req["anzahl"])
        out.append({"chat": _fang(AL._chat_parse, t), "rueck": _fang(AL._rueckfragen_parse, t, req["felder"]),
                    "antwort": list(AL._antwort_parse(t)), "aussagen": _fang(AL._aussagen_parse, t, gef),
                    "zuordnung": {"je_aussage": je, "getroffen": getroffen}})
    return out


def _llm_beleg(req: dict) -> list:
    m = _module()
    AL, PF = m["AL"], m["PF"]
    out = []
    for beleg, freitext in req["paare"]:
        behalten, _ = AL._beleg_geprueft([{"beleg": beleg}], PF.filtere(freitext)[0])
        out.append(bool(behalten))
    return out


def _llm_katalog(_req: dict) -> dict:
    """Katalog wie `api.py:1112-1122` (alle llm-vorschlagbaren Felder) + Instanz-Gruppen."""
    m = _module()
    ST, TR, bindung = m["ST"], m["TR"], m["bindung"]
    check = ST.lade_katalog(bindung)
    kat = [{"feld_id": fid, "fragetext_laie": b.get("fragetext_laie", ""), "hilfe_kurz": b.get("hilfe_kurz", ""),
            "typ": b.get("typ"), "bereich": b.get("bereich"), "enum_werte": b.get("enum_werte"),
            "regel_id": (b.get("quelle") or {}).get("regel_id"), "instanz_gruppe": b.get("instanz_gruppe"),
            # JSON-Objekte verlieren auf der Rust-Seite die Reihenfolge; `repr(bereich)` im Prompt
            # braucht sie. Deshalb zusaetzlich als Paarliste.
            "bereich_paare": list(b["bereich"].items()) if isinstance(b.get("bereich"), dict) else None}
           for fid, b in bindung.items() if fid in check["llm"]]
    gruppen = [[name, g["anzahl_feld"]] for name, g in TR.lade_instanz_gruppen().items()]
    return {"katalog": kat, "gruppen": gruppen}


def _llm_dialog(req: dict) -> dict:
    """`_llm_dialog` mit Fixture-Antworten; zeichnet jeden Aufruf (Nachrichten, Schema) auf."""
    m = _module()
    AL, LC = m["AL"], m["LC"]
    antworten = list(req["antworten"])
    aufrufe = []

    def complete(role, messages, fixture_id=None, schema=None):
        aufrufe.append({"messages": copy.deepcopy(messages), "schema": (schema or {}).get("name")})
        a = antworten.pop(0)
        if "fehler" in a:
            e = LC.LlmNichtVerfuegbar("fixture")
            e.grund = a["fehler"]
            raise e
        return LC.Completion(text=a["text"], provider=a.get("provider", ""), finish=a.get("finish", ""))

    alt = LC.complete
    LC.complete = complete
    katalog = req["katalog"]
    if isinstance(katalog, dict):          # {"indizes": [...]} in den gecachten Voll-Katalog
        voll = _M.setdefault("katalog_voll", _llm_katalog({})["katalog"])
        katalog = [voll[i] for i in katalog["indizes"]]
    try:
        erg = _fang(AL._llm_dialog, req["freitext"], katalog, req.get("kontext", ""))
    finally:
        LC.complete = alt
    if req.get("nur_hash"):
        import hashlib  # noqa: E402
        for a in aufrufe:
            a["messages"] = [{"role": m["role"], "sha256": hashlib.sha256(m["content"].encode("utf-8")).hexdigest()}
                             for m in a["messages"]]
    return {"ergebnis": erg, "aufrufe": aufrufe}


def _llm_client(req: dict) -> dict:
    """`llm_client.complete` gegen den Fake-Server des Rust-Tests; Zeitgrenzen aus dem Request."""
    LC = _module()["LC"]
    os.environ.update({"LLM_API_BASE": req["base"], "LLM_MODEL": req["model"], "LLM_API_KEY": req["key"]})
    LC._BACKOFF_S = (0, 0)
    LC._TIMEOUT = req["socket_s"]
    LC._FRIST_S = req["frist_s"]
    LC._FRIST_WIEDERHOLUNG_S = req["frist_wdh_s"]
    schema = _module()["AL"].DIALOG_SCHEMA if req.get("schema") else None
    try:
        c = LC.complete("chat", req["messages"], schema=schema)
        return {"ok": {"text": c.text, "provider": c.provider, "finish": c.finish}}
    except LC.LlmNichtVerfuegbar as e:
        grund = getattr(e, "grund", "")
        klasse = ("abgeschnitten" if grund == "abgeschnitten"
                  else "voruebergehend" if hasattr(e, "versuche") else "endgueltig")
        return {"err": {"klasse": klasse, "grund": grund, "versuche": getattr(e, "versuche", 1),
                        "key_in_msg": req["key"] in str(e)}}
    except Exception as e:  # noqa: BLE001
        return {"err": {"klasse": "absturz:" + type(e).__name__}}
    finally:
        for k in ("LLM_API_BASE", "LLM_MODEL", "LLM_API_KEY"):
            os.environ.pop(k, None)


def _llm_kategorie(req: dict) -> list:
    KW = _module()["KW"]
    return [_fang(KW._parse_llm_kategorie, t) for t in req["texte"]]


def _llm_klassifikator(req: dict) -> list:
    KW = _module()["KW"]
    gesehen = []

    class Fake:
        def complete(self, role, msgs, fixture_id=None):
            gesehen.append(msgs)
            return type("C", (), {"text": '{"kategorie": null}'})()
    KW.llm_klassifikator_factory(Fake(), "x")(req["zweck"], req["betrag"])
    return gesehen[0]


def _llm_schemas(_req: dict) -> dict:
    AL = _module()["AL"]
    return {"dialog": AL.DIALOG_SCHEMA, "aussagen": AL.AUSSAGEN_SCHEMA, "zuordnung": AL.ZUORDNUNG_SCHEMA}


# ------------------------------------------------------------------ eingang

def _events(store: dict) -> list:
    return store["events"]


def _ein_csv(req: dict) -> list:
    KW = _module()["KW"]
    return [_fang(KW.parse_csv, t) for t in req["texte"]]


def _ein_cent(req: dict) -> list:
    KW = _module()["KW"]
    return [_fang(KW._eur_cent_signed, w) for w in req["werte"]]


def _ein_pdf_zeilen(req: dict) -> list:
    KW = _module()["KW"]
    return [_fang(lambda t, c: list(KW.parse_pdf_zeilen(t, c)), f["text"], {int(k): v for k, v in f["conf"].items()})
            for f in req["faelle"]]


def _ein_tsv(req: dict) -> list:
    KW = _module()["KW"]
    return [_fang(lambda t: [list(z) for z in KW._tsv_zu_zeilen(t)], t) for t in req["texte"]]


def _ein_konto(req: dict) -> dict:
    """`uebernehme_kontoauszug` auf leerem Store mit voller Bindung und globalem Katalog; der
    LLM-Rueckfall antwortet der Reihe nach mit `llm` (Texte), Fehler → None wie api_llm."""
    m = _module()
    KW, ST, bindung = m["KW"], m["ST"], m["bindung"]
    store = ST.leerer_store(2025)
    texte = list(req.get("llm") or [])
    klass = None
    if req.get("llm") is not None:
        class Fake:
            def complete(self, role, msgs, fixture_id=None):
                return type("C", (), {"text": texte.pop(0) if texte else ""})()
        klass = KW.llm_klassifikator_factory(Fake(), "x")
    r = _fang(KW.uebernehme_kontoauszug, store, req["tx"], bindung, llm_klassifikator=klass, ts=req["ts"],
              katalog=ST.lade_katalog(bindung))
    return {"r": r, "events": _events(store)}


def _ein_pdf(req: dict) -> dict:
    m = _module()
    KW, BW = m["KW"], m["BW"]
    k = _fang(lambda p: list(KW.lies_kontoauszug_pdf(p)), req["pfad"])
    b = _fang(lambda p: list(BW.lies_beleg_text(p)), req["pfad"])
    return {"konto": k, "beleg": b}


def _ein_beleg(req: dict) -> dict:
    m = _module()
    BW, ST, bindung = m["BW"], m["ST"], m["bindung"]
    kand = _fang(BW.extrahiere, req["text"], bindung, confidence_map=req.get("conf"))
    out = {"kandidaten": kand}
    if "ok" in kand and req.get("schreibe"):
        store = ST.leerer_store(2025)
        out["schreibe"] = _fang(lambda: len(BW.schreibe_kandidaten(store, kand["ok"], beleg_ref=req["ref"],
                                                                    bindung=bindung, ts=req["ts"])))
        out["events"] = _events(store)
    return out


def _ein_vorjahr(req: dict) -> dict:
    m = _module()
    VW, ST, bindung = m["VW"], m["ST"], m["bindung"]
    store = ST.leerer_store(2026)
    for fid in req.get("vorbelegt", []):
        ST.append_event(store, feld_id=fid, wert=1, zustand="vorlaeufig",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": None}, ts=req["ts"])
    r = _fang(VW.uebernehme_vorjahr, store, req["felder"], bindung, vorjahr_vz=req["vz"], ts=req["ts"])
    return {"r": r, "events": _events(store), "referenz": store.get("vorjahr_referenz")}


def _ein_vast(req: dict) -> dict:
    VM = _module()["VM"]
    return {"cent": [_fang(VM._cent, w) for w in req.get("werte", [])],
            "lstb": [_fang(VM.aus_lstb, w) for w in req.get("lstb", [])],
            "lersl": [_fang(VM.aus_lersl, l) for l in req.get("lersl", [])]}


def _ein_edaten(req: dict) -> dict:
    m = _module()
    EW, ST, bindung = m["EW"], m["ST"], m["bindung"]
    store = ST.leerer_store(2025)
    r = _fang(EW.uebernehme_edaten, store, req["saetze"], ts=req["ts"],
              bindung=bindung if req.get("mit_bindung") else None)
    return {"r": r, "events": _events(store)}


HANDLER = {
    "korpus": _korpus,
    "auth.validiere": _auth_validiere, "auth.token": _auth_token, "auth.bcrypt": _auth_bcrypt,
    "auth.encode": _auth_encode,
    "auth.handler": _auth_handler,
    "llm.filtere": _llm_filtere, "llm.art9": _llm_art9, "llm.maskiere": _llm_maskiere, "llm.parse": _llm_parse,
    "llm.beleg": _llm_beleg, "llm.katalog": _llm_katalog, "llm.dialog": _llm_dialog, "llm.client": _llm_client,
    "llm.kategorie": _llm_kategorie, "llm.klassifikator": _llm_klassifikator, "llm.schemas": _llm_schemas,
    "eingang.csv": _ein_csv, "eingang.cent": _ein_cent, "eingang.pdf_zeilen": _ein_pdf_zeilen,
    "eingang.tsv": _ein_tsv, "eingang.konto": _ein_konto, "eingang.pdf": _ein_pdf, "eingang.beleg": _ein_beleg,
    "eingang.vorjahr": _ein_vorjahr, "eingang.vast": _ein_vast, "eingang.edaten": _ein_edaten,
}


def handle(req: dict) -> dict:
    name = req["fn"].split(".", 1)[1] if req["fn"].startswith("schritt8.") else req["fn"]
    return _fang(HANDLER[name], req)
