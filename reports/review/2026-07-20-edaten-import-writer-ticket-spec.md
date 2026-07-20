# Folge-Ticket-Spec: eDaten-Import-Writer (`import:elster`)

**Status:** SPEC (kein Bau). Priorität **NIEDRIG**. Blockiert auf: eDaten-Import-Kanal-Entscheidung (Belegabruf/VaSt via ELSTER-API → ERiC/Zertifikat = Julius-Cap).
**Herkunft:** Recon 2026-07-20 (Instructor-K1-Nebenbefund). Adjudikation: KEINE Zwei-Signal-Lücke wie K1 (amtlich verteidigbar).

---

## 1. Kontext & Adjudikation

Instructor-Nebenbefund während K1: „`import:elster` schreibt `bestätigt` → eDaten fließen in die festgesetzte Steuer OHNE menschliches signal_2 = Zwei-Signal-Lücke?"

**Adjudiziert NEIN.** Zwei Gründe:

1. **Kein Live-Leak.** Es existiert HEUTE kein eDaten-Import-Writer. `produkt/import/` = `beleg_writer.py` / `kontoauszug_writer.py` / `vorjahr_writer.py`. `import:elster` ist reine **Vertrags-Identität**: nur in Tests (`test_store` / `test_beleg_writer` / `test_ui_zwei_signal_sicherheit`) + `store.py:129`-Docstring + `api.py:1341`-Kommentar. `est_mapping` = OUTBOUND (Store→ELSTER-Submission), `/elster-ampel` = 503.

2. **Auto-bestätigt ist amtlich defensibel.** §150 Abs. 7 Satz 2 AO (Quelle: `sources/bmf/bmf_riester_foerderung_2023-10-05.txt:1250-1255`):
   > „die von den mitteilungspflichtigen Stellen nach Maßgabe des § 93c AO übermittelten Daten gelten als die erklärten Daten des Steuerpflichtigen, soweit sie … als eDaten gekennzeichnet sind … und er nicht … abweichende Angaben macht."

   → eDaten sind der amtliche DEFAULT-Erklärungswert des Steuerpflichtigen OHNE aktiven per-Feld-Confirm. Das gesetzliche Recht ist die **Abweichung (Override)**, nicht der Confirm. Die FA-§93c-Quelle IST das signal_2-Äquivalent.

**Konsequenz:** Der eDaten-Writer soll `bestätigt` schreiben (nicht `vorläufig` wie beleg/kontoauszug). Per-Feld-Confirm zu erzwingen wäre **amtlich falsch** und UX-schädlich.

## 2. Store-Vertrag (Ist, bleibt)

- `import:elster` exempt von Auflage-A (kein `vorlaeufig`-Zwang) und K1-Katalog (`_vorschlag_typ`=None, `store.py:127-134`).
- Universeller Gate `store.py:220` (`zustand=bestaetigt braucht signal_2`) gilt AUCH für `import:elster` → der Writer MUSS ein `signal_2` liefern.
- `signal_2` = **eDaten-Quell-Anker** (§93c-Datensatz-Referenz, z.B. LStB-Zeile — Test-Präzedenz `"lstb_z23"`). Provenance, kein Mensch-Geste-Vortäuschen; das ist by-design korrekt (§150 Abs.7 = Quelle IST das zweite Signal).

## 3. Zu materialisierender Vertrag (wenn gebaut)

**W1 — Feld-Grenze (Nicht-Unterdrückung).** Der Writer darf NUR echte §93c-eDaten-Felder setzen: Lohn (§19), Renten (§22), Versicherungsbeiträge (§10), Lohnsteuer-Abzüge. Er darf NIE Abzüge/Wahlrechte setzen, die das FA nicht kennt (Werbungskosten / agB / Spenden / Wahlrechte) — die bleiben human-only (K1-Katalog hält sie schon; der Writer ist zusätzlich exempt, also über eine EIGENE Feld-Whitelist zu begrenzen, NICHT über den Katalog). Grund: eDaten dürfen Nutzer-Ergänzungen strukturell nicht verdrängen.

**W2 — Override-Pfad.** Ein `mensch`-Writer (`ui:*`) MUSS jeden `import:elster`-bestätigt-Wert via `ersetzt=<event_id>` überschreiben können (Auflage-B, existiert). UI-Anforderung: die eDaten müssen dem Nutzer SICHTBAR sein und die Abweichung (abweichende Angabe) mit einer Geste möglich — das ist die amtliche „abweichende Angaben"-Pflicht.

**W3 — Provenance-Guard.** Der Provenance-Guard ist SCHREIBER-scoped (`^import:elster`), analog beleg (`^import:beleg`). Herkunft `beleg_import` bleibt gültig (Test-Präzedenz: `import:elster`+`beleg_import`+`bestaetigt` ist erlaubt).

## 4. Explizit NICHT tun

- KEIN per-Feld-Confirm für eDaten erzwingen (amtlich falsch, §150 Abs.7).
- KEIN `vorläufig`-Zwang für `import:elster` (im Gegensatz zu beleg/kontoauszug/vorjahr).
- Die Katalog-Exemption NICHT als Freibrief — die Feld-Grenze (W1) über eine eigene eDaten-Whitelist ziehen.

## 5. Test-Skizze (bei Bau)

- **Positiv:** `import:elster` + §93c-Feld (Lohn) + `signal_2`=eDaten-Anker → `bestaetigt`, fließt in festgesetzte Steuer.
- **Negativ (W1):** `import:elster` versucht ein Abzugs-/Wahlrechts-Feld (agB / Spende / Wahlrecht) → Writer-Whitelist lehnt ab.
- **Override (W2):** `ui:laie` `ersetzt` einen `import:elster`-Wert → mensch-Wert aktiv, eDaten-Event ersetzt.
- **Nicht-Unterdrückung (W2):** Nutzer-Ergänzung (WK, human-only) koexistiert mit eDaten-Lohn → beide in der Deklaration.
- **Gate `store.py:220`:** `import:elster` + `bestaetigt` OHNE `signal_2` → ValueError (bestehender Gate deckt).

## 6. Referenzen

- Recon-Memory: `edaten-import-elster-p150-abs7-ao.md`
- K1-Security-Architektur: `reports/review/2026-07-20-ui-feldkatalog-eventschema-designlock.md`, Memory `k1-security-mapping-gruen-nicht-ring-invariant.md`
- Amtlich: §150 Abs. 7 Satz 2 AO / §93c AO — `sources/bmf/bmf_riester_foerderung_2023-10-05.txt:1250-1255`
