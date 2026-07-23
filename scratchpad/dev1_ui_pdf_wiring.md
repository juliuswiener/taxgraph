# DRAFT (Pre-Review, NICHT integriert) — dev-1s PDF-Upload-UI-Wiring für produkt/haut/static/

Transplant-ready Diff-Sketch nach dem freigegebenen Plan (index.html accept+Label, `_dateiAlsBase64`-Helper,
`kontoauszugHochladen`-PDF-Zweig, Response-hinweis). NICHT lauffähig als eigenständige Datei — reiner
Transplant-Entwurf für `static/index.html` + `static/app.js`. HTML/JS lässt sich ohne Browser nicht
self-checken (kein JS-Testharness im Repo) — daher hier nur sauber vorgedraftet, kein Bau in `static/`.

## 1. index.html:45,47 — accept-Liste + Label

Vorher (Zeile 45+47):
```html
        <button id="konto-toggle" class="btn-link" type="button">🏦 Kontoauszug hochladen (CSV/JSON)</button>
        <div id="konto-panel" class="vorjahr-panel" hidden>
          <input id="konto-file" type="file" accept=".csv,.json,text/csv,application/json">
```

Nachher:
```html
        <button id="konto-toggle" class="btn-link" type="button">🏦 Kontoauszug hochladen (CSV/JSON/PDF)</button>
        <div id="konto-panel" class="vorjahr-panel" hidden>
          <input id="konto-file" type="file" accept=".csv,.json,.pdf,text/csv,application/json,application/pdf">
```

Zeile 48 (Status-Hint) bleibt UNVERÄNDERT — schon formatagnostisch ("Ausgaben werden als Vorschläge erfasst …
IBAN/Kontonummern werden maskiert").

## 2. app.js — neuer Helper `_dateiAlsBase64` (Stil analog `euro`/`$`, direkt unter denen einordnen)

```javascript
function _dateiAlsBase64(datei) {
  // FileReader liefert eine data-URL "data:application/pdf;base64,JVBERi0x..." — der Endpoint erwartet
  // reines base64 (api.py: base64.b64decode(inhalt, validate=True)) -> Präfix vor dem Komma strippen.
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(String(r.result).split(",", 2)[1] || "");
    r.onerror = () => reject(r.error);
    r.readAsDataURL(datei);
  });
}
```

## 3. app.js:352-368 — `kontoauszugHochladen` PDF-Zweig

Vorher:
```javascript
// --- Kontoauszug-Upload: CSV/JSON → Transaktion-Vorschläge (herkunft=kontoauszug), Nutzer bestätigt ---
async function kontoauszugHochladen(datei) {
  const st = $("konto-status");
  if (!datei) return;
  const name = (datei.name || "").toLowerCase();
  const format = name.endsWith(".json") ? "json" : name.endsWith(".csv") ? "csv" : null;
  if (!format) { st.textContent = "Bitte eine CSV- oder JSON-Datei wählen (PDF-Import folgt)."; return; }
  st.textContent = "Lese Auszug …";
  const inhalt = await datei.text();
  const r = await jpost(`/fall/${FALL}/kontoauszug`, { format, inhalt });
  if (r.status === 200) {
    st.textContent = `${r.body.uebernommen} von ${r.body.transaktionen} Buchung(en) als Vorschlag erfasst — bitte im Fluss bestätigen.`;
    await refresh();
  } else {
    st.textContent = "Upload fehlgeschlagen: " + ((r.body && (r.body.vertrag || r.body.fehler)) || r.status);
  }
}
```

Nachher:
```javascript
// --- Kontoauszug-Upload: CSV/JSON/PDF → Transaktion-Vorschläge (herkunft=kontoauszug), Nutzer bestätigt ---
async function kontoauszugHochladen(datei) {
  const st = $("konto-status");
  if (!datei) return;
  const name = (datei.name || "").toLowerCase();
  const format = name.endsWith(".json") ? "json" : name.endsWith(".csv") ? "csv"
               : name.endsWith(".pdf") ? "pdf" : null;
  if (!format) { st.textContent = "Bitte eine CSV-, JSON- oder PDF-Datei wählen."; return; }
  st.textContent = "Lese Auszug …";
  const inhalt = format === "pdf" ? await _dateiAlsBase64(datei) : await datei.text();
  const r = await jpost(`/fall/${FALL}/kontoauszug`, { format, inhalt });
  if (r.status === 200) {
    st.textContent = `${r.body.uebernommen} von ${r.body.transaktionen} Buchung(en) als Vorschlag erfasst — bitte im Fluss bestätigen.`
      + (r.body.hinweis ? ` ${r.body.hinweis}` : "");
    await refresh();
  } else {
    st.textContent = "Upload fehlgeschlagen: " + ((r.body && (r.body.vertrag || r.body.fehler)) || r.status);
  }
}
```

## Geklärt (verifiziert gegen aktuellen Code)
- base64-Contract: `_dateiAlsBase64` liefert reines base64 (Präfix-Strip vor dem Komma) — matcht exakt
  `base64.b64decode(inhalt, validate=True)` in `api.py::kontoauszug`. Der True-e2e-Test (Python-base64,
  gleicher Standard-Alphabet) deckt den Decode-Pfad bereits ab; JS `readAsDataURL` liefert identisches
  Standard-base64 (RFC 4648, kein URL-safe-Alphabet) — kein Sonderfall.
- `.split(",", 2)[1]` statt `.split(",")[1]`: PDF-Bytes selbst enthalten kein Komma NACH dem base64-Encode
  (base64-Alphabet hat kein `,`), daher reicht auch ein simples `.split(",")[1]` — der 2. Parameter ist rein
  defensiv/lesbar, keine funktionale Notwendigkeit.
- Response-`hinweis`: UI dupliziert den K2-Sicherheitstext NICHT neu — hängt nur das Backend-Feld an, falls
  vorhanden (`r.body.hinweis` ist bereits der fertige String "N Zeile(n) unsicher erkannt …").
- Kein neuer Fehlerpfad: FileReader.onerror ist der einzig neue Failure-Mode ggü. `datei.text()` — beide
  Promises werfen bei Lesefehler, `kontoauszugHochladen` selbst hat keinen try/catch (bestehendes Verhalten,
  unverändert — ein Lesefehler bricht wie bisher die async-Funktion ohne UI-Crash, da kein `.catch()` im
  Aufrufer nötig ist; Browser zeigt eine unhandled-rejection-Warnung in der Konsole, kein User-facing Crash).

## Diff-Umfang
~15 Zeilen index.html+app.js zusammen. Kein neuer Import, keine Dependency, kein Cap. Backend unangetastet.
