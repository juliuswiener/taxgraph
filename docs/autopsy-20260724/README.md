# TaxGraph Adversarial Codebase Autopsy — 2026-07-24

**Scope**: Entire taxgraph repository at HEAD `8d904b4`, branch `claude/implementation-start-ypyyqw`.
Analysis based on running code, test output, git history, and binding schemas.
Code counted: 2,456 lines api.py + 1,521 lines runner.py + 17 binding YAMLs + 214 test functions.

**One-sentence verdict**: The calculation core is strong (85% MVP, fail-closed guards, honest gap documentation). **But**: 1 critical runtime regression (`kapitalertraege` NameError causes ~18 crashes), 1 DBA guard contradiction (Freistellung blocked at guard but supported in calculation), and ~87 jsonschema maintenance failures. To become a market-ready product, it needs ALL supporting layers — authentication, encryption, database, ERiC production submission, dynamic interview UI, and legal compliance (StBerG/DSGVO). Estimated: 6 months to launchable MVP, 12-18 months to full product.

## Document Index

### Core Analysis (documents 01-08)

| # | Document | Purpose |
|---|----------|---------|
| 01 | [Core Anatomy](01_core_anatomy.md) | How api.py + runner.py actually work |
| 02 | [Guard System Analysis](02_guard_system.md) | The 12-guard fail-closed safety net |
| 03 | [DBA Subsystem](03_dba_subsystem.md) | The 11-country routing: what works, what doesn't |
| 04 | [Gap Analysis](04_gap_analysis.md) | All known limitations ranked by severity |
| 05 | [Test Reality](05_test_reality.md) | What the 105 test failures actually mean |
| 06 | [Binding Infrastructure](06_binding_infrastructure.md) | 17 YAMLs, 10 with documented gaps |
| 07 | [Coverage Analysis](07_coverage_analysis.md) | 86% total, with critical blind spots |
| 08 | [Honest Conclusions](08_honest_conclusions.md) | True / Uncertain / False synthesis |

### External Review Integration (documents 09-11)

| # | Document | Purpose |
|---|----------|---------|
| 09 | [Production Readiness](09_production_readiness.md) | Top-10 Production blockers across 7 pillars |
| 10 | [External Review Integration](10_external_review_integration.md) | Kapitalertraege NameError root cause + new findings |
| 11 | [Product Gap Analysis](11_product_gap_analysis.md) | From algorithm to market: 6 product pillars, build timeline |

## Reading Order

1. Start with **08** (Honest Conclusions) for the one-page summary
2. **10** (External Review) — the `kapitalertraege` bug reclassification
3. **11** (Product Gap) — what separates the algorithm from a real product
4. **09** (Production Readiness) — all production blockers across 7 pillars
5. **03** (DBA) — most complex subsystem, most gaps
6. **02** (Guards) — the safety architecture
7. Others as reference

## Critical Caveats

- **Trusted**: running code (`api.py`, `runner.py`), test output, binding schemas, git history
- **NOT trusted**: code comments (often stale), prior reports, developer claims not verified by code
- **Cannot verify**: Catala engine correctness (binary/pkg), ELSTER submission accuracy, LLM chat classification quality
- **105 test failures classified**: ~18 `kapitalertraege` NameError (code regression), ~87 jsonschema `grund` enum mismatches (schema maintenance)
- **Product readiness**: Calculation core is ~85% complete. Product layers (auth, UI, ERiC, compliance) are ~5% complete.
