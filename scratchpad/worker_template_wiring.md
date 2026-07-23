# WORKER-TEMPLATE: eine verified_bedingt-Regel verdrahten (Stufe-1)

Verbindliche Konventionen. Die frischen Worker haben diese 4 Dinge systematisch falsch gemacht — HIER EXAKT befolgen. Der Instructor gatet die Voll-Suite auf dem Main-Tree nach cherry-pick (dein Worktree kann die Voll-Suite nicht fahren — Catala-pkg fehlt); du lieferst korrekten Code + korrekte bindung/tests.

## 0. Accessor (runner.py) = PURE-PYTHON, KEIN neues Catala-Modul
Wie §34c/§32b/§10-Nr.5: reine Python-Funktion `def catala_<regel>(s: dict) -> int:`, reproduziert die Snapshot-test_seeds EXAKT. KEINE `from pkg import X`-Zeile, KEIN rules/estg/<regel>/-Verzeichnis. Nimmt EUROS, gibt EUROS (int).

## 1. api.py-Wiring = INLINE im slot, `_c()`-Extraktion
Nicht eine separate Modul-Funktion mit `runner.` (die vergisst `import runner`). Inline im _festzusetzende:
`... + runner.catala_<regel>({"slot": _c("feld_id") // 100, ...})`  (cent→euro via //100 im slot).
Für int-Zähler (kein Geld): `f.get("feld_id", {}).get("wert", 0) or 0`. BEIDE Ringe (gesamt + rentner) — oder via GESAMT_ABZUEGE-Tupel das automatisch in rentner spiegelt.

## 2. bindung = korrektes Format + EXAKTE luecken-Namen  ⚠ HÄUFIGSTER FEHLER
Jedes Feld:
```yaml
  - feld_id: <name>
    quelle: {regel_id: <regel>, signatur_slot: <catala-input>}
    typ: cent            # oder int/bool
    einheit: EUR         # oder null
    askable: true
    vorjahr: vorschlag   # oder uebernehmbar
    fragetext_laie: "..."
    hilfe_kurz: "..."
    beispielwert: 100000
    elster_kz: null
    elster_kz_grund: "MVP: XSD-Sektion unklar, Vordruck-Cross-Ref (Folge-Gate)."
    vz_gueltigkeit: [2024, 2025, 2026]
    anker_ref:
      quelle: "§ ... EStG"
      zitatanker: "<wörtliches Zitat aus der Quelle>"
      datei: "sources/gesetze-im-internet/estg_pXX_....txt"
```
**luecken (KRITISCH):** JEDE Geltungsbedingung der Regel MUSS entweder als Feld gebunden ODER in `luecken:` mit dem **EXAKTEN** geltungsbedingung-Namen stehen. Namen NICHT erfinden — greppe sie:
`grep -A40 "regel_id: <regel>" pipeline/produktion/rules.yaml | grep -i geltung` bzw. den Snapshot. test_bindungstabelle.py::test_b_vollstaendigkeit prüft das hart.
```yaml
luecken:
  - regel_id: <regel>
    geltungsbedingung: <EXAKTER_NAME_AUS_DER_REGEL>
    grund: "... (materialisiert im Accessor / vorgelagert / im Beleg-Writer)."
```

## 3. Test = pytest-Accessor-Unit-Test, KEIN golden-case-yaml
Datei tests/test_<regel>_accessor.py (Muster test_p32b_accessor.py):
```python
import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R
def test_seed_x():
    assert R.catala_<regel>({...}) == <erwartet>
```
KEINE golden/cases/*.yaml anlegen (die brauchen quelle:{datei,zitatanker}-dict + e2e-Format = du kriegst es falsch → test_golden_anker_freeze/test_einheiten brechen).

## 4. §24a-Falle (nur bei neuen EINKÜNFTEN in einkuenfte_sonstige)
Wenn deine Regel Einkünfte in einkuenfte_sonstige/renten legt: prüfe ob die Einkunftsart in der §24a-S.2-Ausschlussliste steht (nur §22 Nr.1/4/5 raus). Wenn NICHT ausgeschlossen → MUSS additiv in die §24a-`positive_andere_einkuenfte`-Bemessung (sonst Over-tax >64J). Vgl. §23-Wiring.

## Ablauf
Recon (Snapshot-seeds + Geltungsbedingungen greppen) → Accessor → bindung (luecken-Namen greppen!) → api-wiring beide Ringe → accessor-unit-test → commit auf deinen worktree-branch → Branch+SHA + geänderte Dateien an main melden. Bei echter Ambiguität STOPP+frag. Instructor gatet Voll-Suite auf main.
