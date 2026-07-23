# DRAFT (Pre-Review, NICHT integriert) — dev-1s entfernung()-except-Verengung + ors_client-Shape-Guard

Revidierter 2-Teile-Fix (dev-3-Regression-Fund): OHNE Teil (a) würde die reine except-Verengung in
api.py einen aktuell sauber gefangenen malformten-ORS-Antwort-Fall (roher `KeyError` in
`geocode()`) von 503-Fallback auf ungefangenen 500-Crash umstellen. Reihenfolge ZWINGEND (a) vor (b).

## (a) ors_client.py::geocode() — Shape-Guard analog _distanz_meter (Z.57-59)

Vorher (Z.40-48):
```python
def geocode(adresse: str) -> list[float]:
    """Adresse → [lon, lat] (erstes Treffer-Feature). Über den API-Key im Query (Geocoding-Endpunkt)."""
    from urllib.parse import urlencode
    q = urlencode({"api_key": _key(), "text": adresse, "size": 1, "boundary.country": "DE"})
    j = _hole(f"{_BASE}/geocode/search?{q}")
    feats = (j or {}).get("features") or []
    if not feats:
        raise OrsNichtVerfuegbar(f"keine Koordinate für Adresse gefunden")
    return feats[0]["geometry"]["coordinates"]   # [lon, lat]
```

Nachher:
```python
def geocode(adresse: str) -> list[float]:
    """Adresse → [lon, lat] (erstes Treffer-Feature). Über den API-Key im Query (Geocoding-Endpunkt)."""
    from urllib.parse import urlencode
    q = urlencode({"api_key": _key(), "text": adresse, "size": 1, "boundary.country": "DE"})
    j = _hole(f"{_BASE}/geocode/search?{q}")
    feats = (j or {}).get("features") or []
    geom = (feats[0] if feats else {}).get("geometry") or {}
    coords = geom.get("coordinates")
    if not feats or not isinstance(coords, list) or len(coords) < 2:
        raise OrsNichtVerfuegbar("keine verwertbare Koordinate für Adresse gefunden")
    return coords   # [lon, lat]
```

Grund für `.get()`-Kette statt `feats[0]["geometry"]["coordinates"]`: `.get()` wirft nie KeyError/TypeError
selbst bei fehlendem/None/falsch-typisiertem `geometry`- oder `coordinates`-Feld — jede Malform-Variante
(fehlendes `geometry`, `geometry=null`, `coordinates` fehlt oder ist kein Array) landet einheitlich im
Shape-Check, NICHT als roher KeyError/TypeError. Analog zum bestehenden `_distanz_meter`-Guard
(`"summary" not in routes[0]`-Stil), nur mit `.get()`-Ketten statt `in`-Checks (robuster gegen
verschachtelte Malform, z. B. `geometry` selbst kein dict).

## (b) api.py::entfernung() — Import VOR try (NameError-frei) + except verengt

Vorher (Z. ~1472-1476):
```python
    try:
        import ors_client
        km = ors_client.entfernung_km(von, nach)
    except Exception:                                    # OrsNichtVerfuegbar / Import — sauberer Fallback
        return 503, ENTFERNUNG_FALLBACK
```

Nachher:
```python
    import ors_client
    try:
        km = ors_client.entfernung_km(von, nach)
    except (ors_client.OrsNichtVerfuegbar, ImportError):  # Cap-Gate/Netzfehler/Import → Erklär-Grenze;
        return 503, ENTFERNUNG_FALLBACK                   # ein echter Logik-Bug propagiert (K2, konsistent zu chat()/kontoauszug)
```

`import ors_client` bewusst VOR dem try (nicht mehr drin) — Grund: würde `import ors_client` selbst im try
bleiben UND das except `ors_client.OrsNichtVerfuegbar` referenzieren, könnte ein fehlschlagender Import
(Modul kaputt/fehlt) einen `NameError` beim Auswerten des except-Tupels auslösen (Name `ors_client` nie
gebunden) STATT sauber zu greifen — exakt das Muster, das `chat()` schon korrekt löst (dort steht
`import llm_client` ebenfalls VOR dem try, Z.1673). Import-Fehlschlag selbst propagiert jetzt ungefangen
(kein stiller Verschluck eines kaputten Moduls) — konsistent zur K2-Devise „ein echter Bug propagiert".

## Test-Plan (Pre-Draft, noch nicht gebaut)
Neuer Test in `tests/test_ors_client.py` oder `test_paket_b_e2e_http.py` (Zone-Zuordnung TBD): monkeypatch
`ors_client._hole` → liefert `{"features": [{"geometry": {}}]}` (geometry ohne coordinates) → `geocode()`
muss `OrsNichtVerfuegbar` werfen (NICHT `KeyError`). Zweiter Test: der bestehende Regressions-Beweis für
api.py::entfernung() — monkeypatch `ors_client.entfernung_km` wirft eine UNERWARTETE Exception (z. B.
`ValueError("bug")`) → `API.entfernung(...)` muss diese PROPAGIEREN (nicht zu 503 verschlucken) — beweist
die Verengung wirkt wie beabsichtigt.

## Diff-Umfang
~10 Zeilen ors_client.py + ~4 Zeilen api.py + 2 neue Tests. Kein Cap, kein Netz (alles über `_hole`/
`entfernung_km`-Monkeypatch wie im bestehenden `tests/test_ors_client.py`-Muster, kein echter ORS-Call).
