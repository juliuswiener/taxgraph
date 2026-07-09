# Setup (Phase 0)

Nachvollziehbares Setup der Toolchain von Null. Getestete Umgebung:

| Baustein | Version |
|----------|---------|
| OS (Referenz-Setup) | Ubuntu 24.04.4 LTS |
| opam | 2.1.5 |
| OCaml | 4.14.2 (opam switch `taxgraph`) |
| Catala | 1.2.0 |
| Clerk | 1.2.0 |
| Python (Oracle + Catala-Python) | 3.12.3 |
| GETTSIM | 1.2.1 |
| NumPy | siehe `oracle/.venv312` |

Hinweis zur Umgebung: Der Handover nennt Arch Linux (arch-desk, Fish). Dieses
Referenz-Setup lief auf Ubuntu 24.04 (Bash). Der einzige Unterschied ist der
Paketmanager fuer opam (`apt` statt `pacman`); alles Weitere ist identisch, da
Catala und GETTSIM ueber opam bzw. pip/venv kommen. Auf Arch entsprechend
`pacman -S opam` statt `apt-get install opam`.

## 1. Catala und Clerk (opam)

```bash
# System-Abhaengigkeiten
sudo apt-get update
sudo apt-get install -y opam m4 build-essential pkg-config libgmp-dev
# (Arch: sudo pacman -S opam m4 base-devel pkgconf gmp)

# opam initialisieren und dedizierten Switch anlegen
opam init --bare -y
opam switch create taxgraph 4.14.2
eval $(opam env --switch=taxgraph --set-switch)

# Catala (bringt Clerk mit)
opam install -y catala

catala --version   # 1.2.0
clerk --version    # 1.2.0
```

Das Skript `scripts/install-catala.sh` fasst diese Schritte zusammen (auf Ubuntu
getestet). Bei jeder neuen Shell muss die opam-Umgebung geladen werden:
`eval $(opam env --switch=taxgraph --set-switch)`. Das Makefile erledigt das pro
Target selbst.

### Toolchain verifizieren

```bash
make tests    # baut Catala-Runtime + stdlib und laeuft alle Clerk-Tests (12 gruen)
```

## 2. GETTSIM und Catala-Python-Runtime (Python)

Es werden zwei virtuelle Umgebungen genutzt (System-Python bleibt unangetastet):

- `oracle/.venv` (Python 3.11): reines GETTSIM, optional.
- `oracle/.venv312` (Python 3.12): GETTSIM **und** die Catala-Python-Runtime.
  Die von Catala erzeugte Python-Runtime nutzt PEP-695-Syntax
  (`class Array[T: Value]`) und benoetigt daher Python >= 3.12. Der
  Differentialtest laeuft in dieser Umgebung.

```bash
# uv wird als Environment-Manager verwendet (schnell, reproduzierbar)
uv venv oracle/.venv312 --python 3.12
uv pip install --python oracle/.venv312/bin/python gettsim   # zieht 1.2.1
```

`scripts/install-gettsim.sh` legt die 3.11-Umgebung an; fuer den Differentialtest
ist die 3.12-Umgebung noetig (Kommandos oben).

## 3. Differentialtest ausfuehren

```bash
make s02
```

Das Kommando kompiliert das Catala-Modul `Einkommensteuertarif` nach Python
(`clerk build p32a-python`), fasst das Python-Paket zusammen
(`oracle/gettsim/assemble_catala.sh`) und startet den Harness
(`oracle/gettsim/harness.py`), der pro Veranlagungszeitraum 1000 geseedete
zvE-Werte plus Randwerte durch Catala und GETTSIM schickt und
`reports/s02-divergenzen.md` neu erzeugt.

## Makefile-Ziele

| Ziel | Wirkung |
|------|---------|
| `make tests` | Alle Clerk-Scope-Tests (S0.1 Tarif + S0.3 Arbeitszimmer/Homeoffice). |
| `make s01` | Nur die § 32a-Tarif-Tests. |
| `make s03` | Nur die Arbeitszimmer/Homeoffice-Tests. |
| `make build-python` | Catala -> Python-Paket bauen und zusammenfuegen. |
| `make s02` | Differentialtest gegen GETTSIM, erzeugt den Divergenzreport. |
| `make params-check` | Ableitung/Validierung der Tarifkoeffizienten. |
| `make clean` | Build- und Zwischenartefakte entfernen. |

## Bekannte Reibungspunkte (dokumentiert, keine Blocker)

- **Cross-Modul-Scope-Aufrufe** erfordern `catala --whole-program` bzw.
  `clerk test -W`. Das Makefile nutzt durchgaengig `-W`.
- **Modulname == Dateiname.** Ein `> Module Foo` verlangt die Datei `Foo.catala_en`
  (Grossschreibung des Basenamens). Beispiel hier: Modul `Einkommensteuertarif`
  in `einkommensteuertarif.catala_en`.
- **`Decimal.floor` / `Decimal.ceiling`** der stdlib sind Cap-Funktionen
  (min/max), kein ganzzahliges Ab-/Aufrunden. Fuer die gesetzliche Abrundung auf
  volle Euro wird `Decimal.truncate` verwendet (bei nichtnegativen Werten gleich
  dem Abrunden).
