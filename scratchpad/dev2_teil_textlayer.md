# Teil-Textlayer-Fix — Design-Draft (transplant-ready)

Betrifft `produkt/import/kontoauszug_writer.lies_kontoauszug_pdf` (committed) UND
`produkt/import/beleg_writer.lies_beleg_text` (committed) — geteiltes Muster, geteilter Fix.

## Bug bestätigt (empirisch, synthetisches 2-Seiten-PDF, kein echtes Sample nötig)

Seite 1 = echter Textlayer (Transaktion A), Seite 2 = Bild-Scan (Transaktion B), via `pdfunite`
zusammengefügt (2 physische Seiten, `pdfinfo` bestätigt `Pages: 2`).

```
pdftotext -layout auf dem 2-Seiten-PDF liefert:
'12.03.2025 Malermeister Schmidt -480,00\n31.03.2025 Gehalt 2.500,00\n\x0c\x0c'
```

`text.strip()` auf dem VOLLEN Text ist non-empty (Seite 1 trägt echten Text) → `lies_kontoauszug_pdf`
gibt das als "sauberer Textlayer", `{}` (implizit Confidence 1.0) zurück → OCR läuft NIE → Seite 2
(Transaktion B, "Spende Rotes Kreuz -200,00") ist SPURLOS weg — kein Gap, kein Hinweis, still
unvollständig. Per-Seiten-OCR (nur Seite 2 rastern + tesseract) holt die Zeile korrekt zurück
(empirisch verifiziert: `pdftoppm -f 2 -l 2` + `tesseract --tsv` → Wörter `20.03.2025 Spende Rotes
Kreuz -200,00`).

`\x0c`-Split-Verhalten (empirisch, wichtig für die Implementierung): `pdftotext` hängt NACH JEDER
Seite ein `\x0c` an (auch nach der letzten) → `text.split("\x0c")` liefert **n_seiten + 1** Segmente,
das letzte immer leer. Korrektes Recovery: `seiten = text.split("\x0c")[:-1]` (das letzte Element
IMMER droppen, nicht nur wenn leer — sonst bricht ein Dokument, dessen letzte Seite zufällig durch
Zufall nicht-leer nach dem letzten `\x0c` wäre — kommt bei pdftotext nicht vor, aber `[:-1]` ist
robuster als ein leer-Check).

## Shared Helper (geteilt kontoauszug_writer + beleg_writer, oder in ein gemeinsames Modul — TBD
## beim Transplant, siehe „Offene Frage" unten)

```python
def _textlayer_ist_plausibel(seiten_text: str) -> bool:
    """Eine Seite gilt als PLAUSIBEL textlayer-erfasst, wenn sie eine Mindest-Zeichendichte hat.
    Konservativ (K2: lieber unnötige Extra-OCR als eine still verpasste Bild-Seite) — ein Schwellwert
    von 20 Zeichen killt sowohl leere Scan-Seiten (0 Zeichen) als auch Garbage-Reste (z.B. eine
    eingebettete Seitenzahl/Fußzeile als einziges echtes Textobjekt auf einer sonst gescannten Seite)."""
    return len(seiten_text.strip()) >= 20
```

Der Schwellwert 20 ist eine grobe Setzung (keine Norm/kein Gesetzeswert — rein technische
Heuristik, KEINE Instructor-Adjudikation nötig). Kann bei echten Bank-/Beleg-Samples nachjustiert
werden (Julius-Cap NUR für die Validierung/Nachjustierung, nicht fürs Bauen).

## Modifizierte `lies_kontoauszug_pdf` (Kern-Idee, Signatur unverändert)

```python
def lies_kontoauszug_pdf(pfad: str) -> tuple[str, dict]:
    text = subprocess.run(["pdftotext", "-layout", pfad, "-"], capture_output=True, text=True).stdout
    text = _fix_bel(text)
    if not text.strip():
        return _ocr_tesseract_zeilen(pfad)              # unverändert: Voll-Scan-Pfad (0 Text überhaupt)

    seiten = text.split("\x0c")[:-1]
    if all(_textlayer_ist_plausibel(s) for s in seiten):
        return text, {}                                  # unverändert: sauberer Voll-Textlayer

    # Teil-Textlayer: nur die IMPLAUSIBLEN Seiten einzeln nach-OCR'n, plausible Seiten-Text behalten.
    text_teile = []
    conf_map: dict = {}
    zeilen_offset = 0
    for seiten_nr, seiten_text in enumerate(seiten, start=1):   # pdftoppm/tesseract sind 1-indiziert
        if _textlayer_ist_plausibel(seiten_text):
            text_teile.append(seiten_text)
            zeilen_offset += seiten_text.count("\n") + (1 if seiten_text and not seiten_text.endswith("\n") else 0)
        else:
            ocr_text, ocr_conf = _ocr_tesseract_seite(pfad, seiten_nr)
            text_teile.append(ocr_text)
            for i, c in ocr_conf.items():
                conf_map[zeilen_offset + i] = c
            zeilen_offset += ocr_text.count("\n") + (1 if ocr_text and not ocr_text.endswith("\n") else 0)
    return "\n".join(text_teile), conf_map
```

Neuer Helper `_ocr_tesseract_seite` (Ein-Seiten-Variante von `_ocr_tesseract_zeilen`, reused
`_tsv_zu_zeilen` unverändert):

```python
def _ocr_tesseract_seite(pfad: str, seiten_nr: int) -> tuple[str, dict]:
    """Wie _ocr_tesseract_zeilen, aber NUR eine Seite (1-indiziert) rastern+OCR'n — für den
    Teil-Textlayer-Fall (Rest der Seiten hat schon einen brauchbaren Textlayer, kein Grund die
    komplett neu zu rastern)."""
    with tempfile.TemporaryDirectory() as td:
        praefix = os.path.join(td, "seite")
        subprocess.run(["pdftoppm", "-png", "-r", "200", "-f", str(seiten_nr), "-l", str(seiten_nr),
                        pfad, praefix], capture_output=True)
        png = sorted(os.listdir(td))[0]
        tsv = subprocess.run(["tesseract", os.path.join(td, png), "stdout", "-l", "deu", "tsv"],
                             capture_output=True, text=True).stdout
        zeilen = _tsv_zu_zeilen(tsv)
    text = "\n".join(z for z, _ in zeilen)
    conf_map = {i: c for i, (_, c) in enumerate(zeilen)}
    return text, conf_map
```

`_ocr_tesseract_zeilen` (Voll-Scan-Pfad) bleibt UNVERÄNDERT — reiner Zusatz-Helper, kein Umbau.

## `beleg_writer.lies_beleg_text` — analoger Fix

Gleiches Muster (`if txt.strip(): return txt` → Teil-Textlayer-Split + Plausibilitäts-Check +
Per-Seiten-OCR). `beleg_writer` hat aber keinen `conf_map`-Rückgabewert (nur `str`) — `extrahiere()`
nimmt einen optionalen `confidence_map`-Parameter separat vom Aufrufer. Beim Transplant: entweder (a)
`lies_beleg_text` auf `-> tuple[str, dict]` erweitern (Signatur-Änderung, Caller in api.py prüfen —
dev-1-Zone, ggf. Rücksprache) oder (b) `lies_beleg_text` gibt weiter nur `str` zurück und die
OCR-Confidence pro nachträglich-OCR'ter Seite geht für Belege verloren (einfacher, aber
Confidence-Info schwächer als bei Kontoauszug). **Offene Frage an main/dev-1**: welche Variante,
da Signatur-Änderung Cross-Zone ist (dev-1s api.py könnte `lies_beleg_text` aufrufen).

## Synthetischer Multi-Seiten-Fixture-Builder (für Tests, kein echtes Sample)

```python
def _write_multiseiten_pdf(text_zeilen: list, scan_zeilen: list, pfad: str) -> None:
    """Seite 1 = Textlayer (_write_text_pdf), Seite 2 = Bild-Scan (_write_scan_pdf), via pdfunite
    zusammengefügt (poppler-utils, schon Dependency von pdftotext/pdftoppm — kein neues Tool)."""
    p1, p2 = pfad + ".seite1.pdf", pfad + ".seite2.pdf"
    _write_text_pdf(text_zeilen, p1)
    _write_scan_pdf(scan_zeilen, p2)
    subprocess.run(["pdfunite", p1, p2, pfad], check=True)
```

(Empirisch verifiziert — siehe oben — genau diese Kombination erzeugt den Bug reproduzierbar.)

## Geplante Tests (analog dev-3-Review-Stil: erst der Bug-Repro, dann der Fix-Beweis)

1. `test_lies_kontoauszug_pdf_teil_textlayer_ohne_fix_wuerde_seite2_verlieren` — dokumentiert den Bug
   als bekannte Grenze VOR dem Fix (optional, ggf. nur als Kommentar/entfällt sobald Fix drin ist,
   da der Fix-Test selbst den Beweis erbringt).
2. `test_lies_kontoauszug_pdf_teil_textlayer_seite2_wird_nachocrt` — 2-Seiten-Fixture, NACH dem Fix:
   `text` enthält BEIDE Transaktionen (Seite 1 UND die OCR'te Seite-2-Zeile), `conf_map` hat einen
   Eintrag für die Seite-2-Zeile (nicht für Seite-1-Zeilen — die bleiben implizit 1.0).
3. `test_lies_kontoauszug_pdf_voller_textlayer_unveraendert` — Regression: ein normales
   Voll-Textlayer-PDF (bestehender Test) bleibt `({} conf_map)`, kein unnötiges OCR (Performance:
   `_textlayer_ist_plausibel` darf den bestehenden schnellen Pfad nicht kaputt machen).
4. `test_lies_kontoauszug_pdf_voller_scan_unveraendert` — Regression: ein komplett leeres
   `pdftotext`-Ergebnis geht weiter über den bestehenden `_ocr_tesseract_zeilen`-Vollpfad (nicht über
   die neue Teil-Textlayer-Logik).
5. Analoge 2 Tests für `beleg_writer` (Teil-Textlayer + Regression) — nach Klärung der offenen
   Signatur-Frage oben.

## Aufwand-Reschätzung nach Recon (für main)

Kern-Logik + Helper: ~1h (Design oben ist schon durchdacht + teil-empirisch verifiziert). Tests
(4 kontoauszug + 2 beleg): ~1h. Cross-Zone-Klärung (beleg_writer-Signatur) braucht kurze Rücksprache
mit main/dev-1, kein Blocker fürs Bauen der kontoauszug-Seite zuerst.
