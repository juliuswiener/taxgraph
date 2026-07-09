"""G2 bake-off evaluation: aggregate run reports into reports/gate-g2.md.

Consumes the report.json files a bake-off run writes (one per candidate per
pairing) and computes the metrics from Nachtrag §5. Decision rule: lowest
escalation rate wins; cost is only a tiebreaker.

Run (after the bake-off, which Julius releases separately):
    pipeline/.venv/bin/python pipeline/bakeoff/evaluate.py pipeline/runs
"""

from __future__ import annotations

import glob
import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))


def load_reports(runs_dir: str) -> list[dict]:
    out = []
    for p in glob.glob(os.path.join(runs_dir, "**", "report.json"), recursive=True):
        try:
            out.append(json.load(open(p, encoding="utf-8")))
        except json.JSONDecodeError:
            print(f"skip unreadable report: {p}", file=sys.stderr)
    return out


def _gate(r: dict, name: str) -> str | None:
    for g in r.get("gates", []):
        if g["name"] == name:
            return g["status"]
    return None


def pairing_of(r: dict) -> str:
    """Identify the pairing.

    A run aborted at a role has empty provenance, so the explicit `pairing` field
    is authoritative; the provenance slug is only a fallback for older reports.
    Otherwise every role_timeout would land in an "unknown" bucket and the
    flakiness would silently disappear from the comparison.
    """
    if r.get("pairing"):
        return r["pairing"]
    for p in r.get("provenance", []):
        if p["role"] == "formalisierer_b":
            return p["slug"].replace(" (dry-run)", "")
    return "unknown"


def rate(n: int, d: int) -> float:
    return (n / d) if d else 0.0


def _judge_missed(r: dict) -> bool:
    """Stage 2 of the round-trip metric: judge said faithful, reality disagreed.

    An assumption is *missed* when the judge reports `faithful` but a
    deterministic gate (equivalence) or the blind reference contradicts it. A
    judge that never flags anything looks perfect on stage 1 and fails here.
    """
    v = r.get("judge_verdict") or {}
    if not v or v.get("parse_error") or not v.get("faithful"):
        return False
    if _gate(r, "equivalence") == "FAIL":
        return True
    if "blind_repro_match" in r and not r["blind_repro_match"]:
        return True
    return False


def _judge_named(r: dict) -> bool:
    """Stage 1: judge explicitly named a silent assumption."""
    v = r.get("judge_verdict") or {}
    return bool(v and not v.get("parse_error") and v.get("stille_zusatzannahmen"))


def _syntax_ok(r: dict, suffix: str = "") -> bool:
    """Both formalisers parsed and typechecked. `suffix='_first'` scores the
    first pass, before any repair round."""
    return (_gate(r, f"syntax_a{suffix}") == "PASS"
            and _gate(r, f"syntax_b{suffix}") == "PASS"
            and _gate(r, f"typecheck_a{suffix}") in ("PASS", None)
            and _gate(r, f"typecheck_b{suffix}") in ("PASS", None))


def _scope_gaps(r: dict) -> int:
    v = r.get("judge_verdict") or {}
    return len(v.get("scope_gap") or [])


def evaluate(reports: list[dict]) -> dict:
    by = defaultdict(list)
    for r in reports:
        by[pairing_of(r)].append(r)

    result = {}
    for pairing, rs in by.items():
        n = len(rs)
        syntax_ok = sum(1 for r in rs if _syntax_ok(r))
        # Erstversuch getrennt: vor der Reparaturrunde. Reports ohne `_first`-Gate
        # stammen aus dem Lauf vor der Protokollaenderung; dort ist der Endstand
        # zugleich der Erstversuch.
        syntax_first = sum(1 for r in rs
                           if _syntax_ok(r, "_first" if _gate(r, "syntax_a_first") else ""))
        repaired = sum(1 for r in rs if any((r.get("repaired") or {}).values()))
        gap_runs = sum(1 for r in rs if _scope_gaps(r) > 0)
        gap_total = sum(_scope_gaps(r) for r in rs)
        equiv_div = sum(1 for r in rs if _gate(r, "equivalence") == "FAIL")
        rt_dev = sum(1 for r in rs if _gate(r, "roundtrip") == "FAIL")
        named = sum(1 for r in rs if _judge_named(r))
        missed = sum(1 for r in rs if _judge_missed(r))
        escalated = [r for r in rs if r.get("queue_status") == "flagged_for_review"]
        # Eskalation getrennt nach ausloesendem Gate
        per_gate = defaultdict(int)
        for r in escalated:
            for g in r.get("failed_gates", []):
                per_gate[g] += 1
        # Blind-Reproduktion: nur Laeufe zaehlen, die ueberhaupt vergleichbar waren.
        # Ein Build-Fehler des Kandidaten ist kein "nicht reproduziert", sondern
        # gar kein Vergleich - er wird getrennt ausgewiesen.
        blind = [r for r in rs if "blind_repro_match" in r]
        blind_ok = sum(1 for r in blind if r["blind_repro_match"])
        blind_tasks = [r for r in rs if "blind_repro_status" in r]
        blind_build_err = sum(1 for r in blind_tasks
                              if r["blind_repro_status"] in
                              ("blind_build_error", "blind_call_error", "blind_no_scope",
                               "blind_runtime_error"))
        blind_sig_mismatch = sum(1 for r in blind_tasks
                                 if r["blind_repro_status"] == "blind_signature_mismatch")
        blind_ref_err = sum(1 for r in blind_tasks
                            if r["blind_repro_status"] == "blind_ref_error")
        approved = [r for r in rs if r.get("queue_status", "").startswith("verified")]
        cost = sum(r.get("total_cost_usd", 0.0) for r in rs)
        # Clerk-Gate n/a-Anteil (kein EStH/BMF-Rechenbeispiel)
        clerk_na = sum(1 for r in rs if _gate(r, "clerk") == "SKIP")

        # Provider-Flakiness getrennt von Modellqualitaet: Timeouts/Retries je
        # Rolle und Provider, plus Laeufe, die an einer Rolle abgebrochen sind.
        role_timeouts = [r for r in rs if r.get("queue_status") in
                         ("role_timeout", "role_error")]
        run_errors = [r for r in rs if r.get("queue_status") == "run_error"]
        transport = {"retries": 0, "timeouts": 0, "rate_limits": 0, "errors": 0}
        by_provider: dict = defaultdict(lambda: dict.fromkeys(transport, 0))
        for r in rs:
            t = r.get("transport") or {}
            for k in transport:
                transport[k] += t.get(k, 0)
            for prov, counts in (t.get("by_provider") or {}).items():
                for k, v in counts.items():
                    by_provider[prov][k] += v
        by_role_timeout = defaultdict(int)
        for r in role_timeouts:
            by_role_timeout[f"{r.get('failed_role')} ({r.get('failed_slug')})"] += 1

        result[pairing] = {
            "n": n,
            "syntaxvaliditaet_first_pass": rate(syntax_first, n),
            "syntaxvaliditaet": rate(syntax_ok, n),   # nach der Reparaturrunde
            "repair_rate": rate(repaired, n),
            # Rueckmeldung zur Task-Spezifikation, nicht zur Modellqualitaet
            "scope_gap_anteil": rate(gap_runs, n),
            "scope_gap_je_task": round(gap_total / n, 3) if n else 0.0,
            "aequivalenz_divergenzrate": rate(equiv_div, n),
            "roundtrip_abweichungsrate": rate(rt_dev, n),
            # zweistufig: benannt (gut) vs verpasst (schlecht)
            "annahme_benannt": rate(named, n),
            "annahme_verpasst": rate(missed, n),
            "blind_reproduktion": rate(blind_ok, len(blind)) if blind else None,
            "blind_build_error_rate": rate(blind_build_err, len(blind_tasks)) if blind_tasks else None,
            "blind_ref_error_rate": rate(blind_ref_err, len(blind_tasks)) if blind_tasks else None,
            "blind_signature_mismatch_rate": rate(blind_sig_mismatch, len(blind_tasks)) if blind_tasks else None,
            "run_error_rate": rate(len(run_errors), n),
            "eskalationsrate": rate(len(escalated), n),
            "eskalation_je_gate": {k: rate(v, n) for k, v in sorted(per_gate.items())},
            "clerk_gate_na_anteil": rate(clerk_na, n),
            "role_timeout_rate": rate(len(role_timeouts), n),
            "transport": transport,
            "transport_by_provider": {k: dict(v) for k, v in by_provider.items()},
            "role_timeouts_by_role": dict(by_role_timeout),
            "kosten_gesamt_usd": round(cost, 6),
            "kosten_pro_approved_usd": round(cost / len(approved), 6) if approved else None,
        }
    return result


PROTOKOLL = """
## Protokollaenderung gegenueber dem ersten G2-Lauf

Der erste Lauf lieferte keinen Entscheid: die Eskalationsrate war gesaettigt
(Minimum 0.929) und 19 von 25 inhaltlichen FAILs waren derselbe Befund - der
Norm-Ausschnitt war breiter als die vorgegebene Scope-Signatur. Gemessen wurde
der Task-Zuschnitt, nicht das Modell. Vier Aenderungen, von Julius freigegeben:

1. **Judge kennt die Scope-Grenze.** Er bewertet `faithful` nur noch innerhalb
   der vorgegebenen Signatur. Norm-Teile ausserhalb gehen als `scope_gap` in eine
   eigene Metrik und fallen kein Gate. `scope_gap` ist Rueckmeldung zur
   Task-Spezifikation, nicht zur Modellqualitaet. (`roundtrip_diff@2`)
2. **Norm-Konstanten raus aus den Signaturen.** `p09` und `p04` reichten Betraege,
   Saetze und Caps als Eingaben herein und verletzten damit das eigene Prinzip aus
   dem Kopf von `tasks.yaml`; bei `p09` erzwang das zusaetzlich eine Staffel, die
   die eingefrorene Fassung 2026 gar nicht mehr kennt (StAendG 2025: einheitlich
   0,38 Euro ab km 1). Die Konstanten stehen jetzt in `ref.fixed_inputs` und
   erreichen nur die Referenz. Signaturen sind VZ-agnostisch.
3. **Genau eine Reparaturrunde**, nur bei Syntax- oder Typecheck-Fehler, Eingabe =
   eigener Quelltext plus woertliche Compiler-Meldung, symmetrisch fuer A und B.
   Der Report weist `syntaxvaliditaet_first_pass` und `syntaxvaliditaet`
   (post-repair) getrennt aus; die Kaskade rechnet mit dem reparierten Quelltext
   weiter. (`formalisierung_repair@1`)
4. **Dritte Paarung `A-glm_B-deepseek`**, Judge Sonnet. Sie beantwortet die Frage,
   die die ersten beiden Paarungen nicht stellen konnten: gehoert Sonnet ueberhaupt
   ins Formalisierer-Paar? Der Judge ist in jeder Paarung die dritte Modellfamilie;
   `check_pairings()` bricht ab, wenn eine Paarung das verletzt.

Vorab-Diagnose (Punkt 5 der Freigabe): Sonnets drei Typecheck-Fails hatten eine
gemeinsame, deterministische Ursache - den Numeric Tower. `p32a` und `p09` fielen
in beiden Paarungen an derselben Stelle (`decimal of (Decimal.truncate of ...)`;
`Decimal.truncate` liefert bereits ein `decimal`). Die Drift stammte nicht aus den
Few-Shots (die enthalten kein `decimal of`), sondern aus dem zu duennen
Syntax-Primer in `formalisierung@2`. `formalisierung@3` schreibt den Numeric Tower
explizit aus; die Regeln sind gegen den Compiler gegengeprueft. Das hebt alle
Paarungen gleichmaessig und verhindert, dass G2 ein Prompt-Artefakt misst.
"""


def to_markdown(res: dict) -> str:
    L = ["# Gate G2: Bake-off-Auswertung\n",
         "Entscheid: niedrigste Eskalationsrate gewinnt; Kosten nur als Tiebreaker.\n",
         PROTOKOLL]
    if not res:
        L.append("Keine Laeufe gefunden. Der Bake-off wurde noch nicht gestartet "
                 "(Freigabe durch Julius steht aus).\n")
        return "\n".join(L)

    cols = ["n", "syntaxvaliditaet_first_pass", "syntaxvaliditaet", "repair_rate",
            "aequivalenz_divergenzrate", "roundtrip_abweichungsrate",
            "annahme_benannt", "annahme_verpasst", "blind_reproduktion",
            "blind_build_error_rate", "eskalationsrate", "kosten_pro_approved_usd"]
    L.append("| Paarung (Formalisierer B) | " + " | ".join(cols) + " |")
    L.append("|" + "---|" * (len(cols) + 1))
    for pairing, m in sorted(res.items()):
        cells = []
        for c in cols:
            v = m[c]
            cells.append("n/a" if v is None else (f"{v:.3f}" if isinstance(v, float) else str(v)))
        L.append(f"| {pairing} | " + " | ".join(cells) + " |")

    L.append("\n`annahme_benannt` = der Judge hat eine stille Zusatzannahme selbst "
             "benannt (gut). `annahme_verpasst` = der Judge hielt die Formalisierung "
             "fuer treu, aber Aequivalenz oder die Blind-Referenz widersprechen "
             "(schlecht). Ein Judge, der nie etwas meldet, sieht in Spalte 1 gut aus "
             "und faellt in Spalte 2 durch.\n")

    L.append("\n`blind_reproduktion` zaehlt nur Laeufe, die ueberhaupt vergleichbar "
             "waren. Ein Kandidat, der nicht baut, ist kein 'nicht reproduziert', "
             "sondern gar kein Vergleich - er steht in `blind_build_error_rate`.\n")

    L.append("\n`syntaxvaliditaet_first_pass` ist der Erstversuch, `syntaxvaliditaet` "
             "der Stand nach genau einer Reparaturrunde (Eingabe: eigener Quelltext "
             "plus woertliche Compiler-Meldung, symmetrisch fuer A und B). "
             "`repair_rate` = Anteil der Laeufe, in denen ueberhaupt repariert wurde. "
             "Die Kaskade arbeitet mit dem reparierten Quelltext weiter.\n")

    L.append("\n## scope_gap: Rueckmeldung zur Task-Spezifikation\n")
    L.append("Norm-Teile, die ausserhalb der vorgegebenen Scope-Signatur liegen. Der "
             "Judge meldet sie getrennt; sie sind KEIN Modellfehler und fallen kein "
             "Gate. Ein hoher Wert heisst: der Norm-Ausschnitt ist breiter geschnitten "
             "als die Signatur - ein Befund ueber den Task-Zuschnitt.\n")
    L.append("| Paarung | scope_gap_anteil | scope_gap_je_task |")
    L.append("|---|---|---|")
    for pairing, m in sorted(res.items()):
        L.append(f"| {pairing} | {m['scope_gap_anteil']:.3f} | {m['scope_gap_je_task']:.3f} |")

    L.append("\n## Eskalation getrennt nach ausloesendem Gate\n")
    gates = sorted({g for m in res.values() for g in m["eskalation_je_gate"]})
    if gates:
        L.append("| Paarung | " + " | ".join(gates) + " |")
        L.append("|" + "---|" * (len(gates) + 1))
        for pairing, m in sorted(res.items()):
            row = [f"{m['eskalation_je_gate'].get(g, 0.0):.3f}" for g in gates]
            L.append(f"| {pairing} | " + " | ".join(row) + " |")
    else:
        L.append("Keine Eskalationen.\n")

    L.append("\nClerk-Gate n/a-Anteil (kein EStH/BMF-Rechenbeispiel; Aequivalenz + "
             "Round-Trip tragen):\n")
    for pairing, m in sorted(res.items()):
        L.append(f"- `{pairing}`: {m['clerk_gate_na_anteil']:.3f}")

    L.append("\n## Provider-Flakiness (Transport, nicht Modellqualitaet)\n")
    L.append("Timeouts, Retries und Rate-Limits sagen etwas ueber den Hoster, nicht "
             "ueber das Modell. Sie fliessen deshalb nicht in die Entscheidung ein.\n")
    L.append("| Paarung | role_timeout_rate | retries | timeouts | rate_limits | errors |")
    L.append("|---|---|---|---|---|---|")
    for pairing, m in sorted(res.items()):
        t = m["transport"]
        L.append(f"| {pairing} | {m['role_timeout_rate']:.3f} | {t['retries']} | "
                 f"{t['timeouts']} | {t['rate_limits']} | {t['errors']} |")
    for pairing, m in sorted(res.items()):
        if m["transport_by_provider"]:
            L.append(f"\n`{pairing}` je Provider:")
            for prov, c in sorted(m["transport_by_provider"].items()):
                L.append(f"- {prov}: retries={c['retries']} timeouts={c['timeouts']} "
                         f"rate_limits={c['rate_limits']} errors={c['errors']}")
        if m["role_timeouts_by_role"]:
            L.append(f"\n`{pairing}` abgebrochene Rollen: "
                     + ", ".join(f"{k} x{v}" for k, v in m["role_timeouts_by_role"].items()))

    ranked = sorted(res.items(), key=lambda kv: (kv[1]["eskalationsrate"],
                                                 kv[1]["kosten_pro_approved_usd"] or 0.0))
    rates = [m["eskalationsrate"] for m in res.values()]
    spread = max(rates) - min(rates)
    saturated = min(rates) >= 0.85   # praktisch jeder Lauf eskaliert
    too_close = spread < 0.10        # Unterschied im Rauschen

    L.append("\n## Empfehlung\n")
    if saturated or too_close:
        L.append("**Kein Entscheid moeglich.** Die Entscheidungsmetrik traegt nicht:\n")
        if saturated:
            L.append(f"- Die Eskalationsrate ist gesaettigt (Minimum {min(rates):.3f}). "
                     f"Praktisch jeder Lauf wird eskaliert, also trennt sie die "
                     f"Paarungen nicht.")
        if too_close:
            L.append(f"- Der Abstand betraegt nur {spread:.3f} und liegt bei "
                     f"n={ranked[0][1]['n']} Tasks je Paarung im Rauschen.")
        L.append("\nEin Sieger waere hier ein Artefakt der Metrik, kein Befund ueber "
                 "die Modelle. Vor einer Entscheidung muss die Saettigungsursache "
                 "behoben und der Bake-off wiederholt werden.\n")
        L.append("Zur Orientierung, ohne Entscheidungscharakter:\n")
        for name, m in ranked:
            L.append(f"- `{name}`: eskalation {m['eskalationsrate']:.3f}, "
                     f"syntax {m['syntaxvaliditaet']:.3f}, "
                     f"aequivalenz-divergenz {m['aequivalenz_divergenzrate']:.3f}, "
                     f"kosten ${m['kosten_gesamt_usd']:.4f}")
    else:
        best, second = ranked[0], ranked[1]
        b_cnt = round(best[1]["eskalationsrate"] * best[1]["n"])
        s_cnt = round(second[1]["eskalationsrate"] * second[1]["n"])
        L.append(f"**`{best[0]}`** (Eskalationsrate "
                 f"{best[1]['eskalationsrate']:.3f}; Kosten nur Tiebreaker). "
                 f"Entscheidung trifft Julius.\n")
        # Prozentzahlen bei n=14 taeuschen Praezision vor. Der Vorsprung wird
        # deshalb in Tasks ausgewiesen, nicht nur als Rate.
        delta = s_cnt - b_cnt
        L.append(f"\nVorsprung in absoluten Zahlen: {b_cnt}/{best[1]['n']} eskalierte "
                 f"Laeufe gegenueber {s_cnt}/{second[1]['n']} beim Zweiten "
                 f"(`{second[0]}`) - ein Unterschied von {delta} Task"
                 f"{'s' if delta != 1 else ''}.")
        if delta <= 2:
            L.append(f"\n**Schwacher Vorsprung.** {delta} Task"
                     f"{'s' if delta != 1 else ''} bei n={best[1]['n']} kann ein "
                     f"einzelner anders geschnittener Norm-Ausschnitt drehen. Die "
                     f"Rangfolge ist ein Hinweis, kein belastbarer Befund; wer sie "
                     f"als Entscheidung liest, ueberdehnt die Datenlage.")
    return "\n".join(L)


def main() -> int:
    runs_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "pipeline", "runs")
    reports = load_reports(runs_dir)
    res = evaluate(reports)
    md = to_markdown(res)
    out = os.path.join(ROOT, "reports", "gate-g2.md")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w", encoding="utf-8") as f:
        f.write(md)
    print(md)
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
