"""Paket-B-Durchstich über HTTP — fährt tests/test_paket_a_e2e.py Schritt für Schritt über die
Haut-Endpunkte nach (gleiche EP-Familie, gleiche Asserts, gleiche 2156). Deterministisch, NULL LLM.

Auflagen (Instructor): (A) POST …/chat -> 501, nie 200-Fake; (B) Server bindet AUSSCHLIESSLICH
127.0.0.1 (asserted); (C) api_schema/*.json wird gegen die echten Responses validiert. Der
numerische Teil (Spanne, 2156) hängt an der Catala-Toolchain -> sauberer Skip wie im A-Test.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402
import store as ST       # noqa: E402

jsonschema = pytest.importorskip("jsonschema")

SCHEMA_DIR = os.path.join(ROOT, "produkt", "haut", "api_schema")
EP_FELDER = {"ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz"}


def _schema(name: str) -> dict:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        return json.load(f)


def _val(name: str, obj: dict) -> None:
    jsonschema.Draft202012Validator(_schema(name)).validate(obj)


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _req(base: str, method: str, path: str, body: dict | None = None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _vorl(fld, w):
    # Generischer VORLÄUFIG-Fixture-Writer (ui:laie): ein Nutzer-Entwurf ohne signal_2 (noch nicht bestätigt).
    # NICHT llm:chat — der K1-Feld-Katalog lässt llm: nur suggestible Felder setzen; diese Fixtures schreiben
    # nicht-suggestible Kegel-Felder (ep_arbeitstage) → ui:laie ist der katalog-freie generische vorläufig-Kanal.
    return {"feld_id": fld, "wert": w, "zustand": "vorlaeufig",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": None}}


@pytest.fixture
def base(tmp_path, monkeypatch):
    # Fall-Daten in ein temporäres Verzeichnis (nie ins Repo, nie in den echten faelle/-Ordner)
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)                      # port=0 -> freier Port; SINGLE-THREADED (kein Request-Thread-Leak)
    assert srv.server_address[0] == "127.0.0.1", "Auflage B: Server muss an 127.0.0.1 binden"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()        # Socket sauber schließen (kein FD-Leak über viele e2e-Tests)


def test_bindet_nur_localhost():
    """Auflage B, hart: die Bind-Adresse ist 127.0.0.1, niemals 0.0.0.0."""
    srv = SRV.make_server(0)
    try:
        assert srv.server_address[0] == "127.0.0.1"
    finally:
        srv.server_close()


def test_chat_501(base):
    """Auflage A: POST /chat liefert 501 mit erklärendem Vertrag, NIE 200."""
    _req(base, "POST", "/fall", {"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": "c1"})
    st, b = _req(base, "POST", "/fall/c1/chat", {"text": "hallo"})
    assert st == 501, f"chat muss 501 sein, war {st}"
    assert "vertrag" in b and "stufe" in b
    assert b.get("fehler") == "not_implemented"


def test_entfernung_kein_key_fallback(base, monkeypatch):
    """Julius-Feature Maps-km: OHNE $ORS_API_KEY (oder Netzfehler) → sauberer 503-Fallback auf manuelle
    Eingabe, NIE Crash, NIE Fake-km. Kein Live-Aufruf (der Client wirft OrsNichtVerfuegbar ohne Key)."""
    monkeypatch.delenv("ORS_API_KEY", raising=False)
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "ent1"})
    st, b = _req(base, "POST", "/fall/ent1/entfernung", {"von": "A-Str 1, Berlin", "nach": "B-Weg 2, Berlin"})
    assert st == 503, f"ohne Key muss der Fallback 503 sein, war {st}"
    assert b.get("fehler") == "unavailable" and "vertrag" in b


def test_entfernung_leere_adressen_400(base):
    """Ohne beide Adressen → 400 (kein Aufruf)."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "ent2"})
    st, _ = _req(base, "POST", "/fall/ent2/entfernung", {"von": "", "nach": "irgendwo"})
    assert st == 400


def test_entfernung_erfolg_provenance(base, monkeypatch):
    """Erfolg (ORS-Client gemockt, KEIN Live-Aufruf): der km-Wert kommt als VORLÄUFIGES herkunft=berechnet-
    Event ins Store (Badge „berechnet/maps", NICHT „selbst") → der Nutzer bestätigt (Zwei-Signal). K2:
    Provenienz je Wert gewahrt, nichts still gesetzt."""
    import sys, os
    sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))
    import ors_client
    monkeypatch.setattr(ors_client, "entfernung_km", lambda von, nach: 30)
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "ent3"})
    st, b = _req(base, "POST", "/fall/ent3/entfernung", {"von": "Musterstr 1, Berlin", "nach": "Beispielweg 2, Berlin"})
    assert st == 200 and b["km"] == 30 and b["herkunft"] == "berechnet"
    st, stand = _req(base, "GET", "/fall/ent3/stand")
    f = stand["felder"]["ep_entfernung_km"]
    assert f["wert"] == 30 and f["zustand"] == "vorlaeufig" and f["herkunft_badge"] == "berechnet"


def test_vorjahr_uebernahme(base):
    """Vorjahr-Haut-Naht: ein bestätigter, vorjahr-flagged Wert im Vorjahres-Fall wird als VORLÄUFIGER
    Vorschlag (herkunft=vorjahr) in den neuen Fall übertragen — der Nutzer bestätigt (Zwei-Signal)."""
    # Vorjahres-Fall: bruttoarbeitslohn bestätigt (vorjahr:vorschlag).
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2024, "fall_id": "vj"})
    st, _ = _req(base, "POST", "/fall/vj/event", _laie("bruttoarbeitslohn", 4000000))
    assert st == 201
    # Neuer Fall + Übernahme.
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "neu"})
    st, b = _req(base, "POST", "/fall/neu/vorjahr", {"vorjahr_fall_id": "vj"})
    assert st == 200 and b["uebernommen"] >= 1
    st, stand = _req(base, "GET", "/fall/neu/stand")
    f = stand["felder"]["bruttoarbeitslohn"]
    assert f["wert"] == 4000000 and f["zustand"] == "vorlaeufig" and f["herkunft_badge"] == "vorjahr"


def test_vorjahr_fehlender_fall_404(base):
    """Übernahme aus einem nicht existierenden Vorjahres-Fall → 404 (kein stiller No-Op)."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "neu2"})
    st, _ = _req(base, "POST", "/fall/neu2/vorjahr", {"vorjahr_fall_id": "gibtsnicht"})
    assert st == 404


def test_kontoauszug_csv_vorsorge_vorschlag(base):
    """Kontoauszug-Upload (CSV, det-Pfad, KEIN LLM): eine Vorsorge-Ausgabe (Rürup) → deterministische
    Kategorie → vor_rv_ausserhalb_lstb (§ 10, in der an_gesamt-Scheibe) als VORLÄUFIGER Vorschlag
    (herkunft=kontoauszug) → Nutzer bestätigt. Einnahmen (betrag ≥ 0) werden ignoriert."""
    _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025, "fall_id": "ka"})
    csv = ("datum;betrag;verwendungszweck\n"
           "15.03.2025;-1200,00;Ruerup-Rente Jahresbeitrag Basisrente\n"
           "01.03.2025;2500,00;Gehalt Arbeitgeber\n")
    st, b = _req(base, "POST", "/fall/ka/kontoauszug", {"format": "csv", "inhalt": csv})
    assert st == 200 and b["transaktionen"] == 2 and b["uebernommen"] == 1
    st, stand = _req(base, "GET", "/fall/ka/stand")
    f = stand["felder"]["vor_rv_ausserhalb_lstb"]
    assert f["wert"] == 120000 and f["zustand"] == "vorlaeufig" and f["herkunft_badge"] == "kontoauszug"


def test_kontoauszug_pdf_ungueltiges_base64_400(base):
    """PDF-Inhalt muss base64-kodiert sein (roher PDF-Text ist es nicht) → 400, nie Crash/Fake."""
    _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025, "fall_id": "kap"})
    st, b = _req(base, "POST", "/fall/kap/kontoauszug", {"format": "pdf", "inhalt": "%PDF-1.4 ..."})
    assert st == 400


def test_kontoauszug_json_liste(base):
    """JSON-Auszug = vorstrukturierte Transaktionsliste → derselbe det-Pfad."""
    _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025, "fall_id": "kaj"})
    tx = [{"datum": "2025-04-01", "betrag": -80000, "verwendungszweck": "Altersvorsorge Rürup"}]
    st, b = _req(base, "POST", "/fall/kaj/kontoauszug", {"format": "json", "inhalt": tx})
    assert st == 200 and b["uebernommen"] == 1


def test_gesamt_zusammen_beide_verdiener(base):
    """#4 Person-B-Ring: Ehepaar-beide-Verdiener (Zusammenveranlagung). A Bruttolohn 40000 (§19-Einkünfte
    38770) + B Bruttolohn 30000 (28770) → kombiniert 67540 in catala_gesamt(zusammen) → Splitting +
    doppelter § 10c → festzusetzende_est 10776 = 1077600 Cent (== catala_est_zusammen, handverifiziert)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "zv", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True,
                                              veranlagung="zusammen", bruttolohn_partner=3000000,
                                              person_b_idnr="12345678901"))
    st, erg = _req(base, "GET", "/fall/zv/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1077600 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_zusammen_partner_kegel_offen(base):
    """K2: Zusammenveranlagung mit unvollständigem Person-B-Kegel (Bruttolohn_partner fehlt) → kein
    halber Ehepaar-Bescheid (partner_kegel_offen)."""
    _gesamt_anlegen(base, "zvo", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True,
                                               veranlagung="zusammen", person_b_idnr="12345678901"))
    st, erg = _req(base, "GET", "/fall/zvo/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "partner_kegel_offen"


def test_gesamt_zusammen_person_b_vorsorge(base):
    """A.2 Person-B-Vorsorge (K2, behebt die frühere A.1-Über-tax): Zusammenveranlagung, BEIDE Ehegatten mit
    Vorsorge — A + B je Bruttolohn 40000, je VOR (AN 3500 + AG 3500) + je KV/PV (Basis 3200, § 10 Abs. 4 S. 4
    Durchbruch). Der gesamt-Ring zieht jetzt BEIDER Vorsorge ab (VOR additiv in die Summen-Slots, KV/PV je eigener
    Höchstbetrag) → festzusetzende_est 9798 = 979800 Cent, NIEDRIGER als nur-Person-A (1179000) und als der frühere
    partner_vorsorge_offen-Block (kein Bescheid). Belegt: korrekter Ehepaar-Bescheid statt Sperre."""
    catala = _catala_da()
    _gesamt_anlegen(base, "pbv", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True,
                                               veranlagung="zusammen", bruttolohn_partner=4000000,
                                               person_b_idnr="12345678901", vor_an=350000, vor_ag=350000,
                                               basis_kv_pv=320000))
    for feld, wert in [("vor_an_anteil_rv_partner", 350000), ("vor_ag_anteil_rv_partner", 350000),
                       ("basis_kv_pv_partner", 320000)]:
        st, _ = _req(base, "POST", "/fall/pbv/event", _laie(feld, wert))
        assert st == 201
    st, erg = _req(base, "GET", "/fall/pbv/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 979800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_zusammen_person_b_vorsorge_null_wie_a_only(base):
    """A.2 no-double-count / B-inert: Zusammenveranlagung mit NUR Person-A-Vorsorge (VOR 3500/3500 + KV/PV 3200),
    Person-B-Vorsorge-Felder ABSENT → Person B trägt 0 bei → festzusetzende_est 11790 = 1179000 Cent (= exakt der
    Person-A-only-Abzug, HÖHER als mit Person-B 979800). Belegt: absente Person-B-Felder addieren 0 — kein
    Doppelzählen von Person A, keine Phantom-Person-B-Vorsorge."""
    catala = _catala_da()
    _gesamt_anlegen(base, "pbn", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True,
                                               veranlagung="zusammen", bruttolohn_partner=4000000,
                                               person_b_idnr="12345678901", vor_an=350000, vor_ag=350000,
                                               basis_kv_pv=320000))
    st, erg = _req(base, "GET", "/fall/pbn/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1179000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_zusammen_person_b_24a(base):
    """A.2 § 24a-B (per Person): Zusammenveranlagung, Ehegatte 65+ (geburtsjahr_partner 1955) mit Bruttolohn 40000 →
    eigener Altersentlastungsbetrag (760, eigene Kohorte, Bemessung Bruttolohn-B) additiv → festzusetzende_est
    13598 = 1359800 Cent, NIEDRIGER als ohne § 24a-B (1383800). Belegt: § 24a wird pro Person mit eigener Kohorte
    gewährt (§ 24a S. 1), nicht nur für Person A."""
    catala = _catala_da()
    _gesamt_anlegen(base, "pb24", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True,
                                                veranlagung="zusammen", bruttolohn_partner=4000000,
                                                person_b_idnr="12345678901"))
    st, _ = _req(base, "POST", "/fall/pb24/event", _laie("geburtsjahr_partner", 1955))
    assert st == 201
    st, erg = _req(base, "GET", "/fall/pb24/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1359800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_alleinerziehend_konsistenz_offen(base):
    """§ 24b D-Fix (K2, Under-tax) Wiring: fam_alleinstehend=True BEI Zusammenveranlagung ist ein Widerspruch
    (§ 24b Abs. 1/3 verlangt „allein stehend", nicht zusammenveranlagt) → alleinerziehend_konsistenz_offen. Der
    § 24b-Entlastungsbetrag würde sonst still gewährt = Unter-Besteuerung. Fail-closed (dev-2s partner_check-Wiring)."""
    _gesamt_anlegen(base, "azk", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True,
                                               veranlagung="zusammen", bruttolohn_partner=3000000,
                                               person_b_idnr="12345678901"))
    st, _ = _req(base, "POST", "/fall/azk/event", _laie("fam_alleinstehend", True))
    assert st == 201
    st, erg = _req(base, "GET", "/fall/azk/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "alleinerziehend_konsistenz_offen"


def test_gesamt_einzel_alleinstehend_kein_block(base):
    """D-Fix Gegenprobe: fam_alleinstehend=True BEI Einzelveranlagung ist KORREKT (kein Widerspruch) → kein
    Konsistenz-Block, regulärer Bescheid (§ 24b greift legitim, hier ohne Kinder = 0). festzusetzende_est 6919 =
    691900 Cent (reiner Job 40000). Belegt: die Sperre trifft NUR den zusammen-Widerspruch, nicht die einzel-Seite."""
    catala = _catala_da()
    _gesamt_anlegen(base, "eas", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True))
    st, _ = _req(base, "POST", "/fall/eas/event", _laie("fam_alleinstehend", True))
    assert st == 201
    st, erg = _req(base, "GET", "/fall/eas/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 691900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_concurrent_ergebnis_kein_race(base):
    """K2-Concurrency-Beweis: die Haut feuert /stand + /ergebnis PARALLEL (Browser serialisiert XHRs nicht)
    und catala_runtime ist NICHT thread-safe (globaler max_decimals steuert die Money-Rundung). Der SINGLE-
    THREADED Server serialisiert die Handler → N parallele /ergebnis liefern IDENTISCH den korrekten est
    (kein Race-Wrong-Value). Mechanischer Beleg wie der Thread-Count-Beweis der Isolations-Härtung."""
    import concurrent.futures
    if not _catala_da():
        pytest.skip("Catala-Toolchain nicht verfügbar")
    # Reiner AN-Fall: Bruttolohn 40000 → § 19-Einkünfte 38770 → festzusetzende_est 6919 = 691900 Cent.
    _gesamt_anlegen(base, "conc", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True))

    def hol():
        st, erg = _req(base, "GET", "/fall/conc/ergebnis")
        return (erg["zahl_cent"], erg["grund"])

    with concurrent.futures.ThreadPoolExecutor(max_workers=12) as ex:
        ergebnisse = {f.result() for f in [ex.submit(hol) for _ in range(24)]}
    assert ergebnisse == {(691900, "bestaetigt")}, f"Concurrency-Race: uneinheitliche/falsche Ergebnisse {ergebnisse}"


def test_fail_closed_llm_kann_nicht_bestaetigen(base):
    """Der fail-closed-Store weist ein llm:-bestaetigt-Event ab — über HTTP -> 422, nie 201."""
    _req(base, "POST", "/fall", {"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": "f1"})
    boese = {"feld_id": "ep_arbeitstage", "wert": 220, "zustand": "bestaetigt",
             "herkunft": {"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
             "schreiber": "llm:chat", "signal": {"signal_1": None, "signal_2": "gefaelscht"}}
    st, b = _req(base, "POST", "/fall/f1/event", boese)
    assert st == 422, f"llm-bestaetigt muss abgewiesen werden, war {st}"
    assert "fail-closed" in b["fehler"]


def test_schema_gate_negativ():
    """Auflage C, Härtung: ein verfälschtes Objekt MUSS das Schema-Gate rot färben."""
    kaputt = {"fall_id": "x", "snapshot_id": "y", "fragen": [{"feld_id": "a", "typ": "UNBEKANNT"}]}
    with pytest.raises(jsonschema.ValidationError):
        _val("fragen", kaputt)


def test_durchstich_http(base):
    catala = _catala_da()
    fid = "e2e-ep"

    # 0) Fall anlegen
    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201 and b["fall_id"] == fid

    # 1) leerer Fall -> fragen == die 4 EP-Felder
    st, b = _req(base, "GET", f"/fall/{fid}/fragen")
    assert st == 200
    _val("fragen", b)                                          # Auflage C
    assert {q["feld_id"] for q in b["fragen"]} == EP_FELDER
    # Fragetexte kommen aus der Bindung (laienverständlich, kein §)
    assert all("§" not in (q["fragetext_laie"] or "") for q in b["fragen"])

    # 2) 3x laie-bestätigt + 1x laie-VORLÄUFIG (Nutzer-Entwurf; ep_arbeitstage ist human-only,
    #    kein KI/Beleg-Kanal darf es setzen — K1-Feld-Katalog, daher ui:laie-vorläufig statt llm:chat)
    for fld, w in [("ep_entfernung_km", 30), ("ep_eigenes_kfz", True), ("ep_oepnv_kosten", 0)]:
        st, b = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
        assert st == 201
        _val("event", b)
    st, vorl = _req(base, "POST", f"/fall/{fid}/event", _vorl("ep_arbeitstage", 220))
    assert st == 201
    llm_ev = vorl["event_id"]

    # nur das offene Feld bleibt in der Queue
    st, b = _req(base, "GET", f"/fall/{fid}/fragen")
    assert [q["feld_id"] for q in b["fragen"]] == ["ep_arbeitstage"]

    # 3) stand: arbeitstage vorläufig (Nutzer-Entwurf), Spanne offen (nur mit Engine numerisch)
    st, stand_a = _req(base, "GET", f"/fall/{fid}/stand")
    assert st == 200
    _val("stand", stand_a)
    assert stand_a["felder"]["ep_arbeitstage"]["herkunft_badge"] == "laie"   # selbst, noch vorläufig
    assert stand_a["felder"]["ep_entfernung_km"]["herkunft_badge"] == "laie"          # selbst bestätigt
    spanne_a = None
    if catala:
        iv = stand_a["intervall"]
        assert iv["max_cent"] - iv["min_cent"] > 0, "offener arbeitstage -> Spanne > 0"
        spanne_a = iv["max_cent"] - iv["min_cent"]

    # 4) FAIL-CLOSED vorher: Input-Kegel enthält vorlaeufig -> keine feste Zahl
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    assert st == 200
    _val("ergebnis", erg)
    assert erg["zahl_cent"] is None
    assert erg["grund"] == "input_kegel_nicht_bestaetigt"

    # 5) LLM-Wert via ZWEI-SIGNAL bestätigen (ersetzt das llm-Event)
    st, b = _req(base, "POST", f"/fall/{fid}/event", {
        "feld_id": "ep_arbeitstage", "wert": 220, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": llm_ev, "signal_2": "klick@beleg_arbeitstage"}, "ersetzt": llm_ev})
    assert st == 201

    # 6) stand: Spanne schrumpft auf Punkt (monoton)
    st, stand_b = _req(base, "GET", f"/fall/{fid}/stand")
    if catala:
        iv2 = stand_b["intervall"]
        spanne_b = iv2["max_cent"] - iv2["min_cent"]
        assert spanne_b < spanne_a and spanne_b == 0

    # 7) FAIL-CLOSED nachher: Kegel bestätigt -> echte Zahl (Naht-Einheit CENT: 215600 = 2156,00 €,
    #    wie test_paket_a_e2e nach der Einheiten-Konvention ad4e22b)
    st, erg2 = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg2)
    if catala:
        assert erg2["zahl_cent"] == 215600
        assert erg2["grund"] == "bestaetigt"
    else:
        assert erg2["zahl_cent"] is None and erg2["grund"] == "engine_unavailable"

    # Vorwärts-Trace/Justification bis anker_ref
    st, w = _req(base, "GET", f"/fall/{fid}/feld/ep_arbeitstage/warum")
    assert st == 200
    _val("warum", w)
    j = w["justification"]
    assert j["herkunft"]["herkunft"] == "laie"      # nach Bestätigung: laie-Beleg
    assert j["zustand"] == "bestaetigt" and j["signal"]["signal_2"]
    assert j["anker_ref"]["zitatanker"] == "jeden Arbeitstag"


# VOR-Summanden (3 Laien-Felder -> EIN signatur_slot gesamtbeitraege_inkl_ag) + GWG-bool.
VOR_FELDER = {"vor_an_anteil_rv": 3500000, "vor_ag_anteil_rv": 3500000, "vor_rv_ausserhalb_lstb": 0}
VOR_KZ = {"E2000401", "E2000801", "E2000601"}


def test_durchstich_n_vor_gwg(base):
    """Option A: Multi-Regel-Scheibe als Interview+Deklaration+Trace. KEIN Gesamt-Bescheid
    (ehrlich), EP behält seinen Teil-Ring, VOR-Summen-Konvention + GWG-bool durch die Haut."""
    catala = _catala_da()
    fid = "e2e-nvg"

    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "n_vor_gwg", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201 and b["scheibe"] == "n_vor_gwg"

    # 1) Interview-Queue enthält alle sechs Regel-Familien (askable Felder), laienverständlich
    st, b = _req(base, "GET", f"/fall/{fid}/fragen")
    assert st == 200
    _val("fragen", b)
    ids = {q["feld_id"] for q in b["fragen"]}
    assert EP_FELDER <= ids                                   # N/EP
    assert set(VOR_FELDER) <= ids                             # VOR (3 Summanden getrennt abgefragt)
    assert {"gwg_netto_ohne_vorsteuer", "gwg_anschaffungskosten_netto"} <= ids   # GWG
    assert any(q["feld_id"].startswith("dhf_") for q in b["fragen"])   # dHf
    assert all("§" not in (q["fragetext_laie"] or "") for q in b["fragen"])

    # 2) Repräsentative Bestätigung: EP (4) + VOR-Split (3) + GWG (bool + cent)
    for fld, w in [("ep_arbeitstage", 220), ("ep_entfernung_km", 30),
                   ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", True)]:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
        assert st == 201
    for fld, w in VOR_FELDER.items():
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
        assert st == 201
    for fld, w in [("gwg_netto_ohne_vorsteuer", True), ("gwg_anschaffungskosten_netto", 60000)]:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
        assert st == 201

    # 3) stand: KEIN Gesamt-Bescheid (K2), EP-Teil-Ring ehrlich getrennt
    st, stand = _req(base, "GET", f"/fall/{fid}/stand")
    assert st == 200
    _val("stand", stand)
    assert stand["intervall"] is None, "Multi-Regel-Scheibe darf keinen Gesamt-Bescheid erfinden"
    if catala:
        assert stand["engine"] == "catala_teilweise"
        assert any(t["familie"] == "ep_werbungskosten" for t in stand["teil_ringe"])
        ep_ring = next(t for t in stand["teil_ringe"] if t["familie"] == "ep_werbungskosten")
        assert ep_ring["intervall"]["min_cent"] == ep_ring["intervall"]["max_cent"] == 215600
    else:
        assert stand["engine"] == "unavailable"
        assert stand["teil_ringe"] == []

    # 4) ergebnis: bewusst KEINE Scheiben-Zahl (kein ehrlicher Gesamt-Accessor)
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    assert st == 200
    _val("ergebnis", erg)
    assert erg["zahl_cent"] is None
    assert erg["grund"] == "kein_scheiben_gesamtbescheid"

    # 5) Deklaration via est_mapping: die drei getrennt erfragten VOR-Felder sind deklariert
    st, dek = _req(base, "GET", f"/fall/{fid}/deklaration")
    assert st == 200
    kz = set(dek.get("deklaration", {}))
    assert VOR_KZ <= kz, f"VOR-Summanden-Kz fehlen in der Deklaration: {kz}"
    assert "E0203503" in kz                                   # ep_arbeitstage
    assert dek["vollstaendig"] is True                        # alle erfassten Felder bestätigt

    # 6) Trace bis anker_ref für ein VOR-Feld
    st, w = _req(base, "GET", f"/fall/{fid}/feld/vor_an_anteil_rv/warum")
    assert st == 200
    _val("warum", w)
    assert w["justification"]["signatur_slot"] == "gesamtbeitraege_inkl_ag"   # Summen-Slot


# ---- Gesamtsteuer-Ring MVP (an_gesamt): erster echter §2-Bescheid, reiner Arbeitnehmerfall ----
AN_GESAMT_VOR = ("vor_an_anteil_rv", "vor_ag_anteil_rv", "vor_rv_ausserhalb_lstb")
AN_GESAMT_DHF = ("dhf_unterkunftskosten_monat", "dhf_monate", "dhf_im_inland",
                 "dhf_beruflich_veranlasst", "dhf_eigener_hausstand",
                 "dhf_finanzielle_beteiligung", "dhf_keine_pflicht_dienstwohnung")
AN_GESAMT_PARTNER = ("bruttoarbeitslohn_partner", "person_b_idnr",
                     "vor_an_anteil_rv_partner", "vor_ag_anteil_rv_partner",
                     "vor_rv_ausserhalb_lstb_partner")
AN_GESAMT_VERPFLEGUNG = ("tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig",
                         "vpf_monate_am_ort", "vpf_keine_mahlzeitengestellung")
AN_GESAMT_KEGEL = [
    ("bruttoarbeitslohn", 4000000),   # 40000 € in Cent (Bindung typ:cent)
    ("veranlagung", "einzel"),
    ("ep_arbeitstage", 220), ("ep_entfernung_km", 30), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", True),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),   # reiner Pendler: keine VOR
    # reiner Pendler: keine dHf (Kosten 0 -> dHf-Abzug 0, Bedingungen egal aber bestätigt)
    ("dhf_unterkunftskosten_monat", 0), ("dhf_monate", 0), ("dhf_im_inland", True),
    ("dhf_beruflich_veranlasst", True), ("dhf_eigener_hausstand", True),
    ("dhf_finanzielle_beteiligung", True), ("dhf_keine_pflicht_dienstwohnung", True),
    # reiner Pendler: keine Verpflegung (alle Tage 0 → Verpflegungs-Abzug 0, Guard irrelevant)
    ("tage_24h", 0), ("tage_an_abreise", 0), ("tage_ueber_8h_eintaegig", 0),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
]


def _an_gesamt_anlegen(base, fid, kegel=AN_GESAMT_KEGEL):
    st, _ = _req(base, "POST", "/fall",
                 {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


def _store_append(fid, feld_id, wert, zustand="bestaetigt"):
    """Setzt ein Feld direkt im Fall-Store (simuliert Erfassung außerhalb des an_gesamt-Interviews) —
    für den Guard-Negativtest. Gleicher Prozess/FAELLE wie der Server."""
    s = API.lade_fall(fid)
    # ui:laie für BEIDE Zustände: der K1-Feld-Katalog lässt llm:chat nur suggestible Felder setzen;
    # am_anschaffungskosten (Guard-Trigger) ist human-only → generischer vorläufig-Kanal = ui:laie (kein Katalog).
    herk = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
    sig = {"signal_1": None, "signal_2": None if zustand == "vorlaeufig" else "ok"}
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand=zustand, herkunft=herk,
                    schreiber="ui:laie", signal=sig)
    API.speichere_fall(fid, s)


def test_an_gesamt_durchstich(base):
    """MVP-Durchstich: voller bestätigter Kegel → echte festzusetzende_est 662900 (=6629 €)."""
    catala = _catala_da()
    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025, "fall_id": "ag"})
    assert st == 201
    st, fr = _req(base, "GET", "/fall/ag/fragen")
    _val("fragen", fr)
    ids = {q["feld_id"] for q in fr["fragen"]}
    assert ({"bruttoarbeitslohn", "veranlagung", "kein_gewinn", "kein_kap", "kein_vuv",
             "kein_sonstige"} | set(EP_FELDER) | set(AN_GESAMT_VOR) | set(AN_GESAMT_DHF)
            | set(AN_GESAMT_PARTNER) | set(AN_GESAMT_VERPFLEGUNG)) == ids
    for feld, wert in AN_GESAMT_KEGEL:
        st, _ = _req(base, "POST", "/fall/ag/event", _laie(feld, wert))
        assert st == 201
    st, stand = _req(base, "GET", "/fall/ag/stand")
    _val("stand", stand)
    assert stand["ring_gesperrt"] is None
    st, erg = _req(base, "GET", "/fall/ag/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert stand["engine"] == "catala"
        assert stand["intervall"]["min_cent"] == stand["intervall"]["max_cent"] == 662900
        assert erg["zahl_cent"] == 662900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_an_gesamt_flag_guard(base):
    """kein_kap=false (Nutzer HAT Kapitalerträge) → Ring unzulässig, kein Fake-Bescheid."""
    kegel = [(f, (False if f == "kein_kap" else v)) for f, v in AN_GESAMT_KEGEL]
    _an_gesamt_anlegen(base, "ag_kap", kegel)
    st, erg = _req(base, "GET", "/fall/ag_kap/ergebnis")
    assert erg["zahl_cent"] is None
    assert erg["grund"] == "einkunftsart_nicht_ring_faehig"


def test_an_gesamt_vor_integration(base):
    """Stufe 1a: VOR (§ 10) echt gerechnet. Bruttolohn 40000 + EP 2156 + VOR (AN 3500 / AG 3500,
    Cent) → Sonderausgaben 3500 (nach Cap-vor-Kürzung) → festzusetzende_est 5570 = 557000 Cent
    (Golden B). Der AG-Anteil wird über den Store-Einzelfeld-Zugriff getrennt behandelt."""
    catala = _catala_da()
    kegel = [(f, w) for f, w in AN_GESAMT_KEGEL if not f.startswith("vor_")]
    kegel += [("vor_an_anteil_rv", 350000), ("vor_ag_anteil_rv", 350000), ("vor_rv_ausserhalb_lstb", 0)]
    _an_gesamt_anlegen(base, "ag_vi", kegel)
    st, erg = _req(base, "GET", "/fall/ag_vi/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 557000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_an_gesamt_am_guard_vorlaeufig(base):
    """Arbeitsmittel (noch kein Modell) bleibt im Guard: ein vorläufiges am-Feld > 0 sperrt (kein Fake)."""
    _an_gesamt_anlegen(base, "ag_am")
    _store_append("ag_am", "am_anschaffungskosten", 60000, zustand="vorlaeufig")
    st, stand = _req(base, "GET", "/fall/ag_am/stand")
    assert stand["engine"] == "gesperrt"
    assert stand["ring_gesperrt"] == "werbungskosten_nicht_ring_faehig"
    st, erg = _req(base, "GET", "/fall/ag_am/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "werbungskosten_nicht_ring_faehig"


def _dhf_kegel(kosten, im_inland=True, weglassen=()):
    """Voller an_gesamt-Kegel mit dHf-Kosten > 0 (Miete 1400 € = 140000 ct, 12 Monate); Bedingungen
    bestätigt-true außer den in `weglassen` genannten (die bleiben offen)."""
    kegel = [(f, w) for f, w in AN_GESAMT_KEGEL if not f.startswith("dhf_")]
    kegel += [("dhf_unterkunftskosten_monat", kosten), ("dhf_monate", 12), ("dhf_im_inland", im_inland)]
    for b in ("dhf_beruflich_veranlasst", "dhf_eigener_hausstand",
              "dhf_finanzielle_beteiligung", "dhf_keine_pflicht_dienstwohnung"):
        if b not in weglassen:
            kegel.append((b, True))
    return kegel


def test_an_gesamt_dhf_ring(base):
    """Stufe 1b: dHf echt gerechnet. EP 2156 + dHf (1400 → gekappt 1000 × 12 = 12000) → WK 14156
    → festzusetzende_est 3143 = 314300 Cent (Golden an_2025_einzel_ep_dhf)."""
    catala = _catala_da()
    _an_gesamt_anlegen(base, "ag_dhf", _dhf_kegel(140000))
    st, erg = _req(base, "GET", "/fall/ag_dhf/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 314300 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_an_gesamt_dhf_tatbestand_offen(base):
    """K2: dHf-Kosten > 0, aber eine Geltungsbedingung nie bestätigt → Ring gesperrt (kein Abzug
    ohne Tatbestand, kein 3143-Fake)."""
    _an_gesamt_anlegen(base, "ag_to", _dhf_kegel(140000, weglassen=("dhf_keine_pflicht_dienstwohnung",)))
    st, erg = _req(base, "GET", "/fall/ag_to/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "dhf_tatbestand_offen"


def test_an_gesamt_dhf_ausland(base):
    """Ausland-dHf ist benannte Lücke MIT Guard: im_inland=false + Kosten > 0 → Ring gesperrt."""
    _an_gesamt_anlegen(base, "ag_ausl", _dhf_kegel(140000, im_inland=False))
    st, erg = _req(base, "GET", "/fall/ag_ausl/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "ausland_dhf_nicht_ring_faehig"


def _verpflegung_kegel(monate=2, keine_mahlzeit=True, tage_24h=10):
    """an_gesamt-Kegel mit Verpflegungs-Reisetagen. Guard-Felder werden nur gesetzt, wenn nicht None
    (None simuliert den UNSET-Fall für den fail-closed-Test)."""
    kegel = [(f, w) for f, w in AN_GESAMT_KEGEL if f != "tage_24h"]
    kegel.append(("tage_24h", tage_24h))
    if monate is not None:
        kegel.append(("vpf_monate_am_ort", monate))
    if keine_mahlzeit is not None:
        kegel.append(("vpf_keine_mahlzeitengestellung", keine_mahlzeit))
    return kegel


def test_an_gesamt_verpflegung_ring(base):
    """Stufe 1b: Verpflegung echt gerechnet. EP 2156 + Verpflegung 280 (10 volle Tage à 28) → WK 2436
    → festzusetzende_est 6542 = 654200 Cent. Reduktion explizit safe (≤3 Monate, keine Mahlzeiten)."""
    catala = _catala_da()
    _an_gesamt_anlegen(base, "vpf", _verpflegung_kegel())
    st, erg = _req(base, "GET", "/fall/vpf/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 654200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_an_gesamt_verpflegung_reduktion_unset(base):
    """fail-closed-on-unset (Instructor-Härtung): Reisetage > 0, aber die Reduktions-Fragen
    (3-Monats-Frist / Mahlzeitenkürzung) UNBEANTWORTET → Ring gesperrt, kein stiller Über-Abzug."""
    _an_gesamt_anlegen(base, "vpu", _verpflegung_kegel(monate=None, keine_mahlzeit=None))
    st, erg = _req(base, "GET", "/fall/vpu/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "verpflegung_reduktion_offen"


def _zusammen_kegel(vor_a=0, ohne=()):
    """Zusammenveranlagung-Kegel: einzel-Basis mit veranlagung=zusammen, KEINE EP (WK_a=0),
    beide 40000 → festzusetzende_est_zusammen 13838. + Partner-Pflichtfelder (außer `ohne`)."""
    k = dict(AN_GESAMT_KEGEL)
    k["veranlagung"] = "zusammen"
    k["ep_arbeitstage"] = 0
    k["ep_entfernung_km"] = 0
    k["ep_eigenes_kfz"] = False
    k["vor_an_anteil_rv"] = vor_a
    kegel = list(k.items())
    for f, w in [("bruttoarbeitslohn_partner", 4000000), ("person_b_idnr", "00000000000")]:
        if f not in ohne:
            kegel.append((f, w))
    return kegel


def test_an_gesamt_zusammen(base):
    """Front 2: Splitting-Ring. Beide Bruttolohn 40000, keine WK/VOR → festzusetzende_est_zusammen
    13838 = 1383800 Cent (Golden an_2025_zusammen_gleich). §9a je Person + Splitting IM Scope."""
    catala = _catala_da()
    _an_gesamt_anlegen(base, "zus", _zusammen_kegel())
    st, erg = _req(base, "GET", "/fall/zus/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1383800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_an_gesamt_zusammen_partner_offen(base):
    """K2: Person-B-Pflichtfeld (person_b_idnr) offen → kein halber Splitting-Bescheid."""
    _an_gesamt_anlegen(base, "zpo", _zusammen_kegel(ohne=("person_b_idnr",)))
    st, erg = _req(base, "GET", "/fall/zpo/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "partner_kegel_offen"


def test_an_gesamt_zusammen_vor_guard(base):
    """MVP-zusammen ohne VOR: ein VOR-Feld (A oder B) > 0 → Ring gesperrt (kein VOR-loser
    Splitting-Bescheid, wenn VOR-Daten vorliegen)."""
    _an_gesamt_anlegen(base, "zvg", _zusammen_kegel(vor_a=350000))
    st, erg = _req(base, "GET", "/fall/zvg/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "partner_vor_offen"


def _gesamt_kegel(einnahmen, afa=0, schuldzinsen=0, kein_vuv=False, bruttolohn=0,
             ep_tage=0, ep_km=0, ep_kfz=False, kein_kap=True, kap_ertraege=0,
             kap_gewinn_aktien=0, kap_verlust_aktien=0, kap_gewinn_sonstige=0, kap_verlust_sonstige=0,
             veranlagung="einzel", bruttolohn_partner=None, person_b_idnr=None,
             kap_ertraege_partner=0, kap_gewinn_aktien_partner=0, kap_verlust_aktien_partner=0,
             kap_gewinn_sonstige_partner=0,
             kap_verlust_sonstige_partner=0, entgelt_quote=100, vor_an=0, vor_ag=0, vor_rv_ausserhalb=0,
             basis_kv_pv=0, weitere_kv_pv=0, mit_anspruch_zuschuss=False, gewinn=0, kein_gewinn=True,
             betriebseinnahmen=0, sonstige_betriebsausgaben=0, afa_jahresbetrag=0, betriebsart=None, gwg=None, vg=0,
             gewst_messbetrag=0, gewst_hebesatz=0, verlustvortrag_bestand=0,
             gewinnanteil=0, verg_taetigkeit=0, verg_darlehen=0, verg_ueberlassung=0,
             geburtsjahr=0, antrag_erm=False, berufsunfaehig=False, einmal_genutzt=False):
    """gesamt-Kegel (§ 19 + § 21 + § 20): § 21 (Einnahmen/WK) + § 19 (Bruttolohn in Cent + EP) + § 20
    Kapital (E0121709-Aggregat ODER Aktien/sonstige-Töpfe, in Cent) — je 0 = Einkunftsart abwesend
    (bestätigte Null) — + veranlagung + Flags. kein_vuv=false wenn V+V vorhanden, kein_kap=false wenn Kapital.
    bruttolohn_partner/person_b_idnr (nur bei veranlagung=zusammen) = Person-B-§19-Kegel (#4). entgelt_quote
    (§ 21 Abs. 2, Pflicht-Kegel, %) = 100 (nicht verbilligt) default; < 66 → WK-Kürzung. vor_an/vor_ag/
    vor_rv_ausserhalb (§ 10 Altersvorsorge, Pflicht-Kegel, cent) = 0 default (keine Vorsorge → kein Abzug).
    basis_kv_pv/weitere_kv_pv (§ 10 Abs. 1 Nr. 3/3a KV/PV, Pflicht-Kegel, CENT wie VOR!) = 0 default; 3200 € = 320000.
    gewinn (§§ 13-18 Gewinneinkünfte, Stufe 1, einkuenfte_gewinn, OPTIONAL/CENT) = 0 default → Feld absent (absent → 0,
    over-tax-safe); > 0 nur mit kein_gewinn=False (sonst flag_konsistenz_offen). kein_gewinn (§ 2 Abs. 1 Nr. 1-3
    Abwesenheits-Flag) = True default (keine Gewinneinkünfte); für einen echten Gewinnfall auf False setzen."""
    k = [("vv_einnahmen", einnahmen), ("vv_gebaeude_afa", afa), ("vv_schuldzinsen", schuldzinsen),
         ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0),
         ("vv_entgelt_quote_prozent", entgelt_quote), ("veranlagung", veranlagung),
         ("bruttoarbeitslohn", bruttolohn),
         ("vor_an_anteil_rv", vor_an), ("vor_ag_anteil_rv", vor_ag),
         ("vor_rv_ausserhalb_lstb", vor_rv_ausserhalb),
         ("basis_kv_pv", basis_kv_pv), ("weitere_vorsorgeaufwendungen", weitere_kv_pv),
         ("mit_anspruch_auf_zuschuss", mit_anspruch_zuschuss),
         ("ep_arbeitstage", ep_tage), ("ep_entfernung_km", ep_km),
         ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", ep_kfz),
         ("kap_kapitalertraege", kap_ertraege), ("kap_gewinn_aktien", kap_gewinn_aktien),
         ("kap_verlust_aktien", kap_verlust_aktien), ("kap_gewinn_sonstige", kap_gewinn_sonstige),
         ("kap_verlust_sonstige", kap_verlust_sonstige), ("kap_zusammenveranlagung", False),
         ("kein_gewinn", kein_gewinn), ("kein_kap", kein_kap), ("kein_vuv", kein_vuv), ("kein_sonstige", True)]
    if gewinn:                                     # §§ 13-18 Stufe 1: einkuenfte_gewinn nur wenn > 0 (OPTIONAL, absent → 0)
        k.append(("einkuenfte_gewinn", gewinn))
    for _fid, _w in (("betriebseinnahmen", betriebseinnahmen),           # § 4 Abs. 3 EÜR-Komponenten (2a, cent, optional)
                     ("sonstige_betriebsausgaben", sonstige_betriebsausgaben),
                     ("afa_jahresbetrag", afa_jahresbetrag)):
        if _w:
            k.append((_fid, _w))
    if betriebsart is not None:                    # gewinn_betriebsart-Weiche (land_forst-Guard-Test)
        k.append(("gewinn_betriebsart", betriebsart))
    for _i, _netto in enumerate(gwg or [], start=1):   # § 6 Abs. 2 GWG-Assets (Liste netto-Cent): Instanz 1 = Basis, 2..N = __n
        k.append(("gwg_anschaffungskosten_netto" if _i == 1 else f"gwg_anschaffungskosten_netto__{_i}", _netto))
    if vg:                                          # § 16 Veräußerungsgewinn im gesamt-Ring (Non-Rentner-§16-vg, REUSE-Feld)
        k.append(("rentner_veraeusserungsgewinn", vg))
    if gewst_messbetrag:                            # § 35 GewSt-Anrechnung (S1, opt-in): Messbetrag (cent) + Hebesatz (%)
        k.append(("gewst_messbetrag", gewst_messbetrag))
    if gewst_hebesatz:
        k.append(("gewst_hebesatz", gewst_hebesatz))
    if verlustvortrag_bestand:                      # § 10d Abs. 2 Verlustvortrag (opt-in, cent)
        k.append(("verlustvortrag_bestand", verlustvortrag_bestand))
    for _mf, _mv in (("gewinnanteil", gewinnanteil), ("verguetung_taetigkeit", verg_taetigkeit),  # § 15 Abs. 1 Nr. 2 Mitunternehmer (cent, opt.)
                     ("verguetung_darlehen", verg_darlehen), ("verguetung_ueberlassung", verg_ueberlassung)):
        if _mv:                                     # gewinnanteil kann NEGATIV (§15a-ausgleichsfähiger Verlustanteil)
            k.append((_mf, _mv))
    for _af, _av in (("antrag_ermaessigter_satz", antrag_erm), ("dauernd_berufsunfaehig", berufsunfaehig),  # § 34 Abs. 3 Chooser-Flags
                     ("ermaessigung_einmal_genutzt", einmal_genutzt)):
        if _av:
            k.append((_af, _av))
    if geburtsjahr:                                  # § 34 Abs. 3 Alter≥55-DERIVE (auch § 24a)
        k.append(("geburtsjahr", geburtsjahr))
    if bruttolohn_partner is not None:
        k.append(("bruttoarbeitslohn_partner", bruttolohn_partner))
    if person_b_idnr is not None:
        k.append(("person_b_idnr", person_b_idnr))
    if veranlagung == "zusammen":                 # Person-B-Kapital-Kegel (bestätigte Null default, #4b)
        k += [("kap_kapitalertraege_partner", kap_ertraege_partner),
              ("kap_gewinn_aktien_partner", kap_gewinn_aktien_partner),
              ("kap_gewinn_sonstige_partner", kap_gewinn_sonstige_partner),
              ("kap_verlust_aktien_partner", kap_verlust_aktien_partner),
              ("kap_verlust_sonstige_partner", kap_verlust_sonstige_partner)]
    return k


def _gesamt_anlegen(base, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


def test_gesamt_vermieter(base):
    """Front V+V: reiner Vermieter-Fall (Bruttolohn = bestätigte Null). § 21-Einkünfte 30000
    (Einnahmen − WK) → catala_gesamt → festzusetzende_est 4293 = 429300 Cent (§ 10c-Pauschbetrag 36
    abgezogen; vermieter_only-Golden)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "vv", _gesamt_kegel(3000000))   # 30000 € in Cent, kein Job
    st, erg = _req(base, "GET", "/fall/vv/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 429300 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_verlust(base):
    """K2: § 21-Verlust (WK > Einnahmen: 8000 − 6000 − 4000 = −2000) → festzusetzende_est 0,
    NIE negativ (keine Negativsteuer)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "vvl", _gesamt_kegel(800000, afa=600000, schuldzinsen=400000))
    st, erg = _req(base, "GET", "/fall/vvl/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 0 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_flag_widerspruch(base):
    """K2: kein_vuv=true (behauptet keine V+V) UND vv_einnahmen > 0 bestätigt → Widerspruch surfacen,
    keine still übergangene Einkunftsart."""
    _gesamt_anlegen(base, "vvw", _gesamt_kegel(3000000, kein_vuv=True))
    st, erg = _req(base, "GET", "/fall/vvw/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "flag_konsistenz_offen"


@pytest.mark.parametrize("gewinn_cent,erwartet_cent", [
    (3000000, 429300),    # Gewinn 30000 € → est 4293 (identisch zum reinen Vermieter 30000: plain § 2-Summand)
    (5000000, 1067800),   # Gewinn 50000 € → est 10678
    (8000000, 2267200),   # Gewinn 80000 € → est 22672
])
def test_gesamt_gewinn_only(base, gewinn_cent, erwartet_cent):
    """§§ 13-18 Gewinneinkünfte (Stufe 1) im gesamt-Ring: der vorberechnete Gewinn-Betrag fließt als
    einkuenfte_gewinn-Summand in die § 2-Summe (kein Job, keine V+V) → festzusetzende_est. kein_gewinn=False
    (es LIEGT Gewinn vor). Belegt: der Ring besteuert §§ 13-18-Gewinn (vorher stiller 0 — der Slot wurde nie
    gesetzt). Werte unabhängig gegen catala_gesamt verifiziert."""
    catala = _catala_da()
    fid = f"gew{gewinn_cent}"
    _gesamt_anlegen(base, fid, _gesamt_kegel(0, kein_vuv=True, gewinn=gewinn_cent, kein_gewinn=False))
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == erwartet_cent and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_gewinn_und_job(base):
    """Konvergenz § 19 + §§ 13-18: Arbeitnehmer (Bruttolohn 40000 → § 19-Einkünfte 38770) MIT Gewinn 30000 →
    catala_gesamt summiert (§ 2 Abs. 3, GdE 68770) → festzusetzende_est 17956 = 1795600 Cent. Belegt: der
    Gewinn ADDIERT sich zum Lohn (nicht ersetzt/verschluckt)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "gwj", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True,
                                               gewinn=3000000, kein_gewinn=False))
    st, erg = _req(base, "GET", "/fall/gwj/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1795600 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_gewinn_flag_widerspruch(base):
    """K2 (Guard non-vacuous): kein_gewinn=true (behauptet keine Gewinneinkünfte) UND einkuenfte_gewinn > 0
    bestätigt → Widerspruch surfacen (flag_konsistenz_offen), keine still übergangene Einkunftsart. Spiegel zu
    test_gesamt_flag_widerspruch (V+V); belegt den flag_check-Fix kein_gewinn → [einkuenfte_gewinn]."""
    _gesamt_anlegen(base, "gwf", _gesamt_kegel(0, kein_vuv=True, gewinn=3000000, kein_gewinn=True))
    st, erg = _req(base, "GET", "/fall/gwf/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "flag_konsistenz_offen"


def test_gesamt_euer_gewinn(base):
    """§ 4 Abs. 3 EÜR (Stufe 2a) im gesamt-Ring: Gewinn KOMPONENTENWEISE statt vorberechnet — Betriebseinnahmen
    100000 − (sonstige BA 30000 + AfA 20000) = Gewinn 50000 → catala_euer_gewinn → einkuenfte_gewinn 50000 →
    festzusetzende_est 10678 = 1067800 Cent (identisch zum direkten Gewinn 50000 aus Stufe 1 — die EÜR ist nur ein
    anderer Eingabepfad). kein_gewinn=False. Werte unabhängig gg. catala_euer_gewinn + catala_gesamt."""
    catala = _catala_da()
    _gesamt_anlegen(base, "eu1", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False,
                    betriebseinnahmen=10000000, sonstige_betriebsausgaben=3000000, afa_jahresbetrag=2000000))
    st, erg = _req(base, "GET", "/fall/eu1/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1067800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_mitunternehmer_gewinnanteil(base):
    """§ 15 Abs. 1 S. 1 Nr. 2 Mitunternehmer (#2): reiner Gewinnanteil 50000 (Beteiligung an PersG, betriebsart
    gewerbe) → catala_mitunternehmer_einkuenfte → einkuenfte_gewinn 50000 → festzusetzende_est 1067800 Cent
    (identisch zum direkten Gewinn/EÜR 50000 — Mitunternehmer ist nur eine weitere §15-Gewinnquelle). kein_gewinn=False."""
    catala = _catala_da()
    _gesamt_anlegen(base, "mu1", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False,
                    betriebsart="gewerbe", gewinnanteil=5000000))
    st, erg = _req(base, "GET", "/fall/mu1/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1067800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_mitunternehmer_sonderverguetungen(base):
    """§ 15 Abs. 1 S. 1 Nr. 2 S. 1: Gewinnanteil 30000 + die 3 SONDERVERGÜTUNGEN (Tätigkeit 12000 + Darlehen 3000
    + Überlassung 5000 = 20000) → einkuenfte_mitunternehmer 50000 → einkuenfte_gewinn 50000 → festzusetzende_est
    1067800 Cent, IDENTISCH zum reinen Gewinnanteil 50000 (mu1) — belegt die additive 4-Summanden-Formel (Sonder-
    vergütungen sind Teil der gewerblichen Einkünfte, § 15 Abs. 1 Nr. 2 S. 1)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "mu2", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, betriebsart="gewerbe",
                    gewinnanteil=3000000, verg_taetigkeit=1200000, verg_darlehen=300000, verg_ueberlassung=500000))
    st, erg = _req(base, "GET", "/fall/mu2/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1067800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_mitunternehmer_verlustanteil(base):
    """§ 15 Abs. 3 S. 2 / § 15a-BOUNDARY (non-vacuous): NEGATIVER Gewinnanteil −20000 (§15a-ausgleichsfähiger
    Verlust-Anteil, Feststellungsbescheid) + Tätigkeitsvergütung 12000 → einkuenfte_mitunternehmer −8000 → mindert
    via § 2 Abs. 3-Ausgleich den § 19-Lohn (60000 → § 19-Einkünfte 58770): GdE 58770 − 8000 = 50770 → festzusetzende_
    est 1095200 Cent, NIEDRIGER als ohne den Verlust-Anteil (der ausgleichsfähige −8000 senkt die GdE). Belegt: der
    ausgleichsfähige Verlust-Mitunternehmeranteil fließt roh in den §-2-Ausgleich (§15a-Beschränkung liegt IM Input)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "mu3", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, bruttolohn=6000000,
                    betriebsart="gewerbe", gewinnanteil=-2000000, verg_taetigkeit=1200000))
    st, erg = _req(base, "GET", "/fall/mu3/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1095200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_mitunternehmer_flag_widerspruch(base):
    """K2 (Guard non-vacuous, #2 JOINT): kein_gewinn=true (behauptet keine Gewinneinkünfte) UND gewinnanteil > 0
    (Mitunternehmer-Beteiligung § 15 Nr. 2) → Widerspruch surfacen (flag_konsistenz_offen), keine still übergangene
    §15-Einkunftsart. Belegt den flag_check-Fix FLAG_NEGIERT[kein_gewinn] += die 4 Mitunternehmer-Felder (dev-2)."""
    _gesamt_anlegen(base, "muw", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=True, gewinnanteil=5000000))
    st, erg = _req(base, "GET", "/fall/muw/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "flag_konsistenz_offen"


def test_gesamt_euer_verlust_durchfluss(base):
    """§ 4 Abs. 3 EÜR-VERLUST (§ 2 Abs. 3-Ausgleich, non-vacuous): Betriebseinnahmen 40000 − (sonstige BA 45000 +
    AfA 10000) = Gewinn −15000 (Verlust) → catala_euer_gewinn gibt negativ durch → mindert den § 19-Lohn (Job
    40000 → § 19-Einkünfte 38770): GdE 38770 − 15000 = 23770 → festzusetzende_est 2592 = 259200 Cent, NIEDRIGER
    als ohne den Verlust (§ 19-only 40000 → 13452). Belegt: der EÜR-Verlust fließt in den §-2-Ausgleich (kein Floor
    auf 0 auf Gewinn-Ebene, wie § 21)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "eu2", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, bruttolohn=4000000,
                    betriebseinnahmen=4000000, sonstige_betriebsausgaben=4500000, afa_jahresbetrag=1000000))
    st, erg = _req(base, "GET", "/fall/eu2/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 259200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_gewinn_quelle_offen(base):
    """K2 fail-closed (Guard non-vacuous, 2a): direkter einkuenfte_gewinn (30000) UND EÜR-Komponente
    (betriebseinnahmen 100000) BEIDE gesetzt → Doppelquelle, welcher laufende Gewinn gilt? → gewinn_quelle_offen
    (kein Rate-Bescheid, _laufender_gewinn nähme sonst still die EÜR und verschluckte den Direktwert). Spiegel
    kapital_semantik_offen."""
    _gesamt_anlegen(base, "gqo", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False,
                    gewinn=3000000, betriebseinnahmen=10000000))
    st, erg = _req(base, "GET", "/fall/gqo/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "gewinn_quelle_offen"


def test_gesamt_luf_euer_offen(base):
    """K2 fail-closed (Guard non-vacuous, 2a Q4): gewinn_betriebsart=land_forst MIT EÜR-Komponente
    (betriebseinnahmen 80000) UND ohne Direktwert → luf_euer_offen. § 13-LuF ist NICHT EÜR-materialisiert
    (EuerGewinn-Bedingungen § 15 Abs. 2/§ 18 Abs. 1, nicht § 13; LuF hat § 13a Durchschnittssätze etc.) → NIE
    silent 0. land_forst + DIREKTwert bliebe erlaubt (einkunftsart-agnostisch)."""
    _gesamt_anlegen(base, "luf", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False,
                    betriebseinnahmen=8000000, betriebsart="land_forst"))
    st, erg = _req(base, "GET", "/fall/luf/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "luf_euer_offen"


def test_gesamt_gwg_multi(base):
    """§ 6 Abs. 2 GWG-Sofortabzug (Stufe 2b) im EÜR: DREI GWG-Assets à 400/600/800 € (alle ≤ 800, je sofort
    abziehbar) → Σ 1800 als Betriebsausgabe. Betriebseinnahmen 50000 − (sonstige BA 10000 + AfA 5000 + GWG-Σ 1800)
    = Gewinn 33200 → festzusetzende_est 5220 = 522000 Cent. Belegt: der Ring summiert die GWG-Instanzen (EM.instanzen
    ,gwg — Basis + __2 + __3) stumpf in den betriebsausgaben-Term."""
    catala = _catala_da()
    _gesamt_anlegen(base, "gwm", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, betriebseinnahmen=5000000,
                    sonstige_betriebsausgaben=1000000, afa_jahresbetrag=500000, gwg=[40000, 60000, 80000]))
    st, erg = _req(base, "GET", "/fall/gwm/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 522000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_gwg_ueber_800_ausgeschlossen(base):
    """§ 6 Abs. 2 PER-ASSET-Deckelung (non-vacuous): zwei „GWG" 400 € (≤ 800 → 400 abziehbar) + 1000 € (> 800 →
    0, KEIN GWG — muss über AfA). Σ-Sofortabzug = 400 (nicht 1400). Betriebseinnahmen 50000 − 400 = Gewinn 49600 →
    festzusetzende_est 10537 = 1053700 Cent. Belegt: der ≤ 800-Schwellwert greift JE ASSET (kein Zusammenzählen zu
    1400 dann Deckelung; genau der B-Over-Tax-Trap den Option A vermeidet)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "gw8", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False,
                    betriebseinnahmen=5000000, gwg=[40000, 100000]))
    st, erg = _req(base, "GET", "/fall/gw8/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1053700 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_gwg_only_verlust(base):
    """§ 6 Abs. 2 GWG als EÜR-present-Trigger + Verlust-Durchfluss: KEINE Betriebseinnahmen, nur zwei GWG à 800
    (Σ 1600) → Gewinn 0 − 1600 = −1600 (Anlaufverlust). Der GWG-Sofortabzug allein triggert den EÜR-Pfad (nicht nur
    betriebseinnahmen) → mindert den § 19-Lohn (Job 40000 → § 19 38770): GdE 37170 → festzusetzende_est 6419 =
    641900 Cent, NIEDRIGER als § 19-only. Belegt: GWG-only zählt zum EÜR-Trigger, Verlust fließt in § 2 Abs. 3."""
    catala = _catala_da()
    _gesamt_anlegen(base, "gwl", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, bruttolohn=4000000,
                    gwg=[80000, 80000]))
    st, erg = _req(base, "GET", "/fall/gwl/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 641900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


@pytest.mark.parametrize("vg_cent,erwartet_cent", [
    (4000000,  0),        # vg 40000 → FB 45000 > vg → netto_vg 0 → kein Fünftel (Guard netto_vg>0), kein Phantom-Verlust
    # ⚠ § 34 Abs. 1 S. 3 (verbleibendes zvE negativ ∧ zvE positiv) → 5×Tarif(zvE/5): 5×Tarif(54964//5=10992); 10992 < GfB
    # 12096 → Tarif 0 → 5×0 = 0. KEIN Under-tax-Bug — moderate Einmal-vg werden durch Fünftelung+Grundfreibetrag steuerfrei.
    (10000000, 0),        # vg 100000 → netto 55000 → § 34 Abs. 1 S. 3 → 0 (war progressiv 12495 = Over-tax-Korrektur)
    (15000000, 1304000),  # vg 150000 → netto 119000 → § 34 Abs. 1 S. 3 → 5×Tarif(23792)=13040 (war progressiv 39052)
    (18100000, 3065000),  # vg 181000 → netto 181000 → § 34 Abs. 1 S. 3 → 5×Tarif(36192)=30650 (war progressiv 65092)
])
def test_gesamt_veraeusserungsgewinn(base, vg_cent, erwartet_cent):
    """Non-Rentner-§16-vg im GESAMT-Ring (REUSE des generellen §16-vg-Felds rentner_veraeusserungsgewinn): ein
    Nicht-Rentner (gesamt-Scheibe, kein Renten-Kontext) mit § 16-Betriebsveräußerungsgewinn → netto nach § 16 Abs. 4-
    Freibetrag (identische Brackets wie rentner-2-I, da die § 2-Rechnung dieselbe ist) → einkuenfte_gewinn →
    festzusetzende_est. ROUTING-UNABHÄNGIGKEIT: der Fall wird als „gesamt" angelegt → via gesamt-slot_fn gerechnet
    (grund=bestaetigt), NICHT in die rentner-Scheibe misrouted (Routing ist Scheibe-fix, nicht vg-feld-getriggert).
    KEIN DOPPEL-COUNT: der exakte Erwartungswert IST die Single-Count-Assertion — bei Doppelzählung (netto ×2) käme
    ein ANDERER (höherer) Wert (z.B. vg 100000: einfach 1249500 vs doppelt 3527200), die Assertion würde brechen."""
    catala = _catala_da()
    fid = f"gvg{vg_cent}"
    _gesamt_anlegen(base, fid, _gesamt_kegel(0, kein_vuv=True, vg=vg_cent, kein_gewinn=False))
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == erwartet_cent and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_veraeusserung_plus_laufender(base):
    """§ 16 Abs. 1 Akkumulation im gesamt-Ring: laufender Gewinn (direkter einkuenfte_gewinn 30000) + § 16-Ver-
    äußerungsgewinn (vg 100000 → netto 55000 nach FB) fließen ADDITIV in DIESELBE § 2-Einkunftsart →
    einkuenfte_gewinn 85000. § 34 Abs. 1 S. 2 (ao=netto_vg 55000, verbleibendes zvE 29964>0): laufender 30000
    progressiv, nur die vg geglättet → festzusetzende_est 2097800 Cent (war voll-progressiv 2477200). Belegt: der
    gesamt-vg-Fold summiert mit dem laufenden Gewinn UND nur die vg kriegt Fünftel (Spiegel des rentner-Akkumulations-Locks)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "gvl", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=3000000, vg=10000000))
    st, erg = _req(base, "GET", "/fall/gvl/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 2097800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_fuenftel_zve_null_skip(base):
    """§ 34 Abs. 1 GUARD (zve2≤0-Skip, non-vacuous): § 16-vg 200000 (netto 200000, FB 0) + § 10d-Verlustvortrag
    200000 → § 10d mindert die GdE auf ~0 → zve2 ≤ 0 → Naht ÜBERSPRINGT den Fünftel (KEIN catala_fuenftel-Aufruf →
    kein ValueError-Pfad; est 0 sowieso). Belegt: der zve2>0-Guard fängt den §34-Abs.1-S.3-Edge (verbleibendes<0 ∧
    zvE≤0) sauber ab, statt zu crashen."""
    catala = _catala_da()
    _gesamt_anlegen(base, "f34z", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, vg=20000000,
                    verlustvortrag_bestand=20000000))
    st, erg = _req(base, "GET", "/fall/f34z/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 0 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_fuenftel_p35_interaktion(base):
    """§ 34 Abs. 1 × § 35 KOPPLUNG (differenziell): laufender Gewerbe-Gewinn 50000 (betriebsart gewerbe) + § 16-vg
    200000 (netto 200000) + GewSt-Messbetrag 3000 / Hebesatz 400 % → der § 35-Ermäßigungshöchstbetrag (Deckel 3)
    nutzt die POST-Fünftel-tarifliche ESt (catala_gesamt_tarifliche liest tarif_modifiziert=True). ao=netto_vg 200000
    (laufender 50000 progressiv, im §35-Zähler). Belegt: die geminderte tarifliche Steuer (§ 35 Abs. 1 S. 4) ist die
    Fünftel-tarifliche, nicht die progressive — saubere Naht-Reihenfolge (Fünftel VOR §35-Deckel-3)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "f34p", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=5000000,
                    betriebsart="gewerbe", vg=20000000, gewst_messbetrag=300000, gewst_hebesatz=400))
    st, erg = _req(base, "GET", "/fall/f34p/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 7964800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_fuenftel_per_kind(base):
    """§ 34 Abs. 1 PER §31-ZWEIG (non-vacuous, Freibetrag-Zweig gewinnt): laufender Gewerbe-Gewinn 80000 + § 16-vg
    200000 (netto 200000) + 2 Kinder → hohes Einkommen → § 31-Günstiger wählt den KINDERFREIBETRAG-Zweig (nicht
    Kindergeld). Der § 31-Günstiger rechnet _festzusetzende ZWEIMAL (ohne/mit Kinderfreibetrag); JEDER Zweig kriegt
    seinen EIGENEN Fünftel (zve2 sinkt mit dem Kinderfreibetrag → verbleibendes zvE/Fünftel je Zweig verschieden).
    Belegt: die Fünftel-Injektion sitzt IN _festzusetzende (per-§31-Zweig), nicht global — sonst würde der
    Kinderfreibetrag-Zweig den falschen (ohne-FB-) Fünftel nutzen = stille Fehlbesteuerung."""
    catala = _catala_da()
    _gesamt_anlegen(base, "f34k", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=8000000,
                    betriebsart="gewerbe", vg=20000000))
    _gesamt_abzuege(base, "f34k", kinder=2)
    st, erg = _req(base, "GET", "/fall/f34k/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 10667200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_abs3_gewaehlt(base):
    """§ 34 Abs. 3 GEWÄHLT (Chooser XOR Abs.1): § 16-vg 500000 (netto 500000, FB 0) + antrag_ermaessigter_satz +
    geburtsjahr 1955 (Alter 70 ≥ 55, §24a korrekt) → ermäßigter Durchschnittssatz statt Abs.1-Fünftel. est = plain grundtarif(zvE−ao
    ≈ 0) + min(ao,5Mio)×max(0.56×Durchschnittssatz; 0.14). Kein 5Mio-Überschuss (500000 < 5Mio)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "a3g", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, vg=50000000,
                    antrag_erm=True, geburtsjahr=1955))
    st, erg = _req(base, "GET", "/fall/a3g/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 11520400 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_abs3_kein_antrag(base):
    """§ 34 Chooser DEFAULT = Abs.1 (von Amts wegen): vg 500000 + dauernd_berufsunfaehig (eligible) ABER KEIN antrag →
    Abs.1-Fünftel (nicht Abs.3). Belegt: Abs.3 ist opt-in (S.1 „auf Antrag"), ohne Antrag greift der Fünftel-Default."""
    catala = _catala_da()
    _gesamt_anlegen(base, "a3n", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, vg=50000000, berufsunfaehig=True))
    st, erg = _req(base, "GET", "/fall/a3n/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 15542000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_abs3_eligibility_fail(base):
    """§ 34 Abs. 3 ELIGIBILITY-FAIL → fail-closed auf Abs.1 (NICHT Abs.3 erzwingen): vg 500000 + antrag ABER
    WEDER 55+/geburtsjahr NOCH berufsunfähig → nicht Abs.3-berechtigt → Abs.1-Fünftel auf GANZES ao.
    est == der kein-antrag-Fall (a3n). Belegt Guard-A-Logik: ¬eligible → Abs.1, kein false-block."""
    catala = _catala_da()
    _gesamt_anlegen(base, "a3f", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, vg=50000000, antrag_erm=True))
    st, erg = _req(base, "GET", "/fall/a3f/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 15542000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_abs3_einmal_genutzt(base):
    """§ 34 Abs. 3 S. 4 EINMAL IM LEBEN verbraucht → fail-closed auf Abs.1: vg 500000 + antrag + berufsunfähig ABER
    ermaessigung_einmal_genutzt=True → nicht mehr berechtigt → Abs.1-Fünftel. est == a3n (Abs.1)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "a3e", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, vg=50000000,
                    antrag_erm=True, berufsunfaehig=True, einmal_genutzt=True))
    st, erg = _req(base, "GET", "/fall/a3e/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 15542000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_abs3_ueber_5mio_guard(base):
    """§ 34 Abs. 3 >5Mio-GUARD (fail-closed, K2): vg 6000000 (netto 6Mio) + antrag + berufsunfähig (eligible) → der ermäßigte
    Satz gilt nur bis 5 Mio (S.1); der Excess braucht Stufe-2b → abs3_ueber_5mio_offen (kein still-auf-Abs.1-fallen =
    das verweigerte den Abs.3-Benefit auf die ersten 5Mio = Over-tax). zahl_cent None."""
    _gesamt_anlegen(base, "a35m", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, vg=600000000,
                    antrag_erm=True, berufsunfaehig=True))
    st, erg = _req(base, "GET", "/fall/a35m/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "abs3_ueber_5mio_offen"


def test_gesamt_abs3_14prozent_minimum(base):
    """§ 34 Abs. 3 S. 2 „mindestens 14 %"-FLOOR (Boundary): vg 100000 (netto 55000) + antrag + berufsunfähig → niedriger
    Durchschnittssatz (0.56×Durchschnittssatz < 14 %) → ermäßigter Satz auf 14 % gefloort. est = grundtarif(zvE−ao≈0)
    + 55000×0.14. ⚠ Hier ist Abs.3 (antrag) TEURER als der Abs.1-Fünftel-Default (der wäre 0, S.3) — der Chooser
    folgt dem ANTRAG, nicht dem Günstiger (§34 Abs.3 „auf Antrag")."""
    catala = _catala_da()
    _gesamt_anlegen(base, "a314", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, vg=10000000,
                    antrag_erm=True, berufsunfaehig=True))
    st, erg = _req(base, "GET", "/fall/a314/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 770000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_abs3(base):
    """§ 34 Abs. 3 im RENTNER-Ring: Rentner mit § 16-vg 500000 + antrag + dauernd_berufsunfaehig (S.1-Alternative zu
    55+) → ermäßigter Durchschnittssatz. Belegt: der Chooser wirkt in BEIDEN Ringen (Rentner-Betriebsveräußerung)."""
    catala = _catala_da()
    _rentner_anlegen(base, "ra3", _rentner_kegel(jahresrente=0, vg=50000000, kein_gewinn=False,
                     antrag_erm=True, berufsunfaehig=True))
    st, erg = _req(base, "GET", "/fall/ra3/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 11522100 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_abs3_age(base):
    """§ 34 Abs. 3 rentner-AGE-Pfad (b, REGRESSION für cb8d084-Latent-Bug): Rentner geb1955 (Alter 70 ≥ 55 via
    geburtsjahr, NICHT berufsunfähig) + § 16-vg 500000 + antrag → Abs.3. VOR RENTNER_GEWINN+=geburtsjahr las
    _abs3_eligible im rentner-Ring geburtsjahr=0 → alter≥55=False → fiel auf Abs.1 = Over-tax. Jetzt: geburtsjahr
    gelesen → Abs.3 via Alter. (geb1955 ist zugleich §24a-eligible Kohorte 2020 → §24a auf die vg-Bemessung.)"""
    catala = _catala_da()
    _rentner_anlegen(base, "ra3a", _rentner_kegel(jahresrente=0, vg=50000000, kein_gewinn=False,
                     antrag_erm=True, geburtsjahr=1955))
    st, erg = _req(base, "GET", "/fall/ra3a/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 11520400 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_abs3_age_fail(base):
    """§ 34 Abs. 3 rentner-AGE-FAIL → Abs.1 (b): Rentner geb1980 (Alter 45 < 55) + vg 500000 + antrag ∧ ¬berufsunfähig
    → NICHT Abs.3-berechtigt → Abs.1-Fünftel. (geb1980 Kohorte 2045 > VZ2025 → §24a 0 auch.) Belegt: der age-Pfad
    weist korrekt ab (nicht jeder Rentner ist 55+; junger Erbe/Betriebsübernehmer)."""
    catala = _catala_da()
    _rentner_anlegen(base, "ra3af", _rentner_kegel(jahresrente=0, vg=50000000, kein_gewinn=False,
                     antrag_erm=True, geburtsjahr=1980))
    st, erg = _req(base, "GET", "/fall/ra3af/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 15542000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_p24a_gewinn(base):
    """§ 24a rentner-ANWENDUNG (b, over-tax-Fix): Rentner geb1958 (eligible) + laufender Gewinn 30000 (§§13-18, KEINE
    Rente) → § 24a auf die Gewinn-Bemessung 30000 (positive Nicht-§19-Eink.) → est niedriger als ohne § 24a. VOR (b)
    kriegte der Rentner-mit-Gewerbe KEINE Altersentlastung = Over-tax."""
    catala = _catala_da()
    _rentner_anlegen(base, "rp24g", _rentner_kegel(jahresrente=0, gewinn=3000000, kein_gewinn=False, geburtsjahr=1958))
    st, erg = _req(base, "GET", "/fall/rp24g/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 410500 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_p24a_pure_leibrente(base):
    """§ 24a S. 2 rentner-AUSSCHLUSS (b, non-vacuous): Rentner geb1958 (eligible) + PURE gesetzl. Leibrente 20000
    (KEIN Gewinn) → § 24a-Bemessung = 0 (Leibrente § 22 Nr. 1 ist NICHT Bemessung, S. 2) → § 24a 0 → est UNVERÄNDERT
    zum reinen Renten-Fall. Belegt: die Leibrente fließt NICHT in die § 24a-Bemessung (sonst falscher Altersentlastungs-
    Abzug = Under-tax)."""
    catala = _catala_da()
    _rentner_anlegen(base, "rp24l", _rentner_kegel(jahresrente=2000000, beginn=2025, geburtsjahr=1958))
    st, erg = _req(base, "GET", "/fall/rp24l/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 81100 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


@pytest.mark.parametrize("geburtsjahr,erwartet_cent,label", [
    (1958, 410500, "eligible"),      # Folgejahr 2023 ≤ VZ2025 → §24a auf Gewinn 30000
    (1960, 411600, "eligible-grenze"),  # Folgejahr 2025 == VZ2025 → §24a (Kohorte 2025)
    (1961, 429300, "gated-grenze"),  # Folgejahr 2026 > VZ2025 → §24a 0 (erbt 64+-Gate von (a))
    (1990, 429300, "gated-under64"), # Folgejahr 2055 > VZ2025 → §24a 0
])
def test_rentner_p24a_64plus_gate(base, geburtsjahr, erwartet_cent, label):
    """§ 24a 64+-Gate ERBT im Rentner-Ring (b): Rentner + Gewinn 30000 + geburtsjahr → §24a nur eligible (Folgejahr
    ≤ VZ). Spiegel des gesamt-Gate-Goldens VZ2025: geb1960 (2025==VZ, eligible) vs geb1961 (2026>VZ, gated 0)."""
    catala = _catala_da()
    fid = f"rp24g{geburtsjahr}"
    _rentner_anlegen(base, fid, _rentner_kegel(jahresrente=0, gewinn=3000000, kein_gewinn=False, geburtsjahr=geburtsjahr))
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == erwartet_cent and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_abs3_est_rest_positiv(base):
    """§ 34 Abs. 3 DEKOMPOSITIONS-SUMME (est_rest + est_ao BEIDE > 0, Instructor-Boundary): laufender Gewerbe-Gewinn
    80000 + § 16-vg 300000 (netto 300000, FB 0) + antrag + berufsunfähig → einkuenfte_gewinn 380000, ao = netto_vg
    300000. zvE ≈ 379964; verbleibendes zvE = 379964 − 300000 = 79964 > 0 → est_rest = plain grundtarif(79964) > 0
    (S.3 allgemeiner Tarif auf den REST). est_ao = 300000 × max(0.56×Durchschnittssatz, 0.14). total = est_rest +
    est_ao — testet die SUMME (nicht nur est_ao wie die pure-vg-Fälle mit est_rest≈0). laufender 80000 NICHT im ao
    (nur die vg = außerordentlich)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "a3r", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=8000000,
                    betriebsart="gewerbe", vg=30000000, antrag_erm=True, berufsunfaehig=True))
    st, erg = _req(base, "GET", "/fall/a3r/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 8976200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


@pytest.mark.parametrize("gewinn,brutto,mb,hebesatz,erwartet,label", [
    (5000000,       0, 200000, 450, 267800,  "deckel1-4xMB"),           # 4×MB=8000 < MB×450%=9000 < d3 → §35 8000
    (10000000,      0, 300000, 300, 2207200, "deckel2-tatsGewSt"),      # MB×300%=9000 < 4×MB=12000 < d3 → §35 9000
    (2000000, 8000000, 300000, 500, 2436900, "deckel3-hoechstbetrag"),  # Ermäßigungshöchstbetrag (Zähler/Nenner×tarifl) bindet
])
def test_gesamt_p35_deckel(base, gewinn, brutto, mb, hebesatz, erwartet, label):
    """§ 35 GewSt-Anrechnung (S1) im gesamt-Ring: Gewerbe-Gewinn (betriebsart=gewerbe) + GewSt-Messbetrag (Input) +
    Hebesatz → Anrechnung auf die tarifliche ESt = min der 3 Deckel (§ 35 Abs. 1): 4×Messbetrag (S. 1 „das
    Vierfache"), Messbetrag×Hebesatz (S. 5 tatsächl. GewSt), Ermäßigungshöchstbetrag (S. 2, Zähler gewerbl. Eink./
    Nenner alle pos. Eink. × geminderte tarifl. Steuer). Deckt alle 3 Deckel-Wechsel. Werte unabhängig gg. runner-
    Kette + § 35-Hand-Rechnung; jeder Gesetzeswert (4× = § 35 Abs. 1 S. 1 „das Vierfache") source-verankert."""
    catala = _catala_da()
    fid = f"p35{label}"
    _gesamt_anlegen(base, fid, _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=gewinn, bruttolohn=brutto,
                    betriebsart="gewerbe", gewst_messbetrag=mb, gewst_hebesatz=hebesatz))
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == erwartet and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p35_selbstaendig_kein_credit(base):
    """§ 35 gilt NUR für Gewerbebetrieb (§ 15): ein § 18-selbständiger Gewinn 50000 mit Messbetrag+Hebesatz →
    Zähler „gewerbliche Einkünfte" = 0 (§ 18 nicht gewerbesteuerpflichtig) → § 35-Anrechnung 0 → festzusetzende_est
    10678 = 1067800 Cent (= ohne § 35, wie reiner Gewinn 50000). Belegt: kein § 35-Credit für Nicht-Gewerbe (kein
    stiller Über-Credit); der Messbetrag-Input allein triggert keine Anrechnung ohne gewerbe-Betriebsart."""
    catala = _catala_da()
    _gesamt_anlegen(base, "p35s", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=5000000,
                    betriebsart="selbstaendig", gewst_messbetrag=200000, gewst_hebesatz=450))
    st, erg = _req(base, "GET", "/fall/p35s/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1067800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p35_kinder_pro_zweig(base):
    """K2 (per-Freibetrag-Zweig, ALL-OR-CORRECT): Gewerbe-Gewinn 30000 + Lohn 70000 + 1 Kind + Messbetrag 8000 €
    Hebesatz 600 % → der Ermäßigungshöchstbetrag (Deckel 3) hängt an der tariflichen ESt, die im Kinderfreibetrag-
    Zweig NIEDRIGER ist → § 35 muss JE § 31-Günstiger-Zweig mit DESSEN tarifl. ESt gerechnet werden (p35 9280 ohne
    Freibetrag vs 8668 mit) → § 31 wählt → festzusetzende_est 21276 = 2127600 Cent. Belegt: global-einmaliges § 35
    (mit dem höheren tarifl.-ESt-Wert) würde den Kinderfreibetrag-Zweig über-crediten = stille Under-tax."""
    catala = _catala_da()
    _gesamt_anlegen(base, "p35k", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=3000000, bruttolohn=7000000,
                    betriebsart="gewerbe", gewst_messbetrag=800000, gewst_hebesatz=600))
    _gesamt_abzuege(base, "p35k", kinder=1)
    st, erg = _req(base, "GET", "/fall/p35k/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 2127600 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p35_hebesatz_offen(base):
    """K2 fail-closed (Guard non-vacuous): GewSt-Messbetrag angegeben (§ 35-opt-in) ABER kein Hebesatz → die
    Anrechnung min(4×MB, MB×Hebesatz, …) ist ohne Hebesatz nicht rechenbar → gewst_hebesatz_offen (KEIN 4×MB-
    Default, der bei Hebesatz < 400 % über-creditete = Under-tax). Ohne Messbetrag feuert der Guard NICHT (opt-out)."""
    _gesamt_anlegen(base, "p35h", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=5000000,
                    betriebsart="gewerbe", gewst_messbetrag=200000))
    st, erg = _req(base, "GET", "/fall/p35h/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "gewst_hebesatz_offen"


def test_gesamt_p35_kapital_nicht_im_nenner(base):
    """K2 (Nenner-Komposition, § 2 Abs. 5b): Gewerbe-Gewinn 100000 + Lohn 50000 + Abgeltung-Kapital 20000, MB 12000 €
    Hebesatz 500 % → der § 35 Abs. 1 S. 2-Ermäßigungshöchstbetrag (Deckel 3) bindet. Das § 32d-Abgeltung-Kapital ist
    NICHT im Nenner „Summe aller positiven Einkünfte" (§ 2 Abs. 5b: „Kapitalerträge nach § 32d Abs. 1 … nicht ein-
    zubeziehen" — es ist nicht im tariflichen zvE, das die tarifliche ESt skaliert) → Ratio 100000/148770 → § 35
    34654 → festzusetzende_est 21652 = 2165200 Cent. Belegt: das Kapital VERWÄSSERT die gewerbliche Ratio NICHT
    (wäre es im Nenner: Ratio 100000/167770 → § 35 nur 30730 → est 25576 = ANDERER Wert → Assertion bräche)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "p35kap", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=10000000,
                    bruttolohn=5000000, betriebsart="gewerbe", gewst_messbetrag=1200000, gewst_hebesatz=500,
                    kein_kap=False, kap_ertraege=2000000))
    st, erg = _req(base, "GET", "/fall/p35kap/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 2165200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p10d_verlustvortrag_partial(base):
    """§ 10d Abs. 2 Verlustvortrag (est-wirksam): Gewinn 80000 (GdE 80000) + festgestellter Verlustvortrag 30000
    (< GdE) → verlustabzug 30000 mindert den GdE VORRANGIG vor Sonderausgaben/agB (Fold in sonstige_abzuege_vom_
    einkommen) → zvE 50000 → festzusetzende_est 10678 = 1067800 Cent, NIEDRIGER als ohne § 10d (Gewinn 80000 →
    2267200). Belegt: der Verlustvortrag senkt die Steuer; Höchstbetrag min(GdE, 1 Mio) greift nicht (bestand < GdE)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "p10p", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=8000000,
                    verlustvortrag_bestand=3000000))
    st, erg = _req(base, "GET", "/fall/p10p/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1067800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p10d_min_gde_cap(base):
    """§ 10d Abs. 2 min(GdE)-Cap (der gefixte p10d_2-Defekt): Gewinn 50000 (GdE 50000) + Verlustvortrag 60000
    (> GdE) → verlustabzug auf GdE 50000 gedeckelt (§ 10d Abs. 2 „bis zu einem GdE von 1 Mio UNBESCHRÄNKT" = 100 %
    nur bis GdE-Höhe, NICHT 60000) → zvE 0 → festzusetzende_est 0. Belegt: kein Über-Abzug über den GdE hinaus
    (die alte cap-lose Regel hätte 60000 „abgezogen" = falscher Vortrags-Verbrauch)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "p10c", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=5000000,
                    verlustvortrag_bestand=6000000))
    st, erg = _req(base, "GET", "/fall/p10c/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 0 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p10d_mindestbesteuerung_70(base):
    """§ 10d Abs. 2 Mindestbesteuerung (70 % über 1 Mio): Gewinn 1500000 (GdE 1500000) + Verlustvortrag 2000000 →
    Höchstbetrag = 1 Mio unbeschränkt + 70 % × 500000 = 1350000 (min mit GdE 1.5 Mio → 1.35 Mio) → zvE 150000
    (= 30 % Mindestbesteuerungs-Rest des Überstiegs) → festzusetzende_est 52072 = 5207200 Cent. Belegt: der Vortrag
    kann den GdE über 1 Mio nur zu 70 % offsetten (§ 10d Abs. 2 S. 1)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "p10m", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=150000000,
                    verlustvortrag_bestand=200000000))
    st, erg = _req(base, "GET", "/fall/p10m/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 5207200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p10d_absent_unveraendert(base):
    """§ 10d OPTIONAL (opt-out): kein verlustvortrag_bestand → kein Verlustabzug (sonstige_abzuege_vom_einkommen 0)
    → festzusetzende_est = reiner Gewinn 50000 → 1067800 Cent (unverändert). Absent ist fail-safe (over-tax-safe)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "p10a", _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=5000000))
    st, erg = _req(base, "GET", "/fall/p10a/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1067800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_kombiniert_job_und_vermietung(base):
    """Konvergenz § 19 + § 21: Arbeitnehmer (Bruttolohn 40000, kein Pendel-WK → § 19-Einkünfte 38770)
    MIT Vermietung 18770 → catala_gesamt summiert (§ 2 Abs. 3) → festzusetzende_est 13452 = 1345200 Cent."""
    catala = _catala_da()
    _gesamt_anlegen(base, "kjv", _gesamt_kegel(1877000, bruttolohn=4000000))   # vv 18770 €, Job 40000 € (Cent)
    st, erg = _req(base, "GET", "/fall/kjv/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1345200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_kombiniert_verlust_mindert_lohn(base):
    """K2-KERN: § 21-Verlust (8000 − 6000 − 7000 = −5000) mindert nach § 2 Abs. 3 den § 19-Lohn
    (Einkünfte 38770) → festzusetzende_est 5388 = 538800 Cent, KLEINER als die reine § 19-ESt (6919).
    Der Verlust wird verrechnet, NICHT verschluckt — und floort nicht auf Negativsteuer."""
    catala = _catala_da()
    _gesamt_anlegen(base, "kvl", _gesamt_kegel(800000, afa=600000, schuldzinsen=700000, bruttolohn=4000000))
    st, erg = _req(base, "GET", "/fall/kvl/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 538800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_kombiniert_mit_pendel_wk(base):
    """Kombiniert mit § 19-Werbungskosten: Job 40000 + Entfernungspauschale (220 Tage, 30 km, Kfz
    = 2156 → § 19-Einkünfte 37844 statt 38770) + Vermietung 18770 → festzusetzende_est 13101 =
    1310100 Cent, KLEINER als ohne Pendel-WK (13452). Belegt, dass die EP-Slots (ep_arbeitstage →
    arbeitstage …) im gesamt-Kegel korrekt in die § 19-WK durchgereicht werden."""
    catala = _catala_da()
    _gesamt_anlegen(base, "kpw", _gesamt_kegel(1877000, bruttolohn=4000000,
                                       ep_tage=220, ep_km=30, ep_kfz=True))
    st, erg = _req(base, "GET", "/fall/kpw/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 1310100 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def _vv_instanz_anlegen(base, fid, idx, einnahmen, afa=0, schuldzinsen=0, erhaltung=0, sonstige=0,
                        entgelt_quote=100, wohnzwecke=None, weglassen=()):
    """Postet die 6 Pflicht-vv-Felder EINER weiteren Objekt-Instanz (base__idx, idx>=2, Multi-Objekt-§21 #5) inkl.
    § 21-Abs.2-Entgelt-Quote (100=nicht verbilligt). wohnzwecke (bool, optional §21-Abs.2-Tatbestand) nur wenn
    nicht None. weglassen = feld_ids, die NICHT gepostet werden (für den Unvollständig-K2-Test)."""
    paare = [("vv_einnahmen", einnahmen), ("vv_gebaeude_afa", afa),
             ("vv_schuldzinsen", schuldzinsen), ("vv_erhaltungsaufwand", erhaltung),
             ("vv_sonstige_wk", sonstige), ("vv_entgelt_quote_prozent", entgelt_quote)]
    if wohnzwecke is not None:
        paare.append(("vv_wohnzwecke", wohnzwecke))
    for feld, wert in paare:
        if feld in weglassen:
            continue
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(f"{feld}__{idx}", wert))
        assert st == 201


def test_gesamt_multi_objekt_zwei_gewinne(base):
    """#5 Multi-Objekt § 21: zwei vermietete Objekte (Basis-Instanz 30000 + Objekt 2 = vv_einnahmen__2 20000),
    einzel, kein Job → est_mapping.instanzen enumeriert BEIDE (index 1 = Basis, index 2 = __2), der Ring
    summiert stumpf → § 21 Σ 50000 → festzusetzende_est 10678 = 1067800 Cent."""
    catala = _catala_da()
    _gesamt_anlegen(base, "mo2", _gesamt_kegel(3000000))   # Objekt 1 = Basis-Instanz 30000
    _vv_instanz_anlegen(base, "mo2", 2, 2000000)           # Objekt 2 = __2 20000
    st, erg = _req(base, "GET", "/fall/mo2/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1067800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_multi_objekt_verlustausgleich(base):
    """#5 K2-KERN horizontaler Verlustausgleich INNERHALB § 21: Objekt 1 +30000, Objekt 2 Verlust
    (8000−6000−7000 = −5000) → Σ 25000 (NICHT 30000 — der Objekt-2-Verlust mindert Objekt 1, catala_vermietung_
    einkuenfte floort NICHT per Objekt) → festzusetzende_est 2917 = 291700 Cent, KLEINER als das Einzelobjekt
    (4293). Belegt, dass die per-Objekt-Σ Verluste durchreicht statt sie per Objekt auf 0 zu floren."""
    catala = _catala_da()
    _gesamt_anlegen(base, "mov", _gesamt_kegel(3000000))
    _vv_instanz_anlegen(base, "mov", 2, 800000, afa=600000, schuldzinsen=700000)
    st, erg = _req(base, "GET", "/fall/mov/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 291700 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_multi_objekt_instanz_unvollstaendig(base):
    """#5 K2 fail-closed: Objekt 2 unvollständig (nur vv_einnahmen__2, die 4 WK-Felder fehlen) → vv_instanz_offen,
    NIE ein still zu niedriges §21-Σ (eine halbe Objekt-Instanz würde sonst mit WK=0 voll versteuert erscheinen).
    Der per-Instanz-Guard verlangt alle 5 Basis-vv-Felder present + bestätigt je Zusatzobjekt."""
    _gesamt_anlegen(base, "moi", _gesamt_kegel(3000000))
    _vv_instanz_anlegen(base, "moi", 2, 2000000,
                        weglassen=("vv_gebaeude_afa", "vv_schuldzinsen",
                                   "vv_erhaltungsaufwand", "vv_sonstige_wk"))
    st, erg = _req(base, "GET", "/fall/moi/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "vv_instanz_offen"


def test_gesamt_multi_objekt_schreibpfad_akzeptiert_instanz(base):
    """#5 Schreibpfad: der Event-Endpunkt akzeptiert base__n einer instanz-fähigen Basis (parse_instanz),
    lehnt aber ein __n einer NICHT-instanz-fähigen Basis ab (400) — kein beliebiges Suffix-Schlupfloch."""
    _gesamt_anlegen(base, "mos", _gesamt_kegel(3000000))
    st, _ = _req(base, "POST", "/fall/mos/event", _laie("vv_einnahmen__2", 1500000))
    assert st == 201                                   # vv_einnahmen ist instanz_gruppe:vv_objekt -> ok
    st, _ = _req(base, "POST", "/fall/mos/event", _laie("bruttoarbeitslohn__2", 1000000))
    assert st == 400                                   # bruttoarbeitslohn NICHT instanz-fähig -> abgewiesen


def test_gesamt_p21_2_verbilligt_kuerzung(base):
    """§ 21 Abs. 2 K2-KERN (behebt Unter-Besteuerung): verbilligte Vermietung Entgelt-Quote 50 % (< 66 %) → WK
    nur anteilig (8000 × 0,5 = 4000 statt voll) → § 21-Einkünfte 16000 statt 12000 → HÖHERE Steuer. Job 40000 +
    Vermietung (Einnahmen 20000, AfA 8000) @ quote 50 → festzusetzende_est 12410 = 1241000 Cent (vs. voll-WK 1095200)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "vk", _gesamt_kegel(2000000, afa=800000, bruttolohn=4000000, entgelt_quote=50))
    st, erg = _req(base, "GET", "/fall/vk/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1241000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p21_2_voll_bei_100(base):
    """§ 21 Abs. 2 Gegenzweig: Entgelt-Quote 100 % (≥ 66 %) → volle WK (keine Kürzung) → § 21-Einkünfte 12000 →
    festzusetzende_est 10952 = 1095200 Cent. Belegt: der Regelfall (nicht verbilligt) bleibt unverändert."""
    catala = _catala_da()
    _gesamt_anlegen(base, "v100", _gesamt_kegel(2000000, afa=800000, bruttolohn=4000000, entgelt_quote=100))
    st, erg = _req(base, "GET", "/fall/v100/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 1095200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p21_2_gewerblich_voll_wk(base):
    """§ 21 Abs. 2 Tatbestands-Gate: Entgelt-Quote 50 %, ABER vv_wohnzwecke=false (gewerblich, keine
    Wohnraumvermietung) → § 21 Abs. 2 greift NICHT → volle WK → festzusetzende_est 1095200 Cent (wie quote 100).
    Belegt: die Kürzung ist Wohnzweck-/Dauer-spezifisch, nicht jede verbilligte Überlassung."""
    catala = _catala_da()
    _gesamt_anlegen(base, "vg", _gesamt_kegel(2000000, afa=800000, bruttolohn=4000000, entgelt_quote=50))
    st, _ = _req(base, "POST", "/fall/vg/event", _laie("vv_wohnzwecke", False))
    assert st == 201
    st, erg = _req(base, "GET", "/fall/vg/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 1095200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p21_2_unentgeltlich_quote_null(base):
    """§ 21 Abs. 2 C-Fix (K2, Under-tax): Entgelt-Quote 0 % = UNENTGELTLICHE Überlassung (keine Einkünfteerzielungs-
    absicht, § 21 greift nicht). Objekt Einnahmen 0 + WK (AfA) 5000 → Beitrag 0 (kein WK-Verlust der den § 19-Lohn
    mindert) → festzusetzende_est 6919 = 691900 Cent (= reiner Job 40000). VORHER kollabierte `_ci or 100` die 0
    auf 100 → voller −5000-WK-Verlust → est 538800 (Under-tax um 1531 €). Belegt: 0 % erzeugt keinen Scheinverlust."""
    catala = _catala_da()
    _gesamt_anlegen(base, "vq0", _gesamt_kegel(0, afa=500000, bruttolohn=4000000, entgelt_quote=0))
    st, erg = _req(base, "GET", "/fall/vq0/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 691900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p21_2_kegel_quote_fehlt(base):
    """§ 21 Abs. 2 K2 Pflicht-Kegel: eine WEITERE Objekt-Instanz OHNE vv_entgelt_quote_prozent__2 → vv_instanz_offen
    (kein stiller Bescheid bei voller WK — die Quote MUSS je Objekt beantwortet sein, sonst Unter-tax-Risiko)."""
    _gesamt_anlegen(base, "vqf", _gesamt_kegel(3000000, bruttolohn=4000000))
    _vv_instanz_anlegen(base, "vqf", 2, 2000000, afa=800000, weglassen=("vv_entgelt_quote_prozent",))
    st, erg = _req(base, "GET", "/fall/vqf/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "vv_instanz_offen"


def test_gesamt_p21_2_per_objekt(base):
    """§ 21 Abs. 2 PER OBJEKT (Multi-Objekt): Objekt A markt-vermietet (Einnahmen 30000, quote 100 → voll, vv 30000)
    + Objekt B verbilligt (Einnahmen 20000, AfA 8000, quote 50 → WK 4000, vv 16000) → NUR B gekürzt → § 21-Σ 46000
    → festzusetzende_est 9288 = 928800 Cent. Belegt: die Kürzung trifft je Objekt seine eigene Quote, nicht global."""
    catala = _catala_da()
    _gesamt_anlegen(base, "vpo", _gesamt_kegel(3000000, bruttolohn=0, kein_vuv=False, entgelt_quote=100))
    _vv_instanz_anlegen(base, "vpo", 2, 2000000, afa=800000, entgelt_quote=50)
    st, erg = _req(base, "GET", "/fall/vpo/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 928800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_vorsorge_altersvorsorge_abzug(base):
    """§ 10 Abs. 3 Altersvorsorge im gefalteten gesamt-Ring (behebt ÜBER-Besteuerung): gesamt-Nutzer mit RV-
    Beiträgen (AN 3500 + AG 3500 = Gesamtbeiträge 7000, steuerfreier AG-Anteil 3500) → abziehbare Altersvorsorge
    3500 (nach knappschaft-HB-Cap, Kürzung um AG-Anteil) → festzusetzende_est 5849 = 584900 Cent, NIEDRIGER als
    ohne VOR (691900). Belegt: der gesamt-Ring gewährt jetzt den § 10-Altersvorsorge-Abzug (vorher verloren =
    Nutzer überzahlte). VOR im Pflicht-Kegel → immer gefragt, kein stiller Über-tax."""
    catala = _catala_da()
    _gesamt_anlegen(base, "vor", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True,
                                               vor_an=350000, vor_ag=350000))
    st, erg = _req(base, "GET", "/fall/vor/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 584900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_kv_pv_durchbruch_abzug(base):
    """§ 10 Abs. 1 Nr. 3/3a + Abs. 4 S. 4 KV/PV im gefalteten gesamt-Ring: Basis-KV/PV 3200 > Höchstbetrag 2800
    (ohne Zuschuss) → § 10 Abs. 4 S. 4 Durchbruch: die Basisabsicherung ist STETS voll abziehbar (3200, nicht auf
    2800 gedeckelt) → festzusetzende_est 12721 = 1272100 Cent, NIEDRIGER als ohne KV/PV (1392400). Belegt: der
    gesamt-Ring gewährt den KV/PV-Abzug + der Durchbruch schlägt den Höchstbetrag (sonst Über-tax auf den 400 €
    über HB). KV/PV im Pflicht-Kegel → immer gefragt, kein stiller Über-tax."""
    catala = _catala_da()
    _gesamt_anlegen(base, "kvd", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True, basis_kv_pv=320000))
    st, erg = _req(base, "GET", "/fall/kvd/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1272100 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_kv_pv_hoechstbetrag_mit_zuschuss(base):
    """§ 10 Abs. 4 KV/PV mit Zuschussanspruch: Höchstbetrag 1900 (statt 2800). Basis 1500 + weitere Vorsorge 800 =
    2300, aber die Basis (1500) unterschreitet den HB → kein Durchbruch; abziehbar = min(2300, 1900) = 1900 →
    festzusetzende_est 13211 = 1321100 Cent. Belegt: die weitere Vorsorge wird nur bis zum (mit-Zuschuss
    reduzierten) HB angerechnet — Basis + weitere GETRENNT behandelt."""
    catala = _catala_da()
    _gesamt_anlegen(base, "kvz", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True,
                                               basis_kv_pv=150000, weitere_kv_pv=80000, mit_anspruch_zuschuss=True))
    st, erg = _req(base, "GET", "/fall/kvz/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1321100 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_kv_pv_kegel_fehlt(base):
    """§ 10 KV/PV Pflicht-Kegel: basis_kv_pv nicht bestätigt → input_kegel_nicht_bestaetigt (kein stiller Bescheid
    mit KV/PV-Abzug 0 — die KV/PV-Beiträge MÜSSEN beantwortet sein, sonst Über-tax-Risiko der Pflichtbeiträge)."""
    kegel = [(f, w) for (f, w) in _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True) if f != "basis_kv_pv"]
    _gesamt_anlegen(base, "kvk", kegel)
    st, erg = _req(base, "GET", "/fall/kvk/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "input_kegel_nicht_bestaetigt"


def test_gesamt_p10_1_7_berufsausbildung(base):
    """§ 10 Abs. 1 Nr. 7 Berufsausbildung (Tier-1) im gesamt-Ring: reiner Job 40000 + Aufwendungen eigene
    Berufsausbildung 8000 → Höchstbetrag-Cap 6000 (§ 10 Abs. 1 Nr. 7 S. 1) → sonderausgaben +6000 →
    festzusetzende_est 5103 = 510300 Cent, NIEDRIGER als ohne (691900). Belegt: der Fold zieht die Berufsausbildung
    additiv ab (wie § 10b/KV-PV/KiSt), gedeckelt bei 6000. berufsausbildung_aufwendungen ist typ:cent → 8000 € =
    800000."""
    catala = _catala_da()
    _gesamt_anlegen(base, "ba8", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True))
    st, _ = _req(base, "POST", "/fall/ba8/event", _laie("berufsausbildung_aufwendungen", 800000))
    assert st == 201
    st, erg = _req(base, "GET", "/fall/ba8/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 510300 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_p10_1_7_absent_unveraendert(base):
    """§ 10 Abs. 1 Nr. 7 OPTIONAL: berufsausbildung_aufwendungen absent → Abzug 0 (kein Phantom-Sonderausgabe) →
    festzusetzende_est 6919 = 691900 Cent (= reiner Job 40000, unverändert). Belegt: absent ist fail-safe (over-tax),
    kein stiller Abzug — nur wer die Aufwendungen beziffert, bekommt den Abzug."""
    catala = _catala_da()
    _gesamt_anlegen(base, "ba0", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True))
    st, erg = _req(base, "GET", "/fall/ba0/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 691900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_kombiniert_job_und_kapital(base):
    """Konvergenz § 19 + § 20: Job 60000 (§ 19-Einkünfte 58770) + Kapitalerträge 10000 → nach Sparer-PB
    9000; Günstigerprüfung § 32d Abs. 6: Grenzsteuer > 25 % → Abgeltung 2250 gewinnt → festzusetzende_est
    est_ohne 13924 + 2250 = 16174 = 1617400 Cent (dev-2s K4-Zielwert, Brutto-Pipeline)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "kjk", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True,
                                       kein_kap=False, kap_ertraege=1000000))
    st, erg = _req(base, "GET", "/fall/kjk/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1617400 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_reines_kapital_guenstiger_null(base):
    """K2/Günstiger: reiner Kapitalfall (kein anderes Einkommen), Erträge 5000 → nach Sparer-PB 4000;
    Grundtarif auf 4000 = 0 < Abgeltung 1000 → Günstiger greift → festzusetzende_est 0."""
    catala = _catala_da()
    _gesamt_anlegen(base, "rkg", _gesamt_kegel(0, kein_vuv=True, kein_kap=False, kap_ertraege=500000))
    st, erg = _req(base, "GET", "/fall/rkg/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 0 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_kapital_semantik_offen_co_okkurrenz(base):
    """K2 fail-closed (Instructor-Q1): E0121709-Aggregat UND Aktien-Topf beide gesetzt → additiv-vs-subset
    ungeklärt → kapital_semantik_offen (kein Rate-Bescheid, kein stilles Verschlucken des Aggregats)."""
    _gesamt_anlegen(base, "kso", _gesamt_kegel(0, kein_vuv=True, kein_kap=False,
                                       kap_ertraege=500000, kap_gewinn_aktien=300000))
    st, erg = _req(base, "GET", "/fall/kso/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "kapital_semantik_offen"


def test_gesamt_zusammen_kapital_beide(base):
    """#4b § 20-B: Ehepaar zusammen, A Bruttolohn 60000 (§ 19 58770) + B Bruttolohn 40000 (38770) → ns 97540;
    A Kapital 8000 + B Kapital 6000 → roh 14000, gemeinsamer Sparer-PB ×2 → 12000. Günstiger § 32d Abs. 6:
    Grenzsteuer > 25 % → Abgeltung 3000 (0,25×12000) < Differenz → est_ohne 20490 + 3000 = 23490 = 2349000 Cent.
    Belegt: das Kapital BEIDER Ehegatten fließt single-source VOR den gemeinsamen Sparer-PB (nicht je Person)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "zbk", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True,
                                       kein_kap=False, kap_ertraege=800000,
                                       veranlagung="zusammen", bruttolohn_partner=4000000,
                                       person_b_idnr="12345678901", kap_ertraege_partner=600000))
    st, erg = _req(base, "GET", "/fall/zbk/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 2349000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_zusammen_kapital_gewinn_sonstige_partner(base):
    """Register-B-K2-Fix (2026-07-19): Person-B sonstiger Kapitalgewinn (§ 20 Abs. 2, eigener Topf) bei
    Zusammenveranlagung wird JETZT erfasst — VORHER war er in api.py hart 0 = stiller Under-tax des
    Ehegatten-Gewinns. Non-vacuous: die festzusetzende ESt MIT dem Person-B-sonstige-Gewinn ist HÖHER als
    OHNE (der Gewinn erhöht die gemeinsamen Kapitaleinkünfte, nach dem gemeinsamen Sparer-PB besteuert)."""
    catala = _catala_da()
    if not catala:
        pytest.skip("Catala-Toolchain nicht verfügbar")
    _gesamt_anlegen(base, "zbgs_mit", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True, kein_kap=False,
                    veranlagung="zusammen", bruttolohn_partner=4000000, person_b_idnr="12345678901",
                    kap_gewinn_sonstige_partner=500000))
    _, erg_mit = _req(base, "GET", "/fall/zbgs_mit/ergebnis")
    _gesamt_anlegen(base, "zbgs_ohne", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True, kein_kap=False,
                    veranlagung="zusammen", bruttolohn_partner=4000000, person_b_idnr="12345678901",
                    kap_gewinn_sonstige_partner=0))
    _, erg_ohne = _req(base, "GET", "/fall/zbgs_ohne/ergebnis")
    assert erg_mit["grund"] == "bestaetigt" and erg_ohne["grund"] == "bestaetigt"
    assert erg_mit["zahl_cent"] > erg_ohne["zahl_cent"], \
        "Person-B sonstiger Kapitalgewinn muss die est erhöhen (Register-B-Fix: nicht mehr hart 0)"


def test_gesamt_zusammen_kapital_semantik_partner(base):
    """#4b K2 fail-closed: Person-B-Kapital doppelt beschrieben (Aggregat E0121709_partner UND Aktien-Topf_partner
    beide > 0), zusammen → additiv-vs-subset ungeklärt → kapital_semantik_offen (spiegelt die Person-A-Sperre für
    den Ehegatten, kein still verschlucktes Ehegatten-Aggregat)."""
    _gesamt_anlegen(base, "zsp", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True,
                                       kein_kap=False, kap_ertraege=800000,
                                       veranlagung="zusammen", bruttolohn_partner=4000000,
                                       person_b_idnr="12345678901",
                                       kap_ertraege_partner=600000, kap_gewinn_aktien_partner=300000))
    st, erg = _req(base, "GET", "/fall/zsp/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "kapital_semantik_offen"


def _gesamt_abzuege(base, fid, minijob=0, dienstleistung=0, handwerker=0, rechnung_unbar=None,
                    spende=0, agb=0, kinder=0, kist_gezahlt=0, kist_erstattet=0,
                    geburtsjahr=None, alleinerziehend=None, monate=0):
    """Postet die OPTIONALEN gefalteten Sonder-Abzugs- + §24a/§24b-Freibetrag-Felder (Weg ii) auf einen gesamt-Fall
    (cent; kinder/geburtsjahr/monate=int; alleinerziehend=bool). hh_rechnung_unbar/geburtsjahr/alleinerziehend nur
    wenn nicht None. Nicht im Pflicht-Kegel — der Ring rechnet sie additiv."""
    paare = [("hh_minijob_aufwendungen", minijob), ("hh_dienstleistungen", dienstleistung),
             ("hh_handwerker_arbeitskosten", handwerker), ("spenden_betrag", spende),
             ("agb_aufwendungen", agb), ("fam_anzahl_kinder", kinder),
             ("kist_gezahlt", kist_gezahlt), ("kist_erstattet", kist_erstattet),
             ("fam_monate_ohne_voraussetzung", monate)]
    if rechnung_unbar is not None:
        paare.append(("hh_rechnung_unbar", rechnung_unbar))
    if geburtsjahr is not None:
        paare.append(("geburtsjahr", geburtsjahr))
    if alleinerziehend is not None:
        paare.append(("fam_alleinstehend", alleinerziehend))
    for feld, wert in paare:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


def test_gesamt_faltung_komposition(base):
    """Weg (ii) KOMPOSITION (Julius-Entscheid): §19 (Lohn 60000) + §21 (Vermietung 20000) + §35a (Handwerker
    10000, rechnung_unbar=true → 1200) + §10b (Spende 3000) + §33 (agB 5000, 0 Kinder → 150) in EINEM Bescheid.
    GdE 78770 (ECHT = ns 58770 + vv 20000, NICHT §19-only) → festzusetzende_est 19648 = 1964800 Cent. Belegt:
    die Sonder-Abzüge falten additiv auf die volle Einkunfts-Kombination — in den Standalone-Sonder-Scheiben unmöglich."""
    catala = _catala_da()
    _gesamt_anlegen(base, "fk", _gesamt_kegel(2000000, bruttolohn=6000000))   # §21 20000 + §19 60000
    _gesamt_abzuege(base, "fk", handwerker=1000000, rechnung_unbar=True, spende=300000, agb=500000)
    st, erg = _req(base, "GET", "/fall/fk/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1964800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_faltung_gde_echte_basis(base):
    """Weg (ii) KORREKTHEITS-GEWINN: §19 (Lohn 40000 → 38770) + §21 (Vermietung 30000) → GdE 68770. Spende 15000
    → §10b-Deckel auf die ECHTE GdE (20 % von 68770 = 13754), NICHT die §19-only-GdE (die nur 20 % von 38770 =
    7754 zuließe) → festzusetzende_est 12515 = 1251500 Cent. Belegt den GdE-Basis-Fehler-Fix der Faltung."""
    catala = _catala_da()
    _gesamt_anlegen(base, "fg", _gesamt_kegel(3000000, bruttolohn=4000000))   # §21 30000 + §19 40000
    _gesamt_abzuege(base, "fg", spende=1500000)
    st, erg = _req(base, "GET", "/fall/fg/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1251500 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_faltung_rechnung_unbar_carry(base):
    """Weg (ii) K2-Guard trägt mit: Handwerker > 0 im gefalteten gesamt-Ring OHNE hh_rechnung_unbar →
    rechnung_unbar_offen (§35a Abs.5 S.3, feld-präsenz-getrieben statt scheiben-flag-gated)."""
    _gesamt_anlegen(base, "fru", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True))
    _gesamt_abzuege(base, "fru", handwerker=1000000)   # kein rechnung_unbar
    st, erg = _req(base, "GET", "/fall/fru/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "rechnung_unbar_offen"


def test_gesamt_faltung_erstattungsueberhang_carry(base):
    """Weg (ii) K2-Guard trägt mit: KiSt erstattet > gezahlt im gefalteten gesamt-Ring → erstattungsueberhang_offen
    (§10 Abs.4b, feld-präsenz-getrieben)."""
    _gesamt_anlegen(base, "feu", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True))
    _gesamt_abzuege(base, "feu", kist_gezahlt=20000, kist_erstattet=120000)
    st, erg = _req(base, "GET", "/fall/feu/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "erstattungsueberhang_offen"


def test_gesamt_faltung_35a_est_floor(base):
    """Weg (ii) K2 §35a-ESt-Deckelung im gefalteten Ring: niedriges Einkommen (Lohn 14000, ESt ~93) + Handwerker
    10000 (§35a 1200 > verfügbare ESt) → festzusetzende_est auf 0 gefloort (nicht negativ). p32a wirksame_
    ermaessigung deckelt regel-seitig, auch im Fold. (Ersetzt die Standalone-haushalt-Floor-Coverage.)"""
    catala = _catala_da()
    _gesamt_anlegen(base, "ff", _gesamt_kegel(0, bruttolohn=1400000, kein_vuv=True))
    _gesamt_abzuege(base, "ff", handwerker=1000000, rechnung_unbar=True)
    st, erg = _req(base, "GET", "/fall/ff/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 0 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_faltung_rechnung_unbar_false_nullt_abs23(base):
    """Weg (ii) §35a Abs.5 S.3 im Fold: rechnung_unbar=false → Abs.2/3 (Handwerker) 0, NUR Minijob (510) zählt →
    festzusetzende_est 13414 = 1341400 Cent (statt mit Beleg niedriger)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "frf", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True))
    _gesamt_abzuege(base, "frf", minijob=280000, handwerker=1000000, rechnung_unbar=False)
    st, erg = _req(base, "GET", "/fall/frf/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 1341400 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_faltung_agb_kinder_staffel(base):
    """Weg (ii) §33 Abs.3 Staffelung im Fold: Lohn 60000 + agB 5000 + 2 Kinder → niedrigere zumutbare Belastung
    → höherer agB-Abzug (3313) → festzusetzende_est 12666 = 1266600 Cent. fam_anzahl_kinder REGEL-seitig in die
    zumutbar-Staffel. (Ersetzt die Standalone-agb-Kinder-Coverage.)"""
    catala = _catala_da()
    _gesamt_anlegen(base, "fak", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True))
    _gesamt_abzuege(base, "fak", agb=500000, kinder=2)
    st, erg = _req(base, "GET", "/fall/fak/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 1266600 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_gewinn_agb_gde_staffel(base):
    """§33-K2-Fix: der §33-zumutbar-Staffel-GdE (gde-Zwilling api.py) enthält jetzt einkuenfte_gewinn (§2 Abs.3 SdE =
    ALLE Arten). Reiner Gewinnfall (§§13-18 Gewinn 80000, KEIN §19/§21) + agB 10000, 0 Kinder → zumutbare Belastung
    auf GdE 80000 (7 %-Staffel), NICHT auf die frühere gewinn-lose GdE 0 (die den vollen agB durchließ = Under-tax).
    Belegt den geschlossenen §33-K2-Under-Tax (pre-existing seit §§13-18): festzusetzende_est STEIGT auf 2054600 Cent."""
    catala = _catala_da()
    _gesamt_anlegen(base, "fgab", _gesamt_kegel(0, kein_vuv=True, gewinn=8000000, kein_gewinn=False))
    _gesamt_abzuege(base, "fgab", agb=1000000)
    st, erg = _req(base, "GET", "/fall/fgab/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 2054600 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_gewinn_spende_deckel(base):
    """§33-K2-Fix (Kehrseite §10b): der §10b-20%-Spenden-Deckel nutzt jetzt den vollen gde-Zwilling inkl. gewinn.
    Gewinn 80000 + Spende 10000 → Deckel 20 % × 80000 = 16000 ≥ 10000 → volle Spende abziehbar, NICHT die frühere
    gewinn-lose GdE 0 (Deckel 0 → keine Spende = Over-tax). festzusetzende_est FÄLLT auf den korrekten Wert 1848800 Cent
    (18488 vs. gewinn-allein 20546 — die volle Spende 10000 mindert das Einkommen)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "fgsp", _gesamt_kegel(0, kein_vuv=True, gewinn=8000000, kein_gewinn=False))
    _gesamt_abzuege(base, "fgsp", spende=1000000)
    st, erg = _req(base, "GET", "/fall/fgsp/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1848800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_gewinn_24a_bemessung(base):
    """K2-Sweep-Fix §24a: die Altersentlastungsbetrag-Bemessung (positive_andere_einkuenfte) enthält jetzt
    einkuenfte_gewinn (§24a S.1: Arbeitslohn + positive Summe der Nicht-§19-Einkünfte; §§13-18-Gewinn NICHT in den
    S.2-Ausnahmen = nur Versorgungsbez./Leibrenten). Senior (geburtsjahr 1958 → Kohorte 2023: 14 %/665) mit reinem
    Gewinn 30000, KEIN Lohn/V+V → Bemessung 30000 → Altersentlastungsbetrag min(14%×30000, 665) = 665 (vorher
    gewinn-los 0). GdE 30000 − 665 → festzusetzende_est 410500 Cent (4105, vorher über-taxt 4293). Über-tax-Fix,
    gedeckelt auf den Kohorten-Höchstbetrag (kein Under-tax-Risiko)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "fg24a", _gesamt_kegel(0, kein_vuv=True, gewinn=3000000, kein_gewinn=False))
    _gesamt_abzuege(base, "fg24a", geburtsjahr=1958)
    st, erg = _req(base, "GET", "/fall/fg24a/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 410500 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


@pytest.mark.parametrize("geburtsjahr,erwartet_cent,label", [
    (1958, 410500, "eligible-67"),         # Folgejahr 2023 ≤ VZ2025 → §24a 665 (14 %-Kohorte 2023), GdE 30000−665
    (1960, 411600, "eligible-grenze"),     # Folgejahr 2025 == VZ2025 → §24a 627 (Kohorte 2025), eligible (Grenze)
    (1961, 429300, "gated-grenze"),        # Folgejahr 2026 > VZ2025 → §24a 0 (Gate greift, noch nicht 64+ vor VZ-Beginn)
    (1990, 429300, "gated-under64"),       # Folgejahr 2055 > VZ2025 → §24a 0 (35-Jähriger, Phantom-Zeroing = Under-tax-Fix)
])
def test_gesamt_p24a_64plus_gate(base, geburtsjahr, erwartet_cent, label):
    """§ 24a S. 3 64+-GATE (Under-tax-Fix, non-vacuous): der Altersentlastungsbetrag wird erst gewährt, wenn das
    64. Lj VOR Beginn des VZ vollendet ist (maßgebendes Folgejahr geburtsjahr+65 ≤ VZ). Reiner Gewinnfall 30000 +
    geburtsjahr. eligible (1958/1960) → §24a gewährt (est niedriger); GATED (1961/1990) → §24a 0 → est 429300 (=
    Gewinn 30000 ohne §24a). ⚠ EXAKTE GATE-GRENZE geb1960 (Folgejahr 2025==VZ, eligible, 411600) vs geb1961
    (2026>VZ, gated, 429300) = 177 € Differenz = der § 24a-627-Effekt. Vor dem Fix bekam JEDES geburtsjahr §24a
    (geb1990/alter35 → phantom 57 € = Under-tax). ⚠ Jan-1-Kante (born-1961-01-01 vollendet 64. Lj am 31.12.2024 →
    eigentlich VZ2025-eligible) → geburtsjahr-only-Näherung denied = winzige Over-tax, K2-konservativ = akzeptiert."""
    catala = _catala_da()
    fid = f"p24g{geburtsjahr}"
    _gesamt_anlegen(base, fid, _gesamt_kegel(0, kein_vuv=True, kein_gewinn=False, gewinn=3000000,
                    geburtsjahr=geburtsjahr))
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == erwartet_cent and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_faltung_24a_24b_freibetraege(base):
    """Weg (ii) Stage 2: §24a Altersentlastungsbetrag + §24b Entlastungsbetrag Alleinerziehende im gefalteten
    Ring (§2 Abs.3 GdE-mindernd). Senior (geburtsjahr 1958 → Kohorte 2023 → 14 %/665; Bemessung Arbeitslohn 30000
    + positive V+V 10000 → 14 %×40000=5600 gedeckelt auf 665) + alleinerziehend (1 Kind → §24b 4260). ns 28770 +
    vv 10000 − 665 − 4260 → festzusetzende_est 5411 = 541100 Cent (Ref ohne Freibeträge 6919)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "f24", _gesamt_kegel(1000000, bruttolohn=3000000))   # §21 10000 + §19 30000
    _gesamt_abzuege(base, "f24", geburtsjahr=1958, alleinerziehend=True, kinder=1)
    st, erg = _req(base, "GET", "/fall/f24/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 541100 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_faltung_24b_alleinerziehend_gate(base):
    """Weg (ii) §24b Abs.3-Flag: fam_alleinstehend=true (1 Kind) → §24b 4260 mindert die GdE →
    festzusetzende_est 5609 = 560900 Cent (vs. ohne §24b = 6919). fam_alleinstehend IST die §24b-Abs.3-Bedingung
    (fragetext „ohne anderen Erwachsenen im Haushalt")."""
    catala = _catala_da()
    _gesamt_anlegen(base, "f24b", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True))
    _gesamt_abzuege(base, "f24b", alleinerziehend=True, kinder=1)
    st, erg = _req(base, "GET", "/fall/f24b/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 560900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_faltung_24a_kein_geburtsjahr_fail_safe(base):
    """Weg (ii) §24a fail-safe: geburtsjahr NICHT erfasst → §24a 0 (kein Phantom-Freibetrag ohne Kohorte) →
    festzusetzende_est = wie ohne Freibetrag = 6919 = 691900 Cent (reiner §19-Fall, kein alleinerziehend)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "f24n", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True))
    _gesamt_abzuege(base, "f24n")   # kein geburtsjahr, kein alleinerziehend
    st, erg = _req(base, "GET", "/fall/f24n/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 691900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_faltung_31_kindergeld_besser(base):
    """Weg (ii) Stage 2 §31 Familienleistungsausgleich — Zweig KINDERGELD besser (Regelfall Mitteleinkommen):
    einzel Lohn 40000, 1 Kind → Kinderfreibetrag-Ersparnis (6919 − 5448 = 1471) < Kindergeld (3060) → Kindergeld
    gewinnt → festzusetzende_est = est_ohne_Freibetrag 6919 = 691900 Cent (der Freibetrag wird NICHT angesetzt,
    Kindergeld bleibt). Belegt: bei Kindern greift § 31, ändert den Bescheid aber NICHT wenn Kindergeld günstiger."""
    catala = _catala_da()
    _gesamt_anlegen(base, "f31k", _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True))
    _gesamt_abzuege(base, "f31k", kinder=1)
    st, erg = _req(base, "GET", "/fall/f31k/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 691900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_faltung_31_freibetrag_besser(base):
    """Weg (ii) §31 — Zweig KINDERFREIBETRAG besser (Hocheinkommen): Zusammenveranlagung (Einverdiener) Lohn
    150000, 1 Kind → Freibetrag 9600 (§32 Abs.6 verdoppelt), Ersparnis (40628 − 36596 = 4032) > Kindergeld 3060
    → Freibetrag gewinnt → est_mit_Freibetrag 36596 + Kindergeld-Hinzurechnung (§31 S.4) 3060 = 39656 = 3965600
    Cent. NON-VACUOUS Gegenzweig zu §31_kindergeld_besser (Instructor-K2: beide Günstiger-Ausgänge belegt)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "f31f", _gesamt_kegel(0, bruttolohn=15000000, kein_vuv=True,
                                                veranlagung="zusammen", bruttolohn_partner=0,
                                                person_b_idnr="12345678901"))
    _gesamt_abzuege(base, "f31f", kinder=1)
    st, erg = _req(base, "GET", "/fall/f31f/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 3965600 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def _rentner_kegel(renten_art="gesetzliche_rente", jahresrente=2000000, beginn=2025, alter=0,
                   gdb=0, hilflos=False, pflegegrad=0, gepflegter_hilflos=False, hinterbliebenen=False,
                   veranlagung="einzel", rentenfreibetrag=None, gdb_partner=0, hilflos_partner=False,
                   renten_art_partner=None, jahresrente_partner=0, beginn_partner=2025,
                   alter_partner=0, rentenfreibetrag_partner=None, gewinn=0, vg=0, kein_gewinn=True,
                   betriebseinnahmen=0, sonstige_betriebsausgaben=0, afa_jahresbetrag=0,
                   betriebsart=None, gewst_messbetrag=0, gewst_hebesatz=0, verlustvortrag_bestand=0,
                   gewinnanteil=0, verg_taetigkeit=0, verg_darlehen=0, verg_ueberlassung=0,
                   geburtsjahr=0, antrag_erm=False, berufsunfaehig=False, einmal_genutzt=False):
    """rentner_gesamt-Kegel: § 22 (renten_art/jahresrente Cent/beginn/alter) + § 33b-Block + Flags
    (kein_sonstige=False = Rente IST § 22-sonstige; kein_kap/vuv=True). rentenfreibetrag (Cent) nur
    aa-Folgejahr; Partner-Behinderung + Partner-Rente (renten_art_partner gesetzt) nur zusammen (#4b).
    gewinn (§ 15/§ 18 laufender Gewinn, einkuenfte_gewinn, OPTIONAL/CENT) + vg (§ 16-Veräußerungsgewinn,
    rentner_veraeusserungsgewinn, OPTIONAL/CENT, 2-I) = 0 default → Feld absent (absent → 0, over-tax-safe); > 0
    nur mit kein_gewinn=False (sonst flag_konsistenz_offen). kein_gewinn (§ 2 Abs. 1 Nr. 1-3) = True default."""
    k = [("rentner_renten_art", renten_art), ("rentner_jahresrente", jahresrente),
         ("rentner_renten_beginn_jahr", beginn), ("rentner_alter_bei_rentenbeginn", alter),
         ("rentner_grad_der_behinderung", gdb), ("rentner_hilflos_blind_taubblind", hilflos),
         ("rentner_pflegegrad", pflegegrad), ("rentner_gepflegter_hilflos", gepflegter_hilflos),
         ("rentner_hinterbliebenenbezuege", hinterbliebenen), ("veranlagung", veranlagung),
         ("kein_gewinn", kein_gewinn), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False)]
    if gewinn:                                     # § 15/§ 18 laufender Gewinn (2-I, optional)
        k.append(("einkuenfte_gewinn", gewinn))
    for _fid, _w in (("betriebseinnahmen", betriebseinnahmen),           # § 4 Abs. 3 EÜR-Komponenten (2a Scope-A, cent, optional)
                     ("sonstige_betriebsausgaben", sonstige_betriebsausgaben),
                     ("afa_jahresbetrag", afa_jahresbetrag)):
        if _w:
            k.append((_fid, _w))
    if betriebsart is not None:                     # gewinn_betriebsart-Weiche (§ 35-Zähler-Gating gewerbe)
        k.append(("gewinn_betriebsart", betriebsart))
    if gewst_messbetrag:                            # § 35 GewSt-Anrechnung im Rentner-Ring (opt-in)
        k.append(("gewst_messbetrag", gewst_messbetrag))
    if gewst_hebesatz:
        k.append(("gewst_hebesatz", gewst_hebesatz))
    if verlustvortrag_bestand:                      # § 10d Abs. 2 Verlustvortrag im Rentner-Ring (opt-in, cent)
        k.append(("verlustvortrag_bestand", verlustvortrag_bestand))
    if vg:                                          # § 16 Veräußerungsgewinn (2-I, optional)
        k.append(("rentner_veraeusserungsgewinn", vg))
    for _mf, _mv in (("gewinnanteil", gewinnanteil), ("verguetung_taetigkeit", verg_taetigkeit),  # § 15 Nr. 2 Mitunternehmer (cent, opt.)
                     ("verguetung_darlehen", verg_darlehen), ("verguetung_ueberlassung", verg_ueberlassung)):
        if _mv:
            k.append((_mf, _mv))
    for _af, _av in (("antrag_ermaessigter_satz", antrag_erm), ("dauernd_berufsunfaehig", berufsunfaehig),  # § 34 Abs. 3 Chooser-Flags
                     ("ermaessigung_einmal_genutzt", einmal_genutzt)):
        if _av:
            k.append((_af, _av))
    if geburtsjahr:
        k.append(("geburtsjahr", geburtsjahr))
    if rentenfreibetrag is not None:
        k.append(("rentner_rentenfreibetrag", rentenfreibetrag))
    if gdb_partner:
        k.append(("rentner_grad_der_behinderung_partner", gdb_partner))
    if hilflos_partner:
        k.append(("rentner_hilflos_blind_taubblind_partner", hilflos_partner))
    if renten_art_partner is not None:            # § 22-Rente Person B (#4b)
        k += [("rentner_renten_art_partner", renten_art_partner),
              ("rentner_jahresrente_partner", jahresrente_partner),
              ("rentner_renten_beginn_jahr_partner", beginn_partner),
              ("rentner_alter_bei_rentenbeginn_partner", alter_partner)]
        if rentenfreibetrag_partner is not None:
            k.append(("rentner_rentenfreibetrag_partner", rentenfreibetrag_partner))
    return k


def _rentner_anlegen(base, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


def test_rentner_gesetzl_erstjahr(base):
    """§ 22 gesetzl. Rente Erstjahr 20000 @ 83,5 % Kohorte → einkuenfte_sonstige 16598 →
    festzusetzende_est 811 = 81100 Cent (dev-2 Kreuzprobe S1)."""
    catala = _catala_da()
    _rentner_anlegen(base, "r1", _rentner_kegel(jahresrente=2000000, beginn=2025))
    st, erg = _req(base, "GET", "/fall/r1/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 81100 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


@pytest.mark.parametrize("vg_cent,erwartet_cent", [
    (4000000,  0),        # vg 40000 € → FB 45000 > vg → netto_vg 0 → kein Fünftel (Guard), KEIN Phantom-Verlust
    # ⚠ § 34 Abs. 1 S. 3 (verbleibendes zvE −36<0 ∧ zvE>0) → 5×Tarif(54964//5=10992); 10992 < GfB 12096 → 0. KEIN Bug.
    (10000000, 0),        # vg 100000 € → netto 55000 → § 34 Abs. 1 S. 3 → 0 (war progressiv 12495 = Over-tax-Korrektur)
    (15000000, 1304000),  # vg 150000 € → netto 119000 → § 34 Abs. 1 S. 3 → 5×Tarif(23792)=13040 (war 39052)
    (18100000, 3065000),  # vg 181000 € → netto 181000 → § 34 Abs. 1 S. 3 → 5×Tarif(36192)=30650 (war 65092)
])
def test_rentner_p16_4_veraeusserungsgewinn(base, vg_cent, erwartet_cent):
    """§ 16 Abs. 4 Betriebsveräußerungs-Freibetrag (2-I) im Rentner-Ring: rentner_veraeusserungsgewinn (kein
    laufender Gewinn, keine Rente) → catala_p16_4_freibetrag mindert VOR § 2 (netto = max(0, vg − FB), Cap bei 0)
    → einkuenfte_gewinn → festzusetzende_est. kein_gewinn=False (es LIEGT Gewinn vor). Deckt die 3 § 16-Abs.4-
    Brackets (voller FB / Abschmelzung / FB=0) + den Cap-Fall (FB > vg → 0). Werte unabhängig gg. catala_gesamt."""
    catala = _catala_da()
    fid = f"rvg{vg_cent}"
    _rentner_anlegen(base, fid, _rentner_kegel(jahresrente=0, vg=vg_cent, kein_gewinn=False))
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == erwartet_cent and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_gewinn_plus_veraeusserung_additiv(base):
    """K2-Akkumulations-Lock (Instructor-C): laufender § 15/§ 18-Gewinn (einkuenfte_gewinn 30000) + § 16-Ver-
    äußerungsgewinn (vg 100000 → netto 55000 nach FB) fließen ADDITIV in DIESELBE § 2-Einkunftsart (§ 16 Abs. 1
    „gehören auch") → einkuenfte_gewinn 85000. § 34 Abs. 1 S. 2 (ao=netto_vg 55000, verbleibendes zvE 29964>0):
    laufender 30000 progressiv, NUR die vg geglättet → festzusetzende_est 2097800 Cent (war voll-progressiv 2477200 =
    Over-tax-Korrektur). Belegt: der Fold summiert (kein Assignment-Bug) UND nur die vg kriegt Fünftel."""
    catala = _catala_da()
    _rentner_anlegen(base, "rgva", _rentner_kegel(jahresrente=0, gewinn=3000000, vg=10000000, kein_gewinn=False))
    st, erg = _req(base, "GET", "/fall/rgva/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 2097800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_mitunternehmer(base):
    """§ 15 Abs. 1 S. 1 Nr. 2 Mitunternehmer im RENTNER-Ring (Rentner-mit-PersG-Beteiligung): gesetzl. Rente 20000
    (→ einkuenfte_sonstige 16598) + Gewinnanteil 30000 (betriebsart gewerbe) → einkuenfte_mitunternehmer 30000 →
    einkuenfte_gewinn 30000 → catala_gesamt summiert (§ 2 Abs. 3): GdE 46598 → festzusetzende_est 949200 Cent.
    Belegt: die Mitunternehmer-Naht wirkt in BEIDEN Ringen (Rentner-mit-Mitunternehmer)."""
    catala = _catala_da()
    _rentner_anlegen(base, "rmu", _rentner_kegel(jahresrente=2000000, beginn=2025, kein_gewinn=False,
                     betriebsart="gewerbe", gewinnanteil=3000000))
    st, erg = _req(base, "GET", "/fall/rmu/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 949200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_rente_plus_veraeusserung(base):
    """§ 22 Rente + § 16 Veräußerungsgewinn im selben Ring: gesetzl. Rente 20000 (→ einkuenfte_sonstige 16598)
    + vg 100000 (→ netto 55000 nach § 16 Abs. 4-FB) → catala_gesamt summiert (§ 2 Abs. 3). § 34 Abs. 1 S. 2 (ao=netto_vg
    55000, verbleibendes zvE 16562>0): Rente progressiv, nur die vg geglättet → festzusetzende_est 1486100 Cent (war
    voll-progressiv 1914400). Belegt: der § 16-vg addiert sich zur Rente UND kriegt Fünftel (Rente bleibt progressiv)."""
    catala = _catala_da()
    _rentner_anlegen(base, "rrv", _rentner_kegel(jahresrente=2000000, beginn=2025, vg=10000000, kein_gewinn=False))
    st, erg = _req(base, "GET", "/fall/rrv/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1486100 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_veraeusserung_flag_widerspruch(base):
    """K2 (Guard non-vacuous, 2-I JOINT-D): kein_gewinn=true (behauptet keine Gewinneinkünfte) UND
    rentner_veraeusserungsgewinn > 0 bestätigt → Widerspruch surfacen (flag_konsistenz_offen), keine still
    übergangene § 16-Einkunftsart. Belegt den flag_check-Fix kein_gewinn → [..., rentner_veraeusserungsgewinn]
    (dev-2) zusammen mit der fremd_arten-Removal (dev-1) — ohne beide entweder Sperre oder stille K2."""
    _rentner_anlegen(base, "rvw", _rentner_kegel(jahresrente=0, vg=15000000, kein_gewinn=True))
    st, erg = _req(base, "GET", "/fall/rvw/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "flag_konsistenz_offen"


def test_rentner_euer_plus_veraeusserung(base):
    """Scope-A (2a im Rentner-Ring) + Akkumulation: laufender § 15/§ 18-Gewinn KOMPONENTENWEISE via EÜR
    (Betriebseinnahmen 80000 − sonstige BA 20000 − AfA 10000 = 50000) + § 16-Veräußerungsgewinn (vg 100000 →
    netto 55000 nach § 16 Abs. 4-FB) → einkuenfte_gewinn 105000. § 34 Abs. 1 S. 2 (ao=netto_vg 55000, verbleibendes
    zvE 49964>0): EÜR-Gewinn 50000 progressiv, nur die vg geglättet → festzusetzende_est 3124800 Cent (war voll-
    progressiv 3317200). Belegt: _laufender_gewinn (EÜR) greift im Rentner-Pfad UND nur die vg kriegt Fünftel."""
    catala = _catala_da()
    _rentner_anlegen(base, "reuv", _rentner_kegel(jahresrente=0, kein_gewinn=False, vg=10000000,
                     betriebseinnahmen=8000000, sonstige_betriebsausgaben=2000000, afa_jahresbetrag=1000000))
    st, erg = _req(base, "GET", "/fall/reuv/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 3124800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_p35_s22_im_nenner(base):
    """§ 35 GewSt-Anrechnung im RENTNER-Ring + § 22-im-Nenner (K2, lockt die in S1 untestbare Formel): Rentner mit
    § 22-Rente 20000 (→ einkuenfte_sonstige 16598) + laufendem Gewerbe-Gewinn 50000 + GewSt-Messbetrag 8000 €
    Hebesatz 500 % → Ermäßigungshöchstbetrag (Deckel 3) bindet mit Ratio 50000/(16598+50000) → § 35 12800 →
    festzusetzende_est 4250 = 425000 Cent. BEWEIS § 22 IM Nenner: ohne die Rente im Nenner (Ratio 1.0) wäre § 35
    17050 → est 0 ≠ 425000 → Assertion bräche. Anders als der gesamt-Ring (sonstige=0) ist § 22 hier ECHT im Nenner."""
    catala = _catala_da()
    _rentner_anlegen(base, "rp35n", _rentner_kegel(jahresrente=2000000, beginn=2025, kein_gewinn=False,
                     gewinn=5000000, betriebsart="gewerbe", gewst_messbetrag=800000, gewst_hebesatz=500))
    st, erg = _req(base, "GET", "/fall/rp35n/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 425000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_p35_basis(base):
    """§ 35 im Rentner-Ring, Deckel-1 (4×Messbetrag) bindet: Rentner-Rente 20000 + Gewerbe-Gewinn 50000 + MB 2000 €
    Hebesatz 450 % → § 35 = min(4×2000=8000, 2000×450%=9000, Deckel-3) = 8000 → festzusetzende_est 9050 = 905000
    Cent (= tarifliche ESt 17050 − § 35 8000). Belegt: der § 35-Fold greift auch im Rentner-Ring (single-computation,
    kein § 31-Günstiger)."""
    catala = _catala_da()
    _rentner_anlegen(base, "rp35b", _rentner_kegel(jahresrente=2000000, beginn=2025, kein_gewinn=False,
                     gewinn=5000000, betriebsart="gewerbe", gewst_messbetrag=200000, gewst_hebesatz=450))
    st, erg = _req(base, "GET", "/fall/rp35b/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 905000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_p35_selbstaendig_kein_credit(base):
    """§ 35 gilt NUR Gewerbe (§ 15) — auch im Rentner-Ring: Rente 20000 + § 18-selbständiger Gewinn 50000 +
    Messbetrag+Hebesatz → Zähler 0 (§ 18 nicht gewerbesteuerpflichtig) → § 35 0 → festzusetzende_est 17050 =
    1705000 Cent (= ohne § 35). Belegt: kein § 35-Credit für Nicht-Gewerbe im Rentner-Ring (kein stiller Über-Credit)."""
    catala = _catala_da()
    _rentner_anlegen(base, "rp35s", _rentner_kegel(jahresrente=2000000, beginn=2025, kein_gewinn=False,
                     gewinn=5000000, betriebsart="selbstaendig", gewst_messbetrag=200000, gewst_hebesatz=450))
    st, erg = _req(base, "GET", "/fall/rp35s/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1705000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_p10d_verlustvortrag(base):
    """§ 10d Abs. 2 Verlustvortrag im RENTNER-Ring (lockt den rentner-§10d-Pfad — Symmetrie-Argument reicht nach
    der Register-B/§22-Nenner-Lehre NICHT): Rentner mit § 22-Rente 40000 (→ einkuenfte_sonstige 33298) +
    festgestellter Verlustvortrag 10000 → verlustabzug 10000 (< GdE 33298) mindert die rentner-GdE VORRANGIG →
    sonstige_abzuege_vom_einkommen → zvE 23298 → festzusetzende_est 2469 = 246900 Cent, NIEDRIGER als ohne § 10d
    (524800). Belegt: der Verlustvortrag-Fold greift auch im Rentner-Ring (voller rentner-GdE, est-wirksam)."""
    catala = _catala_da()
    _rentner_anlegen(base, "rp10d", _rentner_kegel(jahresrente=4000000, beginn=2025, verlustvortrag_bestand=1000000))
    st, erg = _req(base, "GET", "/fall/rp10d/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 246900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_private_leibrente(base):
    """§ 22 bb private Leibrente 80000 @ Alter 65 (Ertragsanteil 18 %) → es 14298 → 346 = 34600 Cent (S4)."""
    catala = _catala_da()
    _rentner_anlegen(base, "r4", _rentner_kegel(renten_art="private_leibrente", jahresrente=8000000, alter=65))
    st, erg = _req(base, "GET", "/fall/r4/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 34600 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_behinderung(base):
    """§ 22 + § 33b: gesetzl. Rente Erstjahr (es 16598) + GdB 50 (Pauschbetrag 1140 agB) → 568 = 56800 Cent (S5)."""
    catala = _catala_da()
    _rentner_anlegen(base, "r5", _rentner_kegel(jahresrente=2000000, beginn=2025, gdb=50))
    st, erg = _req(base, "GET", "/fall/r5/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 56800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_folgejahr_fixierung_offen(base):
    """K2-KERN (S3): aa-Folgejahr (Rentenbeginn 2015 < VZ) OHNE fixierten Rentenfreibetrag → fail-closed
    rentenfreibetrag_fixierung_offen (kein %×aktuelle-Rente, das würde Rentenerhöhungen unterbesteuern)."""
    _rentner_anlegen(base, "r3", _rentner_kegel(jahresrente=2100000, beginn=2015))   # kein rentenfreibetrag
    st, erg = _req(base, "GET", "/fall/r3/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "rentenfreibetrag_fixierung_offen"


def test_rentner_folgejahr_mit_rentenfreibetrag(base):
    """aa-Folgejahr MIT fixiertem Rentenfreibetrag 6000: (21000 − 6000) − 102 → es 14898 → 458 = 45800 Cent (S2)."""
    catala = _catala_da()
    _rentner_anlegen(base, "r2", _rentner_kegel(jahresrente=2100000, beginn=2015, rentenfreibetrag=600000))
    st, erg = _req(base, "GET", "/fall/r2/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 45800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_partner_behinderung_ohne_zusammen_gesperrt(base):
    """partner_check LIVE (K2): Partner-Behinderung (GdB 60) gesetzt, aber veranlagung=einzel → Widerspruch
    partner_konsistenz_offen (ein Einzelveranlagter hat keinen mitzuveranlagenden Ehe-/Lebenspartner)."""
    _rentner_anlegen(base, "rp", _rentner_kegel(jahresrente=2000000, beginn=2025,
                                                veranlagung="einzel", gdb_partner=60))
    st, erg = _req(base, "GET", "/fall/rp/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "partner_konsistenz_offen"


def test_rentner_zusammen_beide_renten(base):
    """#4b § 22-B: Rentner-Ehepaar zusammen, beide gesetzl. Rente Erstjahr 20000 @ 83,5 % → es je 16598,
    Σ einkuenfte_sonstige 33196 (Ertragsanteil JE PERSON, dann summiert) → catala_est(zusammen, Splitting) →
    festzusetzende_est 1622 = 162200 Cent. Belegt: die Ehegatten-Rente wird als weiterer § 22-Summand geführt."""
    catala = _catala_da()
    _rentner_anlegen(base, "rzb", _rentner_kegel(jahresrente=2000000, beginn=2025, veranlagung="zusammen",
                                       renten_art_partner="gesetzliche_rente", jahresrente_partner=2000000,
                                       beginn_partner=2025))
    st, erg = _req(base, "GET", "/fall/rzb/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 162200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_partner_fixierung_offen(base):
    """#4b K2 fail-closed: Ehegatten-Rente aa-Folgejahr (Rentenbeginn_partner 2015 < VZ) OHNE fixierten
    Rentenfreibetrag_partner, zusammen → rentenfreibetrag_fixierung_offen (dieselbe %×erhöhte-Rente-Sperre
    wie Person A, für den Ehegatten). Person A rechenbar (Erstjahr), der Ehegatte sperrt den Ring."""
    _rentner_anlegen(base, "rpf", _rentner_kegel(jahresrente=2000000, beginn=2025, veranlagung="zusammen",
                                       renten_art_partner="gesetzliche_rente", jahresrente_partner=2100000,
                                       beginn_partner=2015))   # kein rentenfreibetrag_partner
    st, erg = _req(base, "GET", "/fall/rpf/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "rentenfreibetrag_fixierung_offen"


def _rente_instanz_anlegen(base, fid, idx, renten_art, jahresrente, beginn=2025, alter=0,
                           rentenfreibetrag=None, weglassen=()):
    """Postet die Felder EINER weiteren Rente-Instanz (base__idx, idx>=2, Multi-Rente-§22 #6). Kern = 4
    Felder (renten_art/jahresrente/renten_beginn_jahr/alter_bei_rentenbeginn); rentenfreibetrag optional
    (nur aa-Folgejahr). weglassen = Kern-feld_ids, die NICHT gepostet werden (Unvollständig-K2-Test)."""
    paare = [("rentner_renten_art", renten_art), ("rentner_jahresrente", jahresrente),
             ("rentner_renten_beginn_jahr", beginn), ("rentner_alter_bei_rentenbeginn", alter)]
    if rentenfreibetrag is not None:
        paare.append(("rentner_rentenfreibetrag", rentenfreibetrag))
    for feld, wert in paare:
        if feld in weglassen:
            continue
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(f"{feld}__{idx}", wert))
        assert st == 201


def test_rentner_multi_rente_zwei_renten(base):
    """#6 Multi-Rente § 22: zwei Renten der Person A — gesetzl. Erstjahr 20000 (aa, es 16598) + private
    Leibrente 80000 @ Alter 65 (bb Ertragsanteil 18 %, es 14298) → est_mapping.instanzen("rente") summiert
    JE RENTE den eigenen aa/bb-Anteil → einkuenfte_sonstige 30896 → festzusetzende_est 4549 = 454900 Cent
    (Ref nur Rente 1 = 811). Belegt: der Ertragsanteil wird JE Rente-Art bestimmt, dann summiert."""
    catala = _catala_da()
    _rentner_anlegen(base, "mr1", _rentner_kegel(jahresrente=2000000, beginn=2025))   # Rente 1 = Basis
    _rente_instanz_anlegen(base, "mr1", 2, "private_leibrente", 8000000, alter=65)    # Rente 2 = __2
    st, erg = _req(base, "GET", "/fall/mr1/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 454900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_multi_rente_folgejahr_mit_freibetrag(base):
    """#6 per-Instanz-Rentenfreibetrag: Rente 1 gesetzl. Erstjahr (es 16598) + Rente 2 gesetzl. aa-Folgejahr
    (Beginn 2015, jahresrente 21000, rentenfreibetrag__2 6000 → (21000−6000)−102 = 14898) → Σ 31496 →
    festzusetzende_est 4722 = 472200 Cent. Belegt: der fixierte Rentenfreibetrag wird JE Rente-Instanz gelesen."""
    catala = _catala_da()
    _rentner_anlegen(base, "mr2", _rentner_kegel(jahresrente=2000000, beginn=2025))
    _rente_instanz_anlegen(base, "mr2", 2, "gesetzliche_rente", 2100000, beginn=2015, rentenfreibetrag=600000)
    st, erg = _req(base, "GET", "/fall/mr2/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 472200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_multi_rente_instanz_unvollstaendig(base):
    """#6 K2 fail-closed: Rente 2 unvollständig (nur rentner_jahresrente__2, die 3 anderen Kern-Felder fehlen) →
    rente_instanz_offen, NIE ein still zu niedriges §22-Σ. Der per-Instanz-Guard verlangt alle 4 Kern-Felder
    (renten_art/jahresrente/beginn/alter) present + bestätigt je Zusatz-Rente (rentenfreibetrag bleibt optional)."""
    _rentner_anlegen(base, "mr3", _rentner_kegel(jahresrente=2000000, beginn=2025))
    _rente_instanz_anlegen(base, "mr3", 2, "gesetzliche_rente", 2000000,
                           weglassen=("rentner_renten_art", "rentner_renten_beginn_jahr",
                                      "rentner_alter_bei_rentenbeginn"))
    st, erg = _req(base, "GET", "/fall/mr3/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "rente_instanz_offen"


def test_rentner_multi_rente_fixierung_per_instanz(base):
    """#6 K2-KERN per-Instanz-Fixierung: Rente 1 rechenbar (Erstjahr), aber Rente 2 = aa-Folgejahr (Beginn__2
    2015 < VZ) OHNE fixierten Rentenfreibetrag__2 → rentenfreibetrag_fixierung_offen. Die aa-Folgejahr-ohne-RF-
    Sperre greift JE Rente-Instanz (nicht nur die Basis-Rente) — kein %×erhöhte-Rente für die Zweit-Rente."""
    _rentner_anlegen(base, "mr4", _rentner_kegel(jahresrente=2000000, beginn=2025))
    _rente_instanz_anlegen(base, "mr4", 2, "gesetzliche_rente", 2100000, beginn=2015)   # kein rentenfreibetrag__2
    st, erg = _req(base, "GET", "/fall/mr4/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "rentenfreibetrag_fixierung_offen"


def test_graph_uebersicht(base):
    """Read-only Desktop-Graph: Knoten = Regeln der Scheibe mit Status, Kanten = Feld→Regel mit
    Zustand. Ein Traverser-Aufruf, kein Bescheid, kein Schreibpfad."""
    fid = "g1"
    _req(base, "POST", "/fall", {"scheibe": "n_vor_gwg", "veranlagungszeitraum": 2025, "fall_id": fid})
    st, g = _req(base, "GET", f"/fall/{fid}/graph")
    assert st == 200
    _val("graph", g)
    rids = {k["regel_id"] for k in g["knoten"]}
    assert "p09_entfernungspauschale" in rids
    assert len(g["knoten"]) == 7          # EP + dHf + Verpflegung + Arbeitsmittel + VOR + GWG + KV/PV
    # frischer Fall: alle Kanten offen; beide Rollen vertreten
    assert all(k["zustand"] == "offen" for k in g["kanten"])
    assert any(k["rolle"] == "slot" for k in g["kanten"])
    assert any(k["rolle"] == "gate" for k in g["kanten"])
    # nach Bestätigung eines Felds → dessen Kante bestätigt (Store spiegelt sich im Graph)
    _req(base, "POST", f"/fall/{fid}/event", _laie("ep_arbeitstage", 220))
    st, g2 = _req(base, "GET", f"/fall/{fid}/graph")
    kante = next(k for k in g2["kanten"] if k["feld_id"] == "ep_arbeitstage")
    assert kante["zustand"] == "bestaetigt"
