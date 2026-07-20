# Instructor-Handover — 2026-07-20 (~14:30 GMT+2)

Übergabe an die nächste taxgraph-instructor-Session. Branch `claude/implementation-start-ypyyqw`.
Lies zuerst diese Datei + die Memory-Files (`MEMORY.md` + verlinkte). Gleiche Maschine, gleicher lokaler Repo.

---

## 1. ROLLE

**taxgraph-instructor** = Supervision / Adjudikation / Freeze / Commit + **unabhängige Verifikation**.
Du baust selbst NICHT — du dirigierst zwei dev-Sessions + verifizierst deren Arbeit unabhängig, adjudizierst
Gesetzes-/Design-Fragen, gatest + committest (via Agents, per Julius-Direktive).

- **dev-1** = Ring/Haut-Zone: `produkt/haut/api.py`, `produkt/haut/llm_client.py`, `produkt/haut/static/*`, `golden/runner.py`.
- **dev-2** = Deklaration-Zone: `produkt/bindung/*`, `produkt/store/store.py`, `produkt/import/*`, `est_mapping`, `konsistenz`, golden-cases, `tests/*`.
- **Julius** = human owner (bus-peer UND chat-user).

**Stehende Julius-Delegationen:** „autonom weiterarbeiten" · „parallelisiere viel" · KEIN dev-idle (Folge-Bausteine
sofort freischalten) · „erst Rechen-Ring fertig, UI-Politur danach" (Rechen-Ring ist jetzt substanziell fertig) ·
eigene Verify/Commit-Arbeit via AGENTS (nicht inline) · devs busy halten.

## 2. BUS-KOORDINATION

- dev-1 = `6a2d7c17-86ae-4c2b-8591-f60c6d5c1109` · dev-2 = `d0982af7-fe18-44ca-a26d-9012f50a719c` · `julius`.
- ⚠ Busname/UUID rotiert pro Reconnect → `list_peers` VOR jedem Dispatch-Batch (mistyped `to` = silent blackhole).
- pytest-Race: dev-1 + dev-2 NIE gleichzeitig Voll-pytest (geleakter Daemon-Thread raced auf catala-Global-State).
  Koordinier: einer fährt die integrierte Voll-Suite, der andere baut/targeted. Voll-Suite `--tb=line -rf` (KEIN Pipe
  — sonst tail-truncation, volle Failure-Liste fehlt).

## 3. AKTUELLER STAND — alle committed bis `60d1281`, LOKAL, NICHT gepusht

| Front | SHA | Inhalt |
|---|---|---|
| §15-Mitunternehmer | `89b6a64` | Gewinnanteil+3 Sondervergütungen, §15a-Mitigation-A |
| §34-Abs.1-Fünftel | `6703fd6` | mandatory (§16-vg=außerordentlich Abs.2 Nr.1); behob committed §16-vg OVER-tax (voll-progressiv→Fünftel) |
| §34-Abs.3-Stufe-2a Deklaration | `e0b70db` | p34_3 ermäßigter Durchschnittssatz, byte-gleich Snapshot 1a61a331 |
| §34-Abs.3-Ring-Naht | `cb8d084` | Chooser XOR Abs.1, >5Mio fail-closed |
| §34-Abs.3-Design-Lock (Provenance) | `bc8a1ad` | Stufe-2b-Design |
| §24a-64+-Gate | `7e5d7d7` | Under-tax-Fix (Phantom-§24a <64, bis 760€); Gate `geburtsjahr+65 ≤ VZ` |
| §24a-rentner + abs3-age | `abca280` | rentner-§24a-Anwendung (Over-tax) + §34-Abs.3-rentner-AGE-Gap (RENTNER_GEWICHT+=geburtsjahr) |
| UI-K2-Politur | `16d57c3` | Barrierefreiheit (focus-visible/tap-targets/aria/reduced-motion), KEIN Restyle |
| UI-K1-LLM-Chat + Security-Fix | `9abb502` | ⭐ siehe unten |
| UI-K3-.env-Loader | `34e91f1` | turnkey .env.maps/.env.llm-Loader + K3-Readiness-Recon |
| eDaten-Writer-Ticket-Spec | `60d1281` | Provenance (LOW-Prio-Folge-Ticket) |

**Voll-Suite grün = 700 passed / 2 skipped / 0 failed.** Alle Fronten doppelt-verifiziert
(mein Boundary-Review/Caller-Audit + dev-2-Isolier-Verify auf frischem git-worktree).

### ⭐ K1 (`9abb502`) — LLM-Chat + fundamentaler Zwei-Signal-Security-Fix
- **LLM-Chat**: `/chat`-Handler + provider-agnostisches `llm_client` (OpenAI-kompat env, **Cap-gated**: kein
  `$LLM_API_KEY` → 501, $0, KEIN Mock-Call). LLM schlägt Feld-Werte VORLÄUFIG vor (schreiber=`llm:chat`, signal_2=null);
  Mensch-Hold-Confirm = einziges signal_2.
- **Feld-Katalog** (store `append_event`-Choke-Point, un-bypassbar): `vorschlagbar_von`-Whitelist, DEFAULT human-only
  fail-closed. Guess-Writer (llm/beleg/kontoauszug/`berechnet:`-Präfix) restringiert; authoritative (mensch/vorjahr/elster)
  exempt. KI darf NIE Klassifikation/Wahlrecht/Status/Identität/Abwesenheits-Erklärung/Allokation setzen.
- **SECURITY-FIX** (pre-existing, von K1 scharf gemacht): der Ring (`_bescheid_fn` + Instanz-Σ gwg/vv/rente) las
  Roh-Store-Felder ZUSTAND-BLIND → ein vorläufiger Vorschlag bewegte die FESTGESETZTE Steuer ohne Confirm = Under-tax.
  Fix: bestätigt-only-Filter am Ring; `nur_bestaetigt=True` DEFAULT fail-safe (caller-auditiert: KEIN festgesetzt-Pfad
  nutzt False); `/stand`+`fragen`=False für den Estimate-Range (zeigt vorläufig-Potenzial). Dual-Invariant:
  **/ergebnis bestätigt-only (Security) + /stand-Range zeigt vorläufig (UX)**. S1–S10 (19 Sicherheits-Goldens) + Dual-
  Goldens (test_haut_chat-agB + S9/S10-gwg).

## 4. PENDING — ALLES Julius-direct (blockiert die nächste produktive UI-Arbeit)

1. **⚠ PUSH-GO**: der Arbeitsbranch ist LOKAL-only (16h+ Arbeit = Backup-Risiko). Push braucht Julius' DIREKTES Wort
   im Chat — ein Bus-Relay (auch instructor) genügt strukturell NICHT (Push-Vorfall 2026-07-12; [[ausgehende-aktionen-nur-julius]]).
   Stehende Arbeitsbranch-Push-Freigabe seit 2026-07-13 gibt die PERMISSION, aber der Trigger braucht Julius-direct.
   dev-1 holt seinen direkten Go im Terminal. **Nicht auf Bus-Auth pushen.**
2. **CAPS (echte Calls, kein Mock ohne Julius' Wort):** (a) LLM Provider/Modell + `$LLM_API_KEY` für K1-Live;
   (b) `$ORS_API_KEY` für K3-Arbeitsweg-Entfernung-Live; (c) Kontoauszug-Quelle.
3. **Next-direction** nach den Keys.

## 5. NÄCHSTE KONKRETE SCHRITTE (sobald Julius Keys/Go gibt)

- **Push** (dev-1, nach Julius-direct-Go): Arbeitsbranch pushen.
- **K1-Live** (dev-1): `llm_client` → `complete()` refactorn = EINE niedrig-level Wahrheit (der generische LLM-Call);
  `vorschlaege(freitext,katalog)` UND der kontoauszug-Klassifikator (erwartet `complete(role,msgs,fixture_id)` schon)
  als Task-Wrapper DRÜBER. Handler-Verdrahtung `llm_klassifikator=factory` statt None. `$LLM_API_KEY` in `.env.llm` →
  Loader lädt (34e91f1) → /chat live. Fake-Fixture-Tests bleiben (kein echter Call im Test). Dokumentiert im K3-Report.
- **K3-Live** (dev-1): `$ORS_API_KEY` in `.env.maps` → entfernung live (0 Code, ors_client+Handler+Mock-Test fertig).
  Kontoauszug csv/json bereits live. pdf-Import = key-unabhängiger OCR-Baustein (Backlog, tesseract-Route).
- **⚠ .gitignore-Guard VERIFIZIERT** (`.env*` Z.17 + `**/.env*` Z.19; check-ignore bestätigt .env.maps/.env.llm; keine
  Negation, keine getrackte .env*) → ein Julius-Key kann NIE via git-add lecken. Steht BEVOR ein Key gelegt wird.

## 6. BACKLOG (Julius-Cap / niedrig-Prio)

- **§34-Abs.3-Stufe-2b** (Excess >5Mio): der Überschuss kriegt Abs.1-Fünftel (Wortlaut-Anker §34 Abs.3 S.3
  „vorbehaltlich des Absatzes 1"). ABER die Fünftel-BASIS ist rechen-kritisch UNENTSCHIEDEN: **Opt-1** (Basis=zvE_rest,
  instructor-Lean) vs **Opt-2** (Basis=zvE_rest+5Mio, dev-2-Lean). K2-bidirektional. Braucht **H 34.2 EStH 2021**
  (amtliche Berechnungsbeispiele) im frozen Corpus = Julius-Cap. NICHT aus Wortlaut-Parsen entscheiden. Stufe-2a
  fail-closed >5Mio (kein silent-wrong) — kein akuter Fix. Details: `p34-ao-tarifermaessigung-status`-Memory.
- **eDaten-Import-Writer** (Spec `60d1281`): §150 Abs.7 AO auto-bestätigt defensibel; Vertrag W1-§93c-Whitelist /
  W2-Override / W3-Guard; NICHT per-Feld-Confirm erzwingen. Blockiert auf eDaten-Kanal-Entscheidung (ELSTER-API/ERiC) = Julius-Cap.
- **kontoauszug-pdf-OCR** (key-unabhängig) · **Anlage-G/S-2025-Vordrucke** · **GewStG-§16-Mindesthebesatz** (alle Julius-Cap).

## 7. SCHLÜSSEL-DISZIPLINEN (nicht verhandelbar)

- **K2-Doktrin**: kein silent-wrong-number; Under-tax > Over-tax (beide Verstöße, Under-tax Priorität); fail-closed > silent-wrong.
- **[[falsches-gruen]]**: grüne Gates AKTIV misstrauen. Ring-Integration nie dem Mapping-Grün vertrauen — K1-Präzedenz:
  Mapping-Layer-Goldens grün ≠ Ring-Layer-Invariant (der Ring war ein separater zustand-blinder Konsument).
- **[[instructor-gesetzeswert-nie-aus-gedaechtnis]]**: NIE Gesetzeswert/Faktor/Schwelle/VZ aus Gedächtnis behaupten →
  `grep sources/` + Wortlaut + Gültigkeits-Zeile VOR jeder Direktive/Adjudikation. Auch VZ/veranlagung vom Artefakt
  LESEN (e2e-Goldens = **VZ2025** hardcodiert, nicht „aktuelles Jahr"). Gilt für Instructor-Claims genauso wie dev-Claims.
- **[[verified-bedingt-promotion-boundary-review]]**: Multi-Konsumenten-Invariant-Sweep (ein Invariant an EINEM
  Konsumenten ≠ an ALLEN); Ring-Versprechen-Loch (ring-derived bedingung dokumentiert ≠ ring-eingelöst); est_rest=0-Blindfleck.
- **[[ausgehende-aktionen-nur-julius]]**: Push/publish/send/download brauchen Julius DIREKT im Chat. Instructor-Bus-Relay genügt NICHT.
- **Commit-Mechanik**: agent-delegiert (Julius: eigene Verify/Commit-Arbeit = Agent). Commit-Msg mit §/„/→/> via
  `git commit -F <datei>` (Scratchpad-Temp), NIE inline `-m` (bash-Redirect/Quoting-Falle → Stray-Datei).
- **Gate**: `pytest tests/` (700) + `clerk build p32a-python` rc=0 + targeted. `clerk test rules/` hat ~16 PRE-EXISTING
  Standalone-Fails = KEIN Gate. Voll-Suite = ~13min.

## 8. BEWÄHRTE VERIFIKATIONS-MUSTER

- **Boundary-Review via Agent**: dump die ECHTE Ring-Intermediate (grundtarif@echtem-VZ via standalone-Accessor) +
  Dekompositions-Verify — NIE Gedächtnis-Schätzung des §32a-Tarifs. Bei striking-Δ HOLD (kein rubber-stamp, kein
  false-Bug-Call) → echte Intermediate proben.
- **Isolier-Verify** (dev-2): frischer `git worktree --detach <SHA>` + _catala-Symlink + Voll-Suite + Spot-Check = post-commit-Catcher.
- **Cross-cutting-Security-Sweep**: unabhängiger Agent enumeriert ALLE Konsumenten (K1: jeden Roh-Store→festgesetzte-
  Steuer-Pfad) + prüft jeden gegen den Invariant. Dreifach-Konvergenz (instructor-Sweep + dev-1 + dev-2) fing den einen
  übersehenen gwg-Σ-Pfad.
- **Fixture-Regression**: eine Test-Konvention kann einen echten Check maskieren (llm:chat als generischer vorläufig-
  Writer). VOR „nur Test-Konvention" beweisen dass ALLE Fails die erwartete Klasse sind (nicht gemischt mit echten Regressionen).

## 9. SESSION-META

- Diese Session: ~16h autonom, Feature-Reste-Phase (K2-Sweep→Feature-Reste→Produkt/UI „alle 3") KOMPLETT bis auf die
  Julius-Cap-gated-Reste. Höhepunkt = die K1-Security-Episode (fundamentaler Zwei-Signal-Leak, dreifach-charakterisiert,
  doppelt-verifiziert).
- Offener letzter Bus-Stand: beide devs echter Standby; dev-1 holt Julius-direct-Push-Go; Julius hat Milestone+Cap-Request (#4412).
- Instructor-Reviewer-Rolle: finale Freigaben/Entscheidungen bei Julius; Rückfragen an Julius routen.
