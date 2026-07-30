# 11 — Product Gap Analysis: From Algorithm to Market-Ready Product

## Source: Product-Level Review (2026-07-24)

This document integrates findings from a third review focused on **what separates the current algorithmic core from a shippable end-user or tax-advisor product**. Cross-references to existing autopsy documents use `(see 04, GAP-X)` and `(see 09, Block #Y)`.

---

## Gap 1: ERiC/ELSTER Production Interface (Phase 4 Roadmap)

**Current state**: TaxGraph has offline schema validation (`elster/eric_gate.py`) and structural ERiC checks. The `elster_writer.py` generates XML but never submits it. (see 09, Block #4)

**What's missing for a real product:**

| # | Missing Component | Impact |
|---|-------------------|--------|
| 1.1 | **Official ELSTER manufacturer registration** — Hersteller-ID from the Konsens-Gruppe (ELSTER developer portal), integration of the official C library (ERiC) with a manufacturer-specific ID | Cannot submit any returns to tax authorities |
| 1.2 | **Actual return submission (`ERIC_ENCRYPT_AND_SEND`)** — encrypted and signed transmission of the tax return to tax authority data centers | The core value proposition (filing taxes) is impossible without this |
| 1.3 | **Certificate & authentication management** — user certificates (.pfx files) or modern methods (ElsterSecure, nPA/eID) | No way to identify the taxpayer to the tax authority |
| 1.4 | **Automated assessment retrieval (DIVA)** — automatic pickup of the electronic tax assessment notice (DIVA procedure) and automated comparison (target/actual) between TaxGraph calculation and the tax office's assessment | Users must manually compare the TaxGraph result with the official assessment — the most valuable feedback loop is missing |

**Severity**: CRITICAL — without ERiC submission, the product cannot file taxes, which is its primary purpose.

**Cross-reference**: (see 09, Block #4; 04, ELSTER-Submission gaps 2.1-2.5)

---

## Gap 2: Interview Layer, UX & Explainability (Phase 5 Roadmap)

**Current state**: The API has `/fragen` (returns raw field catalog), `/stand` (field state), and `/graph` (rule dependency graph). No user-facing UI exists. (see 09, Section 6)

**What's missing for a real product:**

| # | Missing Component | Impact |
|---|-------------------|--------|
| 2.1 | **Dynamic interview with relevance propagation** — an assistant that, based on the rule graph, asks ONLY the questions relevant to the individual case (e.g., no questions about home office if the user has no employment income) | Users are overwhelmed by 150+ raw fields with no guidance on which apply to them |
| 2.2 | **Layperson-friendly help & translation** — translation of legal EStG terms ("außergewöhnliche Belastungen," "Günstigerprüfung § 31," "Fünftelregelung") into plain language with practical examples | The current `/fragen` output uses legal terminology that non-experts cannot understand |
| 2.3 | **Catala Explain-UI** — visual and understandable presentation of the calculation path ("Why do I pay X € in tax?") based on Catala execution traces | The `/warum` endpoint exists but returns raw provenance data, not a user-facing explanation |
| 2.4 | **Pre-flight plausibility check** — automatic hints before submission about forgotten allowances, contradictory statements, or incomplete information | Users can submit obviously suboptimal returns (e.g., missing the Sparer-Pauschbetrag) with no warning |

**Severity**: CRITICAL for end-user product — the calculation core is useless to non-experts without an interview layer.

**Cross-reference**: (see 09, Section 6; 04, GAP-M2)

---

## Gap 3: eDaten (VaSt), Document Management & Prior-Year Data

**Current state**: `elster_writer.py` can import eDaten and writes them as `vorlaeufig`. `produkt/import/` has `elster_writer.py`, `beleg_writer.py`, `vorjahr_writer.py`, `kontoauszug_writer.py`. (see 08, Finding 3)

**What's missing for a real product:**

| # | Missing Component | Impact |
|---|-------------------|--------|
| 3.1 | **Pre-filled tax return (VaSt/eDaten)** — retrieval of data stored at the tax authority (wage tax certificates, pension payment notifications, health/long-term care insurance contributions) directly into the TaxGraph store | Users must manually enter data that the tax authority already possesses |
| 3.2 | **Document OCR & smart import** — upload and automatic extraction from documents (donation receipts, craftsman invoices, utility bills) via OCR | Paper-based supporting documents are a major friction point |
| 3.3 | **Prior-year data carryover** — automatic transfer of unchanged master data, loss carryforwards, and depreciation schedules from the previous year's return | Users re-enter the same data year after year |

**Severity**: HIGH — these features exist in competing products (WISO, ELSTER, Taxfix) and are expected by users.

**Cross-reference**: (see 08, Finding 3; 04, GAP-M1)

---

## Gap 4: Legal Framework & Compliance (StBerG & DSGVO)

**Current state**: No AGB, no Datenschutzerklärung, no legal review of the product architecture. (see 09, Section 7)

**What's missing for a real product:**

| # | Missing Component | Impact |
|---|-------------------|--------|
| 4.1 | **StBerG boundary analysis** — pure self-application software is permissible under the Steuerberatungsgesetz. However, if the system issues automated action recommendations or tax optimization advice for third parties, this touches § 2 StBerG | Legal risk: operating as an unlicensed tax advisor is a criminal offense in Germany |
| 4.2 | **Legally reviewed product architecture** — clear separation of declaration (permitted) from advisory/recommendation functions (restricted), with legally vetted disclaimers | Without this, an injunction under UWG § 3a in connection with StBerG § 5 is possible |
| 4.3 | **DSGVO Art. 9 special category data** — tax data contains highly sensitive health data (extraordinary burdens, disability status/GdB) and religious data (church tax). The product requires end-to-end encryption (E2EE) for stored cases | Standard encryption is insufficient — Art. 9 requires "appropriate technical and organizational measures" for special category data |

**Severity**: CRITICAL — StBerG violation can result in fines up to €50,000 and cease-and-desist orders. DSGVO Art. 9 violations can result in fines up to €20M or 4% of annual turnover.

**Cross-reference**: (see 09, Section 7; 04, GAP-H3)

---

## Gap 5: Specialist Tax Coverage & Edge Cases

**Current state**: TaxGraph covers employment income, EÜR, V+V, capital income, pensions, and co-entrepreneurs. Approximately 85% MVP coverage. (see 01, "Stufe-1 vs Stufe-2")

**What's missing for a universal full product:**

| # | Missing Component | Impact |
|---|-------------------|--------|
| 5.1 | **Agriculture & forestry (§ 13 EStG)** — special valuation rules, partial value, usage value taxation | Excludes farmers and foresters from using the product |
| 5.2 | **Photovoltaic special rules (§ 3 Nr. 72 EStG)** — tax exemption for small PV systems, highly relevant for the mass market | Excludes the fastest-growing segment of micro-entrepreneurs |
| 5.3 | **Extended DBA foreign scenarios** — currently 11 countries mapped at state level, missing per-income-type routing for all DBA partner states (see 03, GAP-001) | Excludes expats, cross-border workers, and international investors |
| 5.4 | **Trade tax & E-Bilanz** — full integration of balance-sheet accounting (instead of EÜR) including E-Bilanz taxonomy transmission (if self-employed/business filers are to be fully served) | Excludes incorporated businesses and larger Gewerbe operations |

**Severity**: MEDIUM for MVP, HIGH for full product — these gaps exclude specific but significant user segments.

**Cross-reference**: (see 03; 04, GAP-H1)

---

## Gap 6: Product Infrastructure & Operability

**Current state**: No authentication, single-tenant JSON file store, no CI/CD pipeline. (see 09, Sections 3-5)

**What's missing for a real product:**

| # | Missing Component | Impact |
|---|-------------------|--------|
| 6.1 | **Client & user management** — registration, login, AuthN/AuthZ, user roles (e.g., client vs. preparer vs. advisor) | Cannot serve multiple users or support advisor-client relationships |
| 6.2 | **Annual maintenance & update process** — automated release process for tax year changes (e.g., incorporating new statutory rates/parameters for 2026/2027), keeping the parametric layer (`params/`) and Catala rules synchronized with the legislature | The system becomes outdated and incorrect within one tax year without updates |

**Severity**: HIGH — without multi-tenancy, the product cannot scale. Without annual updates, it becomes obsolete.

**Cross-reference**: (see 09, Section 3-5; 04, GAP-M3)

---

## Combined Product Roadmap

```
[Current Foundation]              [Missing Product Components]
┌────────────────────────┐        ┌──────────────────────────────┐
│ • Catala Rule Core     │        │ • ERiC Submission & DIVA     │
│ • GETTSIM Oracle       │  ────► │ • Dynamic Interview UI       │
│ • Snapshot Governance  │        │ • eDaten / VaSt Import       │
│ • FastAPI / Store      │        │ • StBerG Check & Privacy E2EE│
└────────────────────────┘        │ • Client/User Management     │
                                  │ • Annual Update Process      │
                                  │ • Specialist Tax Coverage    │
                                  └──────────────────────────────┘
```

## Prioritized Build Order

| Phase | Duration | Components | Rationale |
|-------|----------|-----------|-----------|
| **A: Secure Foundation** | Month 1-2 | Auth, encryption, database migration, user management (6.1, 4.3) | Cannot go live without security |
| **B: Legal Clearance** | Month 1-2 (parallel) | StBerG review, AGB, DSGVO documentation (4.1, 4.2) | Must clear legal before any public launch |
| **C: ELSTER Production** | Month 3-4 | Hersteller-ID registration, ERiC submission, DIVA retrieval (1.1-1.4) | Core value proposition |
| **D: Interview & UX** | Month 4-6 | Dynamic interview, layperson help, Catala Explain-UI (2.1-2.4) | Makes the product usable by non-experts |
| **E: Data Import** | Month 5-7 | VaSt/eDaten retrieval, OCR, prior-year carryover (3.1-3.3) | Reduces manual data entry |
| **F: Coverage Expansion** | Month 8+ | §13, §3 Nr. 72, extended DBA, E-Bilanz (5.1-5.4) | Broader market reach |
| **G: Operations** | Ongoing | Annual update process, monitoring, CI/CD (6.2) | Sustainability |

## Total Estimated Time to MVP Product Launch

**6 months** to a product that can:
- Authenticate users securely
- Calculate taxes for the core MVP scenarios
- Submit returns to ELSTER via ERiC
- Guide users through a dynamic interview
- Import eDaten from the tax authority
- Operate within StBerG and DSGVO boundaries

**12-18 months** to full market-ready product with specialist coverage, advisor multi-tenancy, and full automation.
