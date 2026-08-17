"use strict";
// TaxGraph-Haut — Mobile-Wegpunkt-Fluss (Vanilla-JS, kein Build). Redet NUR über die HTTP-API;
// keine Steuerlogik im Frontend, jede Zahl kommt vom Server. KI erklärt, setzt nie Werte (POST /chat -> 501).

let FALL = null;
let AKTUELL = null;   // aktuelle Frage (aus /fragen)
let STAND = null;     // letzter /stand
let SPANNE0 = null;   // Referenz-Spanne (für den Schrumpf-Anteil)
let KORREKTUR_FID = null;  // feld_id bei Korrektur; null = neue Frage
let VERSTANDEN_OFFEN = false;  // Verstanden-Seite liegt vorn -> refresh() darf sie nicht wegschieben

// --- P1.1-Verdrahtung: Token-Haltung ---
// Sicherheitsentscheidung, keine Geschmacksfrage (team-lead-Auftrag): sessionStorage statt
// localStorage. Der Server bindet ausschließlich an 127.0.0.1 (kein Zugriff von außen), aber die
// Oberfläche hat einen KI-Chat, der Fremdtext ins DOM setzt — ein XSS-Fenster ist nicht mit
// Sicherheit ausgeschlossen, auch wenn der bestehende Code konsequent textContent statt innerHTML
// für Fremdtext nutzt. sessionStorage begrenzt den Schaden eines gestohlenen Tokens auf die
// Lebensdauer des Tabs statt auf die vollen 24h der JWT_TTL_H (auth.py) wie bei localStorage.
// Rein In-Memory (nur die JS-Variable) wäre noch enger, würde aber bei jedem Seiten-Reload einen
// Neu-Login erzwingen — für eine mehrteilige Steuererklärung zu hart für den Nutzer.
const TOKEN_KEY = "taxgraph_token";
let TOKEN = sessionStorage.getItem(TOKEN_KEY) || null;
let AUTH_USER = null;   // Benutzername des angemeldeten Kontos (nur Anzeige, keine Sicherheitsgrenze)
function setToken(t) { TOKEN = t; sessionStorage.setItem(TOKEN_KEY, t); }
function clearToken() { TOKEN = null; AUTH_USER = null; sessionStorage.removeItem(TOKEN_KEY); }

const NETZ_FEHLER_TEXT = "Netzwerkfehler — bitte Verbindung prüfen und erneut versuchen.";
function zeigeNetzFehler(msg) {
  // Steht die Anmeldemaske bereits vorn (401 abgefangen, s. jget/jpost unten), gewinnt sie: ein
  // technischer Banner ("Abgewiesen: Authentifizierung erforderlich") daneben wäre nur verwirrend
  // — der eigentliche Grund ist längst als Anmeldemaske sichtbar, nicht als Netzwerkfehler.
  const login = document.getElementById("login");
  if (login && !login.hidden) return;
  const b = document.getElementById("netz-banner"); b.textContent = msg; b.hidden = false;
}
function versteckeNetzFehler() { const b = document.getElementById("netz-banner"); b.hidden = true; }
function okStatus(s) { return s >= 200 && s < 300; }

// 401 zentral abfangen: EIN Ort statt an jeder der acht jpost-Aufrufstellen (und jeder jget-Stelle).
// /auth/* ist ausgenommen — /auth/login und /auth/register liefern ihre eigenen 4xx (falsches
// Passwort etc.) und der Aufrufer muss die Meldung selbst zeigen; /auth/session liefert beim
// Start-Check bewusst 401 auch im Einzelnutzer-Modus (der Endpunkt kennt TAXGRAPH_NO_AUTH nicht,
// s. initAuth()) — das darf die Maske dort NICHT auslösen, sonst bräche der Einzelnutzer-Modus.
function _401Abfangen(url, status) {
  if (status === 401 && !url.startsWith("/auth/")) zeigeAnmeldemaske();
}

async function jget(url) {
  try {
    const headers = TOKEN ? { "Authorization": "Bearer " + TOKEN } : {};
    const r = await fetch(url, { headers });
    const body = await r.json();
    versteckeNetzFehler();
    _401Abfangen(url, r.status);
    return { status: r.status, body };
  } catch (e) {
    zeigeNetzFehler(NETZ_FEHLER_TEXT);
    return { status: 0, body: { fehler: NETZ_FEHLER_TEXT } };
  }
}
async function jpost(url, obj) {
  try {
    const headers = { "Content-Type": "application/json" };
    if (TOKEN) headers["Authorization"] = "Bearer " + TOKEN;
    const r = await fetch(url, { method: "POST", headers, body: JSON.stringify(obj || {}) });
    const body = await r.json();
    versteckeNetzFehler();
    _401Abfangen(url, r.status);
    return { status: r.status, body };
  } catch (e) {
    zeigeNetzFehler(NETZ_FEHLER_TEXT);
    return { status: 0, body: { fehler: NETZ_FEHLER_TEXT } };
  }
}
function euro(cent) {
  if (cent === null || cent === undefined) return "—";
  return (cent / 100).toLocaleString("de-DE", { style: "currency", currency: "EUR" });
}
const $ = (id) => document.getElementById(id);
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

// --- P0: Scheiben-Wahl (2 Kacheln) — scheibe NICHT hardcoden ---
async function waehleScheibe(scheibe) {
  const kacheln = document.querySelectorAll(".kachel");
  if (kacheln[0] && kacheln[0].disabled) return;   // Doppel-Submit-Schutz
  kacheln.forEach(k => k.disabled = true);
  const fid = "demo-" + Date.now();
  const a = await jpost("/fall", { scheibe, veranlagungszeitraum: 2025, fall_id: fid });
  if (!okStatus(a.status)) {
    zeigeNetzFehler("Konnte nicht starten: " + (a.body.fehler || a.status));
    kacheln.forEach(k => k.disabled = false);
    return;
  }
  kacheln.forEach(k => k.disabled = false);
  FALL = a.body.fall_id;
  SPANNE0 = null;
  $("start").hidden = true;
  $("flow").hidden = false;
  await refresh();
}

async function refresh() {
  const st = await jget(`/fall/${FALL}/stand`);
  // 401 (Token abgelaufen/fehlt): der jget-Interceptor hat die Anmeldemaske schon gezeigt — hier
  // abbrechen statt auf st.body.felder zu crashen (der Fehlerbody hat kein .felder).
  if (st.status === 401) return;
  STAND = st.body;
  zeigeRing(STAND);
  zeigeBelegt(STAND.felder);
  // Die Verstanden-Seite arbeitet eine eigene Liste ab. Ring und Belegt-Liste sollen dabei
  // mitlaufen (jede Bestätigung bewegt den Ring), aber der Fragefluss darf sich nicht davorschieben
  // — sonst springt der Nutzer nach der ersten Bestätigung aus seiner Liste heraus.
  if (VERSTANDEN_OFFEN) return;
  const fr = await jget(`/fall/${FALL}/fragen`);
  if (fr.status === 401) return;
  const fragen = fr.body.fragen;
  if (fragen.length === 0) { $("wegpunkt").hidden = true; await zeigeErgebnis(); }
  else { AKTUELL = fragen[0]; zeigeFrage(AKTUELL, STAND); }
}

// --- Dim 4: der Bescheid als schrumpfender Ring ---
function zeigeRing(stand) {
  const spanneEl = $("spanne"), hintEl = $("spanne-hint"), ringEl = $("ring"), mitteEl = $("ring-mitte");
  const fortschritt = $("fortschritt");
  const felder = Object.values(stand.felder || {});
  const fest = felder.filter(f => f.zustand === "bestaetigt").length;
  const anteil = felder.length ? fest / felder.length : 0;
  ringEl.style.setProperty("--anteil", anteil);
  mitteEl.textContent = felder.length ? `${fest}/${felder.length}` : "";
  fortschritt.max = Math.max(1, felder.length);
  fortschritt.value = fest;

  if (stand.ring_gesperrt) {
    spanneEl.textContent = "Vereinfachter Bescheid hier nicht möglich"; hintEl.textContent = "siehe Ergebnis unten";
    ringEl.style.setProperty("--schrumpf", 1); return;
  }
  const iv = stand.intervall;
  if (iv) {
    const breite = Math.max(0, iv.max_cent - iv.min_cent);
    if (SPANNE0 === null || breite > SPANNE0) SPANNE0 = breite || 1;
    ringEl.style.setProperty("--schrumpf", SPANNE0 ? breite / SPANNE0 : 0);
    if (iv.min_cent === iv.max_cent) { spanneEl.textContent = `Bescheid: ${euro(iv.min_cent)}`; hintEl.textContent = "steht"; }
    else {
      spanneEl.textContent = `${euro(iv.min_cent)} – ${euro(iv.max_cent)}`;
      const offen = felder.length - fest;
      hintEl.textContent = `▼ schrumpft mit jeder Antwort · noch ${offen} offen`;
    }
  } else if (stand.teil_ringe && stand.teil_ringe.length) {
    spanneEl.textContent = stand.teil_ringe.map(t => `${t.familie}: ${euro(t.intervall.min_cent)}–${euro(t.intervall.max_cent)}`).join(" · ");
    hintEl.textContent = "einzelne Abzüge — noch kein Gesamt-Bescheid";
  } else { spanneEl.textContent = "Bescheid-Spanne"; hintEl.textContent = "(Rechen-Engine nicht verfügbar)"; }
}

// --- Dim 1: Herkunft zum Anfassen (per-Quelle-Badge) ---
const BADGE = {
  laie:          { kl: "b-laie",   sym: "✓", lab: "selbst" },
  beleg_import:  { kl: "b-beleg",  sym: "▤", lab: "Beleg" },
  kontoauszug:   { kl: "b-beleg",  sym: "🏦", lab: "Kontoauszug" },
  vorjahr:       { kl: "b-abgel",  sym: "↻", lab: "Vorjahr" },
  berechnet:     { kl: "b-abgel",  sym: "∑", lab: "berechnet" },
  orakel:        { kl: "b-orakel", sym: "◆", lab: "amtlich" },
  llm_vorschlag: { kl: "b-ki",     sym: "✦", lab: "KI" },
};
const VORSCHLAG_QUELLEN = ["llm_vorschlag", "berechnet", "vorjahr", "kontoauszug"];
function badgeInfo(k) { return BADGE[k] || BADGE.laie; }

function zeigeBelegt(felder) {
  const ul = $("belegt-liste"); ul.innerHTML = "";
  for (const [fid, f] of Object.entries(felder || {})) {
    const li = document.createElement("li"); li.className = "zeile zeile-klickbar";
    li.addEventListener("click", () => korrigiereBestaetigt(fid));
    const bi = badgeInfo(f.herkunft_badge);
    const badge = document.createElement("button");
    badge.className = "badge " + bi.kl + (f.zustand === "vorlaeufig" ? " badge-vorlaeufig" : "");
    badge.textContent = bi.sym + " " + bi.lab;
    badge.title = "Herkunft ansehen"; badge.type = "button";
    badge.addEventListener("click", (e) => { e.stopPropagation(); herkunftKette(fid, f); });
    li.appendChild(badge);
    const t = document.createElement("span"); t.className = "z-name"; t.textContent = fid;
    const v = document.createElement("span"); v.className = "z-wert"; v.textContent = JSON.stringify(f.wert);
    li.appendChild(t); li.appendChild(v); ul.appendChild(li);
  }
}

// --- Dim 1: Herkunft-Kette (Euro -> Regel -> Norm -> Beleg) im Bestätigungsmoment ---
async function herkunftKette(fid, f) {
  const r = await jget(`/fall/${FALL}/feld/${fid}/warum`);
  if (r.status === 401) return;   // Anmeldemaske hat übernommen — kein Overlay mehr darüberlegen
  const j = (r.status === 200 && r.body.justification) ? r.body.justification : {};
  const bi = badgeInfo(f.herkunft_badge);
  $("kette-titel").textContent = `${fid}: ${JSON.stringify(f.wert)}`;
  const body = $("kette-body"); body.innerHTML = "";
  // Struktur per createElement, Text per textContent — NIE interpoliert. Vorher stand hier
  // `d.innerHTML = ...${txt}...`, und `txt` trägt bei der Beleg-Zeile den OCR-ROHTEXT eines
  // hochgeladenen Dokuments (s1.roh_text). Ein Dokument schreibt aber nicht der Nutzer, sondern
  // wer immer ihm die Rechnung geschickt hat — präpariertes Markup darin lief damit im Kontext
  // der Anwendung und konnte das Anmelde-Token aus sessionStorage lesen. Genau das Fenster, mit
  // dem oben die Wahl von sessionStorage begründet ist; solange es offen war, beschrieb die
  // Begründung nur das Problem.
  const step = (dot, titel, txt) => {
    const d = document.createElement("div"); d.className = "step";
    const s = document.createElement("span"); s.className = "dot"; s.textContent = dot;
    const b = document.createElement("b"); b.textContent = titel;
    d.append(s, " ", b);
    if (txt) d.append(" · " + txt);        // append(string) erzeugt einen TextNode, kein Markup
    body.appendChild(d);
  };
  step("◗", "Wert", `${JSON.stringify(f.wert)} (${bi.lab}, ${f.zustand})`);
  if (j.regel_id) step("◗", "Regel", j.regel_id);
  const a = j.anker_ref || AKTUELL_ANKER(fid);
  if (a && (a.quelle || a.zitatanker)) step("§", "Paragraph", `${a.quelle || ""} — „${a.zitatanker || ""}"`);
  const s1 = j.signal && j.signal.signal_1;
  if (s1 && typeof s1 === "object") step("▤", "Beleg", `${s1.typ || ""} ${s1.ref || ""}${s1.roh_text ? ' · „' + s1.roh_text + '"' : ""}`);
  else if (f.herkunft_badge === "llm_vorschlag") step("✦", "KI-Vorschlag", "erklärt, nicht selbst gesetzt");
  $("kette-overlay").hidden = false;
}
function AKTUELL_ANKER(fid) { return (AKTUELL && AKTUELL.feld_id === fid) ? AKTUELL.anker_ref : null; }

// --- Wegpunkt (Frage) ---
function zeigeFrage(q, stand) {
  $("wegpunkt").hidden = false; $("fertig").hidden = true;
  $("frage").textContent = q.fragetext_laie || q.feld_id;
  $("hilfe").textContent = q.hilfe_kurz || "";
  $("anker").hidden = true;
  $("wegpunkt").focus({ preventScroll: true });   // Screen-Reader: Wegpunkt-Wechsel per Fokus + aria-live auf #frage ansagen

  // Gibt es schon einen vorläufigen Vorschlag für dieses Feld (KI / Karten-berechnet / Vorjahr)?
  // -> Hold-to-confirm (Dim 2): der Nutzer bestätigt den Vorschlag bewusst (Zwei-Signal).
  const vorhanden = stand.felder && stand.felder[q.feld_id];
  const kiVorschlag = vorhanden && vorhanden.zustand === "vorlaeufig"
    && VORSCHLAG_QUELLEN.includes(vorhanden.herkunft_badge);

  // Julius-Feature: Arbeitsweg-km über Karten-Dienst (Vorschlag-Fluss; Backend ge-stubbt, PII/Cap offen).
  const altMaps = document.getElementById("maps-affordanz"); if (altMaps) altMaps.remove();
  if (q.feld_id === "ep_entfernung_km") mapsAffordanz(q);

  const box = $("eingabe"); box.innerHTML = ""; let input;
  if (q.typ === "bool") {
    // Buttons statt Dropdown: eine Ja/Nein-Frage hinter einem Klapp-Menü zu verstecken kostet
    // zwei Interaktionen für eine Information. Der Wert landet in einem hidden input, damit
    // leseWert() unverändert `#feld-input`.value liest.
    input = wahlFeld([["Ja", "true"], ["Nein", "false"]], q.beispielwert);
  } else if (q.typ === "enum") {
    // Anzeigetext statt Rohwert: ohne das las der Nutzer "land_forst" oder bei den
    // Kindschaftsverhältnissen nur "1". Fallback auf den Rohwert, falls ein Feld noch kein
    // Label hat — lieber technisch als leer.
    const opt = (q.enum_werte || []).map(v => [(q.enum_labels && q.enum_labels[v]) || v, v]);
    // Ab WAHL_MAX wird die Button-Reihe länger als der Bildschirm (16 Bundesländer, 16
    // DBA-Staaten) — dort bleibt das Dropdown die bessere Bedienung.
    input = opt.length <= WAHL_MAX ? wahlFeld(opt, q.beispielwert) : selectFeld(opt, q.beispielwert);
  } else {
    input = document.createElement("input"); input.type = "number";
    input.inputMode = q.typ === "cent" ? "decimal" : "numeric";
    if (q.bereich) { if ("min" in q.bereich) input.min = q.bereich.min; if ("max" in q.bereich) input.max = q.bereich.max; }
    input.placeholder = q.typ === "cent" ? "Betrag in Euro" : String(q.beispielwert ?? "");
    if (kiVorschlag && vorhanden.wert !== null) input.value = (q.typ === "cent") ? (vorhanden.wert / 100) : vorhanden.wert;
  }
  // Bei der Button-Gruppe trägt das versteckte input im Container die id — der Container selbst
  // darf sie nicht überschreiben, sonst findet leseWert() ein <div> ohne .value.
  if (!input.classList.contains("wahl")) input.id = "feld-input";
  input.setAttribute("aria-labelledby", "frage");   // a11y: Screenreader liest die Frage als Feldnamen
  box.appendChild(input);
  if (q.einheit) { const s = document.createElement("span"); s.className = "einheit"; s.textContent = " " + q.einheit; box.appendChild(s); }

  // Bestätigungs-Geste skaliert mit KI-Konfidenz (Dim 2): KI-Vorschlag -> Halten; sonst 1-Tipp.
  rüsteBestaetigen(kiVorschlag ? vorhanden : null, q);
}

function rüsteBestaetigen(kiFeld, q) {
  const btn = $("bestaetigen");
  const neu = btn.cloneNode(false);  // Listener zurücksetzen
  btn.replaceWith(neu);
  if (kiFeld) {
    neu.classList.add("btn-hold");
    neu.innerHTML = '<span class="fill"></span><span class="lbl">Halten zum Bestätigen …</span>';
    holdGeste(neu, () => bestaetigen(kiFeld));   // Hold liefert signal_2
  } else {
    neu.classList.remove("btn-hold");
    neu.textContent = "Bestätigen";
    neu.addEventListener("click", () => bestaetigen(null));
  }
}

// Hold-to-confirm: 600ms drücken, Fortschritts-Fill; loslassen vorher = kein Commit (Versehens-Schutz).
function holdGeste(btn, onDone) {
  const fill = btn.querySelector(".fill"); let t0 = 0, raf = 0, fertig = false;
  const DAUER = 600;
  const tick = () => {
    const p = Math.min(1, (performance.now() - t0) / DAUER);
    fill.style.width = (p * 100) + "%";
    if (p >= 1 && !fertig) { fertig = true; onDone(); }
    else if (!fertig) raf = requestAnimationFrame(tick);
  };
  const start = (e) => { e.preventDefault(); if (fertig) return; t0 = performance.now(); raf = requestAnimationFrame(tick); };
  const stop = () => { cancelAnimationFrame(raf); if (!fertig) fill.style.width = "0%"; };
  btn.addEventListener("pointerdown", start);
  btn.addEventListener("pointerup", stop);
  btn.addEventListener("pointerleave", stop);
}

// Maps-Affordance: Adressen + „Entfernung berechnen" → Karten-Vorschlag. Backend STUB (POST /entfernung
// → 501, kein Live-Aufruf ohne Julius-Service+Cap). Bei Erfolg (später) füllt der km-Vorschlag das manuelle
// Feld VOR (vorlaeufig) — der Nutzer bestätigt/korrigiert (K2: die kürzeste Straßenverbindung ist der Default,
// eine längere bei regelmäßiger Nutzung zulässig, § 9 Abs. 1 S. 3 Nr. 4).
function mapsAffordanz(q) {
  const wrap = document.createElement("div"); wrap.className = "maps"; wrap.id = "maps-affordanz";
  wrap.innerHTML = `<div class="maps-titel">🗺️ Entfernung automatisch berechnen (optional)</div>
    <input id="maps-von" placeholder="Wohnung (Adresse)" autocomplete="off">
    <input id="maps-nach" placeholder="Erste Tätigkeitsstätte (Adresse)" autocomplete="off">
    <button id="maps-go" class="maps-berechnen" type="button">Entfernung berechnen</button>
    <div id="maps-status" class="maps-status"></div>`;
  $("eingabe").before(wrap);
  wrap.querySelector("#maps-go").addEventListener("click", async () => {
    const von = wrap.querySelector("#maps-von").value.trim(), nach = wrap.querySelector("#maps-nach").value.trim();
    const st = wrap.querySelector("#maps-status");
    if (!von || !nach) { st.textContent = "Bitte beide Adressen angeben."; return; }
    st.textContent = "Berechne …";
    const r = await jpost(`/fall/${FALL}/entfernung`, { von, nach });
    if (r.status === 200 && r.body && typeof r.body.km === "number") {
      // Der km liegt jetzt als VORLÄUFIGER berechnet-Vorschlag im Store → Wegpunkt neu laden: er zeigt
      // den Wert mit Herkunfts-Badge „berechnet" + Hold-to-confirm (Nutzer bestätigt bewusst, Zwei-Signal).
      await refresh();
    } else {
      st.textContent = (r.body && r.body.vertrag) ? r.body.vertrag : "Karten-Dienst nicht verbunden — bitte km manuell eingeben.";
    }
  });
}

// Bis zu so vielen Optionen werden Buttons gezeigt, darüber ein Dropdown. 6 passt auf 360px
// (Steuerklassen sind der Grenzfall); 16 Bundesländer als Button-Reihe wären eine Tapete.
const WAHL_MAX = 6;

// Button-Gruppe statt Dropdown. optionen: [[anzeigetext, wert], ...].
// Der gewählte Wert landet in einem versteckten input mit id="feld-input" — dadurch bleibt
// leseWert() unverändert und muss nicht wissen, wie die Auswahl aussieht.
function wahlFeld(optionen, vorauswahl) {
  const wrap = document.createElement("div");
  wrap.className = "wahl";
  const hidden = document.createElement("input");
  hidden.type = "hidden"; hidden.id = "feld-input";
  wrap.appendChild(hidden);
  for (const [text, wert] of optionen) {
    const b = document.createElement("button");
    b.type = "button"; b.className = "wahl-opt"; b.textContent = text;
    b.dataset.wert = String(wert);
    b.setAttribute("aria-pressed", "false");
    b.addEventListener("click", () => {
      hidden.value = String(wert);
      for (const x of wrap.querySelectorAll(".wahl-opt")) {
        x.classList.remove("aktiv"); x.setAttribute("aria-pressed", "false");
      }
      b.classList.add("aktiv"); b.setAttribute("aria-pressed", "true");
    });
    if (vorauswahl !== undefined && vorauswahl !== null && String(wert) === String(vorauswahl)) {
      hidden.value = String(wert); b.classList.add("aktiv"); b.setAttribute("aria-pressed", "true");
    }
    wrap.appendChild(b);
  }
  // Ohne Vorauswahl bleibt der Wert leer statt still auf der ersten Option zu stehen — ein
  // Dropdown zeigt die erste Option an, ein Button-Feld zeigt gar keine, und genau das ist
  // ehrlicher: der Nutzer hat noch nicht geantwortet.
  return wrap;
}

function selectFeld(optionen, vorauswahl) {
  const sel = document.createElement("select");
  for (const [text, wert] of optionen) {
    const o = document.createElement("option"); o.value = wert; o.textContent = text;
    if (String(wert) === String(vorauswahl)) o.selected = true;
    sel.appendChild(o);
  }
  sel.id = "feld-input";
  return sel;
}

// Stille-Null-Fix (team-lead-Auftrag, Befund B): vorher machte `parseFloat(el.value || "0")` /
// `parseInt(el.value || "0", 10)` aus einem LEEREN oder browser-ungültigen Feld (z.B. "12,5" mit
// Komma statt Punkt -> parseFloat liest nur "12" korrekt, der Rest verwirft -> "12,5x" wäre sogar
// NaN) kommentarlos eine 0 — ununterscheidbar von einer echten, absichtlich eingegebenen 0. Jetzt:
// leer/ungültig -> `undefined` (der Aufrufer in bestaetigen() muss das prüfen und abbrechen statt
// zu schreiben); eine explizit eingegebene 0 bleibt eine echte, gültige 0 (roh="0" ist NICHT leer).
function leseWert(q) {
  const el = $("feld-input");
  if (q.typ === "enum") return el.value;
  if (q.typ === "bool") { const ja = el.value === "true"; return q.feld_id.startsWith("kein_") ? !ja : ja; }
  const roh = (el.value ?? "").trim();
  if (roh === "") return undefined;
  if (q.typ === "cent") {
    const eur = parseFloat(roh);
    return Number.isFinite(eur) ? Math.round(eur * 100) : undefined;
  }
  const n = parseInt(roh, 10);
  return Number.isFinite(n) ? n : undefined;
}

// --- Korrektur: Belegt-Feld erneut bearbeiten (event_id aus /warum holen + mit ersetzt überschreiben)
// Rückgabe true/false: der Aufrufer muss wissen, ob jetzt eine Frage sichtbar ist. Die Verstanden-
// Seite blendet sich für „Ändern" aus und stünde sonst bei einem Fehlschlag vor einem leeren Bild.
async function korrigiereBestaetigt(fid) {
  // event_id für dieses Feld holen (aus justification)
  const r = await jget(`/fall/${FALL}/feld/${fid}/warum`);
  if (r.status !== 200 || !r.body.justification) {
    zeigeNetzFehler("Korrektur konnte nicht geladen werden.");
    return false;
  }
  const event_id = r.body.justification.event_id;
  if (!event_id) {
    zeigeNetzFehler("Feld hat keine event_id (nicht belegbar?).");
    return false;
  }

  // Frage aus /fragen laden — wir brauchen Feldtyp, Optionen etc.
  const fragen_r = await jget(`/fall/${FALL}/fragen`);
  if (fragen_r.status !== 200) {
    zeigeNetzFehler("Fragen konnten nicht geladen werden.");
    return false;
  }
  const frage = (fragen_r.body.fragen || []).find(q => q.feld_id === fid);
  if (!frage) {
    // Feld steht nicht mehr in den offenen Fragen — könnte sein, dass andere
    // Felder es obsolet gemacht haben. Für Korrektur brauchen wir die Frage.
    zeigeNetzFehler(`Feld ${fid} ist nicht mehr im Fragenfluss.`);
    return false;
  }

  KORREKTUR_FID = fid;
  AKTUELL = frage;  // Jetzt ist dieses Feld die "aktuelle" Frage
  zeigeFrage(AKTUELL, STAND);
  return true;
}

// --- Bestätigen: Zwei-Signal über den EINZIGEN Schreibpfad. kiFeld gesetzt -> ersetzt das vorläufige KI-Event. ---
async function bestaetigen(kiFeld) {
  if (!AKTUELL) return;
  const btn = $("bestaetigen");
  if (btn.disabled) return;   // Doppel-Submit-Schutz
  btn.disabled = true;
  const altLabel = kiFeld ? null : btn.textContent;
  if (!kiFeld) btn.textContent = "Wird gespeichert …";
  const wert = leseWert(AKTUELL);
  if (wert === undefined) {
    // Stille-Null-Fix (Befund B): leer/ungültig -> KEIN Event, kein "0" -- Nutzer wird informiert
    // statt dass eine falsche Zahl bestätigt in den Store wandert (derselbe Fehler-Anzeige-Stil
    // wie jede andere Ablehnung in dieser Funktion, s. unten).
    zeigeNetzFehler("Bitte einen gültigen Wert eingeben.");
    btn.disabled = false;
    if (!kiFeld) btn.textContent = altLabel;
    return;
  }
  const ev = {
    feld_id: AKTUELL.feld_id, wert, zustand: "bestaetigt",
    herkunft: { herkunft: "laie", pruef_tiefe: "ungeprueft", haftung: "nutzer" },
    schreiber: "ui:laie",
    signal: { signal_1: null, signal_2: (kiFeld ? "hold@" : "klick@") + AKTUELL.feld_id },
  };
  if (kiFeld) {
    // U12: das aktive KI-Event holen (event_id via /warum) und ersetzen.
    const j = (await jget(`/fall/${FALL}/feld/${AKTUELL.feld_id}/warum`)).body.justification || {};
    if (j.event_id) { ev.ersetzt = j.event_id; ev.signal.signal_1 = j.event_id; }
  } else if (KORREKTUR_FID) {
    // Korrektur: event_id aus justification holen + mit ersetzt überschreiben.
    const j = (await jget(`/fall/${FALL}/feld/${AKTUELL.feld_id}/warum`)).body.justification || {};
    if (j.event_id) { ev.ersetzt = j.event_id; ev.signal.signal_1 = j.event_id; }
  }
  const r = await jpost(`/fall/${FALL}/event`, ev);
  if (!okStatus(r.status)) {
    zeigeNetzFehler("Abgewiesen: " + (r.body.fehler || r.status));
    btn.disabled = false;
    if (!kiFeld) btn.textContent = altLabel;
    return;
  }
  btn.disabled = false;   // refresh() klont den Button (rüsteBestaetigen) — Attribut sonst dauerhaft übernommen
  KORREKTUR_FID = null;  // Korrektur abgeschlossen
  await refresh();
}

// --- „Das habe ich verstanden": alle KI-Vorschläge auf einer Seite, jeder mit seinem Zitat,
//     jeder EINZELN zu bestätigen. Julius 2026-08-14: „dann eine Seite anzeigen und dem Nutzer
//     sagen, okay, das habe ich jetzt verstanden … soll ich das so eintragen? Und dann schon,
//     dass der Nutzer das dann nochmal bestätigt". Bis eine Zeile bestätigt ist, bleibt ihr Wert
//     vorläufig und zählt in keiner Summe — die Seite ist die Sichtbarmachung genau dieser Grenze.

// Wert -> Anzeigetext, mit denselben Regeln wie der Fragefluss.
// Die bool-Umkehr bei `kein_` ist der heikle Teil (vgl. leseWert): das FELD behauptet die
// Abwesenheit, die FRAGE fragt nach der Anwesenheit. `kein_kap = true` unter „Hattest du dieses
// Jahr Kapitalerträge?" als „Ja" anzuzeigen wäre das glatte Gegenteil dessen, was gespeichert
// wird — und hier bestätigt der Nutzer mit einem Klick, was er liest.
function verstandenWertText(v) {
  if (v.typ === "bool") {
    const ja = String(v.feld_id).startsWith("kein_") ? !v.wert : !!v.wert;
    return ja ? "Ja" : "Nein";
  }
  if (v.typ === "enum") return (v.enum_labels && v.enum_labels[v.wert]) || String(v.wert);
  if (v.typ === "cent") return euro(v.wert);
  return String(v.wert) + (v.einheit ? " " + v.einheit : "");
}

function verstandenZeile(v) {
  const li = document.createElement("li");
  li.className = "v-zeile";
  li.dataset.feld = v.feld_id;
  const frage = document.createElement("div");
  frage.className = "v-frage";
  frage.textContent = v.frage || v.feld_id;   // ohne Bindungs-Eintrag lieber die ID als nichts
  const wert = document.createElement("div");
  wert.className = "v-wert";
  wert.textContent = verstandenWertText(v);
  li.appendChild(frage);
  li.appendChild(wert);
  if (v.beleg) {
    // Das geprüfte Nutzerzitat. Es ist der Unterschied zwischen „bestätige 62.000 €" und
    // „bestätige 62.000 €, weil du sagtest: 62000 Euro brutto verdient" — der Nutzer kann die
    // Behauptung an seinem eigenen Satz nachprüfen, statt der KI zu glauben.
    const b = document.createElement("div");
    b.className = "v-beleg";
    b.textContent = "weil du sagtest: „" + v.beleg + "“";
    li.appendChild(b);
  }
  const akt = document.createElement("div");
  akt.className = "v-aktionen";
  const ok = document.createElement("button");
  ok.type = "button"; ok.className = "v-ok"; ok.textContent = "Stimmt";
  ok.addEventListener("click", () => verstandenBestaetigen(v, li, ok));
  const aendern = document.createElement("button");
  aendern.type = "button"; aendern.className = "v-aendern"; aendern.textContent = "Ändern";
  aendern.addEventListener("click", () => verstandenAendern(v));
  akt.appendChild(ok);
  akt.appendChild(aendern);
  li.appendChild(akt);
  return li;
}

// Konflikt-Zeile: die KI schlägt etwas für ein Feld vor, das schon einen Wert trägt (Auflage B —
// höchstens ein aktives Event je Feld). Bis 2026-08-14 fiel das lautlos unter den Tisch: der
// Server meldete `konflikte`, die Oberfläche zeigte sie NIRGENDS. Für den Nutzer sah das aus, als
// hätte die KI seine Angabe überhört.
//
// Hier stehen deshalb BEIDE Werte nebeneinander, und keiner gewinnt von allein: "Meins behalten"
// schreibt gar nichts, "Ändern" ersetzt das bestehende Event. Das ist kein Vorschlag mehr, den man
// nur bestätigt — es ist ein Widerspruch, den nur der Nutzer auflösen kann.
function verstandenKonfliktZeile(k) {
  const li = document.createElement("li");
  li.className = "v-zeile v-konflikt";
  li.dataset.feld = k.feld_id;
  const frage = document.createElement("div");
  frage.className = "v-frage";
  frage.textContent = k.frage || k.feld_id;
  li.appendChild(frage);

  const alt = { ...k, wert: k.aktueller_wert };
  const neu = { ...k, wert: k.vorschlag_wert };
  const paar = document.createElement("div");
  paar.className = "v-paar";
  const seite = (label, text, cls) => {
    const d = document.createElement("div");
    d.className = "v-seite " + cls;
    const l = document.createElement("div"); l.className = "v-seite-label"; l.textContent = label;
    const w = document.createElement("div"); w.className = "v-seite-wert"; w.textContent = text;
    d.appendChild(l); d.appendChild(w);
    return d;
  };
  paar.appendChild(seite("bisher", verstandenWertText(alt), "v-seite-alt"));
  paar.appendChild(seite("KI schlägt vor", verstandenWertText(neu), "v-seite-neu"));
  li.appendChild(paar);

  if (k.beleg) {
    const b = document.createElement("div");
    b.className = "v-beleg";
    b.textContent = "weil du sagtest: „" + k.beleg + "“";
    li.appendChild(b);
  }
  // `gross` heißt: dieses Feld steuert andere Regeln. Eine Änderung wechselt nicht bloß einen
  // Wert, sondern WELCHE FRAGEN überhaupt noch gestellt werden — dafür reicht ein Häkchen nicht.
  if (k.gross) {
    const w = document.createElement("div");
    w.className = "v-warnung";
    w.textContent = "Achtung: diese Angabe entscheidet, welche weiteren Fragen überhaupt gestellt "
      + "werden. Eine Änderung wirkt sich auf andere Teile deiner Erklärung aus.";
    li.appendChild(w);
  }

  const akt = document.createElement("div");
  akt.className = "v-aktionen";
  const behalten = document.createElement("button");
  behalten.type = "button"; behalten.className = "v-ok"; behalten.textContent = "Meins behalten";
  behalten.addEventListener("click", () => {
    // Bewusst KEIN Schreibvorgang: der bestehende Wert bleibt, wie er ist. Alles andere wäre ein
    // überflüssiges Event auf denselben Wert — und ein zweites Signal, das der Nutzer nie gab.
    li.classList.add("v-fertig");
    behalten.textContent = "✓ unverändert";
    behalten.disabled = true;
    const a = li.querySelector(".v-uebernehmen");
    if (a) a.remove();
  });
  const uebernehmen = document.createElement("button");
  uebernehmen.type = "button"; uebernehmen.className = "v-aendern v-uebernehmen";
  uebernehmen.textContent = "Auf " + verstandenWertText(neu) + " ändern";
  uebernehmen.addEventListener("click", () => verstandenKonfliktUebernehmen(k, li, uebernehmen));
  akt.appendChild(behalten);
  akt.appendChild(uebernehmen);
  li.appendChild(akt);
  return li;
}

async function verstandenKonfliktUebernehmen(k, li, btn) {
  if (btn.disabled) return;   // Doppel-Submit-Schutz
  btn.disabled = true;
  const r = await jpost(`/fall/${FALL}/event`, {
    feld_id: k.feld_id, wert: k.vorschlag_wert, zustand: "bestaetigt",
    herkunft: { herkunft: "laie", pruef_tiefe: "ungeprueft", haftung: "nutzer" },
    schreiber: "ui:laie",
    // Ersetzt das BESTEHENDE Event (nicht den Vorschlag — der wurde nie geschrieben, genau
    // deshalb ist es ja ein Konflikt).
    signal: { signal_1: k.aktuelles_event_id, signal_2: "konflikt@" + k.feld_id },
    ersetzt: k.aktuelles_event_id,
  });
  if (!okStatus(r.status)) {
    zeigeNetzFehler("Abgewiesen: " + (r.body.fehler || r.status));
    btn.disabled = false;
    return;
  }
  li.classList.add("v-fertig");
  btn.textContent = "✓ geändert";
  const b = li.querySelector(".v-ok");
  if (b) b.remove();
  await refresh();
}

function zeigeVerstanden(vorschlaege, konflikte) {
  const ul = $("verstanden-liste");
  ul.innerHTML = "";
  for (const v of vorschlaege) ul.appendChild(verstandenZeile(v));
  for (const k of (konflikte || [])) ul.appendChild(verstandenKonfliktZeile(k));
  VERSTANDEN_OFFEN = true;
  $("wegpunkt").hidden = true;
  $("fertig").hidden = true;
  $("verstanden").hidden = false;
  $("verstanden").focus({ preventScroll: true });   // Screen-Reader: Wechsel des Screens ansagen
}

async function verstandenBestaetigen(v, li, btn) {
  if (btn.disabled) return;   // Doppel-Submit-Schutz
  btn.disabled = true;
  const r = await jpost(`/fall/${FALL}/event`, {
    feld_id: v.feld_id, wert: v.wert, zustand: "bestaetigt",
    herkunft: { herkunft: "laie", pruef_tiefe: "ungeprueft", haftung: "nutzer" },
    schreiber: "ui:laie",
    // `ersetzt` ist Pflicht, nicht Kosmetik: das Feld trägt bereits das vorläufige KI-Event, und
    // Auflage B lässt höchstens ein aktives Event je Feld zu. Die event_id liefert /chat gleich
    // mit — dieser Pfad braucht also keinen zusätzlichen /warum-Aufruf.
    signal: { signal_1: v.event_id, signal_2: "verstanden@" + v.feld_id },
    ersetzt: v.event_id,
  });
  if (!okStatus(r.status)) {
    zeigeNetzFehler("Abgewiesen: " + (r.body.fehler || r.status));
    btn.disabled = false;
    return;
  }
  li.classList.add("v-fertig");
  btn.textContent = "✓ bestätigt";
  const ae = li.querySelector(".v-aendern");
  if (ae) ae.remove();
  await refresh();   // Ring + Belegt-Liste ziehen mit; die Seite selbst bleibt vorn
}

async function verstandenAendern(v) {
  $("verstanden").hidden = true;
  VERSTANDEN_OFFEN = false;
  if (!await korrigiereBestaetigt(v.feld_id)) {
    if (!$("login").hidden) return;   // Anmeldemaske hat übernommen (401) — Verstanden-Seite nicht zurückholen
    // Frage nicht ladbar -> die Liste ist immer noch die bessere Anzeige als ein leerer Bildschirm.
    VERSTANDEN_OFFEN = true;
    $("verstanden").hidden = false;
  }
}

async function verstandenWeiter() {
  // Nicht bestätigte Zeilen bleiben vorläufig und stehen damit weiter im Fragefluss — dort
  // begegnen sie dem Nutzer erneut, mit Hold-to-confirm. Nichts geht verloren, nichts zählt
  // ungefragt.
  VERSTANDEN_OFFEN = false;
  $("verstanden").hidden = true;
  await refresh();
}

// --- Dim 5: der KI-Berater. Dauerhaft offen (kein Modal), zwei getrennte Wege:
//     „Werte übernehmen" schreibt VORLÄUFIGE Vorschläge, „Nachfragen" schreibt nichts.
//     Kein Key -> 501-Erklär-Grenze, nie ein Fake-Wert. ---
function beraterZeile(cls, text) {
  const p = document.createElement("p");
  p.className = cls;
  p.textContent = text;   // textContent, nicht innerHTML: alles hier ist Fremdtext (YAML, Modell, Nutzer)
  return p;
}

// „Erklär mir" erklärt jetzt wirklich. Julius 2026-08-14: „würde ich erwarten als Nutzer, dass es
// wirklich erklärt und nicht dann die KI aufgeht und ich ihr eine Frage stellen kann."
// Die erste Antwort kommt OHNE Modell: Fragetext, Kurzhilfe und Zitatanker liegen bereits vor
// (aus /fragen). Damit ist sie sofort da, kostet nichts und funktioniert auch ohne LLM-Key —
// und sie ist Gesetzestext, keine Paraphrase. Nachfragen gehen danach an die KI.
async function erklaereFeld() {
  if (!AKTUELL) return;
  const q = AKTUELL;
  const body = $("chat-body");
  body.innerHTML = "";
  // Alles hier steht SYNCHRON auf dem Bildschirm — kein await davor. Jedes askable Feld trägt
  // einen Zitatanker (gemessen: 263 von 263), die Erklärung braucht also keinen Netzaufruf und
  // erscheint sofort und vollständig statt in zwei Etappen.
  body.appendChild(beraterZeile("chat-frage-titel", q.fragetext_laie || q.feld_id));
  if (q.hilfe_kurz) body.appendChild(beraterZeile("chat-erklaer", q.hilfe_kurz));
  const gesetz = beraterZeile("chat-gesetz", "");
  const setzeGesetz = (a) => {
    if (!a || !(a.quelle || a.zitatanker)) return false;
    gesetz.textContent = `${a.quelle || ""}: „${a.zitatanker || ""}“`;
    return true;
  };
  if (setzeGesetz(q.anker_ref)) body.appendChild(gesetz);
  body.appendChild(beraterZeile("chat-nachfrage-hint",
    "Noch unklar? Frag einfach unten nach — oder schreib gleich deine Angabe hin."));
  $("berater").scrollIntoView({ block: "nearest" });
  // Hat das Feld schon ein Event, ist der Anker aus /warum der genauere (er folgt der Regel, die
  // tatsächlich gegriffen hat). Reine Verfeinerung — der Nutzer liest schon, während das läuft.
  const r = await jget(`/fall/${FALL}/feld/${q.feld_id}/warum`);
  if (r.status === 200 && r.body.justification) setzeGesetz(r.body.justification.anker_ref);
}

// EIN Weg für alles, was der Nutzer schreibt. Julius 2026-08-14: „‚Ein Satz an die KI' kann aber
// auch einfach eine Nachfrage sein." Vorher gab es zwei Knöpfe — der Nutzer musste seinen eigenen
// Satz vorher einsortieren, obwohl ein Satz oft beides ist („Ich fahre 15 km — zählt Homeoffice
// eigentlich als Arbeitstag?"). Jetzt kommt beides zurück: Vorschläge UND Antwort, eines darf
// leer sein.
//
// Der Verlauf wird ANGEHÄNGT, nicht ersetzt: erst dadurch sind Rückfragen möglich — man sieht,
// worauf man sich bezieht. `feld_id` geht mit, damit die Antwort weiß, bei welcher Frage der
// Nutzer gerade steht.
async function chatSenden() {
  const sendBtn = $("chat-send");
  if (sendBtn.disabled) return;   // Doppel-Submit-Schutz
  const t = $("chat-text"); const freitext = (t && t.value || "").trim();
  if (!freitext) return;
  sendBtn.disabled = true;
  const body = $("chat-body");
  body.appendChild(beraterZeile("chat-du", "Du: " + freitext));
  const warte = beraterZeile("chat-erklaer", "Die KI liest mit …");
  body.appendChild(warte);
  const r = await jpost(`/fall/${FALL}/chat`,
                        { text: freitext, feld_id: AKTUELL ? AKTUELL.feld_id : null });
  warte.remove();
  if (r.status === 501) {
    body.appendChild(beraterZeile("chat-vertrag",
      (r.body && r.body.vertrag) || "Der KI-Kanal ist noch nicht verbunden."));
  } else if (r.status === 200 && r.body) {
    const vorschlaege = r.body.vorschlaege || [];
    const konflikte = r.body.konflikte || [];
    if (r.body.antwort) {
      body.appendChild(beraterZeile("chat-antwort", r.body.antwort));
      // Das Modell muss im Schema angeben, ob es sich sicher ist. Wird das verschwiegen, sieht
      // eine Vermutung aus wie eine Auskunft — und der Nutzer trägt sie in seine Erklärung ein.
      if (r.body.unsicher) {
        body.appendChild(beraterZeile("chat-unsicher",
          "Die KI ist sich hier nicht sicher — bitte nachprüfen, im Zweifel steuerlich beraten lassen."));
      }
    }
    if (vorschlaege.length || konflikte.length) {
      const teile = [];
      if (vorschlaege.length) {
        teile.push(vorschlaege.length === 1 ? "ein Vorschlag" : vorschlaege.length + " Vorschläge");
      }
      // Konflikte gehören in dieselbe Meldung: sie sind der Grund, warum die KI eine Angabe
      // scheinbar überhört hat — sie durfte ein belegtes Feld nicht überschreiben.
      if (konflikte.length) {
        teile.push(konflikte.length === 1 ? "ein Widerspruch zu deinen Angaben"
                                          : konflikte.length + " Widersprüche zu deinen Angaben");
      }
      body.appendChild(beraterZeile("chat-erklaer", "Dazu " + teile.join(" und ") + " — oben."));
      // Die Verstanden-Seite tritt vor: der Nutzer sieht alles auf einmal, was aus seinem Satz
      // geworden ist, samt dem Satzteil, auf den es sich stützt. Der Berater bleibt darunter
      // stehen, die Antwort ist also weiter lesbar.
      zeigeVerstanden(vorschlaege, konflikte);
      await refresh();   // Ring/Belegt aktualisieren; zeigeVerstanden hält die Seite vorn
    } else if (!r.body.antwort) {
      body.appendChild(beraterZeile("chat-erklaer",
        "Daraus konnte die KI weder einen Wert ableiten noch eine Frage erkennen. "
        + "Schreib es etwas genauer — oder trag den Wert oben direkt ein."));
    }
  } else {
    body.appendChild(beraterZeile("chat-vertrag",
      "Der KI-Kanal antwortete unerwartet (" + r.status + ")."));
  }
  t.value = "";
  body.scrollTop = body.scrollHeight;   // das Neueste im Blick behalten
  sendBtn.disabled = false;
}

async function zeigeWarum() {
  if (!AKTUELL) return;
  const el = $("anker");
  if (!el.hidden) { el.hidden = true; return; }   // Toggle zu — bereits offen
  const r = await jget(`/fall/${FALL}/feld/${AKTUELL.feld_id}/warum`);
  if (r.status === 404) {
    el.textContent = AKTUELL.anker_ref ? `${AKTUELL.anker_ref.quelle}\n„${AKTUELL.anker_ref.zitatanker}"` : "(kein Anker)";
  } else {
    const a = (r.body.justification || {}).anker_ref || {};
    el.textContent = `${a.quelle || ""}\n„${a.zitatanker || ""}"`;
  }
  el.hidden = false;
}

const GUARD = {
  werbungskosten_nicht_ring_faehig: "Du hast weitere Werbungskosten (z.B. doppelte Haushaltsführung) — der vereinfachte Bescheid gilt nur für den reinen Pendlerfall.",
  sonderausgaben_nicht_ring_faehig: "Du hast Sonderausgaben (z.B. Altersvorsorge) — der vereinfachte Bescheid gilt nur ohne gesondert erfasste Sonderausgaben (folgt).",
  einkunftsart_nicht_ring_faehig: "Du hast weitere Einkunftsarten — dafür ist die vollständige Berechnung nötig (folgt).",
  dhf_tatbestand_offen: "Zur doppelten Haushaltsführung fehlt noch eine Angabe (z.B. beruflicher Anlass, eigener Hausstand) — bitte vervollständigen.",
  ausland_dhf_nicht_ring_faehig: "Deine Zweitwohnung liegt im Ausland — dafür gelten andere Grenzen (folgt).",
  partner_kegel_offen: "Für die gemeinsame Erklärung fehlen noch Angaben zu deinem Partner (Bruttolohn, Identifikationsnummer).",
  partner_vor_offen: "Die gemeinsame Vorsorge-Berechnung folgt — der vereinfachte Splitting-Bescheid gilt vorerst ohne Vorsorgeaufwendungen.",
  verpflegung_reduktion_offen: "Verpflegungspauschale bei mehr als 3 Monaten am selben Ort oder gestellten Mahlzeiten reduziert — bitte gib an, ob das zutrifft (die Reduktion folgt).",
  partner_konsistenz_offen: "Du hast einen Behinderten-Pauschbetrag für deinen Partner angegeben, aber keine Zusammenveranlagung gewählt — das setzt eine gemeinsame Erklärung voraus. Bitte Veranlagung oder Partner-Angabe prüfen.",
  partner_vorsorge_offen: "Bei der gemeinsamen Veranlagung fehlt noch die Vorsorge (Renten-/Kranken-/Pflegebeiträge) deines Partners — den gemeinsamen Bescheid mit beider Vorsorge können wir noch nicht erstellen (folgt). Bis dahin kein halber Bescheid.",
  alleinerziehend_konsistenz_offen: "Du hast angegeben, alleinstehend zu sein (Entlastungsbetrag für Alleinerziehende), aber gleichzeitig Zusammenveranlagung gewählt — das schließt sich aus (§ 24b). Bitte prüfe, ob du alleinstehend ODER zusammenveranlagt bist.",
  kapital_semantik_offen: "Du hast sowohl die Gesamt-Kapitalerträge als auch einzelne Aktien-Gewinne/Verluste angegeben — das lässt sich noch nicht eindeutig zusammenrechnen. Bitte gib entweder die Gesamtsumme ODER die Einzeltöpfe an (die kombinierte Erfassung folgt).",
  rentenfreibetrag_fixierung_offen: "Deine Rente hat vor diesem Jahr begonnen — dann ist der steuerfreie Teil (Rentenfreibetrag) als fester Euro-Betrag eingefroren. Bitte gib deinen Rentenfreibetrag aus dem letzten Steuerbescheid an, sonst können wir die Rente nicht korrekt berechnen.",
  gewinn_quelle_offen: "Du hast deinen Gewinn sowohl als fertigen Betrag als auch über die Einzelposten (Betriebseinnahmen/-ausgaben, AfA) angegeben — das lässt sich nicht eindeutig zusammenrechnen. Bitte gib entweder den fertigen Gewinn ODER die Einzelposten an.",
  luf_euer_offen: "Für Einkünfte aus Land- und Forstwirtschaft können wir den Gewinn noch nicht aus den Einzelposten (Einnahmen-Überschuss-Rechnung) ermitteln — dort gelten Sonderregeln. Bitte trage den bereits ermittelten Gewinn als fertigen Betrag ein.",
  gewst_hebesatz_offen: "Du hast einen Gewerbesteuer-Messbetrag angegeben (für die Anrechnung nach § 35) — dafür brauchen wir noch den Hebesatz deiner Gemeinde (meist zwischen 200 % und 900 %, steht auf dem Gewerbesteuerbescheid). Bitte ergänze ihn.",
  abs3_ueber_5mio_offen: "Du hast den ermäßigten Steuersatz nach § 34 Abs. 3 beantragt (Betriebsveräußerung ab 55). Dieser gilt aber nur für den Teil des Veräußerungsgewinns bis 5 Millionen Euro — der übersteigende Betrag braucht eine gesonderte Berechnung, die wir noch nicht anbieten. Bitte lass diesen Fall steuerlich prüfen.",
};
async function zeigeErgebnis() {
  const r = (await jget(`/fall/${FALL}/ergebnis`)).body;
  $("fertig").hidden = false;
  $("fertig").focus({ preventScroll: true });   // Screen-Reader: Wechsel zum Ergebnis-Screen, konsistent zu #wegpunkt
  const el = $("ergebnis");
  if (r.zahl_cent === null) {
    el.className = "ergebnis ergebnis-guard";
    if (r.grund in GUARD) el.textContent = GUARD[r.grund];
    else if (r.grund === "kein_scheiben_gesamtbescheid") el.textContent = "Alle Angaben erfasst — die Gesamtsteuer wird in einem späteren Schritt berechnet.";
    else if (r.grund === "engine_unavailable") el.textContent = "Alle Angaben bestätigt — die Rechen-Engine ist hier nicht verfügbar.";
    else el.textContent = "Noch offen: " + (r.offen || []).join(", ");
  } else {
    el.className = "ergebnis";
    // Auch hier ohne innerHTML-Interpolation — nicht weil euro() gefaehrlich waere (es
    // formatiert eine selbst gerechnete Zahl), sondern damit die Regel AUSNAHMSLOS gilt:
    // eine Ausnahmeliste fuer "diese eine Stelle ist harmlos" verrottet, und die naechste
    // Stelle wird dann nach ihrem Vorbild gebaut. tests/test_kein_innerhtml_sink.py haelt es.
    el.replaceChildren(
      Object.assign(document.createElement("span"),
                    {className: "erg-zahl", textContent: euro(r.zahl_cent)}),
      Object.assign(document.createElement("span"),
                    {className: "erg-label", textContent: "festzusetzende Einkommensteuer"}));
    // stille-null-klasse-c (Variante b): grund bleibt "bestaetigt", zahl_cent gilt — aber offen listet
    // Felder aus vorlaeufigen Zusatz-Instanzen (gwg/kind/p23), die NICHT in der Zahl stecken. Hinweis,
    // keine Sperre — anderer Stil als ergebnis-guard (der ist rot/blockierend, das hier ist informativ).
    if ((r.offen || []).length) {
      const hinweis = document.createElement("p");
      hinweis.className = "ergebnis-hinweis-offen";
      hinweis.textContent = "Diese Angaben sind noch nicht bestätigt und in der Zahl NICHT enthalten: " + r.offen.join(", ");
      el.appendChild(hinweis);
    }
    // P5.4 Rechenweg-Kette — Stufen-Liste aus r.kette (vier EURO-Werte, zwei Delta-Zeilen dazwischen)
    const rw = $("rechenweg");
    const tb = $("rechenweg-tabelle");
    const hn = $("rechenweg-hinweis");
    if (r.kette) {
      const k = r.kette;
      const body = $("rechenweg-body");
      body.innerHTML = "";
      // Kurzerklärung unter Stufen-Label (dauerhaft sichtbar, kein Hover nötig)
      const RW_SUB = {
        "Gesamtbetrag der Einkünfte": "Summe aller Einkunftsarten",
        "zu versteuerndes Einkommen": "Einkommen nach allen Abzügen",
        "→ tarifliche Einkommensteuer": "Steuer nach dem Grundtarif (§ 32a)",
        "festzusetzende Einkommensteuer": "Endgültige Steuer nach Ermäßigungen und Zuschlägen",
      };
      const _rw = (label, wert, cls) => {
        const tr = document.createElement("tr");
        tr.className = "rw-reihe" + (cls ? " " + cls : "");
        const th = document.createElement("th"); th.scope = "row"; th.className = "rw-label";
        th.textContent = label;
        // "→" ist rein visuell — Screenreader liest Label ohne Pfeil
        if (label.charCodeAt(0) === 0x2192) th.setAttribute("aria-label", label.slice(2));
        const sub = RW_SUB[label];
        if (sub) {
          const span = document.createElement("span"); span.className = "rw-sub"; span.textContent = sub;
          th.appendChild(span);
        }
        const td = document.createElement("td"); td.className = "rw-wert";
        td.textContent = wert.toLocaleString("de-DE") + " €";
        tr.appendChild(th); tr.appendChild(td); body.appendChild(tr);
      };
      const _rwDelta = (label, delta) => {
        const tr = document.createElement("tr");
        tr.className = "rw-reihe rw-delta";
        const th = document.createElement("th"); th.scope = "row"; th.className = "rw-label";
        th.textContent = label;
        const td = document.createElement("td"); td.className = "rw-wert";
        const vz = delta < 0 ? "− " : "+ ";
        td.textContent = vz + Math.abs(delta).toLocaleString("de-DE") + " €";
        tr.appendChild(th); tr.appendChild(td); body.appendChild(tr);
      };
      _rw("Gesamtbetrag der Einkünfte", k.gesamtbetrag_der_einkuenfte);
      _rwDelta("Abzüge", k.zu_versteuerndes_einkommen - k.gesamtbetrag_der_einkuenfte);
      _rw("zu versteuerndes Einkommen", k.zu_versteuerndes_einkommen);
      _rw("→ tarifliche Einkommensteuer", k.tarifliche_est);
      _rwDelta("Ermäßigungen/Zuschläge", k.festzusetzende_est - k.tarifliche_est);
      _rw("festzusetzende Einkommensteuer", k.festzusetzende_est, "rw-end");
      rw.hidden = false;
      tb.hidden = false;
      hn.hidden = true;
    } else {
      // Ergebnis da, aber keine Kette (Rentner, Kinder, an_gesamt)
      tb.hidden = true;
      hn.hidden = false;
      rw.hidden = false;
    }
    // P5.5 Preflight-Check: Konsistenz + vergessene Pauschalen
    zeigePreflight();
  }
}

async function zeigePreflight() {
  const pf = $("preflight"), pl = $("preflight-liste");
  const r = await jget(`/fall/${FALL}/preflight`);
  if (r.status !== 200 || !r.body || !r.body.items || r.body.items.length === 0) {
    pf.hidden = true;
    return;
  }
  pl.innerHTML = "";
  for (const item of r.body.items) {
    const li = document.createElement("li");
    li.className = "preflight-eintrag";
    const badge = document.createElement("span");
    badge.className = "preflight-badge";
    if (item.typ === "widerspruch") {
      badge.textContent = "Widerspruch";
      li.className += " preflight-widerspruch";
    } else {
      badge.textContent = "Hinweis";
      li.className += " preflight-hinweis";
    }
    const txt = document.createElement("span");
    txt.className = "preflight-text";
    txt.textContent = item.text;
    li.appendChild(badge);
    li.appendChild(txt);
    pl.appendChild(li);
  }
  pf.hidden = false;
}

// --- Vorjahr-Übernahme: Vorjahres-Fall → vorläufige Vorschläge (herkunft=vorjahr), Nutzer bestätigt ---
async function vorjahrUebernehmen() {
  const btn = $("vorjahr-go");
  if (btn.disabled) return;   // Doppel-Submit-Schutz
  const vf = $("vorjahr-fid").value.trim(), st = $("vorjahr-status");
  if (!vf) { st.textContent = "Bitte die Vorjahres-Fall-ID angeben."; return; }
  btn.disabled = true;
  st.textContent = "Übernehme …";
  const r = await jpost(`/fall/${FALL}/vorjahr`, { vorjahr_fall_id: vf });
  if (r.status === 200) {
    st.textContent = `${r.body.uebernommen} Feld(er) aus dem Vorjahr als Vorschlag übernommen — bitte im Fluss bestätigen.`;
    await refresh();
  } else {
    st.textContent = "Übernahme fehlgeschlagen: " + ((r.body && r.body.fehler) || r.status);
  }
  btn.disabled = false;
}

// --- Kontoauszug-Upload: CSV/JSON/PDF → Transaktion-Vorschläge (herkunft=kontoauszug), Nutzer bestätigt ---
async function kontoauszugHochladen(datei) {
  const input = $("konto-file"), st = $("konto-status");
  if (!datei || input.disabled) return;   // Doppel-Submit-Schutz
  const name = (datei.name || "").toLowerCase();
  const format = name.endsWith(".json") ? "json" : name.endsWith(".csv") ? "csv"
               : name.endsWith(".pdf") ? "pdf" : null;
  if (!format) { st.textContent = "Bitte eine CSV-, JSON- oder PDF-Datei wählen."; return; }
  input.disabled = true;
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
  input.disabled = false;
}

// --- P1.1-Verdrahtung: Anmeldung, Registrierung, Abmeldung ---
// Bislang schickte die Oberfläche nie einen Authorization-Header — das Backend (/auth/register,
// /auth/login, /auth/logout, /auth/session) existierte, wurde aber nie erreicht (team-lead-Auftrag).

let LOGIN_MODUS = "anmelden";   // "anmelden" | "registrieren" — teilen sich dieselben Felder

function loginFehler(text) {
  const el = $("login-fehler");
  if (!text) { el.hidden = true; el.textContent = ""; return; }
  el.textContent = text; el.hidden = false;
}

function loginModusUmschalten() {
  LOGIN_MODUS = LOGIN_MODUS === "anmelden" ? "registrieren" : "anmelden";
  $("login-go").textContent = LOGIN_MODUS === "anmelden" ? "Anmelden" : "Registrieren";
  $("login-umschalten").textContent = LOGIN_MODUS === "anmelden" ? "Neu hier? Registrieren" : "Schon registriert? Anmelden";
  loginFehler("");
}

// Anmeldemaske zeigen: entweder beim Start (initAuth() stellt vorab fest, dass diese Instanz Auth
// verlangt) oder mitten im Fluss, wenn jget/jpost ein 401 auffängt (Token abgelaufen/fehlt).
function zeigeAnmeldemaske(hinweis) {
  VERSTANDEN_OFFEN = false;
  $("start").hidden = true;
  $("flow").hidden = true;
  $("verstanden").hidden = true;
  $("kette-overlay").hidden = true;
  $("login").hidden = false;
  loginFehler(hinweis || "");
  $("login").focus({ preventScroll: true });
}

function aktualisiereKontoLeiste() {
  const leiste = $("konto-leiste");
  if (TOKEN) { $("konto-user").textContent = AUTH_USER ? ("Angemeldet als " + AUTH_USER) : ""; leiste.hidden = false; }
  else { leiste.hidden = true; }
}

// EIN Knopf für Anmelden UND Registrieren (LOGIN_MODUS unterscheidet) — Registrierung loggt direkt
// im Anschluss ein, damit sie kein zweiter Schritt für den Nutzer ist.
async function loginGo() {
  const btn = $("login-go");
  if (btn.disabled) return;
  const user = $("login-user").value.trim(), pw = $("login-pw").value;
  if (!user || !pw) { loginFehler("Bitte Benutzername und Passwort angeben."); return; }
  btn.disabled = true;
  if (LOGIN_MODUS === "registrieren") {
    const rr = await jpost("/auth/register", { username: user, password: pw });
    if (rr.status !== 201) {
      btn.disabled = false;
      loginFehler((rr.body && rr.body.fehler) || "Registrierung fehlgeschlagen.");
      return;
    }
  }
  const r = await jpost("/auth/login", { username: user, password: pw });
  btn.disabled = false;
  if (r.status === 200 && r.body && r.body.token) {
    setToken(r.body.token);
    AUTH_USER = r.body.username || user;
    $("login-pw").value = "";
    aktualisiereKontoLeiste();
    nachAnmeldungWeiter();
  } else {
    loginFehler((r.body && r.body.fehler) || "Anmeldung fehlgeschlagen.");
  }
}

// Nach erfolgreicher (Neu-)Anmeldung: war schon ein Fall offen (Token mitten im Fluss abgelaufen),
// denselben Fall weiterführen; sonst zum Start. Ein VOR dem Login angelegter Fall käme hier nie an
// (initAuth() lässt den Nutzer keinen Fall anlegen, bevor der Auth-Zustand geklärt ist) — sonst
// bliebe ein Fall ohne user_id für immer unlesbar (403, s. api.py _fall_owner_check).
function nachAnmeldungWeiter() {
  $("login").hidden = true;
  loginFehler("");
  if (FALL) { $("start").hidden = true; $("flow").hidden = false; refresh(); }
  else { $("start").hidden = false; }
}

async function abmelden() {
  const btn = $("logout-btn");
  if (btn.disabled) return;
  btn.disabled = true;
  await jpost("/auth/logout", { token: TOKEN });
  clearToken();
  FALL = null; AKTUELL = null; STAND = null; SPANNE0 = null; KORREKTUR_FID = null; VERSTANDEN_OFFEN = false;
  btn.disabled = false;
  aktualisiereKontoLeiste();
  zeigeAnmeldemaske();
}

// Start-Sequenz: entscheidet, ob die Login-Maske oder der Start-Screen zuerst erscheint — OHNE
// vorher einen Fall anzulegen (POST /fall prüft nie den Besitz, ein so entstandener herrenloser
// Fall wäre nach einem Login für niemanden mehr lesbar, s. api.py _fall_owner_check).
async function initAuth() {
  if (TOKEN) {
    const r = await jget("/auth/session");   // /auth/* -> kein automatisches 401-Abfangen, s. _401Abfangen()
    if (r.status === 200 && r.body && r.body.username) {
      AUTH_USER = r.body.username;
      aktualisiereKontoLeiste();
      $("start").hidden = false;
      return;
    }
    // Token ungültig/abgelaufen: räumen, aber NICHT die Maske zeigen — ob diese Instanz überhaupt
    // Auth verlangt (TAXGRAPH_NO_AUTH), kennt /auth/session nicht (s. server.py _session_check).
    // Das klärt erst die Sonde unten.
    clearToken();
  }
  // Sonde gegen einen garantiert nicht existierenden Fall, OHNE Authorization-Header. Der Handler
  // (produkt/haut/api.py stand()/_fall_owner_check) prüft die Berechtigung VOR dem Laden der Datei:
  // im Einzelnutzer-Modus (TAXGRAPH_NO_AUTH=1) kommt die Prüfung durch und die Sonde endet bei 404
  // ("Fall existiert nicht"); ist Auth Pflicht, bricht sie schon davor mit 401 ab. So lässt sich der
  // Modus feststellen, ohne selbst einen Fall anzulegen.
  const probe = await jget("/fall/auth-sonde-taxgraph/stand");
  if (probe.status !== 401) $("start").hidden = false;
  // bei 401 hat der jget-Interceptor (_401Abfangen) die Anmeldemaske bereits gezeigt.
}

// --- Verdrahtung ---
document.querySelectorAll(".kachel").forEach(k => k.addEventListener("click", () => waehleScheibe(k.dataset.scheibe)));
$("chat").addEventListener("click", erklaereFeld);
$("chat-send").addEventListener("click", chatSenden);
$("warum").addEventListener("click", zeigeWarum);
$("verstanden-weiter").addEventListener("click", verstandenWeiter);
$("kette-zu").addEventListener("click", () => $("kette-overlay").hidden = true);
$("vorjahr-toggle").addEventListener("click", () => { const p = $("vorjahr-panel"); p.hidden = !p.hidden; });
$("vorjahr-go").addEventListener("click", vorjahrUebernehmen);
$("konto-toggle").addEventListener("click", () => { const p = $("konto-panel"); p.hidden = !p.hidden; });
$("konto-file").addEventListener("change", (e) => kontoauszugHochladen(e.target.files[0]));
$("login-go").addEventListener("click", loginGo);
$("login-umschalten").addEventListener("click", loginModusUmschalten);
$("logout-btn").addEventListener("click", abmelden);
initAuth();
