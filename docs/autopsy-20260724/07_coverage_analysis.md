# 07 — Coverage Analysis

## 86% Total, With Critical Blind Spots

### Overall Coverage (from `coverage run`)

| Module | Statements | Missed | Coverage |
|--------|-----------|--------|----------|
| `golden/runner.py` | 593 | 88 | 85% |
| `produkt/haut/api.py` | 979 | 108 | 89% |
| `produkt/haut/llm_client.py` | 33 | 12 | 64% |
| `produkt/haut/ors_client.py` | 39 | 3 | 92% |
| `produkt/haut/server.py` | 109 | 33 | 70% |
| **TOTAL** | **1,753** | **244** | **86%** |

### Coverage Blind Spots (api.py)

**LLM Chat path** (lines missed: 1239-1256, 1303-1308, 1357, 1396, 1468-1469):
- LLM classification is entirely untested
- `_chat_prompt()`, `_chat_parse()`, `_llm_vorschlaege()` — 0% coverage
- This is expected (LLM calls are non-deterministic), but means chat feature correctness is unverified

**DBA calculation branches** (lines missed: 1137, 1139, 1143-1145):
- Freistellung path: `dba_method_auto == "freistellung"` — partially covered
- `p32b_progressionseinkuenfte` setter — covered
- Anrechnung path: `catala_p34c_1` call — tested via unit tests

**Server.py** (lines missed: 74-88, 94-95, 103-105):
- CORS middleware, error handlers, and startup logic — untested
- These are infrastructure concerns, low tax-calculation impact

**LLM Client** (lines missed: 30-33, 50-60):
- API key loading and provider configuration — untestable (requires real keys)

### Coverage Blind Spots (runner.py)

**15 missed lines are Catala engine fallbacks** (valid):
```python
try:
    from pkg import Einkommensteuertarif as E
except ModuleNotFoundError:
    E = None  # engine_unavailable flag
```

**~20 missed lines are edge cases**:
- Negative zvE handling (`catala_p34c_1`: zve <= 0 → 0)
- Zero-value inputs in sparer_pb, verlustvortrag

**~30 missed lines are Stufe-2 deferred logic**:
- `catala_p35_anrechnung` >5Mio-excess path
- `catala_p24a_altersentlastung` Härte-Kappung
- `catala_solz_berechnung` multi-kombi edge case

**~20 genuinely uncovered lines**:
- `catala_p10_kist` (church tax deduction as Sonderausgabe) — 0% coverage
- `catala_mitunternehmer_einkuenfte` edge cases — partial coverage
- `catala_p21_2_verbilligt` Vermietung WK after verbilligt — 0% coverage

### What Coverage Doesn't Tell You

**Coverage measures execution, not correctness.** The 86% number is misleading in several ways:

1. **15 lines of Catala engine fallbacks**: Executed but never tested with real Catala engine
2. **DBA Freistellung path** (covered): Code executes, but guard blocks it — coverage doesn't know the code is unreachable
3. **LLM chat path** (uncovered): May produce wrong results, but coverage can't validate LLM output
4. **Ring-Diff tests**: Execute code paths but don't verify calculation **correctness** — only verify that a Δ exists

### True Test Quality Assessment

| Quality | Count | What |
|---------|-------|------|
| Ring-Diff (strong) | ~20 | Proves field → tax Δ exists |
| Unit accessor (strong) | ~10 | Direct function call with seeded inputs |
| E2E schema (medium) | ~50 | Validates API contract shape |
| E2E calculation (medium) | ~30 | Tests specific calculation values |
| E2E smoke (weak) | ~40 | Creates fall, posts events, checks grund |
| Pre-existing failures | 105 | Not testing anything — failing |
| Catala-dependent (blocked) | 14 | Cannot execute without `pkg` build |
