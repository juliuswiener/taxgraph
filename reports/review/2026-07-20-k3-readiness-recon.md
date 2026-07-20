# K3-Readiness-Recon (read-only, dev-1, 2026-07-20) — was fehlt zur Live-Schaltung

Ziel: K3 (externe Dienste) sofort live-schaltbar sobald Julius die Caps gibt. Read-only Landkarte, KEIN Bau.

## (a) entfernung / ORS — FAST FERTIG, fehlt nur der Key
| Baustein | Stand |
|---|---|
| `produkt/haut/ors_client.py` | ✅ KOMPLETT: geocode (Adresse→lon/lat, `boundary.country=DE`) + `driving-car preference=shortest` (§9 kürzeste Straßenverbindung) + km-Rundung. stdlib-urllib, `$ORS_API_KEY` aus Env, jeder Fehler → `OrsNichtVerfuegbar`. PII-bewusst (nur auf Nutzer-Klick). |
| `api.py::entfernung` Handler | ✅ KOMPLETT: fängt `OrsNichtVerfuegbar`/Import → **503 `ENTFERNUNG_FALLBACK`** (manuelle Eingabe), bei Erfolg VORLÄUFIGES `berechnet:maps`-Event (ersetzt aktives, signal_1 trägt Provenance OHNE Adressen). |
| `tests/test_ors_client.py` | ✅ KOMPLETT: **gemocktes urlopen (Fixture-Replay, $0, kein Live-Call)** — km-Faltung, Rundung, kein-Key→`OrsNichtVerfuegbar`. Der von Instructor gefragte „Test ohne echten Call" existiert schon. |

**Live-Switch braucht NUR:** `$ORS_API_KEY`. **KEIN Code nötig.**
✅ **`.env.maps`-Loader GEBAUT** (2026-07-20, Instructor-greenlit, key-unabhängig): `server._lade_env_dateien(root)`
lädt gitignored `.env.maps` (+ `.env.llm`) aus dem Repo-Root in `os.environ` — NUR unsetzte Schlüssel (Prozess-Env
gewinnt, kein Override; Sicherheits-Invariant getestet), fehlt/unlesbar = still no-op, nur in `main()` (nicht beim
Import → keine Test-Verschmutzung), Werte nie geloggt. Tests: `tests/test_env_loader.py` (4 grün, kein Netz/Key).
→ ORS ist jetzt **schlüsselfertig**: Julius legt den Key in `.env.maps` (Repo-Root), Server startet, fertig. (Shell-
Export/`settings.json`-`env` bleibt als Alternative — beides sticht die Datei.)

## (b) Kontoauszug — csv/json LIVE, pdf + LLM-Fallback offen
| Format/Pfad | Stand |
|---|---|
| **csv** | ✅ LIVE JETZT (kein Key): `parse_csv` → `klassifiziere_det` → vorläufiges `import:kontoauszug`-Event. |
| **json** (Transaktions-Liste) | ✅ LIVE JETZT: gleiche Pipeline. |
| Det-Klassifikator | ✅ KOMPLETT: `KATEGORIE_FELD` (handwerker→§35a Abs.3, dienstleistung→Abs.2, minijob→Abs.1, spende→§10b, vorsorge→§10) + konservative Keyword-Heuristik (nur eindeutig, sonst None). |
| PII-Maskierung | ✅ `maskiere` (IBAN/Kontonummer vor LLM/Speicherung). |
| `tests/test_kontoauszug_writer.py` | ✅ grün. |
| **pdf** | ⛔ **501-Stub** (`KONTOAUSZUG_PDF_501`). Live-Switch braucht einen **PDF→Transaktions-Parser (OCR/Layout, NICHT der deterministische Spalten-Parser)**. KEY-UNABHÄNGIG (ein Parser, kein externer Dienst) — baubar jederzeit, aber echtes Stück Arbeit (Team hat tesseract-Route, s. Memory). |
| **LLM-Klassifikator-Fallback** (mehrdeutige Zeilen, `klassifiziere_det`=None) | ⛔ GATED (`llm_klassifikator=None` im Handler). Factory `llm_klassifikator_factory(client, role, fixture_id=)` + `_LLM_PROMPT` + `_parse_llm_kategorie` existieren. |

### ⚠ Integrations-Lücke LLM-Klassifikator (konkret, für die Live-Schaltung):
`llm_klassifikator_factory` erwartet `client.complete(role, msgs, fixture_id=)` — **K1s `llm_client` exponiert aber `vorschlaege(freitext, katalog)`, KEIN `complete(role, msgs, fixture_id)`**. → Interface-MISMATCH. Zum Live-Schalten des LLM-Fallbacks:
1. `$LLM_API_KEY` (K1-Cap, dieselbe wie /chat).
2. **Client-Interface reconciliieren**: entweder `llm_client.complete(role, msgs, fixture_id)` ergänzen ODER einen Adapter (factory-Erwartung → llm_client-Form). Eine Wahrheit für den LLM-Call-Layer wäre sauberer (heute zwei Erwartungen: /chat=vorschlaege, kontoauszug=complete).
3. `llm_klassifikator = KW.llm_klassifikator_factory(client, role)` in den Handler-Aufruf verdrahten (statt None).

## LLM-Live-Wiring-Requirement (Instructor-adjudiziert 4418 — die EINE Code-Aufgabe für LLM-Live)
Heute ZWEI LLM-Client-Wahrheiten (widerspricht „eine-Wahrheit"): K1-Chat `llm_client.vorschlaege(freitext,
katalog)` vs kontoauszug `client.complete(role, msgs, fixture_id=)`. **Adjudizierte Reconcile-Richtung** (sobald
$LLM_API_KEY da):
1. `llm_client` exponiert EINE niedrig-level Methode **`complete(...)`** = der generische LLM-Call (die eine
   Wahrheit, dünner Client).
2. K1-Chat's `vorschlaege` wird auf `complete` **refactored** (Task-Wrapper OBEN DRAUF, nicht als Basis —
   `vorschlaege` ist zu task-spezifisch, um Fundament zu sein).
3. Der kontoauszug-LLM-Fallback baut ebenfalls auf `complete` (via `llm_klassifikator_factory`, die schon
   `client.complete(role, msgs, fixture_id)` erwartet) → `llm_klassifikator = KW.llm_klassifikator_factory(client,
   role)` in den `kontoauszug`-Handler verdrahten (statt `None`).
Ergebnis: EIN Client, EIN generischer Call, task-spezifische Wrapper (Vorschläge / Klassifikation) in den
Handlern. Das ist die saubere Schicht. Ohne diesen Reconcile bleibt der kontoauszug-LLM-Fallback nicht wirbar
(Interface-Mismatch).

## Zusammenfassung — Live-Schalt-Reihenfolge (sobald Julius Caps gibt)
1. **entfernung/ORS**: schlüsselfertig — Key in `.env.maps` (Repo-Root, .env-Loader gebaut) ODER exportieren → SOFORT live, 0 Code.
2. **/chat Live-LLM** (K1): `$LLM_API_KEY` + `$LLM_API_BASE` + `$LLM_MODEL` setzen → SOFORT live (0 Code, llm_client fertig).
3. **kontoauszug csv/json**: schon live, keine Aktion.
4. **kontoauszug LLM-Fallback**: `$LLM_API_KEY` + Client-Interface-Reconcile (klein) + Handler-Verdrahtung.
5. **kontoauszug pdf**: OCR/Layout-Parser (key-unabhängig, größer) — eigener Baustein.

**Fazit:** ORS + /chat-LLM sind reine Key-Schalter (Code fertig). kontoauszug-pdf + LLM-Fallback brauchen kleine bis mittlere Bau-Schritte. Kein Bau bis Julius-Cap (Instructor-Auflage).
