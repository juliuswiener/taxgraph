"""Paket-B Haut — reine Endpunkt-Logik über die Paket-A-Naht (traverser/API.md). LLM-frei.

Jede Funktion ist eine reine `(fall/eingabe) -> (http_status, obj)`-Hülle über GENAU eine Naht-
Funktion — kein Transport, kein Socket (den setzt server.py drum). So bleibt der Upgrade-Pfad
(stdlib -> uvicorn) ein reiner Austausch der Transport-Schicht.

Naht-Grenze (API.md): LESEN über traverser/intervall/est_mapping/bindung; SCHREIBEN
ausschliesslich `store.append_event`. Keine Steuerlogik, keine zweite Wahrheit hier.

Bescheid-Ehrlichkeit (K2): ein numerischer [min,max]-Ring erscheint NUR, wo ein exponierter
golden-Accessor die Größe wirklich rechnet. Eine Scheibe mit Gesamt-Accessor (EP) trägt einen
`intervall` (Scheiben-Bescheid); eine Multi-Regel-Scheibe ohne ehrlichen Gesamt-Accessor
(N+VOR+GWG) trägt KEINEN Gesamt-Bescheid — nur ring-fähige Teilfamilien (EP-Abzug) als
`teil_ringe`, der Rest ist ehrlich engine=unavailable. Kein erfundener Betrag.
"""
from __future__ import annotations

import base64
import json
import os
import re
import subprocess   # nur für subprocess.TimeoutExpired am OCR-Endpunkt — api.py startet selbst
                    # keinen Unterprozess (das tun die Writer unter produkt/import/)
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
ROOT = os.path.dirname(PRODUKT)
for _sub in ("produkt/store", "produkt/traverser", "produkt/unsicherheit", "produkt/mapping",
             "produkt/konsistenz", "produkt/import", "produkt/bescheid", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import store as ST          # noqa: E402
import audit                # noqa: E402 — P1.6 Audit-Log
import traverser as TR      # noqa: E402
import intervall as IV      # noqa: E402
import est_mapping as EM    # noqa: E402
import flag_check as FC     # noqa: E402  (Flag↔Einkunftsart-Widersprüche, dev-2)
import partner_check as PC  # noqa: E402  (Partner-Behinderungsfeld↔Zusammenveranlagung, dev-2)
import vorjahr_writer as VW  # noqa: E402  (Vorjahres-Übernahme, dev-2 Store-Writer)
import elster_xml as EX      # noqa: E402  (P3.2 Deklaration → ELSTER-Submission-XML)
from api_constants import *  # noqa: E402, F401, F403  (55 Feld-Konstanten + Scheiben)
import api_llm  # noqa: E402  (LLM-Integration: _llm_vorschlaege, _kontoauszug_llm_klassifikator)
import preflight as PF  # noqa: E402  (P5.5 Preflight-Check: Konsistenz + vergessene Pauschalen)
import api_auth  # noqa: E402  (Request-scoped Auth, Modul-Attribute für Mutation-Sicherheit)
# Rechenkern (Phase 3): die Steuerlogik liegt in produkt/bescheid/, api.py ist ihre Hülle.
# Namentlich importiert, nicht per Star — und re-exportiert, weil 23 Testdateien und mehrere
# Endpunkte diese Namen über `api.` auflösen. tests/test_bescheid_grenze.py prüft, dass es
# DASSELBE Objekt ist (`is`) und nicht eine zweite Bindung desselben Namens.
from bescheid import (  # noqa: E402, F401
    _abs3_eligible,
    _abschlusszahlung_cent,
    _an_gesamt_sperrgrund,
    _bescheid_fn,
    _gewinn_partner_anteil,
    _gwg_sofortabzug_summe,
    _kind_behinderten_pb_daten,
    _kind_kv_pv_summe,
    _kinderbetreuung_summe,
    _laufender_gewinn,
    _laufender_gewinn_partner,
    _mit_ring_werten,
    _oepnv_eur,
    _p20_kapitaleinkuenfte,
    _p23_ansonsten_einkuenfte,
    _p33b_kind_pauschbetraege,
    _p35_gezahlte_gewst,
    _p35_partner_anteile,
    _p35_summen,
    _schulgeld_summe,
    _shared_dba_sonstige,
    _shared_steuer_sonder_agb,
    _zweig_abziehbarer_betrag,
    _zweig_festzusetzende_est,
    _zweig_festzusetzende_est_gesamt,
    _zweig_festzusetzende_est_rentner,
)


class ApiError(ValueError):
    """Trägt einen HTTP-Status mit (fail-closed-Antwort statt 500)."""
    def __init__(self, status: int, msg: str):
        super().__init__(msg)
        self.status = status


def _auth_uid_oder_401() -> str | None:
    """Gemeinsame Auth-Politik für _fall_owner_check() (Zugriff auf bestehende Fälle)
    UND fall_anlegen() (neue Fälle): kein Auth-Kontext → 401, AUSSER TAXGRAPH_NO_AUTH=1
    (bewusster Einzelnutzer-Opt-out, UI ohne Login) — zur LAUFZEIT gelesen, nicht beim
    Import (cfg-env-load-order: Modul-Import läuft vor dem .env-Loader). Rückgabe None
    NUR im Opt-out; dort ist "kein Nutzer" die ehrliche Antwort, nicht ein Bypass.
    EIN Ort für diese Entscheidung, weil fall_anlegen() sie sonst leicht driften lassen
    könnte (separat dupliziertes Fail-Closed ist ein Fail-Closed, das irgendwann nur noch
    an einer Stelle stimmt)."""
    uid = api_auth._AUTH_USER
    if uid is None and os.environ.get("TAXGRAPH_NO_AUTH") != "1":
        raise ApiError(401, "Authentifizierung erforderlich")
    return uid


def _fall_owner_check(fall_id: str) -> None:
    """Prüft Zugriff auf fall_id gegen api_auth._AUTH_USER. FAIL-CLOSED (Audit 2026-08-16,
    sec-authz-fail-open-no-token): kein Auth-Kontext → 401, Fall ohne user_id → 403."""
    uid = _auth_uid_oder_401()
    if uid is None:
        return  # Einzelnutzer-Opt-out: kein Nutzer da, gegen den geprüft werden könnte
    try:
        store = lade_fall(fall_id)
    except ApiError:
        return  # 404 wird der Aufrufer werfen — kein Grund zur Sperre
    stored = store.get("user_id")
    if stored is None or stored != uid:
        audit.append(uid, "zugriff_verweigert", fall_id, f"user={uid}, owner={stored}")
        raise ApiError(403, f"Zugriff auf Fall {fall_id!r} verweigert")


# ----------------------------------------------------------------- Fall-Persistenz (atomar JSON)

def _fall_pfad(fall_id: str) -> str:
    if not _FALL_RE.fullmatch(fall_id):
        raise ApiError(400, f"ungültige fall_id (nur [A-Za-z0-9_-]{{1,64}}): {fall_id!r}")
    return os.path.join(FAELLE, f"{fall_id}.json")


def lade_fall(fall_id: str) -> dict:
    p = _fall_pfad(fall_id)
    if not os.path.exists(p):
        raise ApiError(404, f"Fall {fall_id!r} existiert nicht")
    with open(p, encoding="utf-8") as f:
        return json.load(f)


def speichere_fall(fall_id: str, store: dict) -> None:
    p = _fall_pfad(fall_id)
    os.makedirs(FAELLE, exist_ok=True)
    tmp = tempfile.NamedTemporaryFile("w", dir=FAELLE, delete=False, encoding="utf-8", suffix=".tmp")
    try:
        json.dump(store, tmp, ensure_ascii=False)
        tmp.flush()
        os.fsync(tmp.fileno())
    finally:
        tmp.close()
    os.replace(tmp.name, p)


# ----------------------------------------------------------------- Scheibe -> Bindung/Engine

def _cfg(store: dict) -> dict:
    sch = store.get("scheibe")
    if sch not in SCHEIBEN:
        raise ApiError(400, f"unbekannte Scheibe {sch!r}")
    return SCHEIBEN[sch]


def _datei_felder(dateiname: str) -> tuple:
    import yaml
    d = yaml.safe_load(open(os.path.join(PRODUKT, "bindung", dateiname), encoding="utf-8"))
    return tuple(b["feld_id"] for b in d.get("bindungen", []))


def _scheibe_felder(store: dict) -> tuple:
    cfg = _cfg(store)
    return cfg["felder"] if cfg["felder"] is not None else _datei_felder(cfg["felder_datei"])


def _scheibe_bindung(store: dict) -> dict:
    felder = _scheibe_felder(store)
    b = TR.lade_bindung()
    fehlend = [f for f in felder if f not in b]
    if fehlend:
        raise ApiError(500, f"Bindungstabelle unvollständig für Scheibe: {fehlend}")
    return {f: b[f] for f in felder}


def _relevante_kegel_felder(scheibe_felder: tuple, bindung: dict, store: dict | None) -> tuple:
    """Der Pflicht-Kegel ohne die Felder, deren Regel der Nutzer selbst abbestellt hat.

    _feste_zahl verlangt für den Meet jedes Kegel-Feld als bestätigt. Ein Feld, dessen Regel
    traverser.relevanz() bereits ausgeschlossen hat, wird vom Traverser nie gefragt — es sperrte
    den Ring dauerhaft auf input_kegel_nicht_bestaetigt, ohne dass der Nutzer die Frage je zu
    sehen bekam (BACKLOG traverser-ring-kegel-relevanz-naht; gefunden am Fall vv_wohnzwecke=False,
    das p21_2_verbilligte_vermietung_wk ausschließt und vv_entgelt_quote_prozent im Kegel zurückließ).

    Fail-closed bleibt erhalten, und zwar an zwei Stellen:
    - relevanz() schließt NUR bei einem BESTÄTIGTEN False aus (traverser.py:122). Unbeantwortet
      oder vorläufig schließt NICHT aus, der Kegel bleibt also gesperrt, solange der Nutzer nicht
      geantwortet hat.
    - Ohne store (Alt-Aufrufer, Teil-Ringe) wird gar nichts ausgeschlossen — dann gilt der volle
      Kegel wie bisher.

    Nebenwirkung auf die slot_fn: ein weggelassenes Feld fehlt auch im Dict, das
    bescheid_via_slots an die slot_fn übergibt. Das ist geprüft und abgesichert — kein
    ausschließbares Kegel-Feld trägt einen Slot, den eine slot_fn liest
    (test_kegel_relevanz_naht::test_kein_ausschliessbares_kegelfeld_ist_ein_gelesener_slot)."""
    if store is None:
        return scheibe_felder
    rel = TR.relevanz(store, bindung)
    return tuple(f for f in scheibe_felder
                 if rel.get((bindung.get(f) or {}).get("quelle", {}).get("regel_id"),
                            {}).get("status") != "ausgeschlossen")


def _feste_zahl(felder: dict, bindung: dict, cfg: dict, vz: int, scheibe_felder: tuple,
                store: dict | None = None):
    """Fail-closed: die festzusetzende Zahl NUR bei Scheiben-Gesamt-Accessor UND vollständig
    bestätigtem Input-Kegel (Meet). Ohne Gesamt-Accessor gibt es KEINE Scheiben-Zahl (ehrlich).
    `store` erlaubt dem §21-Ring die Multi-Objekt-Instanz-Σ (#5).
    Returns (zahl_euro, solz_cent, extras) — extras = Post-Engine-Zuschlag-/Prämien-Dict
    (kist_cent § 51a, mobilitaetspraemie_cent § 101; Schlüssel absent = nicht rechenbar)."""
    q = cfg["gesamt_ring"]
    if q is None:
        return None
    scheibe_felder = _relevante_kegel_felder(scheibe_felder, bindung, store)
    zustaende = [felder[f]["zustand"] for f in scheibe_felder if f in felder]
    if len(zustaende) < len(scheibe_felder) or ST.meet_zustand(zustaende) != "bestaetigt":
        return None
    solz_out = [None]   # mutable container — slot_fn schreibt SolZ hinein
    extras = {}         # slot_fn schreibt kist_cent (§51a) + mobilitaetspraemie_cent (§101) hinein
    bf = _bescheid_fn(q, vz, bindung, felder, store, solz_container=solz_out, extras=extras)
    if bf is None:
        return None
    zahl = bf({f: felder[f]["wert"] for f in scheibe_felder})
    return zahl, solz_out[0], extras


# ----------------------------------------------------------------- Endpunkte (reine Logik)

def fall_loeschen(fall_id: str) -> tuple[int, dict]:
    """DSGVO-Löschung: Fall-Datei entfernen, Audit-Eintrag hinterlassen.
    Kein Soft-Delete (§ 147 AO Abs. 1 zählt Buchführungsunterlagen auf —
    trifft Arbeitnehmer/Rentner als Zielgruppe nicht).

    Owner-Check ZUERST, vor jeder Existenz-Prüfung (Audit 2026-08-16,
    sec-authz-fail-open-no-token: dieser Endpunkt hatte eine eigene, unveränderte
    Kopie der ALTEN fail-open-Prüfung statt _fall_owner_check() zu nutzen — anonymer
    DELETE auf einen fremden Fall kam mit 200 durch). Reihenfolge wie überall sonst
    im Modul (_fall_owner_check() vor lade_fall(), siehe warum()/ergebnis()/...):
    ein Anonymer ohne Auth-Kontext bekommt 401, BEVOR irgendetwas über die fall_id
    verrät, ob sie überhaupt existiert.

    Reihenfolge Owner-Check → remove → audit ist bewusst:
    - Scheitert das Löschen (Permission, Datei weg), gibt es keinen Eintrag
      über einen Vorgang, der nicht stattfand.
    - Scheitert umgekehrt das Audit nach erfolgreichem Löschen, bleibt eine
      Protokoll-Lücke — hinnehmbar, weil ein falscher "fall_geloescht"-Eintrag
      als Nachweis schlechter wäre als gar keiner.
    - Beide Schreibpfade zeigen ins selbe Verzeichnis (audit.py:AUDIT_DIR =
      produkt/haut/faelle/), sodass Permission-/Disk-Fehler meist beide treffen.
      Ein Audit-Write VOR dem Löschen würde bei voller Platte die Löschung
      selbst verhindern — inakzeptabel für DSGVO.
    """
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    pfad = _fall_pfad(fall_id)
    uid = api_auth._AUTH_USER
    os.remove(pfad)
    audit.append(uid or "unbekannt", "fall_geloescht", fall_id,
                 f"scheibe={store.get('scheibe')}, vz={store.get('veranlagungszeitraum')}")
    return 200, {"geloescht": True, "fall_id": fall_id,
                 "scheibe": store.get("scheibe"),
                 "veranlagungszeitraum": store.get("veranlagungszeitraum")}


def fall_anlegen(body: dict) -> tuple[int, dict]:
    """Neuen Fall anlegen. Speichert zuerst, protokolliert danach — Regel
    im Modul: erst wirken (store/speichern), dann auditieren. Der Audit-
    Eintrag steht NUR, wenn die Datei auf Platte ist.

    Auth-Pflicht ZUERST (_auth_uid_oder_401, dieselbe Politik wie beim Zugriff auf
    bestehende Fälle): vorher hing das Setzen von user_id nur an "ist _AUTH_USER
    zufällig gesetzt" — ein anonymer Request legte auch dann einen Fall an, wenn
    Auth eigentlich aktiv war (TAXGRAPH_NO_AUTH nicht 1), nur eben OHNE Besitzer.
    Ein solcher Fall ist für niemanden — auch nicht den Ersteller — je wieder über
    _fall_owner_check() erreichbar (stored is None sperrt IMMER, s. dort, ohne
    Ausnahme). Diese Funktion ist die EINZIGE Stelle, an der ein Fall je einen
    Besitzer bekommt — die Sperre hier verhindert, dass ownerless Fälle überhaupt
    entstehen, statt sie im Nachhinein zu reparieren.
    Im TAXGRAPH_NO_AUTH=1-Opt-out bleibt ein Fall bewusst ownerless — dort gibt es
    keinen Nutzer, den man eintragen könnte, das ist die ehrliche Antwort."""
    uid = _auth_uid_oder_401()
    scheibe = body.get("scheibe", "ep")
    if scheibe not in SCHEIBEN:
        raise ApiError(400, f"unbekannte Scheibe {scheibe!r}")
    vz = int(body.get("veranlagungszeitraum", 2025))
    fall_id = body.get("fall_id")
    if not fall_id or not _FALL_RE.fullmatch(str(fall_id)):
        raise ApiError(400, "fall_id fehlt oder ungültig (nur [A-Za-z0-9_-]{1,64})")
    if os.path.exists(_fall_pfad(fall_id)):
        raise ApiError(409, f"Fall {fall_id!r} existiert bereits")
    store = ST.leerer_store(vz, fall_id=fall_id)
    store["scheibe"] = scheibe
    if uid is not None:
        store["user_id"] = uid
    speichere_fall(fall_id, store)
    if uid is not None:
        audit.append(uid, "fall_angelegt", fall_id, f"scheibe={scheibe}")
    return 201, {"fall_id": fall_id, "scheibe": scheibe, "veranlagungszeitraum": vz}


def _badge(herkunft: dict) -> str:
    """Herkunfts-Kategorie je Wert (UI-Lab Dim 1, „Herkunft zum Anfassen"): die 6 Store-Herkünfte
    direkt als anzeigbare Badge-Klasse (statt binär solide/schimmernd). Die Haut stylt jede Kategorie
    (laie=selbst · beleg_import=Beleg · vorjahr/berechnet/orakel=abgeleitet · llm_vorschlag=KI-schimmernd)."""
    return herkunft.get("herkunft", "laie")


def _ring_bindung(cfg: dict, bindung: dict) -> dict:
    """Bindung für die Spannen-/intervall-Rechnung: nur die Pflicht-Kegel-Felder. Sonst zögen die
    (bei einzel ungesetzten) Partner-Felder als unbounded-ohne-Wert das Intervall auf nicht_fixierbar."""
    kegel = cfg.get("kegel")
    return {f: bindung[f] for f in kegel if f in bindung} if kegel else bindung


def _gesamt_beitrag(store: dict, cfg: dict, bindung: dict, felder: dict, sid: str, vz: int):
    """Frage-Reihenfolge-Gewichte aus dem verfügbaren Ring (Gesamt bevorzugt, sonst erster Teil)."""
    if cfg["gesamt_ring"]:
        rb = _ring_bindung(cfg, bindung)
        bf = _bescheid_fn(cfg["gesamt_ring"], vz, rb, felder, store, nur_bestaetigt=False)  # Estimate-Pfad: vorläufig zeigt Wirkung im Range
        if bf is not None:
            return {b["feld_id"]: b["spanne_cent"]
                    for b in IV.intervall(felder, rb, bf, snapshot_id=sid)["beitraege"]}
    for _name, q, tfelder in cfg["teil_ringe"]:
        tb = {f: bindung[f] for f in tfelder if f in bindung}
        bf = _bescheid_fn(q, vz, tb, nur_bestaetigt=False)   # Estimate-Pfad (fragen-Gewichte)
        if bf is not None:
            return {b["feld_id"]: b["spanne_cent"]
                    for b in IV.intervall(felder, tb, bf, snapshot_id=sid)["beitraege"]}
    return None


def fragen(fall_id: str) -> tuple[int, dict]:
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    cfg = _cfg(store)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    beitrag = _gesamt_beitrag(store, cfg, bindung, felder, sid, int(store["veranlagungszeitraum"]))
    queue = TR.naechste_fragen(store, bindung, beitrag)
    out = []
    for fid in queue:
        b = bindung[fid]
        out.append({
            "feld_id": fid,
            "fragetext_laie": b.get("fragetext_laie"),
            "hilfe_kurz": b.get("hilfe_kurz"),
            "typ": b["typ"],
            "einheit": b.get("einheit"),
            "bereich": b.get("bereich"),
            "enum_werte": b.get("enum_werte"),
            # Anzeigetexte je enum-Wert (2026-08-14). Ohne sie zeigte die Oberfläche den Rohwert:
            # "land_forst", "gesetzlich_an", oder bei den Kindschaftsverhältnissen nur "1"/"2"/"3".
            # Absent, wenn das Feld kein enum ist — die UI fällt dann auf den Rohwert zurück.
            "enum_labels": ENUM_LABELS.get(fid),
            "beispielwert": b.get("beispielwert"),
            "anker_ref": b.get("anker_ref"),
        })
    return 200, {"fall_id": fall_id, "snapshot_id": sid, "fragen": out}


def stand(fall_id: str) -> tuple[int, dict]:
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    cfg = _cfg(store)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    rel = TR.relevanz(store, bindung)
    vz = int(store["veranlagungszeitraum"])

    # event_id je feld_id: aus aktiven Events (ST._aktives ist Quelle der Wahrheit)
    aktiv = ST._aktives(store)

    felder_out = {
        fid: {"wert": v["wert"], "zustand": v["zustand"], "herkunft": v["herkunft"],
              "herkunft_badge": _badge(v["herkunft"]), "event_id": (aktiv.get(fid) or {}).get("event_id")}
        for fid, v in felder.items()
    }

    gesamt_iv, engine, teil = None, "unavailable", []
    gesperrt = _an_gesamt_sperrgrund(felder, cfg, vz, store, bindung) if cfg.get("guard") else None
    if gesperrt:
        engine = "gesperrt"          # nicht-ring-fähiger Abzug/Einkunftsart -> kein Ring (K2)
    elif cfg["gesamt_ring"]:
        rb = _ring_bindung(cfg, bindung)
        bf = _bescheid_fn(cfg["gesamt_ring"], vz, rb, felder, store, nur_bestaetigt=False)  # Estimate-Pfad: vorläufig zeigt Wirkung im Range
        if bf is not None:
            gesamt_iv = IV.intervall(felder, rb, bf, snapshot_id=sid)["intervall"]
            engine = "catala"
    else:
        for name, q, tfelder in cfg["teil_ringe"]:
            tb = {f: bindung[f] for f in tfelder if f in bindung}
            bf = _bescheid_fn(q, vz, tb, felder, nur_bestaetigt=False)   # Estimate-Pfad (/stand-Teil-Range)
            if bf is not None:
                tiv = IV.intervall(felder, tb, bf, snapshot_id=sid)["intervall"]
                teil.append({"familie": name, "quantitaet": q, "intervall": tiv})
        engine = "catala_teilweise" if teil else "unavailable"

    return 200, {"fall_id": fall_id, "snapshot_id": sid, "engine": engine,
                 "felder": felder_out, "relevanz": rel, "intervall": gesamt_iv,
                 "teil_ringe": teil, "ring_gesperrt": gesperrt}


def event(fall_id: str, body: dict) -> tuple[int, dict]:
    """DER einzige Schreib-Endpunkt — dünne Hülle über store.append_event. Die fail-closed-Garantien
    (llm->vorlaeufig, bestaetigt->signal_2, ein aktives Event/feld) erzwingt der Store, nicht die Haut."""
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    fid = body.get("feld_id")
    if fid not in bindung:
        # Repeated-Instance (#5): base__n einer instanz-fähigen Basis-Bindung ist ein gültiges Instanz-Feld
        # (Instanz 2..N eines Multi-Objekt-/Multi-Rente-Konsumenten). Die Instanz ist reine est_mapping-
        # Konvention — der Store lernt sie nicht, aber der Schreibpfad muss sie durchlassen (parse_instanz =
        # DIESELBE Enumerations-Wahrheit wie instanzen/deklariere, kein zweites Regex).
        parsed = EM.parse_instanz(fid) if isinstance(fid, str) else None
        basis = parsed[0] if parsed else None
        if not (basis and bindung.get(basis, {}).get("instanz_gruppe")):
            raise ApiError(400, f"feld_id {fid!r} nicht in dieser Scheibe")
    zustand = body.get("zustand")
    if zustand not in _ERLAUBTE_ZUSTAENDE:
        raise ApiError(400, f"zustand muss {_ERLAUBTE_ZUSTAENDE} sein")
    herkunft = body.get("herkunft")
    if not isinstance(herkunft, dict) or "herkunft" not in herkunft:
        raise ApiError(400, "herkunft-Objekt (mit Schlüssel 'herkunft') ist Pflicht")
    schreiber = body.get("schreiber")
    if not isinstance(schreiber, str) or not schreiber:
        raise ApiError(400, "schreiber ist Pflicht")
    # dev-2-Kontrakt: der Katalog-Check gilt für Vorschlags-Schreiber (llm:/berechnet:/import:beleg/kontoauszug) —
    # ein Client-gesetzter llm:-Schreiber über /event darf die human-only-Felder NICHT umgehen. mensch (ui:/import:
    # vorjahr/import:elster) ist nicht betroffen → kein Katalog (Confirm braucht keins).
    _vorschlag = isinstance(schreiber, str) and (
        schreiber.startswith(("llm:", "berechnet:", "import:beleg", "import:kontoauszug")))
    try:
        ev = ST.append_event(
            store, feld_id=fid, wert=body.get("wert"), zustand=zustand, herkunft=herkunft,
            schreiber=schreiber, signal=body.get("signal"), ersetzt=body.get("ersetzt"),
            ts=body.get("ts"),
            # GLOBALER Katalog (TR.lade_bindung()), nicht per-Scheibe: die Autorisierung hängt am Feld, dev-2-Kontrakt.
            katalog=(ST.lade_katalog(TR.lade_bindung()) if _vorschlag else None),
            # Auflage T (Stille-Null-Klasse, team-lead-Auftrag): body["wert"] kommt roh vom Client (Befund A —
            # jeder Typ wurde bisher akzeptiert). bindung = die schon oben (Z.2655) geprüfte Scheiben-Bindung.
            bindung=bindung)
    except ValueError as e:
        # fail-closed-Abweisung des Stores -> 422 (nicht 500): die Haut hat korrekt weitergereicht.
        raise ApiError(422, str(e))
    speichere_fall(fall_id, store)
    return 201, {"event_id": ev["event_id"], "feld_id": fid, "zustand": zustand}


def warum(fall_id: str, feld_id: str) -> tuple[int, dict]:
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    j = TR.justification(store, feld_id, bindung)
    if j is None:
        raise ApiError(404, f"Feld {feld_id!r} hat (noch) kein Event")
    return 200, {"fall_id": fall_id, "justification": j}


def ergebnis(fall_id: str) -> tuple[int, dict]:
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    cfg = _cfg(store)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    # Pflicht-Kegel: einzel-Basis (cfg["kegel"]); die Partner-Pflichtfelder gehören nur bei
    # veranlagung=zusammen dazu und werden dort vom Guard geprüft.
    scheibe_felder = cfg.get("kegel") or _scheibe_felder(store)
    vz = int(store["veranlagungszeitraum"])
    if cfg.get("guard"):
        # K2: ein nicht-ring-fähiger Abzug/Einkunftsart sperrt den Ring VOR jeder Zahl — nie Fake-Bescheid.
        sperr = _an_gesamt_sperrgrund(felder, cfg, vz, store, bindung)
        if sperr:
            return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": None,
                         "solz_cent": None, "kist_cent": None, "mobilitaetspraemie_cent": None,
                         "abschlusszahlung_cent": None,
                         "grund": sperr, "offen": [], "trace": None}
    result = _feste_zahl(felder, bindung, cfg, vz, scheibe_felder, store)
    if result is None:
        if cfg["gesamt_ring"] is None:
            # Multi-Regel-Scheibe ohne ehrlichen Gesamt-Accessor: bewusst KEINE Scheiben-Zahl.
            return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": None,
                         "solz_cent": None, "kist_cent": None, "mobilitaetspraemie_cent": None,
                         "abschlusszahlung_cent": None,
                         "grund": "kein_scheiben_gesamtbescheid", "offen": [], "trace": None}
        # dieselbe Relevanz-Sicht wie _feste_zahl: ein Feld, dessen Regel der Nutzer abbestellt
        # hat, darf auch nicht als "offen" gemeldet werden — sonst zeigt die API weiter auf eine
        # Frage, die der Traverser gar nicht mehr stellt.
        offen = [f for f in _relevante_kegel_felder(scheibe_felder, bindung, store)
                 if f not in felder or felder[f]["zustand"] != "bestaetigt"]
        bf = _bescheid_fn(cfg["gesamt_ring"], vz, bindung, felder)
        grund = "engine_unavailable" if (bf is None and not offen) else "input_kegel_nicht_bestaetigt"
        return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": None,
                     "solz_cent": None, "kist_cent": None, "mobilitaetspraemie_cent": None,
                     "abschlusszahlung_cent": None,
                     "grund": grund, "offen": sorted(offen), "trace": None}
    zahl, solz, extras = result
    trace = TR.trace_ergebnis(store, bindung, snapshot_id=sid)
    # Klasse-C "stille Null" (BACKLOG stille-null-klasse-c, Variante b): gwg/kind/p23_veraeusserung
    # sind reine additive Σ-Funktionen OHNE Kegel-/Sperrgrund-Gate (anders als vv_objekt/rente) — ihr
    # nur_bestaetigt-Filter laesst eine vorlaeufige Instanz korrekt aus der Zahl raus (Zahl bleibt
    # richtig), meldet das aber nirgends. Hinweis statt Sperre: grund bleibt "bestaetigt", zahl_cent
    # bleibt die gefilterte Zahl, offen listet die Basis-Feld-IDs der vorlaeufigen Instanzen.
    offen_c = sorted({
        fid for gruppe in ("gwg", "kind", "p23_veraeusserung")
        for inst in EM.instanzen(store, bindung, gruppe)
        for fid, fw in inst["felder"].items() if fw.get("zustand") != "bestaetigt"
    })
    return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": zahl,
                 "solz_cent": solz, "kist_cent": extras.get("kist_cent"),
                 "mobilitaetspraemie_cent": extras.get("mobilitaetspraemie_cent"),
                 "abschlusszahlung_cent": _abschlusszahlung_cent(felder, zahl),
                 "grund": "bestaetigt", "offen": offen_c, "trace": trace,
                 "kette": extras.get("kette")}


def preflight_check(fall_id: str) -> tuple[int, dict]:
    """P5.5 Preflight-Check: Konsistenz-Prüfungen + vergessene Pauschalen. NULL LLM."""
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    felder, _ = ST.materialisiere(store)
    ergebnis = PF.preflight(felder)
    # Nur hinweise/items ausliefern, die auch was zu sagen haben
    items = []
    for w in ergebnis.get("widersprueche_flag", []):
        items.append({"typ": "widerspruch", "bereich": "flag", "text": w["grund"]})
    for w in ergebnis.get("widersprueche_partner", []):
        items.append({"typ": "widerspruch", "bereich": "partner", "text": w["grund"]})
    for w in ergebnis.get("widersprueche_alleinerziehend", []):
        items.append({"typ": "widerspruch", "bereich": "alleinerziehend", "text": w["grund"]})
    for h in ergebnis.get("hinweise_pauschalen", []):
        items.append({"typ": "hinweis", "bereich": "pauschale", "text": h["hinweis"]})
    return 200, {"fall_id": fall_id, "status": ergebnis["status"], "items": items}


def deklaration(fall_id: str) -> tuple[int, dict]:
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    vz = int(store.get("veranlagungszeitraum") or 0)
    felder = _mit_ring_werten(felder, vz)
    result = EM.deklariere(felder, bindung, snapshot_id=sid)
    return 200, {"fall_id": fall_id, **result}


def einreichen(fall_id: str, body: dict) -> tuple[int, dict]:
    """P3.2b — Deklaration → ELSTER-XML → checkESt-Plausibilitätsprüfung. KEIN Versand.

    Nur validieren: `EricBearbeiteVorgang` läuft mit ERIC_VALIDIERE (ohne ERIC_SENDE), also
    rein lokal im checkESt-Plugin — kein Netz, keine Credentials, nichts verlässt die Maschine.
    Der eigentliche Versand (ERIC_ENCRYPT_AND_SEND) braucht Nutzer-Zertifikat + PIN und ist
    bewusst NICHT verdrahtet; er kommt als eigener, explizit freigeschalteter Schritt.

    Fail-closed an drei Stellen: unvollständige Deklaration → 409, XML nicht baubar → 422,
    Plausibilitätsfehler → 422 mit der ERiC-Antwort. Nur rc==0 ist grün — ein leerer
    Fehlerpuffer bei rc!=0 heißt „nicht geprüft", nicht „fehlerfrei" (I/O-Gate-Short-Circuit).
    """
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)

    cfg = _cfg(store)

    # Scheibenwahl VOR allem anderen: nicht jede Scheibe kann eine Erklaerung tragen.
    # Kriterium ist NICHT `gesamt_guard` (der meint Rechen-Vollstaendigkeit), sondern ob die
    # STAMMDATEN_FELDER ueberhaupt im Kegel liegen. Gemessen 2026-08-10:
    #   an_gesamt  69 Felder,   0/12 Stammdaten   -> strukturell nicht abgabefaehig
    #   gesamt     201 Felder, 12/12 Stammdaten
    #   rentner_gesamt 148 Felder, 12/12
    # Ohne diese Pruefung laeuft so ein Fall bis checkESt und scheitert dort an fehlenden
    # Stammdaten, die der Nutzer auf dieser Scheibe nie haette liefern koennen: POST /event
    # lehnt sie mit 400 "nicht in dieser Scheibe" ab. Die Meldung zeigte auf den Nutzer
    # ("Angaben fehlen") statt auf die Scheibenwahl. Fail-closed war es vorher schon —
    # ehrlich erst jetzt. (BACKLOG einreichen-ohne-scheiben-pruefung)
    _fehlend = [f for f in STAMMDATEN_FELDER if f not in set(cfg.get("felder") or ())]
    if _fehlend:
        return 409, {
            "fall_id": fall_id, "eingereicht": False,
            "grund": "scheibe_nicht_abgabefaehig",
            "scheibe": store.get("scheibe"),
            "fehlende_stammdatenfelder": _fehlend,
            "hinweis": (
                f"Die Scheibe {store.get('scheibe')!r} ist eine Teilrechnung und kann keine "
                f"Einkommensteuererklaerung tragen: {len(_fehlend)} Stammdatenfelder "
                f"(Name, Anschrift, Steuernummer) liegen nicht in ihrem Fragenkegel und "
                f"koennen dort auch nicht beantwortet werden. Legen Sie den Fall auf einer "
                f"abgabefaehigen Scheibe an ('gesamt' oder 'rentner_gesamt'). Die Berechnung "
                f"auf dieser Scheibe bleibt nutzbar.")}

    # Sperrgrund-Prüfung VOR Deklaration: Ring ist rechnerunfähig → 409 mit unserem Grund,
    # nicht ERiCs falschem Grund später. Identisch wie in ergebnis() (Zeile 2075).
    vz = int(store.get("veranlagungszeitraum") or 0)
    felder = _mit_ring_werten(felder, vz)
    if cfg.get("guard"):
        sperr = _an_gesamt_sperrgrund(felder, cfg, vz, store, bindung)
        if sperr:
            return 409, {"fall_id": fall_id, "eingereicht": False, "grund": sperr,
                         "hinweis": "Die Deklaration kann nicht erstellt werden, weil eine erforderliche Angabe fehlt."}

    result = EM.deklariere(felder, bindung, snapshot_id=sid)
    if not result["vollstaendig"]:
        return 409, {"fall_id": fall_id, "eingereicht": False, "grund": "deklaration_unvollstaendig",
                     "unvollstaendig": result["unvollstaendig"]}
    try:
        # abgabefaehig=True haengt den <Vorsatz>-Block an. Ohne ihn beanstandet checkESt
        # 9 Pflichtfelder (Absender, Unterfallart, Vorgang, Zeitraum, Copyright, OrdNrArt,
        # Rueckuebermittlung) — gemessen 2026-08-09, in Einzel- wie Zusammenveranlagung
        # gleich. Der Endpunkt fuhr bis dahin den Default False und damit einen Pfad, der
        # nie einreichbar war; die Ratsche in tests/test_checkest_durchstich.py mass
        # gleichzeitig die abgabefaehige Variante und zeigte deshalb weniger Fehler an,
        # als der Nutzer tatsaechlich bekam (BACKLOG: endpunkt-naht-abgabepfad).
        # snapshot=felder: der Vorsatz-Block leitet Absender und Steuernummer aus den
        # bereits deklarierten Stammdaten ab (elster_xml._leite_absender_ab /
        # _leite_steuernummer_ab) — keine zweite Repraesentation derselben Angaben.
        xml = EX.erzeuge_xml(result, vz=vz,
                             empfaenger_land=str(body.get("empfaenger_land") or "BY"),
                             testmerker=EX.TESTMERKER_ERIC,
                             abgabefaehig=True, snapshot=felder)
    except EX.XmlFehler as e:
        return 422, {"fall_id": fall_id, "eingereicht": False, "grund": "xml_nicht_baubar",
                     "detail": str(e)}

    try:
        import checkest_gate as CE
    except ImportError as e:
        return 503, {"fall_id": fall_id, "eingereicht": False, "grund": "eric_nicht_verfuegbar",
                     "detail": str(e)}
    try:
        rc, antwort = CE.validate(xml, f"ESt_{vz}")
    except (RuntimeError, OSError) as e:
        return 503, {"fall_id": fall_id, "eingereicht": False, "grund": "eric_nicht_verfuegbar",
                     "detail": str(e)}

    klasse = CE.klassifiziere_rc(rc)
    basis = {"fall_id": fall_id, "eingereicht": False, "basis_snapshot": sid,
             "vz": vz, "rc": rc, "klasse": klasse, "xml_bytes": len(xml.encode("utf-8"))}
    if rc == CE.RC_DATENARTVERSION_UNBEKANNT:
        # Kein Pruefmodul fuer diesen VZ (z.B. ESt_2026 in ERiC 44.2.4.0). Die Erklaerung
        # wurde NICHT geprueft — sie als "plausibilitaet_verletzt" zu melden waere eine
        # Falschaussage ueber die Erklaerung des Nutzers, nicht ueber unser Werkzeug.
        return 422, {**basis, "grund": "kein_pruefmodul_fuer_vz",
                     "detail": f"ERiC kennt die Datenartversion ESt_{vz} nicht — fuer diesen "
                               f"Veranlagungszeitraum liegt kein Pruefmodul vor. Die Erklaerung "
                               f"wurde nicht geprueft.", "ericantwort": antwort}
    if klasse == "plausibilitaet_fehler":
        return 422, {**basis, "grund": "plausibilitaet_verletzt", "ericantwort": antwort,
                     "moeglicherweise_gekappt": CE.gekappt_verdacht(antwort)}
    if rc != CE.RC_OK:
        # Fail-closed: JEDER andere rc — bekannt (io_gate_nicht_geprueft,
        # hersteller_id_gesperrt, io_reader_unerwartete_elemente, ...) oder unbekannt
        # ("sonstig") — ist KEIN Plausibilitaetsverdikt. "plausibilitaet_verletzt" wuerde
        # hier eine Falschaussage ueber die Erklaerung sein (Fund 2026-08-12: rc=610301106
        # / person_b_idnr fiel bisher auf "sonstig" und landete trotzdem hier).
        return 422, {**basis, "grund": "rc_kein_plausibilitaetsverdikt",
                     "detail": CE.unerwarteter_rc_hinweis(rc, antwort), "ericantwort": antwort}
    audit.append(api_auth._AUTH_USER or "dev", "fall_validiert", fall_id, f"vz={vz} rc=0")
    return 200, {**basis, "plausibel": True,
                 "hinweis": "checkESt bestanden. Versand ist nicht verdrahtet — "
                            "ERIC_ENCRYPT_AND_SEND braucht Zertifikat + explizite Freigabe."}


def graph(fall_id: str) -> tuple[int, dict]:
    """Read-only Abhängigkeits-Übersicht (Desktop): Knoten = Regeln der Scheibe mit ihrem
    Relevanz-Status (aus traverser.relevanz), Kanten = Feld→Regel (welches Abfrage-Feld welche Regel
    speist, mit Feld-Zustand). Reine Ableitung, EIN Traverser-Aufruf, kein Bescheid, kein Schreibpfad."""
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    rel = TR.relevanz(store, bindung)
    knoten = [{"regel_id": rid, "status": s["status"],
               "gates_offen": s["gates_offen"], "annahmen_offen": s["annahmen_offen"]}
              for rid, s in sorted(rel.items())]
    kanten = []
    for fid in sorted(bindung):
        q = bindung[fid]["quelle"]
        ev = felder.get(fid)
        kanten.append({
            "feld_id": fid,
            "regel_id": q["regel_id"],
            "rolle": "slot" if "signatur_slot" in q else "gate",
            "zustand": ev["zustand"] if ev else "offen",
            "fragetext_laie": bindung[fid].get("fragetext_laie"),
        })
    return 200, {"fall_id": fall_id, "snapshot_id": sid, "knoten": knoten, "kanten": kanten}


# POST /chat und POST /elster-ampel: bewusst KEINE 200-Antwort in dieser Stufe.
CHAT_501 = {
    "fehler": "not_implemented",
    "vertrag": ("LLM-Chat schreibt qua Store-Auflage A ausschliesslich vorlaeufig-Events "
                "(schreiber='llm:…', herkunft.herkunft='llm_vorschlag', signal_2=null); "
                "Bestätigung bleibt der menschliche Zwei-Signal-Klick."),
    "stufe": "spätere Stufe mit eigenem Julius-Cap — kein LLM-Call in dieser Stufe.",
}
AMPEL_503 = {
    "fehler": "unavailable",
    "grund": ("ELSTER-Ampel (warmer checkESt-Daemon) ist für diese Scheibe noch nicht verdrahtet "
              "— ein gültiger ESt-Fall entsteht erst mit der Gesamtsteuer-Integration. Kein Fake-Grün."),
    "regel": "gekappt_verdacht=true ist nie grün (API.md-Garantie 5).",
}
# Arbeitsweg-Entfernung über Karten-Dienst (Julius-Feature): der Geocoding+Routing-Aufruf ist LIVE
# (ors_client, echter Call) — eine AUSGEHENDE Integration mit PII (Adressen verlassen das Gerät), daher
# nur env-key-gated ($ORS_API_KEY, .env.maps). Kein Key / Netz-/Antwort-Fehler → sauberer Fallback auf
# die manuelle km-Eingabe (ENTFERNUNG_FALLBACK unten), nie Crash, nie Fake-km.
ENTFERNUNG_FALLBACK = {
    "fehler": "unavailable",
    "vertrag": ("Der Karten-Dienst ist nicht verbunden (kein Schlüssel gesetzt oder Netz-/Antwort-Fehler) "
                "— bitte gib die Entfernung manuell ein (kürzeste Straßenverbindung, § 9 Abs. 1 S. 3 Nr. 4 EStG)."),
}


def entfernung(fall_id: str, body: dict) -> tuple[int, dict]:
    """Arbeitsweg-km über den Karten-Dienst (Julius-Feature). AUSGEHENDE PII-Integration: die Adressen
    gehen an OpenRouteService (nur auf Nutzer-Klick). Das Ergebnis ist ein VORSCHLAG — die Haut prefillt
    das km-Feld, der Nutzer bestätigt/überschreibt (Zwei-Signal, § 9 kürzeste Straßenverbindung; eine
    längere ist bei regelmäßiger Nutzung zulässig). Kein Key / Netzfehler → 503-Fallback (manuell), nie
    Crash, nie still gesetzt. Der API-Key kommt nur aus $ORS_API_KEY (nie im Repo)."""
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)                            # 404, wenn der Fall nicht existiert
    von = (body.get("von") or "").strip()
    nach = (body.get("nach") or "").strip()
    if not von or not nach:
        raise ApiError(400, "von und nach (Adressen) sind Pflicht")
    bindung = _scheibe_bindung(store)
    if "ep_entfernung_km" not in bindung:
        raise ApiError(400, "diese Scheibe hat kein Arbeitsweg-km-Feld")
    import ors_client
    try:
        km = ors_client.entfernung_km(von, nach)
    except (ors_client.OrsNichtVerfuegbar, ImportError):  # Cap-Gate/Netzfehler/Import → Erklär-Grenze;
        return 503, ENTFERNUNG_FALLBACK                   # ein echter Logik-Bug propagiert (K2, konsistent zu chat()/kontoauszug)
    # PROVENIENZ (K2, „Herkunft je Wert"): der km-Wert kommt aus dem Karten-Dienst → als VORLÄUFIGES
    # Event mit herkunft=berechnet ins Store (Badge zeigt „berechnet/maps", NICHT „selbst"). Der Nutzer
    # bestätigt/überschreibt (Zwei-Signal). Ein aktives Event des Felds wird ersetzt (Nutzer hat „berechnen"
    # geklickt = er will den Karten-Vorschlag). signal_1 trägt die Provenienz (KEINE Adressen — PII-sparsam).
    aktiv_ev = ST._aktives(store).get("ep_entfernung_km")
    try:
        ev = ST.append_event(
            store, feld_id="ep_entfernung_km", wert=km, zustand="vorlaeufig",
            herkunft={"herkunft": "berechnet", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            schreiber="berechnet:maps",
            signal={"signal_1": {"typ": "maps", "dienst": "openrouteservice"}, "signal_2": None},
            ersetzt=(aktiv_ev["event_id"] if aktiv_ev else None),
            katalog=ST.lade_katalog(TR.lade_bindung()),  # berechnet:-Schreiber → GLOBALER Katalog-Check (dev-2-Kontrakt)
            bindung=bindung)   # Auflage T (Stille-Null-Klasse) — s. event() oben
    except ValueError as e:
        raise ApiError(422, str(e))
    speichere_fall(fall_id, store)
    return 200, {"km": km, "event_id": ev["event_id"], "herkunft": "berechnet",
                 "hinweis": "Vorschlag aus dem Karten-Dienst — bitte prüfen und bestätigen."}


def vorjahr(fall_id: str, body: dict) -> tuple[int, dict]:
    """Vorjahr-Übernahme (dev-2s vorjahr_writer): überträgt die vorjahr-flagged, im Vorjahres-Fall
    BESTÄTIGTEN Felder als VORLÄUFIGE Vorschläge (herkunft=vorjahr) in den aktuellen Fall — der Nutzer
    bestätigt/überschreibt (Zwei-Signal). Der Store-Guard ^import:vorjahr erzwingt vorläufig strukturell;
    schon belegte Felder bleiben unangetastet (One-Active-Event)."""
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    vj_id = body.get("vorjahr_fall_id")
    if not vj_id or not _FALL_RE.fullmatch(str(vj_id)):
        raise ApiError(400, "vorjahr_fall_id fehlt oder ungültig")
    if vj_id == fall_id:
        raise ApiError(400, "vorjahr_fall_id muss ein ANDERER (Vorjahres-)Fall sein")
    # Die QUELLE gehoert derselben Pruefung wie das Ziel. Ohne diese Zeile konnte ein
    # eingeloggter Nutzer Felder aus einem FREMDEN Fall in seinen eigenen ziehen: die Kennung
    # kommt aus dem Request-Body, und _fall_owner_check() oben deckt nur fall_id ab.
    # Dieselbe Klasse wie das DELETE-Loch (39fcf79): eine Route, die die Naht nur halb benutzt.
    # tests/test_zweite_fall_kennung_gate.py haelt fest, dass JEDE aus dem Body aufgeloeste
    # zweite Fall-Kennung geprueft wird — der Fix ist eine Zeile, das Wiederkommen ist das Problem.
    _fall_owner_check(vj_id)
    vj_store = lade_fall(vj_id)                           # 404, wenn der Vorjahres-Fall fehlt
    vj_felder, _ = ST.materialisiere(vj_store)
    n = VW.uebernehme_vorjahr(store, vj_felder, bindung,
                              vorjahr_vz=int(vj_store.get("veranlagungszeitraum", 0)))
    speichere_fall(fall_id, store)
    return 200, {"uebernommen": n, "vorjahr_fall_id": vj_id}


def kontoauszug(fall_id: str, body: dict) -> tuple[int, dict]:
    """Kontoauszug-Upload (dev-2s kontoauszug_writer): parst den Auszug und schreibt je AUSGABEN-Transaktion
    mit eindeutiger deterministischer Kategorie + Ziel-Feld in DIESER Scheibe einen VORLÄUFIGEN Vorschlag
    (herkunft=kontoauszug, Store-Guard ^import:kontoauszug erzwingt vorläufig). Der Nutzer bestätigt neben
    dem Auszug (K2). Deterministik-first: der LLM-Klassifikator-Fallback (mehrdeutige Zwecke) ist verdrahtet,
    aber selbst cap-gated (kein $LLM_API_KEY → jede Transaktion fällt still auf "unklassifiziert" zurück, kein
    Crash, kein Mock-Call). IBAN/Kontonummern werden vom Writer maskiert (PII). Kein Überschreiben aktiver
    Felder. § 35a-Kategorien greifen nur, wenn die Scheibe die Ziel-Felder führt (sonst 0 Vorschläge)."""
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    fmt = (body.get("format") or "").strip().lower()
    inhalt = body.get("inhalt")
    import kontoauszug_writer as KW
    n_verworfen = 0
    if fmt == "csv":
        tx = KW.parse_csv(inhalt if isinstance(inhalt, str) else "")
    elif fmt == "json":
        try:
            tx = inhalt if isinstance(inhalt, list) else json.loads(inhalt or "[]")
        except (ValueError, TypeError):
            raise ApiError(400, "json-Inhalt nicht parsebar")
        if not isinstance(tx, list):
            raise ApiError(400, "json muss eine Liste von Transaktionen sein")
    elif fmt == "pdf":
        # PDF kommt als base64 im JSON-Body (server.py macht NUR json.loads, kein Multipart) — auf
        # Disk entpackt, weil der OCR/Layout-Pfad (wie beleg_writer.lies_beleg_text) pfad-basiert ist,
        # nicht bytes-basiert. tmp-Datei trägt den ROHEN Bank-Auszug (PII/IBAN vor Writer-Maskierung)
        # → finally: os.unlink, UNBEDINGT auch bei Exception im OCR/Parse-Pfad (kein Disk-Leck).
        if not isinstance(inhalt, str) or not inhalt.strip():
            raise ApiError(400, "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)")
        try:
            pdf_bytes = base64.b64decode(inhalt, validate=True)
        except ValueError:              # binascii.Error ist eine ValueError-Unterklasse (verifiziert)
            raise ApiError(400, "pdf-Inhalt nicht gültig base64-kodiert")
        fd, pfad = tempfile.mkstemp(suffix=".pdf")
        try:
            with os.fdopen(fd, "wb") as fh:
                fh.write(pdf_bytes)
            try:
                text, conf_map = KW.lies_kontoauszug_pdf(pfad)
            except (subprocess.TimeoutExpired, KW.OcrZuAufwendig) as e:
                # Der Server ist einfädig; ein hängendes pdftoppm/tesseract legt ihn ganz still
                # (Audit res-ocr-subprocess-no-timeout). Die Zeitlimits im Writer brechen das ab,
                # hier wird daraus eine Antwort, die der Nutzer versteht — 422, weil die Ursache
                # in aller Regel die eingereichte Datei ist und er handeln kann (kürzen, als CSV
                # exportieren), nicht ein vorübergehender Systemzustand.
                raise ApiError(422, f"Kontoauszug nicht lesbar: {e}")
            tx, n_verworfen = KW.parse_pdf_zeilen(text, conf_map)
        finally:
            os.unlink(pfad)
    else:
        raise ApiError(400, "format muss csv, json oder pdf sein")
    # katalog GLOBAL (dev-2-Kontrakt): Enforcement decoupled vom per-Scheibe-Targeting.
    n = KW.uebernehme_kontoauszug(store, tx, bindung, llm_klassifikator=api_llm._kontoauszug_llm_klassifikator(),
                                  katalog=ST.lade_katalog(TR.lade_bindung()))
    speichere_fall(fall_id, store)
    out = {"uebernommen": n, "transaktionen": len(tx), "verworfen": n_verworfen}
    if n_verworfen > 0:
        out["hinweis"] = f"{n_verworfen} Zeile(n) unsicher erkannt (Confidence < 60%) — bitte manuell prüfen/nachtragen."
    return 200, out


def _ist_struktureller_konflikt(fid: str) -> bool:
    """'Gross' (Stufe 2: Rückfrage statt Inline-Häkchen) heisst: das strittige Feld steuert SELBST eine
    andere Regel — es taucht als `feld` in einer regel_bedingung auf (bindung_regel_bedingungen.yaml,
    schema.json $defs/regel_bedingung; traverser.relevanz() wertet das regel-weit aus, traverser.py:113).
    Dann ändert eine Übernahme nicht bloss einen Wert, sondern WELCHE FRAGEN im Interview überhaupt noch
    gelten — ein stilles Häkchen wäre zu wenig. Bewusst NICHT hardcodiert (veraltet sonst lautlos): das
    Kriterium liest dieselbe Quelle, die auch relevanz() bindet, und wächst mit ihr mit."""
    bedingungen = TR.lade_regel_bedingungen()
    return any(rb["feld"] == fid for liste in bedingungen.values() for rb in liste)


def _anzeige_metadaten(fid: str, bindung: dict) -> dict:
    """Anzeige-Metadaten eines Felds — dieselben Schlüssel und dieselbe Quelle wie in fragen().

    Ohne sie zeigt die Verstanden-Seite das, was der Store führt: `bruttoarbeitslohn = 6200000`.
    Das ist die Feld-ID und der Cent-Rohwert, also genau die zwei Dinge, die ein Laie nicht lesen
    kann — und er soll hier ja BESTÄTIGEN, was verstanden wurde. Mit `frage`/`typ`/`einheit`/
    `enum_labels` kann die Oberfläche daraus „Wie hoch war dein Bruttoarbeitslohn? 62.000,00 €"
    machen, mit denselben Formatierern, die sie im Fragefluss schon benutzt.

    Fehlt das Feld in der Bindung (kann nach einem Scheiben-Wechsel passieren), bleiben die Werte
    None — die Oberfläche fällt dann auf feld_id + Rohwert zurück, statt nichts anzuzeigen."""
    b = bindung.get(fid) or {}
    return {"frage": b.get("fragetext_laie"), "typ": b.get("typ"),
            "einheit": b.get("einheit"), "enum_labels": ENUM_LABELS.get(fid)}


def chat(fall_id: str, body: dict) -> tuple[int, dict]:
    """Chat-Berater (K1), EIN Kanal für beides: Werte vorschlagen UND Fragen beantworten.

    Julius 2026-08-14: „‚Ein Satz an die KI' kann aber auch einfach eine Nachfrage sein." Vorher
    gab es zwei Knöpfe und zwei Endpunkte — der Nutzer musste seinen eigenen Satz vorher
    einsortieren, obwohl ein Satz oft beides ist. Jetzt geht jede Nachricht denselben Weg, und die
    Antwort trägt `vorschlaege` UND `antwort`/`unsicher`; eines von beidem darf leer sein.

    Die Trennung liegt damit nicht mehr im Kanal, sondern im UMGANG mit dem Ergebnis: aus
    `vorschlaege` werden VORLÄUFIGE Events (Auflage A, Katalog-Check, Beleg-Gate), `antwort` ist
    reiner Text und wird nirgends gespeichert. Ein Modell, das im Fließtext behauptet „ich trage
    dir 220 Tage ein", ändert deshalb nichts — geschrieben wird nur, was durch das Beleg-Gate und
    den Katalog kommt.

    Ursprüngliche Beschreibung des Vorschlags-Teils:
    der Nutzer beschreibt seine Situation in Freitext → das LLM SCHLÄGT Feld-Werte VOR →
    jeder Vorschlag wird als VORLÄUFIGES Event geschrieben (schreiber='llm:chat'). Store-Auflage A + der Katalog-
    Check (katalog=lade_katalog) erzwingen strukturell: herkunft=llm_vorschlag, zustand=vorlaeufig, signal_2=null
    (nie in die Summe ohne menschlichen Hold-Confirm) UND nur Felder die der Katalog als LLM-vorschlagbar führt
    (identitäts-/rechtskritische Felder lehnt der Check ab). CAP-GATED: kein LLM-Key/Provider → 501 + Erklär-
    Vertrag ($0, kein Mock-Call). Die KI setzt NIE einen Wert.

    ZWEI sichtbare Abweisungs-Formen (vorher beide gleich still in `abgelehnt`, ununterscheidbar — Auftrag
    Konfliktdialog): additiv, `abgelehnt`/dessen Form bleibt UNVERÄNDERT (Konsumenten-Vertrag).
      - `abgelehnt` (+ NEU `abgelehnt_gruende`, feld_id→Text): Katalog/Auflage-A/F2-Abweisung — das Feld ist
        human-only oder der Vorschlag selbst fällt durch (z.B. Magnitude-Bound). ENDGÜLTIG, die KI darf das nie.
      - `konflikte` (NEU, additiv): das Feld ist grundsätzlich LLM-vorschlagbar, hat aber schon ein aktives
        Event (Auflage B — höchstens ein aktives Event/Feld) — der Nutzer hat es selbst gesetzt. KEIN
        stiller Abbruch mehr: feld_id/aktueller Wert/Vorschlag/Begründung gehen raus, dazu `gross`
        (_ist_struktureller_konflikt) für Felder, die selbst andere Regeln steuern — dort reicht ein Häkchen
        nicht (Stufe 2, Rückfrage-Formulierung ist UI/Julius). Auflage B bleibt SCHARF: die Übernahme selbst
        läuft nie über llm:chat, sondern über den bestehenden /event-Pfad mit menschlichem signal_2."""
    _fall_owner_check(fall_id)
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    freitext = (body.get("text") or "").strip()
    # ZWEI Kataloge (dev-2-Kontrakt, msg 4365 — NICHT verwechseln):
    #  (1) PROMPT-Katalog (Haut-Zone): die LLM-vorschlagbaren Felder DIESER Scheibe als Metadaten-LISTE
    #      [{feld_id, fragetext_laie, typ, bereich, enum_werte}] — nur Kontext für die KI, welche Felder sie
    #      überhaupt vorschlagen darf. Wird an _llm_vorschlaege übergeben (dessen _chat_prompt eine Liste will).
    #  (2) CHECK-Katalog (Store-Enforcement): GLOBAL via TR.lade_bindung(), Form {typ→frozenset(feld_id)} — die
    #      un-bypassbare Untergrenze in append_event. GLOBAL, NICHT per-Scheibe: die Autorisierung eines Felds
    #      hängt an seinem `vorschlagbar_von`, nicht an der offenen Scheibe (ein per-Scheibe-Check-Katalog würde
    #      Vorschlags-Schreiber auf global-autorisierte Nicht-Scheibe-Felder fälschlich abweisen — fail-OPEN wäre
    #      es NIE, aber falsch-abweisen bricht legitime beleg/kontoauszug-Writes).
    check_katalog = ST.lade_katalog(TR.lade_bindung())
    # DERSELBE Maßstab wie der Check-Katalog. Vorher filterte diese Liste eigenständig auf
    # `vorschlagbar_von` — seit die Freigabe in lade_katalog() entschieden wird (Julius-Entscheid
    # 2026-08-14, alle askable Felder) hätte das LLM weiter nur die 17 alten Felder GESEHEN,
    # während der Store 263 erlaubt. Zwei Listen mit zwei Regeln für dieselbe Frage: genau die
    # Naht, an der hier schon mehrfach etwas auseinanderlief.
    # `hilfe_kurz` kommt mit, weil dort steht, WAS zum Feld gehört ("Steht auf der
    # Lohnsteuerbescheinigung Nr. 3", "Nach Abzug von Erstattungen"). Ohne diese Erklärung kann
    # die KI weder sauber zuordnen noch merken, dass eine Angabe unvollständig ist.
    prompt_katalog = [
        {"feld_id": fid, "fragetext_laie": b.get("fragetext_laie", ""),
         "hilfe_kurz": b.get("hilfe_kurz", ""), "typ": b.get("typ"),
         "bereich": b.get("bereich"), "enum_werte": b.get("enum_werte")}
        for fid, b in bindung.items() if fid in check_katalog["llm"]]
    # Kontext für die ANTWORT-Hälfte: das gerade offene Feld (schickt die Oberfläche mit), sein
    # Zitatanker und die schon bestätigten Angaben. Für die Vorschläge ist er ohne Bedeutung.
    kontext = _erklaer_kontext(store, bindung, body.get("feld_id") or None)
    try:
        erg = api_llm._llm_dialog(freitext, prompt_katalog, kontext, user_id=api_auth._AUTH_USER)
    except (api_llm.LlmNichtVerfuegbar, ImportError):   # Cap-Gate/Import → reine Erklär-Grenze (kein Key, $0);
        return 501, CHAT_501                             # echte Logik-/Parse-Bugs propagieren (konsistent zu kontoauszug)
    vorschlaege = erg["vorschlaege"]
    geschrieben, abgelehnt, konflikte = [], [], []
    abgelehnt_gruende = {}
    for v in vorschlaege:
        fid = v.get("feld_id")
        # Auflage-B-Vorprüfung: fid schon aktiv UND grundsätzlich katalog-erlaubt? -> KONFLIKT (Fall 2), nicht
        # abgelehnt. Reihenfolge spiegelt append_event: Katalog sticht immer zuerst — ein human-only-Feld bleibt
        # abgelehnt, auch wenn es zufällig schon einen Wert trägt (die KI durfte es nie vorschlagen, Fall 1).
        bestehendes = ST._aktives(store).get(fid) if fid else None
        if bestehendes is not None and fid in check_katalog["llm"]:
            konflikte.append({
                "feld_id": fid,
                "aktueller_wert": bestehendes["wert"],
                "aktuelles_event_id": bestehendes["event_id"],
                "vorschlag_wert": v["wert"],
                "begruendung": v.get("begruendung", ""),
                "beleg": v.get("beleg", ""),
                "gross": _ist_struktureller_konflikt(fid),
                # Dieselben Anzeige-Metadaten wie bei den Vorschlägen. Ein Konflikt zeigt ZWEI
                # Werte nebeneinander — ohne sie stünde dort zweimal Speicherform, und genau hier
                # muss der Nutzer zwei Zahlen vergleichen können.
                **_anzeige_metadaten(fid, bindung),
            })
            continue
        try:
            ev = ST.append_event(
                store, feld_id=fid, wert=v["wert"], zustand="vorlaeufig",
                herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                schreiber="llm:chat",
                # Der Beleg (wörtliches Nutzerzitat) wandert ins signal_1 und bleibt damit im
                # Store — später ist am Event nachvollziehbar, WORAUF sich der Vorschlag stützte.
                signal={"signal_1": {"typ": "llm", "begruendung": v.get("begruendung", ""),
                                     "beleg": v.get("beleg", "")}, "signal_2": None},
                katalog=check_katalog,               # dev-2s GLOBALER Katalog-Check lehnt human-only-Felder fail-closed ab
                bindung=bindung)   # Auflage T (Stille-Null-Klasse) — die KI liefert Werte auch mal als JSON-String
            # `beleg` geht mit in die Antwort: die Oberfläche soll neben jedem Wert zeigen können,
            # aus welchem Satzteil er stammt — das ist der Unterschied zwischen "bestätige 62000"
            # und "bestätige 62000, weil du sagtest: 62000 Euro brutto verdient".
            # Dazu die Anzeige-Metadaten, damit die Verstanden-Seite eine Frage und einen lesbaren
            # Wert zeigen kann statt feld_id und Cent-Rohwert.
            geschrieben.append({"feld_id": fid, "event_id": ev["event_id"], "wert": v["wert"],
                                "beleg": v.get("beleg", ""),
                                **_anzeige_metadaten(fid, bindung)})
        except (ValueError, KeyError) as e:
            abgelehnt.append(fid)                    # Katalog/Auflage-A/F2-Abweisung → still überspringen, Rest gilt
            if fid:
                abgelehnt_gruende[fid] = str(e)       # NEU: Grund (kein Wert/Freitext enthalten, PII-frei)
    speichere_fall(fall_id, store)
    _abg = [a for a in abgelehnt if a]
    if _abg:                                         # Security-Observability (feld_ids, KEIN Wert/Freitext = PII-frei):
        sys.stderr.write(f"[haut.chat] LLM-Vorschläge außerhalb Katalog abgelehnt: {sorted(set(_abg))}\n")
    return 200, {"vorschlaege": geschrieben, "abgelehnt": _abg, "abgelehnt_gruende": abgelehnt_gruende,
                 "konflikte": konflikte,
                 # Die Antwort-Hälfte. Leer, wenn der Nutzer nichts gefragt hat — dann zeigt die
                 # Oberfläche nur die Vorschläge. `unsicher` sagt, ob das Modell die Antwort selbst
                 # für nicht gesichert hält; das gehört sichtbar zum Nutzer, sonst liest sich eine
                 # Vermutung wie eine Auskunft.
                 "antwort": erg["antwort"], "unsicher": erg["unsicher"],
                 "hinweis": "Vorschläge erfasst — bitte jeden einzeln bestätigen (die KI setzt nichts)."}


def _wert_klartext(fid: str, wert, bindung: dict) -> str:
    """Ein gespeicherter Wert so, wie ein Mensch ihn liest — für den Erklär-Kontext ans Modell.

    Die Speicherform ist an drei Stellen irreführend, und alle drei erzeugen falsche Erklärungen:
    6200000 liest ein Modell als sechs Millionen, `zusammen` sagt ihm nichts, und `kein_kap=true`
    heißt das GEGENTEIL dessen, was die zugehörige Frage stellt. Dieselben drei Regeln wie in der
    Oberfläche (verstandenWertText in app.js)."""
    b = bindung.get(fid) or {}
    typ = b.get("typ")
    if typ == "cent" and isinstance(wert, (int, float)):
        return f"{wert / 100:.2f} EUR".replace(".", ",")
    if typ == "bool":
        ja = (not wert) if fid.startswith("kein_") else bool(wert)
        return "ja" if ja else "nein"
    if typ == "enum":
        return (ENUM_LABELS.get(fid) or {}).get(wert, str(wert))
    return f"{wert} {b['einheit']}" if b.get("einheit") else str(wert)


# So viele bestätigte Angaben gehen höchstens in den Erklär-Kontext. Julius wollte, dass die KI
# „schon Sachen mit in Betracht zieht, die der Nutzer bereits geantwortet hat" — aber ein voller
# Fall hat über 200 Felder, und der Prompt ist ohnehin die teuerste Zeile im Betrieb.
_ERKLAER_KONTEXT_MAX = 40


def _erklaer_kontext(store: dict, bindung: dict, fid: str | None) -> str:
    """Kontext für eine Nachfrage: das Feld, seine Kurzhilfe, sein Zitatanker — und was der Nutzer
    schon bestätigt hat. Reiner Text; die PII-Filterung passiert im ausgehenden Pfad (api_llm)."""
    teile = []
    b = (bindung.get(fid) or {}) if fid else {}
    if b:
        teile.append(f"Die Frage, um die es geht: „{b.get('fragetext_laie', '')}“")
        if b.get("hilfe_kurz"):
            teile.append(f"Dazu gehört laut Feldbeschreibung: {b['hilfe_kurz']}")
        a = b.get("anker_ref") or {}
        if a.get("quelle") or a.get("zitatanker"):
            # Der Zitatanker ist der einzige Teil des Kontexts, der Gesetzestext IST statt ihn zu
            # umschreiben. Er ist der Grund, warum die Antwort mehr sein kann als Allgemeinwissen.
            teile.append(f"Wörtlicher Gesetzestext dazu — {a.get('quelle', '')}: "
                         f"„{a.get('zitatanker', '')}“")
    bestaetigt = [(f, e) for f, e in ST._aktives(store).items()
                  if e.get("zustand") == "bestaetigt"]
    if bestaetigt:
        zeilen = [f"- {(bindung.get(f) or {}).get('fragetext_laie', f)} "
                  f"→ {_wert_klartext(f, e['wert'], bindung)}"
                  for f, e in bestaetigt[:_ERKLAER_KONTEXT_MAX]]
        teile.append("Das hat der Nutzer bereits bestätigt:\n" + "\n".join(zeilen))
    return "\n".join(teile)


# ----------------------------------------------------------------- P8.3 Health / Ready

def health() -> tuple:
    """GET /health — Lebendtest, keine Abhängigkeiten."""
    return 200, {"status": "ok"}


def ready() -> tuple:
    """GET /ready — prüft, ob Store-Pfad erreichbar ist."""
    if not os.path.isdir(FAELLE):
        return 503, {"status": "not_ready", "detail": "faelle-verzeichnis fehlt"}
    return 200, {"status": "ok"}
