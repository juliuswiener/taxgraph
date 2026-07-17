# `produkt/` — der fragbare UI-Kern (Task #11, Paket A)

Deterministischer, **LLM-freier**, source-verankerter, fail-closed Kern zwischen der Steuer-Engine
(`rules.yaml`/Catala/Golden) und der Privat-Oberfläche (Paket B, „die Haut"). **Der Store ist die
Wahrheit; alles andere ist Ableitung.** Die Haut liest hier und schreibt über **genau einen** Pfad —
der Vertrag steht in [`traverser/API.md`](traverser/API.md) (autoritativ).

## Die 6 Bausteine

| # | Baustein | Ort | Rolle | Public-Einstieg |
|---|---|---|---|---|
| 1 | **Bindungstabelle** | `bindung/` (4 Scheiben + `schema.json`) | bindet Regel-Slot/Geltungsbedingung → laienverständliches Feld + amtliches ELSTER-Kz + Zitatanker | YAML (via `traverser.lade_bindung()`) |
| 2 | **Store** | `store/store.py` | event-sourcierte Wahrheit; Zwei-Signal-Bestätigung; Zustands-/Herkunft-Meet; content-addressiert | `append_event`, `materialisiere`, `erzeuge_snapshot` |
| 3 | **Unsicherheits-Derivat** | `unsicherheit/intervall.py` | `[min,max]`-Bescheid + Beitrag je unsicherem Feld aus den `bereich`-Grenzen | `intervall`, `bescheid_via_slots` |
| 4 | **Traverser** | `traverser/traverser.py` (+ `guenstiger_liste.yaml`) | Regel-Graph bidirektional: Relevanz, Interview-Queue, Vorwärts-Trace | `relevanz`, `naechste_fragen`, `justification`, `trace_ergebnis` |
| 5 | **est_mapping** | `mapping/est_mapping.py` | Store-Snapshot → ELSTER-Deklaration (5 Fall-Klassen); fail-closed; Round-Trip | `deklariere`, `zuruecklesen`, `konsistenz_feldmapping` |
| 6 | **API-Vertrag** | `traverser/API.md` | die EINZIGE Kern↔Haut-Schnittstelle (lesen + der eine Schreibpfad) | — (Doku) |

## Echte Signaturen (Konsum-Naht für Paket B)

```
# 1. Metadaten + Fragetexte laden (Anzeige/Validierung, read-only)
bindung = traverser.lade_bindung()                          # {feld_id -> {typ, fragetext_laie, hilfe_kurz, bereich, elster_kz, anker_ref, ...}}

# 2. Was fragt die Haut als nächstes? (Gating zuerst, dann Unsicherheits-Beitrag)
fragen  = traverser.naechste_fragen(store, bindung)          # -> [feld_id, ...] geordnet

# 3. Antwort schreiben — GENAU EIN Schreibpfad
store.append_event(store, feld_id=…, wert=…, zustand="vorlaeufig"|"bestaetigt",
                   herkunft=…, schreiber="ui:laie"|"llm:chat", signal=…, ersetzt=None)
#   LLM-Chat -> nur zustand="vorlaeufig" + schreiber="llm:…" + signal_2=None (Store-Auflage A, hart erzwungen)
#   Bestätigen (Zwei-Signal) -> zustand="bestaetigt" + signal_2 + schreiber="ui:…" (menschlicher Klick)

# 4. Aktueller Stand + Steuer-at-Risk-Band
snapshot, sid = store.materialisiere(store)
#   bescheid_fn aus der Engine bauen — quantitaet = golden-Erwartungswert-Key; der Adapter
#   normalisiert die Engine-Nativ-Ausgabe (euro ODER cent) auf die kanonische Naht-Einheit CENT:
bescheid_fn = unsicherheit.bescheid_via_slots(bindung, slot_fn, quantitaet="abziehbarer_betrag")
band = unsicherheit.intervall(snapshot, bindung, bescheid_fn)   # {intervall: {min_cent, max_cent, offene_achsen, nicht_fixierbar, ...}, beitraege: [{feld_id, spanne_cent, ...}]}  — alles in CENT

# 5. Deklaration (fail-closed: nur zustand=bestaetigt fließt; ein vorlaeufig -> vollstaendig=False)
dekl = est_mapping.deklariere(snapshot, bindung, snapshot_id=sid)  # {deklaration E-Nr->Wert, dokumentiert (Σ nicht-deklariert), nicht_deklariert, unvollstaendig, vollstaendig}

# 6. Nachvollziehbarkeit
just  = traverser.justification(store, feld_id, bindung)     # Vorwärts-Trace: wert/zustand/herkunft/anker_ref/regel_id
```

## Fail-closed (mechanisch, nicht per Bitte) — Kurzform, Detail in API.md

1. Ein KI-Wert (`vorlaeufig`) kann strukturell **nicht** in eine festzusetzende Summe fließen (`store.meet_zustand`).
2. `bestaetigt` erfordert `signal_2` (Zwei-Signal) — Schema + `append_event`.
3. `llm:`-Schreiber ist an `llm_vorschlag`/`vorlaeufig` gekoppelt.
4. Höchstens ein aktives Event je `feld_id`; Überschreiben nur via gültiges `ersetzt`.
5. ELSTER-Befund bindet an `snapshot_id`; `plausibel` mit `gekappt_verdacht=true` ist nie grün.

## Abdeckung (fragbarer Kern, Stand 2026-07-17)

4 Bindungs-Scheiben — **Angestellte / Rentner / Klein-Vermieter**:
`bindung_n_vor_gwg` (N/VOR/GWG), `bindung_sonder_agb_35a` (KiSt/Ausbildung/agB/§35a),
`bindung_kap_vv_familie` (§20/§21/Familie), `bindung_rentner` (§22/§24a/§33b/§16 Abs. 4).

**Benannte Lücken statt Rate-Mapping:** fehlende ELSTER-Kz (`elster_kz: null` + Grund) und nicht-abgefragte
Slots/Geltungsbedingungen (`luecken`) sind explizit; der Gate akzeptiert sie als bewusste Nicht-Abdeckung,
nie als stilles Loch. Offen: §34 ao-Betrag (kein Regel-Slot → Task #12, dev-1-Zone), Multi-Objekt-§21,
Per-Kind-Kz.

## Einheiten-Konvention (kanonische Naht-Einheit CENT)

Store-Inputs sind Cent (`bindung typ:cent`). Die Engine (`golden/runner.py`) liefert GEMISCHT — Euro
(`int(...)//100`: EP, Arbeitszimmer, festzusetzende/tarifliche ESt, §34-Fünftel) oder Cent (GewSt,
KStG-Nenner-B, §35c, Kfz). `unsicherheit.bescheid_via_slots(..., quantitaet=…)` normalisiert die
Nativ-Ausgabe je Quantität (`NATIV_EINHEIT`, Schlüssel = golden-Erwartungswert-Key) verlustfrei auf
CENT — so ist die ganze Naht in EINER Einheit und die Haut zeigt konsistent `euro()=cent/100`. Eine
ungemappte Quantität wirft (kein stiller euro/cent-Default); `tests/test_einheiten.py` sichert Map-
Vollständigkeit, Konvention und Exaktheit (EP 2156→215600, Nenner-B unverändert). ELSTER-Kz-Format
(euro/cent je Feld) bleibt Submission-Layer-Sache.

## Gates

`tests/test_bindungstabelle.py` · `test_store.py` · `test_unsicherheit.py` · `test_traverser.py` ·
`test_est_mapping.py` · `test_paket_a_e2e.py` · `test_einheiten.py` — **80/80 grün**, NULL LLM. Jeder
Zitatanker wird voll-Länge via `pipeline/gates._normalize` gegen die Quelldatei geprüft; die
Einheiten-Konvention ist Map-Tamper-verifiziert. est_mapping deckt die 1:1-Kz der Scheiben 2-4 ab
(Kapital §20, V+V-Mieteinnahmen, §35a/agB); §21-WK = dokumentierte Nicht-Deklaration (Anlage-V-Ruling).
