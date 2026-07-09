# Formalisierungspipeline (Phase 2)

LLM schlaegt vor, deterministische Gates verifizieren, Mensch entscheidet.

## Key-Handling (verbindlich)
- `OPENROUTER_API_KEY` kommt ausschliesslich als Umgebungsvariable in die Session.
- Der Client liest den Key nur aus `os.environ`, nie aus Dateien; er wird nie
  ausgegeben. Jedes Logging laeuft durch `mask_key()` (`sk-or-***`).
- Fehlt der Key: sauberer Abbruch mit Hinweis, kein Prompt.
- `.claude/settings.json` verbietet Read auf `~/.config/taxgraph/**` und `**/.env*`;
  `.gitignore` schliesst `.env*` / `openrouter.env` aus.

## Rollen (`models.yaml`)
Formalisierer A, Formalisierer B und Judge stammen aus drei verschiedenen
Modellfamilien (dekorrelierte Fehler). Provider-Pinning ist Pflicht
(`allow_fallbacks: false`), nur unquantisierte westliche Hoster; niemals die
offiziellen chinesischen Endpoints. Slugs voll gepinnt, keine `latest`-Aliase.

## Gate-Kaskade
Extraktion (worker) -> Doppelformalisierung (A, B) -> Syntax -> Compiler-Typecheck
(clerk) -> extensionale Aequivalenz A vs B auf Input-Raster (clerk -> Python) ->
Round-Trip-Diff (Judge) -> Clerk-Tests -> Review-Queue.

## Ausfuehren
    PIPELINE_DRY_RUN=1 pipeline/.venv/bin/python pipeline/run_smoke.py   # ohne Key/Kosten
    pipeline/.venv/bin/python pipeline/run_smoke.py --real               # echte Calls

Dry-Run nutzt gemockte Modellantworten aus `fixtures/`; die deterministischen
Gates laufen echt (benoetigen `clerk` auf dem PATH, sonst SKIP).

Provenienz je Output: Rolle, Modell-Slug, Provider, models.yaml-Hash,
Prompt-Template-Version, Few-Shot-Set-Version, Timestamp, Tokens, Kosten.
Ein Lauf ist nur gueltig, wenn alle Rollen gegen denselben models.yaml-Hash liefen.
