"""FastAPI-Backend der Triage-UI (M-UI.1).

Lesen ist offen (Nordserver im Tailnet). SCHREIBEN verlangt ein Token
(Env TAXGRAPH_UI_TOKEN, Header `X-Taxgraph-Token`) - kein offener Schreib-
Endpoint. Ist kein Token konfiguriert, sind Schreib-Endpoints gesperrt
(fail-closed).

Start:
  export TAXGRAPH_UI_TOKEN=... ; uvicorn pipeline.ui.app:app --host 0.0.0.0 --port 8000
oder:
  python -m pipeline.ui.app
"""

from __future__ import annotations

import logging
import os
import secrets

from fastapi import FastAPI, Header, HTTPException
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel

from . import praezedenz, service

log = logging.getLogger(__name__)

HERE = os.path.dirname(os.path.abspath(__file__))
STATIC_DIR = os.path.join(HERE, "static")

app = FastAPI(title="TaxGraph Triage-UI", version="1.0")


def _require_token(x_taxgraph_token: str | None) -> None:
    token = os.environ.get("TAXGRAPH_UI_TOKEN")
    if not token:
        raise HTTPException(503, "TAXGRAPH_UI_TOKEN nicht gesetzt - Schreiben gesperrt")
    # m4: zeitkonstanter Vergleich (secrets.compare_digest) statt `!=`.
    if not x_taxgraph_token or not secrets.compare_digest(x_taxgraph_token, token):
        raise HTTPException(401, "Token fehlt oder falsch")


# -- Lese-Endpoints -----------------------------------------------------------

@app.get("/api/queue")
def queue():
    return {"regeln": service.queue_overview()}


@app.get("/api/rule/{rule_id}")
def rule(rule_id: str):
    try:
        ctx = service.rule_context(rule_id)
    except KeyError:
        raise HTTPException(404, f"unbekannte Regel: {rule_id}")
    # Praezedenz-Treffer je offenem Item vormerken (Vorschau, kein Schreiben)
    for it in ctx["items"]:
        f = praezedenz.treffer(rule_id, it)
        it["praezedenz_treffer"] = None if f is None else {
            "entscheidungstyp": f["entscheidungstyp"], "ziel_id": f["ziel_id"],
            "quelle_rule_id": f["quelle_rule_id"]}
    return ctx


class ItemContextReq(BaseModel):
    quote: str


@app.post("/api/rule/{rule_id}/item-context")
def item_context(rule_id: str, req: ItemContextReq):
    try:
        return service.item_context(rule_id, req.quote)
    except KeyError:
        raise HTTPException(404, f"unbekannte Regel: {rule_id}")


@app.get("/api/merge-log")
def merge_log():
    return {"merges": service.merge_log()}


@app.get("/api/entscheidungen")
def entscheidungen():
    return {"entscheidungen": service.entscheidungs_log()}


@app.get("/api/praezedenz")
def praez():
    return {
        "faelle": praezedenz.alle_faelle(),
        "anwendungen": praezedenz.anwendungen(),
        "chargen": praezedenz.chargen(),
    }


# -- Schreib-Endpoints (Token) ------------------------------------------------

class SubmitReq(BaseModel):
    item: dict
    triage: str
    bedingung: str | None = None
    weitere_bedingungen: list[str] | None = None
    konvention: str | None = None
    neue_bedingung: dict | None = None


@app.post("/api/rule/{rule_id}/submit")
def submit(rule_id: str, req: SubmitReq,
           x_taxgraph_token: str | None = Header(default=None)):
    _require_token(x_taxgraph_token)
    try:
        res = service.submit(
            rule_id, req.item, req.triage,
            bedingung=req.bedingung, weitere_bedingungen=req.weitere_bedingungen,
            konvention=req.konvention, neue_bedingung=req.neue_bedingung)
    except (ValueError, KeyError) as e:
        raise HTTPException(400, str(e))
    # Julius-Entscheidung als Praezedenzfall registrieren
    try:
        praezedenz.record_precedent(rule_id, req.item, req.triage,
                                    bedingung=req.bedingung, konvention=req.konvention)
    except Exception:  # noqa: BLE001
        # Praezedenz ist Beschleuniger, kein Blocker des Schreibens - aber ein
        # stiller Fehlschlag darf nicht spurlos bleiben (m3).
        log.exception("record_precedent fehlgeschlagen fuer %s", rule_id)
    return res


class AutoApplyReq(BaseModel):
    dry_run: bool = False


@app.post("/api/rule/{rule_id}/praezedenz-anwenden")
def praezedenz_anwenden(rule_id: str, req: AutoApplyReq | None = None,
                        x_taxgraph_token: str | None = Header(default=None)):
    dry = bool(req and req.dry_run)
    # m1: Token auch fuer dry_run - der Endpoint fasst die Registry an (auch dry_run
    # laeuft open_draft/discover) und ist kein anonymer Lese-Endpoint.
    _require_token(x_taxgraph_token)
    try:
        return praezedenz.auto_apply(rule_id, dry_run=dry)
    except KeyError as e:  # m2: unbekannte rule_id -> 404 statt Pfad-Konstruktion
        raise HTTPException(404, str(e))


class WiderrufReq(BaseModel):
    anwendung_id: str


@app.post("/api/praezedenz/widerruf")
def praezedenz_widerruf(req: WiderrufReq,
                        x_taxgraph_token: str | None = Header(default=None)):
    _require_token(x_taxgraph_token)
    try:
        return praezedenz.widerruf(req.anwendung_id)
    except KeyError as e:
        raise HTTPException(404, str(e))


# -- Frontend -----------------------------------------------------------------

@app.get("/")
def index():
    return FileResponse(os.path.join(STATIC_DIR, "index.html"))


if os.path.isdir(STATIC_DIR):
    app.mount("/static", StaticFiles(directory=STATIC_DIR), name="static")


def main() -> None:
    import uvicorn
    uvicorn.run(app, host=os.environ.get("TAXGRAPH_UI_HOST", "127.0.0.1"),
                port=int(os.environ.get("TAXGRAPH_UI_PORT", "8000")))


if __name__ == "__main__":
    main()
