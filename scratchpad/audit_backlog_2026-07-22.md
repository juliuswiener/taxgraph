# Audit Backlog Punch-List 2026-07-22

Found across 4 independent Wiring-Correctness-Audits (fail-open, erreichbarkeit, komplement, partner-symmetrie).  
Sequenziert in dev-1s serielle Post-KiSt-Queue (api.py/runner.py, seine Zone).

---

## P1 — Under-tax (echt)

| # | Titel | File:Line | Richtung | Exposure | Fix-Skizze | PRIO |
|---|-------|-----------|----------|----------|-----------|------|
| 1 | §16 Abs.4 Veräußerungsfreibetrag ohne Age-Gate | `runner.py:516` (Backlog-Bekenntnis) | **Under-tax** — FB stets gewährt, aber S.1 verlangt 55.Lj/berufsunfähig, S.2 Einmaligkeit | max 45k€ stpfl. Gewinn (~15k€ Steuer), selten (<55-§16-Kombi) | Gate `p16_4_gate_offen` im _an_gesamt_sperrgrund bei vg>0 ohne bestätigtes Alter/Berufsunfähigkeit; Accessor auf Felder `rentner_alter_55_oder_berufsunfaehig` + `rentner_freibetrag_erstmalig` gaten (zuerst Felder in SCHEIBEN.felder aufnehmen, dann Guard). | NIEDRIG |

## P2 — Over-tax (sicher, aber Nutzer-nachteilig)

| # | Titel | File:Line | Richtung | Exposure | Fix-Skizze | PRIO |
|---|-------|-----------|----------|----------|-----------|------|
| 2 | Rentner Person-B KV/PV-Vorsorge fehlt | `api.py:1444-1445` (dokumentierte Lücke) | **Over-tax** — Bs KV/PV-Abzug fehlt bei Zusammenveranlagung im Rentner-Ring | 2800€ Höchstbetrag × ~14-40% = 400-1100€ Over-tax; halbwegs häufig (Rentner-Ehepaare) | Resolve `Person-B-KV/PV DEFER` im rentner slot_fn: selbe `catala_p10_kv_pv` mit _partner-Feldern, unter `if zusammen` additiv, exakt wie gesamt L984-988. | MITTEL |
| 3 | Rentner Person-B VOR (Basisvorsorge RV) fehlt | `api.py` implizit (VOR_PARTNER_FELDER nicht in RENTNER_FELDER) | **Over-tax** — Bs RV-Vorsorge fehlt bei Rentner-Zusammenveranlagung | ~27566€ HB, Over-tax seltener (Rentner ohne aktive RV-Beiträge) | Identisches Muster wie gesamt L921-924: `vor_*_partner` unter `if zusammen` additiv in die Summen-Slots. Felder brauchen SCHEIBEN.felder-Nachtrag + Wiring. | NIEDRIG |
| 4 | Person-B-WK hart 0 in gesamt-Ring | `api.py:864` (wk=0) | **Over-tax** — Bs Werbungskosten (§9) fehlen im gesamt zusammen-Pfad | EP-Pauschale 1230€ (~170-400€ Steuer), häufig bei Paaren mit B-Pendler | `wk_b` aus _partner-WK-Feldern beziehen (wg. EP/dHf/Verpflegung/GWG je Person — mehrjähriger Nachtrag). | NIEDRIG |
| 5 | an_gesamt Person-B VOR fehlt (Guard) | `api.py:670` (Guard sperrt Partner-VOR) | **Over-tax** — Bs RV-Vorsorge fehlt im AN-only-Ring, aber Ring wechselt zu gesamt (Kinder/Verlustvortrag zwingen) | s.o.; tritt auf bei Paaren mit Kinder/Verlustvortrag | Guard wird ohnehin getriggert → kaum Exposure in der Praxis. Fix: VOR_PARTNER_FELDER in den zusammen-Pfad + K2-check. | NIEDRIG |

## P3 — Cleanup/Kosmetik

| # | Titel | File:Line | Richtung | Fix-Skizze | PRIO |
|---|-------|-----------|----------|-----------|------|
| 6 | KiSt-Felder totes Wiring | `bindung_p51a_kirchensteuer.yaml` (kist_konfession, kist_bundesland) | **Neutral** (aktuell tot → KiSt rechnet immer 0) | Fix läuft bei dev-1 (SCHEIBEN.felder-Nachtrag). Gate `test_erreichbarkeit_gate.py` rot bis dahin → wird grün. | KRITISCH (Gate rot) |
| 7 | ~12 vestigiale n_vor_gwg-Felder | `bindung_n_vor_gwg.yaml` (gwg_*, vpf_*) | **Neutral** (Design-Vestigial, kein Bescheid aus dieser Scheibe) | YAML-Cleanup (Felder entfernen) — kein ESt-Impact, könnte graph-count-Test zerreißen. | VERY-LOW |

---

**Legende**: P1 = Under-tax (echte Steuerausfälle). P2 = Over-tax (sicher, Nutzer-nachteilig). P3 = Neutral/formal.  
**Nächster Dev-Schritt**: P1 → P2 → P3. Fix #6 läuft bereits (dev-1 KiSt).
