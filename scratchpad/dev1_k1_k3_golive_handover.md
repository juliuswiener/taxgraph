# K1/K3-Live Go-Live — Handover-Sektion (dev-1, 2026-07-20)

Read-only-Recon, end-to-end gegen echten Code verifiziert. Beide Fronten sind Code-fertig — der
verbleibende Schritt ist reine Konfiguration/Cap, kein Bau.

## K3-Live: ORS-Entfernung (Arbeitsweg-km, § 9 Abs. 1 S. 3 Nr. 4 EStG)

**Julius' Schritt:** GENAU EINER. `$ORS_API_KEY` in `.env.maps` (Repo-Root, gitignored) eintragen —
kein Provider-/Modell-Entscheid nötig, Basis-URL ist hartkodiert (`https://api.openrouteservice.org`,
der offizielle ORS-Endpunkt). Reiner 1-Wert-Key-Schalter, 0 Entscheidung.

**Code-Readiness (alles fertig, Commit-Refs):**
- `server.py::_lade_env_dateien` (34e91f1) lädt `.env.maps` → `os.environ["ORS_API_KEY"]` (nur wenn
  noch nicht gesetzt — Prozess-Env gewinnt).
- `ors_client.py`: `_key()` liest exakt diesen Namen; `geocode()` (mit Shape-Guard seit 5a45fd1 —
  `.get()`-Kette statt Direktindex, malformte ORS-Antwort → `OrsNichtVerfuegbar` statt `KeyError`)
  + `_distanz_meter()` (driving-car/shortest) → echter Call.
- `api.py::entfernung()` (except-Verengung seit 5a45fd1: `(OrsNichtVerfuegbar, ImportError)`, ein
  echter Bug propagiert statt still zu 503 zu werden).

**Was der Nutzer sieht:** Adresse „von"/„nach" eingeben → Klick „Entfernung berechnen" → echter
Karten-Call → km-Wert kommt als VORLÄUFIGER Vorschlag (`herkunft=berechnet`, Badge „berechnet/maps")
ins Feld — Nutzer bestätigt/überschreibt (Zwei-Signal, nie automatisch in die Steuer). Kein Key /
Netzfehler → sauberer 503-Fallback auf manuelle km-Eingabe, nie Crash, nie Fake-km.

## K1-Live: /chat (KI-Berater) + kontoauszug-LLM-Klassifikator

**Julius' Schritt:** 0 Code, ABER 1 Config-Entscheidung — nicht nur der Key reicht. ALLE DREI in
`.env.llm` (Repo-Root, gitignored) eintragen:
- `LLM_API_KEY` (der Schlüssel)
- `LLM_API_BASE` (OpenAI-kompatibler Endpunkt des gewählten Providers, z. B. OpenRouter/OpenAI/eigener)
- `LLM_MODEL` (Modell-Slug im Provider-Format, z. B. `openai/gpt-4o` bei OpenRouter)

Julius muss Provider + Modell wählen — der Client ist bewusst provider-agnostisch (kein Anbieter
hartkodiert), das ist die eine Entscheidung, die K1 von K3 unterscheidet.

**Code-Readiness (alles fertig, Commit-Refs):**
- `llm_client.py` `complete()`-Refactor (ba2922b): EINE niedrig-level Wahrheit, OpenAI-kompatibler
  Chat-Completions-Call, provider-agnostisch. `_call()` prüft `LLM_API_KEY`+`LLM_API_BASE`+`LLM_MODEL`
  — fehlt einer der drei → `LlmNichtVerfuegbar` (nicht nur bei fehlendem Key).
- `api.py::chat()` → `_llm_vorschlaege()` → `llm_client.complete("chat", msgs)`, except verengt auf
  `(LlmNichtVerfuegbar, ImportError)` (9abb502) — Bug propagiert, kein 501-Fake bei echtem Logikfehler.
- `api.py::_kontoauszug_llm_klassifikator()` baut `KW.llm_klassifikator_factory(llm_client,
  "kontoauszug_klassifikation")` — Factory ruft `client.complete(role, msgs, fixture_id=)`, Signatur
  matcht `llm_client.complete` EXAKT (Interface-Mismatch aus dem K3-Recon ist seit ba2922b gelöst).
  Fängt NUR `LlmNichtVerfuegbar` → `None` (unklassifiziert), Bug propagiert.

**Sekundär-Risiko (nicht blockierend):** `_call()` sendet `response_format: {"type": "json_object"}`.
Unterstützt der gewählte Provider das nicht, scheitert der Call sauber (`LlmNichtVerfuegbar`, kein
Crash, kein Fake) — nur eben kein Vorschlag, bis Julius ggf. Provider/Modell wechselt. Kein Code-Fix
nötig, nur ggf. Provider-Wahl anpassen.

**Was live geht:** (1) `/chat` — Nutzer beschreibt Situation im Freitext, KI schlägt Feldwerte vor
(`herkunft=llm_vorschlag`, `zustand=vorläufig`, `signal_2=null`), Nutzer bestätigt (Zwei-Signal, Store-
Auflage A + Katalog-Check erzwingen das strukturell — die KI setzt nie selbst einen Wert). (2)
Kontoauszug-Upload — mehrdeutige Zeilen (kein Determinist-Treffer) bekommen zusätzlich einen LLM-
Klassifikationsversuch statt `unklassifiziert` zu bleiben, gleiches Zwei-Signal-Muster.

## Gemeinsam für beide

- Zwei-Signal-Sicherheitsmuster unverändert: LLM/Karten-Vorschlag ist IMMER vorläufig, nie automatisch
  in `/ergebnis`. Mensch bestätigt (`signal_2`) — erst dann zählt der Wert.
- Ohne Key/Config: sauberer Fallback (K1 → 501 + Erklär-Vertrag; K3 → 503 + manuelle Eingabe), NIE ein
  Mock-Call, NIE ein Fake-Wert, NIE ein Crash.
- Echter Logik-Bug (kein erwarteter Cap-Gate-Fall) propagiert bei beiden bewusst statt still geschluckt
  zu werden (except-Verengung auf die jeweilige `NichtVerfuegbar`-Klasse + `ImportError`).
