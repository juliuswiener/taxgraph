# Catala Issue (ABGESENDET 2026-07-09)

Status: von Julius eroeffnet als
[CatalaLang/catala#1074](https://github.com/CatalaLang/catala/issues/1074).

Betrifft die Catala/Clerk-Version 1.2.0. Dieses Dokument bleibt die
Arbeitsfassung (Minimalrepro, Workarounds).

Sprache im Entwurf: Englisch (Projekt-Konvention).

---

## Title: OCaml backend: cross-module enum types are not exported in the generated `.mli`

**Version:** Catala 1.2.0, Clerk 1.2.0 (opam), OCaml 4.14.2.

**Summary.** When a module `A` declares an enumeration and a second module `B`
(`> Using A`) uses that enumeration as a *type* in a declaration (e.g. a scope
input `input x content A.MyEnum` or a toplevel function
`declaration f content ... depends on x content A.MyEnum`), compiling `B` with
the OCaml backend fails with `Unbound module A.MyEnum`. The enum type is emitted
in `A`'s generated `.ml` (as `module MyEnum = struct ... end`) but is **not
re-exported in `A`'s generated `.mli`**, so `B` cannot reference it.

Cross-module **struct** types and cross-module **scope calls** work; only
cross-module **enum** *type references* fail. Interpretation with
`--whole-program` works in all cases; the failure is specific to separate OCaml
module compilation (which `clerk test` performs).

**Minimal reproduction.**

`A.catala_en`:
```
> Module A
```catala
declaration enumeration MyEnum:
  -- One
  -- Two
```
```

`B.catala_en`:
```
> Module B
> Using A
```catala
declaration scope S:
  input e content A.MyEnum
  output r content integer
scope S:
  definition r equals match e with pattern -- One: 1 -- Two: 2
```
```

`clerk test -W` (which compiles `B` to OCaml) fails:
```
Error: Unbound module A.MyEnum
```
The generated `A.mli` contains only the stdlib re-exports and `val loc`, not
`module MyEnum`.

**Impact.** It blocks factoring shared enumerations (here: a
`Veranlagungszeitraum` / tax-year enum) into a base module that other rule
modules depend on. In a growing rule library this forces either co-locating all
rules that share the enum into one module, or passing enum-typed values around
as separate primitive inputs.

**Workarounds we use.**
1. Co-locate scopes that need the shared enum in the same module as the enum.
2. Where a rule can be parameterised by primitive values instead of the enum,
   pass those (money/decimal/int/bool) across the module boundary and select
   them by enum in the enum's own module. Struct/primitive types cross module
   boundaries fine.

**Ask.** Export enum type definitions in the generated `.mli` so cross-module
enum-typed declarations compile with the OCaml backend, consistent with structs.

---

## Interne Notiz (nicht Teil des Issues)

Fuer TaxGraph aktuell durch die beiden Workarounds geloest (siehe
`rules/estg/p32a/einkommensteuertarif.catala_en` Kopfnotiz und das Modul
`Entfernungspauschale`, das die VZ-Saetze als Parameter erhaelt). Relevant, wenn
in Phase 3 viele Regeln ueber Modulgrenzen komponieren.
Abgesendet als CatalaLang/catala#1074; Antwort des Projekts hier nachtragen.
