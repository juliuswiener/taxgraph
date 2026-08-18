"""OpenRouteService-Client für die Arbeitsweg-Entfernung (§ 9 Abs. 1 S. 3 Nr. 4 — kürzeste Straßen-
verbindung). stdlib-only (urllib), kein externes Paket. Der API-Key kommt AUSSCHLIESSLICH aus der
Umgebung ($ORS_API_KEY, lokal in der gitignored .env.maps) — nie im Repo, nie geloggt.

⚠ AUSGEHENDE PII-INTEGRATION: Adressen verlassen das Gerät an einen externen Dienst. Der Aufruf
passiert NUR auf ausdrückliche Nutzer-Aktion (Klick „Entfernung berechnen") und liefert einen
VORSCHLAG (herkunft=berechnet, vorläufig) — der Nutzer bestätigt/überschreibt (Zwei-Signal). Kein Key
oder Fehler → sauberer Fallback auf manuelle Eingabe (Aufrufer fängt OrsNichtVerfuegbar)."""
from __future__ import annotations

import json
import os
import urllib.request

_BASE = "https://api.openrouteservice.org"
_TIMEOUT = 8


class OrsNichtVerfuegbar(Exception):
    """Kein $ORS_API_KEY gesetzt oder der Dienst antwortete nicht verwertbar — Aufrufer fällt auf
    manuelle km-Eingabe zurück (nie crashen, nie einen Fake-Wert setzen)."""


def _key() -> str:
    k = os.environ.get("ORS_API_KEY", "").strip()
    if not k:
        raise OrsNichtVerfuegbar("kein ORS_API_KEY in der Umgebung (.env.maps nicht geladen?)")
    return k


def _hole(url: str, *, data: bytes | None = None, headers: dict | None = None) -> dict:
    req = urllib.request.Request(url, data=data, headers=headers or {}, method="POST" if data else "GET")
    try:
        with urllib.request.urlopen(req, timeout=_TIMEOUT) as r:
            return json.loads(r.read())
    except Exception as e:                       # Netz/HTTP/JSON — alles zum Fallback
        # `from None`, NICHT `from e` (Audit 2026-08-16, sec-ors-key-in-query-string): der
        # Geocoding-Aufruf trägt den Schlüssel im Query-String, und urllib.error.HTTPError führt
        # die vollständige angefragte URL in `.url` mit. Mit erhaltener Kette bliebe der
        # Schlüssel über `__cause__.url` ERREICHBAR — für alles, was Ausnahmen samt Attributen
        # serialisiert (Fehler-Meldedienste, Diagnose-Dumps). Im gedruckten Standard-Traceback
        # steht er NICHT; das war die erste, zu weit gehende Begründung dieser Zeile, und die
        # Mutationsprobe hat sie widerlegt (der Test dazu war grün, gleich was hier stand).
        #
        # Der Preis ist Diagnose: die ursprüngliche Ausnahme ist weg. Deshalb wandert ihr Typ
        # in die Meldung (wie bisher) — das reicht, um Netz von HTTP von JSON zu unterscheiden,
        # und mehr braucht ein Client, der ohnehin sauber auf manuelle Eingabe zurückfällt.
        #
        # WAS DAS NICHT LÖST: der Schlüssel steht weiterhin in den Zugriffsprotokollen des
        # Anbieters und jedes Proxys dazwischen. Ihn in den Authorization-Header zu verlegen
        # ist für /geocode/search NICHT belegt — dieser Endpunkt ist keine ORS-Route, sondern
        # eine vorgeschaltete Pelias-Instanz, und sämtliche offiziellen Beispiele dafür, auch
        # die des ORS-Supports, verwenden api_key im Query. Der Routing-Aufruf unten nutzt
        # bereits den Header. Offen für eine Prüfung gegen den echten Dienst.
        raise OrsNichtVerfuegbar(f"ORS-Aufruf fehlgeschlagen: {type(e).__name__}") from None


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


def _distanz_meter(von_lonlat: list[float], nach_lonlat: list[float]) -> float:
    """Routing driving-car, preference=shortest (§ 9 kürzeste Straßenverbindung) → Distanz in Metern."""
    body = json.dumps({"coordinates": [von_lonlat, nach_lonlat], "preference": "shortest",
                       "units": "m"}).encode("utf-8")
    j = _hole(f"{_BASE}/v2/directions/driving-car",
              data=body, headers={"Authorization": _key(), "Content-Type": "application/json"})
    routes = (j or {}).get("routes") or []
    if not routes or "summary" not in routes[0] or "distance" not in routes[0]["summary"]:
        raise OrsNichtVerfuegbar("ORS-Antwort ohne verwertbare Distanz")
    return float(routes[0]["summary"]["distance"])


def entfernung_km(von: str, nach: str) -> int:
    """Wohnung + erste Tätigkeitsstätte → kürzeste Straßen-km (ganzzahlig gerundet, § 9-Konvention).
    Wirft OrsNichtVerfuegbar bei fehlendem Key / Netz-/Antwort-Fehler — der Aufrufer fällt auf die
    manuelle Eingabe zurück und crasht nie."""
    meter = _distanz_meter(geocode(von), geocode(nach))
    return max(0, round(meter / 1000))
