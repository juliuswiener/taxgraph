"use strict";
// TaxGraph-Haut — Mobile-Wegpunkt-Fluss (Vanilla-JS, kein Build). Redet NUR über die HTTP-API;
// keine Steuerlogik im Frontend, jede Zahl kommt vom Server. KI erklärt, setzt nie Werte (POST /chat -> 501).

let FALL = null;
let AKTUELL = null;   // aktuelle Frage (aus /fragen)
let STAND = null;     // letzter /stand
let SPANNE0 = null;   // Referenz-Spanne (für den Schrumpf-Anteil)
let OFFEN_ANZAHL = null;  // zuletzt bekannte Zahl offener Fragen (aus /fragen); null = nie geladen
let GESAMT_VOR = null;    // zuletzt angezeigte Gesamtzahl — allein für die Änderungs-Notiz
let KORREKTUR_FID = null;  // feld_id bei Korrektur; null = neue Frage
let VERSTANDEN_OFFEN = false;  // Verstanden-Seite liegt vorn -> refresh() darf sie nicht wegschieben

// --- Der Rückfragen-Schritt (2026-08-23) ------------------------------------------------------
// Julius, nach einem echten Durchgang: „es ist vom user flow her unklar wenn die ai dinge
// verstanden hat und man felder bestätigen soll UND fragen beantworten, was man zuerst machen
// sollte." Beides kam bis hierher GLEICHZEITIG zurück: die Vorschläge als eigene Seite
// (#verstanden), die Rückfragen als Kästen im Chat-Verlauf — ohne Reihenfolge.
// Jetzt gilt eine: erst die Rückfragen (eine nach der anderen), dann die Bestätigungen, dann
// zurück in den Fragebogen. Nie zwei Aufforderungen nebeneinander.
let RUECKFRAGEN_OFFEN = false;  // wie VERSTANDEN_OFFEN: refresh() darf die Seite nicht wegschieben
let RF_LISTE = [];              // [{frage, feld_id, aussage, meta}] — meta = Frage aus /fragen oder null
let RF_INDEX = 0;
let RF_NACHHER = null;          // {vorschlaege, konflikte} — dran, wenn die Rückfragen durch sind

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
  // Nur die Fallart-Kacheln: die Wegwahl trägt dieselbe Optik, gehört aber nicht zu diesem Vorgang.
  const kacheln = document.querySelectorAll(".kachel[data-scheibe]");
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
  OFFEN_ANZAHL = null; GESAMT_VOR = null;   // neuer Fall -> keine Änderungs-Notiz aus dem alten
  $("start").hidden = true;
  $("wegwahl").hidden = false;
  $("wegwahl").focus({ preventScroll: true });   // Screen-Reader: Wechsel des Screens ansagen
}

// --- P0b: Wegwahl. Zwei Wege, EIN Fluss ---
// Der Fall ist zu diesem Zeitpunkt bereits angelegt (die Fallart steckt in ihm, nicht in dieser
// Wahl). Deshalb schreibt hier nichts und es entsteht kein Zustand: beide Knöpfe öffnen denselben
// Fluss mit denselben Fragen und demselben KI-Panel. Der Unterschied ist allein, wo der Nutzer
// zuerst steht — und genau das war seine Frage („was man zuerst machen sollte").
// Ein eigener „KI-Modus" wäre die falsche Antwort darauf: er überlebte kein Neuladen und
// verspräche dem Nutzer etwas, das die Software nicht halten kann.
async function wegWaehlen(weg) {
  $("wegwahl").hidden = true;
  $("flow").hidden = false;
  await refresh();   // setzt AKTUELL und zeigt den Wegpunkt — auch auf dem KI-Weg
  if (weg !== "ki") return;
  const body = $("chat-body");
  body.appendChild(beraterZeile("chat-erklaer",
    "Schreib einfach los — zum Beispiel: „Ich bin ledig, hatte 62.000 Euro brutto und fahre "
    + "15 km zur Arbeit an 220 Tagen.“ Was ich daraus lese, zeige ich dir zum Bestätigen; "
    + "was unklar bleibt, frage ich einzeln nach."));
  const t = $("chat-text");
  if (t) { t.focus(); t.scrollIntoView({ block: "nearest" }); }
}

async function refresh() {
  const st = await jget(`/fall/${FALL}/stand`);
  // 401 (Token abgelaufen/fehlt): der jget-Interceptor hat die Anmeldemaske schon gezeigt — hier
  // abbrechen statt auf st.body.felder zu crashen (der Fehlerbody hat kein .felder).
  if (st.status === 401) return;
  STAND = st.body;
  // /fragen gehört jetzt VOR den Ring: die offenen Fragen sind sein Nenner (s. zeigeRing), nicht
  // bloß der Nachschub für den Fragefluss. Deshalb wird die Queue bei JEDEM refresh neu geholt —
  // auch wenn die Verstanden-Seite vorn liegt und den Fluss unterdrückt. Sonst stünde der
  // Fortschritt dort auf einer alten Gesamtzahl, obwohl gerade Bestätigungen durchlaufen.
  const fr = await jget(`/fall/${FALL}/fragen`);
  if (fr.status === 401) return;
  // Nicht ladbar (Netz weg -> jget liefert status 0 ohne .fragen): die zuletzt bekannte Zahl
  // BEHALTEN statt sie als 0 zu lesen. `fest + 0` hieße „alles beantwortet" — ein Netzfehler darf
  // keinen Fortschritt behaupten. jget hat den Banner bereits gesetzt.
  const fragen = Array.isArray(fr.body.fragen) ? fr.body.fragen : null;
  if (fragen) OFFEN_ANZAHL = fragen.length;
  zeigeRing(STAND, OFFEN_ANZAHL);
  zeigeBelegt(STAND.felder);
  // Die Verstanden-Seite arbeitet eine eigene Liste ab. Ring und Belegt-Liste sollen dabei
  // mitlaufen (jede Bestätigung bewegt den Ring), aber der Fragefluss darf sich nicht davorschieben
  // — sonst springt der Nutzer nach der ersten Bestätigung aus seiner Liste heraus.
  // Für den Rückfragen-Schritt gilt dasselbe, und aus demselben Grund: jede beantwortete Rückfrage
  // ruft refresh(), damit Ring und Belegt-Liste mitziehen. Schöbe sich dabei der Fragebogen davor,
  // stünde der Nutzer nach der ersten Antwort mitten in einer anderen Frage.
  if (VERSTANDEN_OFFEN || RUECKFRAGEN_OFFEN) return;
  if (!fragen) return;   // ohne Queue keine neue Frage — die bisherige bleibt stehen
  if (fragen.length === 0) { $("wegpunkt").hidden = true; await zeigeErgebnis(); }
  else { AKTUELL = fragen[0]; zeigeFrage(AKTUELL, STAND); }
}

// Was sich seit der letzten Anzeige an der GESAMTZAHL geändert hat, als Text — oder "" beim ersten
// Mal und wenn sie gleich blieb. Setzt GESAMT_VOR gleich mit; zeigeRing() ist der einzige Aufrufer
// und läuft genau einmal je refresh.
function aenderungsNotiz(gesamt) {
  const vor = GESAMT_VOR;
  GESAMT_VOR = gesamt;
  if (vor === null || vor === gesamt) return "";
  const d = Math.abs(gesamt - vor);
  const wieviel = d === 1 ? "1 Frage" : d + " Fragen";
  return gesamt < vor ? ` · ${wieviel} entfallen` : ` · ${wieviel} dazugekommen`;
}

// --- Dim 4: der Bescheid als schrumpfender Ring ---
//
// DER NENNER. Julius 2026-08-21: „wenn bei bescheid 2/2 -> 3/3 -> 4/4 steht hat das keine aussage:
// es sollte 1/(gesamte fragenanzahl die noch beantwortet werden muss (regelmäßig geupdated)) da
// stehen." Hier stand `felder.length` — die Zahl der schon ANGEFASSTEN Felder, also nur derer, zu
// denen bereits ein Event existiert. Dieser Nenner wuchs mit jeder Antwort um genau so viel wie der
// Zähler; daher 2/2, 3/3, 4/4, und Ring wie Leiste standen aus demselben Grund dauerhaft fast voll.
//
// Jetzt: bestätigte + noch OFFENE Fragen. Die offenen kommen aus /fragen, also aus der
// Traverser-Queue (naechste_fragen: unbeantwortete askable Felder nicht-ausgeschlossener Regeln)
// — bei JEDEM refresh neu geholt. Diese Zahl ist keine Konstante: sie hängt an den bisherigen
// Antworten, und genau das meint „regelmäßig geupdated".
//
// KEIN DOPPELZÄHLEN: ein VORLÄUFIGES Feld steht in `stand.felder` UND in der Queue (der Traverser
// hält `vorlaeufig` für unbeantwortet, _unbeantwortet()). Es zählt hier genau einmal, nämlich als
// offene Frage. Deshalb `fest + offen` und nicht `felder.length + offen`.
//
// WENN DIE GESAMTZAHL SICH ÄNDERT — die Entscheidung, nicht der Zufall:
// Eine Antwort kann ganze Blöcke abschalten (Screening-Flags; dann SINKT die Gesamtzahl und der
// Balken springt vor) oder aufmachen (dann STEIGT sie und der Balken springt zurück). Verworfen
// wurden beide „ruhigen" Varianten: ein eingefrorener Höchststand-Nenner meldete nach dem
// Abschalten eines Blocks weiter Arbeit, die es nicht mehr gibt, und ein nur-vorwärts-Balken
// behauptete nach dem Aufmachen eines Blocks Fortschritt, den es nicht gibt. Ein Balken, der
// zurückspringt, irritiert; einer, der lügt, ist schlimmer. Also: der Balken zeigt immer den echten
// Anteil, in beide Richtungen — und jede Änderung der Gesamtzahl wird unter der Leiste BENANNT
// („12 Fragen entfallen"), damit kein Sprung unerklärt bleibt.
function zeigeRing(stand, offen) {
  const spanneEl = $("spanne"), hintEl = $("spanne-hint"), ringEl = $("ring"), mitteEl = $("ring-mitte");
  const fortschritt = $("fortschritt"), textEl = $("fortschritt-text");
  const felder = Object.values(stand.felder || {});
  const fest = felder.filter(f => f.zustand === "bestaetigt").length;
  // offen === null heißt: /fragen war noch nie erreichbar. Dann gibt es keinen ehrlichen Nenner —
  // `fest + 0` läse sich als „fertig". Lieber gar keine Zahl als eine erfundene.
  const gesamt = (offen === null) ? null : fest + offen;
  const anteil = gesamt ? fest / gesamt : 0;
  ringEl.style.setProperty("--anteil", anteil);
  mitteEl.textContent = (gesamt === null) ? "" : `${fest}/${gesamt}`;
  fortschritt.max = Math.max(1, gesamt || 1);
  fortschritt.value = fest;
  textEl.textContent = (gesamt === null) ? ""
    : `${fest} von ${gesamt} Fragen beantwortet` + aenderungsNotiz(gesamt);

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
      // „noch N offen" hieß hier `felder.length - fest` — das sind die VORLÄUFIGEN Felder, nicht die
      // offenen Fragen. Bei einem Nutzer ohne KI-Vorschläge war das dauerhaft 0. Dieselbe
      // Verwechslung wie beim Nenner oben, an derselben Quelle behoben.
      hintEl.textContent = "▼ schrumpft mit jeder Antwort"
        + ((offen === null) ? "" : ` · noch ${offen} offen`);
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

  baueEingabe(q, $("eingabe"), "feld-input", "frage",
              (kiVorschlag && vorhanden.wert !== null) ? vorhanden.wert : null);

  // Bestätigungs-Geste skaliert mit KI-Konfidenz (Dim 2): KI-Vorschlag -> Halten; sonst 1-Tipp.
  rüsteBestaetigen(kiVorschlag ? vorhanden : null, q);
}

// Das Eingabefeld zu einer Frage — die EINE Bauart, für den Fragebogen wie für den
// Rückfragen-Schritt. Sie stand bis 2026-08-23 inline in zeigeFrage(); herausgezogen ist sie,
// weil der Rückfragen-Schritt dieselben Feldtypen bedient. Eine zweite Bauart daneben hieße, dass
// die cent-Umrechnung (Euro-Eingabe -> Cent) und die bool-Umkehr (`frage_invertiert`) an zwei
// Stellen stimmen müssten — und beide sind bereits einmal auseinandergelaufen.
//
// `id`/`labelId` sind Parameter, weil es die id `feld-input` nur EINMAL im Dokument geben darf:
// leseWert() holt sein Element per getElementById, und ein verstecktes #wegpunkt behält seine
// Eingabe im DOM. Zwei gleiche ids, und der Rückfragen-Schritt läse den Wert der Frage darunter.
function baueEingabe(q, box, id, labelId, vorbelegung) {
  box.innerHTML = ""; let input;
  if (q.typ === "bool") {
    // Buttons statt Dropdown: eine Ja/Nein-Frage hinter einem Klapp-Menü zu verstecken kostet
    // zwei Interaktionen für eine Information. Der Wert landet in einem hidden input, damit
    // leseWert() unverändert einen `.value` liest.
    //
    // ACHTUNG, hier NICHT boolAntwort() ergänzen. Die Button-Werte sind ANTWORTEN, `beispielwert`
    // ist laut Bindungs-Schema ein FELDWERT — bei den 7 invertierten Feldern fallen die
    // auseinander. Gemessen 2026-08-20: die Bindung führt dort trotzdem die ANTWORT (`kein_kap`
    // hat beispielwert false und die Kurzhilfe sagt ausdrücklich „Nur Arbeitslohn → Nein"), die
    // Vorauswahl trifft also heute den Normalfall richtig. Wer hier eine Umkehr einbaut, dreht
    // die Vorbelegung der fünf Screening-Fragen um. Die Doppeldeutigkeit von `beispielwert` ist
    // gemeldet und ungelöst — sie ist eine Bindungs-Frage, keine Frage dieser Zeile.
    input = wahlFeld([["Ja", "true"], ["Nein", "false"]], q.beispielwert, id);
  } else if (q.typ === "enum") {
    // Anzeigetext statt Rohwert: ohne das las der Nutzer "land_forst" oder bei den
    // Kindschaftsverhältnissen nur "1". Fallback auf den Rohwert, falls ein Feld noch kein
    // Label hat — lieber technisch als leer.
    const opt = (q.enum_werte || []).map(v => [(q.enum_labels && q.enum_labels[v]) || v, v]);
    // Ab WAHL_MAX wird die Button-Reihe länger als der Bildschirm (16 Bundesländer, 16
    // DBA-Staaten) — dort bleibt das Dropdown die bessere Bedienung.
    input = opt.length <= WAHL_MAX ? wahlFeld(opt, q.beispielwert, id) : selectFeld(opt, q.beispielwert, id);
  } else {
    input = document.createElement("input"); input.type = "number";
    input.inputMode = q.typ === "cent" ? "decimal" : "numeric";
    if (q.bereich) { if ("min" in q.bereich) input.min = q.bereich.min; if ("max" in q.bereich) input.max = q.bereich.max; }
    input.placeholder = q.typ === "cent" ? "Betrag in Euro" : String(q.beispielwert ?? "");
    // Die Vorbelegung ist ein SPEICHERWERT (Cent), das Feld nimmt Euro — dieselbe Umrechnung wie
    // in leseWert(), nur andersherum.
    if (vorbelegung !== null && vorbelegung !== undefined) {
      input.value = (q.typ === "cent") ? (vorbelegung / 100) : vorbelegung;
    }
  }
  // Bei der Button-Gruppe trägt das versteckte input im Container die id — der Container selbst
  // darf sie nicht überschreiben, sonst findet leseWert() ein <div> ohne .value.
  if (!input.classList.contains("wahl")) input.id = id;
  input.setAttribute("aria-labelledby", labelId);   // a11y: Screenreader liest die Frage als Feldnamen
  box.appendChild(input);
  if (q.einheit) { const s = document.createElement("span"); s.className = "einheit"; s.textContent = " " + q.einheit; box.appendChild(s); }
  return input;
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
// Der gewählte Wert landet in einem versteckten input mit der übergebenen id — dadurch bleibt
// leseWert() unverändert und muss nicht wissen, wie die Auswahl aussieht.
function wahlFeld(optionen, vorauswahl, id) {
  const wrap = document.createElement("div");
  wrap.className = "wahl";
  const hidden = document.createElement("input");
  hidden.type = "hidden"; hidden.id = id;
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

function selectFeld(optionen, vorauswahl, id) {
  const sel = document.createElement("select");
  for (const [text, wert] of optionen) {
    const o = document.createElement("option"); o.value = wert; o.textContent = text;
    if (String(wert) === String(vorauswahl)) o.selected = true;
    sel.appendChild(o);
  }
  sel.id = id;
  return sel;
}

// Antwort <-> gespeicherter Wert bei bool-Feldern, die eine ABWESENHEIT benennen, während ihre
// Frage nach der ANWESENHEIT fragt (`kein_kap` unter „Hattest du Kapitalerträge?"). Die Umkehr
// ist eine Involution — dieselbe Funktion trägt beide Richtungen, und genau deshalb steht sie
// hier EINMAL: leseWert() (Antwort -> Wert) und verstandenWertText() (Wert -> Antwort) müssen
// sich immer einig sein, sonst bestätigt der Nutzer auf der Verstanden-Seite mit einem Klick das
// Gegenteil dessen, was er im Fragefluss gesagt hat.
//
// WELCHE Felder das sind, sagt die Bindung (`frage_invertiert`), nicht der Feldname. Vorher stand
// an beiden Stellen `feld_id.startsWith("kein_")`. Diese Heuristik traf `kein_kap`, verfehlte aber
// jedes Feld mit der Verneinung in der MITTE (`vpf_keine_mahlzeitengestellung`,
// `dhf_keine_pflicht_dienstwohnung` — dort speicherte die Oberfläche das Gegenteil der Antwort)
// und hätte `stammdaten_keine_bankverbindung` falsch umgekehrt, dessen Frage die Verneinung
// selbst führt („Hast du KEINE Bankverbindung?"). Ein Feldname ist keine Aussage über die
// Richtung seiner Frage; eine Präfix-Regel bricht beim nächsten `x_ohne_y`.
function boolAntwort(meta, b) { return meta.frage_invertiert ? !b : !!b; }

// Stille-Null-Fix (team-lead-Auftrag, Befund B): vorher machte `parseFloat(el.value || "0")` /
// `parseInt(el.value || "0", 10)` aus einem LEEREN oder browser-ungültigen Feld (z.B. "12,5" mit
// Komma statt Punkt -> parseFloat liest nur "12" korrekt, der Rest verwirft -> "12,5x" wäre sogar
// NaN) kommentarlos eine 0 — ununterscheidbar von einer echten, absichtlich eingegebenen 0. Jetzt:
// leer/ungültig -> `undefined` (der Aufrufer in bestaetigen() muss das prüfen und abbrechen statt
// zu schreiben); eine explizit eingegebene 0 bleibt eine echte, gültige 0 (roh="0" ist NICHT leer).
//
// `el` ist optional und meint das Eingabefeld, aus dem gelesen wird. Ohne Angabe ist es das des
// Fragebogens (#feld-input); der Rückfragen-Schritt übergibt sein eigenes. Ein zweiter Leser
// daneben wäre der Ort, an dem cent- und bool-Behandlung wieder auseinanderlaufen.
function leseWert(q, el) {
  el = el || $("feld-input");
  if (q.typ === "enum") return el.value;
  if (q.typ === "bool") return boolAntwort(q, el.value === "true");
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
// Die bool-Umkehr ist der heikle Teil (vgl. boolAntwort bei leseWert): das FELD behauptet die
// Abwesenheit, die FRAGE fragt nach der Anwesenheit. `kein_kap = true` unter „Hattest du dieses
// Jahr Kapitalerträge?" als „Ja" anzuzeigen wäre das glatte Gegenteil dessen, was gespeichert
// wird — und hier bestätigt der Nutzer mit einem Klick, was er liest.
function verstandenWertText(v) {
  if (v.typ === "bool") return boolAntwort(v, v.wert) ? "Ja" : "Nein";
  if (v.typ === "enum") return (v.enum_labels && v.enum_labels[v.wert]) || String(v.wert);
  if (v.typ === "cent") return euro(v.wert);
  return String(v.wert) + (v.einheit ? " " + v.einheit : "");
}

// Der Rechenweg unter dem Wert (2026-08-23). Hat die KI den Wert AUSGERECHNET statt ihn abgelesen
// („50.000 € pro Jahr ÷ 12 × 6 Monate"), gehört die Rechnung neben das Ergebnis — sonst bestätigt
// der Nutzer eine Zahl, deren Zustandekommen er nicht sehen kann. Genau daran hing der teuerste
// gemessene Fehler dieses Kanals: aus „bis Juni 100k p.a." wurde ein Jahresbrutto von 100.000 €.
//
// ANZEIGE, KEIN GATE. Nachgerechnet wird bewusst nicht (Julius 2026-08-23) — ein verworfener
// Vorschlag wäre für den Nutzer unsichtbar, und die Grundregel des Hauses ist, dass die KI
// vorschlägt und der Mensch bestätigt. Eine Multiplikation, die er neben dem Wert liest, kann er
// selbst prüfen; sie ihm vorher wegzunehmen, nimmt ihm die Entscheidung.
//
// `rechenweg` fehlt oder ist null, wo nichts gerechnet wurde — dann steht hier nichts, und das ist
// zugleich der abwärtskompatible Fall (der Endpunkt liefert das Feld heute noch nicht).
function rechenwegZeile(v) {
  const rw = v.rechenweg;
  if (!rw) return null;
  let text = String(rw.erklaerung || "").trim();
  if (!text) {
    // Zahlen ohne Satz: lieber die nackte Rechnung als gar nichts. Ein Rechenweg, der still
    // verschwindet, ist derselbe Fehler, gegen den die Aussagen-Anzeige gebaut wurde.
    if (typeof rw.basis !== "number" || typeof rw.faktor !== "number") return null;
    // `basis` steht laut Schema in DERSELBEN Einheit wie der Wert — bei Geld also in Cent.
    text = ((v.typ === "cent") ? euro(rw.basis) : String(rw.basis)) + " × " + rw.faktor;
  }
  const d = document.createElement("div");
  d.className = "v-rechenweg";
  d.textContent = "Rechenweg: " + text;
  return d;
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
  // Reihenfolge: Ergebnis, dann wie es entstand, dann worauf es sich stützt. Der Leser geht vom
  // Wert über die Rechnung zum eigenen Satz zurück — jede Zeile beantwortet die Frage der darüber.
  const rw = rechenwegZeile(v);
  if (rw) li.appendChild(rw);
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

  // Hier wiegt die Rechnung am schwersten: der Nutzer vergleicht ZWEI Zahlen und soll die eigene
  // gegen eine ausgerechnete abwägen. `neu` trägt den Vorschlagswert samt Metadaten (s. oben).
  const rw = rechenwegZeile(neu);
  if (rw) li.appendChild(rw);

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

// --- Der Rückfragen-Schritt: eine Frage, ein Feld, „Weiter" ----------------------------------
//
// DIE ANTWORT GEHT NICHT AN DIE KI ZURÜCK. Die Rückfrage nennt ihr `feld_id` — die Antwort gehört
// direkt dorthin, über denselben /event-Pfad wie jede Fragebogen-Antwort (schreiber "ui:laie",
// zustand "bestaetigt", Klick als signal_2). Der Rückweg über das Modell kostete drei Stufen und
// könnte die Angabe erneut falsch deuten: genau das, was die Rückfrage verhindern soll.
//
// Der TYP kommt aus /fragen, nicht aus der Rückfrage — die trägt nur {frage, feld_id, aussage}.
// Steht das Feld dort nicht (schon beantwortet, von einer Antwort abgeschaltet) oder nennt die
// Rückfrage gar kein Feld, gibt es keine Bauart für ein Eingabefeld und erst recht keinen Typ, in
// den sich ein Wert schreiben ließe. Dann bleibt der Chat der Weg — wie bisher.
async function starteRueckfragen(rueckfragen, vorschlaege, konflikte) {
  // Die Typen frisch holen: der eben gelaufene KI-Aufruf kann vorläufige Werte geschrieben und
  // damit die offene Fragenliste verändert haben.
  const fr = await jget(`/fall/${FALL}/fragen`);
  if (fr.status === 401) return;   // Anmeldemaske hat übernommen — keine Seite darüberlegen
  const katalog = {};
  for (const q of (Array.isArray(fr.body.fragen) ? fr.body.fragen : [])) katalog[q.feld_id] = q;
  RF_LISTE = rueckfragen.map(rf => ({ ...rf, meta: rf.feld_id ? (katalog[rf.feld_id] || null) : null }));
  RF_INDEX = 0;
  RF_NACHHER = { vorschlaege: vorschlaege || [], konflikte: konflikte || [] };
  RUECKFRAGEN_OFFEN = true;
  VERSTANDEN_OFFEN = false;
  $("wegpunkt").hidden = true;
  $("fertig").hidden = true;
  $("verstanden").hidden = true;
  zeigeRueckfrage();
}

function zeigeRueckfrage() {
  if (RF_INDEX >= RF_LISTE.length) { return rueckfragenFertig(); }
  const rf = RF_LISTE[RF_INDEX];
  const q = rf.meta;
  $("rf-zaehler").textContent = RF_LISTE.length === 1
    ? "Eine Rückfrage, bevor es weitergeht."
    : `Rückfrage ${RF_INDEX + 1} von ${RF_LISTE.length}`;
  $("rf-frage").textContent = rf.frage || "";
  $("rf-hilfe").textContent = q ? (q.hilfe_kurz || "") : "";
  const box = $("rf-eingabe");
  if (q) {
    baueEingabe(q, box, "rf-input", "rf-frage", null);
    $("rf-weiter").textContent = "Weiter";
  } else {
    // Ohne Feld kein Eingabefeld: was hier stünde, wüsste nicht, wohin es geschrieben werden soll.
    box.innerHTML = "";
    const p = document.createElement("p");
    p.className = "hilfe rf-ohne-feld";
    p.textContent = "Dazu gibt es kein Feld im Fragebogen — antworte der KI im Berater-Panel, "
      + "dann liest sie deinen Satz noch einmal.";
    box.appendChild(p);
    $("rf-weiter").textContent = "Im Berater beantworten";
  }
  $("rueckfragen").hidden = false;
  $("rueckfragen").focus({ preventScroll: true });   // Screen-Reader: Wechsel des Screens ansagen
}

// „Weiter": den Wert schreiben und zur nächsten. Ohne Feld führt derselbe Knopf in den Chat —
// das ist dort die einzige Art zu antworten, und der Nutzer soll nicht zwei Knöpfe unterscheiden
// müssen, die dasselbe meinen.
async function rueckfrageWeiter() {
  const rf = RF_LISTE[RF_INDEX];
  if (!rf) return;
  const btn = $("rf-weiter");
  if (btn.disabled) return;   // Doppel-Submit-Schutz
  const q = rf.meta;
  if (!q) { rueckfrageBeantworten(rf); RF_INDEX += 1; zeigeRueckfrage(); return; }
  // Dieselbe Lesart wie im Fragebogen: cent rechnet Euro in Cent, bool dreht `frage_invertiert`
  // zurück in den Speicherwert. leer/ungültig -> undefined, und dann wird NICHTS geschrieben
  // (Stille-Null-Fix): eine stille 0 wäre hier so falsch wie dort.
  const wert = leseWert(q, $("rf-input"));
  if (wert === undefined) {
    zeigeNetzFehler("Bitte einen gültigen Wert eingeben — oder „Später beantworten“.");
    return;
  }
  btn.disabled = true;
  const ev = {
    feld_id: q.feld_id, wert, zustand: "bestaetigt",
    herkunft: { herkunft: "laie", pruef_tiefe: "ungeprueft", haftung: "nutzer" },
    schreiber: "ui:laie",
    signal: { signal_1: null, signal_2: "rueckfrage@" + q.feld_id },
  };
  // Trägt das Feld schon ein aktives Event, verlangt Auflage B (höchstens eines je Feld) ein
  // `ersetzt` — sonst weist der Store die Antwort ab und sie wäre verloren. Der Katalog, aus dem
  // die KI ihre Rückfragen zieht, ist NICHT auf unbeantwortete Felder beschränkt (genau daher
  // rührt der Konflikt-Fall in /chat), das ist also kein hypothetischer Zweig.
  //
  // Ob es ein Event GIBT, sagt `STAND` — dieselbe Quelle, aus der auch zeigeFrage() ableitet, ob
  // ein Vorschlag zu bestätigen ist. Der /warum-Aufruf steht bewusst dahinter und nicht davor:
  // ohne Event antwortet er mit 404, und ein 404 je beantworteter Rückfrage wäre eine Fehlermeldung
  // in der Konsole für den Normalfall. Liegt STAND einmal daneben, weist der Store die Antwort ab
  // und der Nutzer SIEHT es (Banner unten) — kein stiller Verlust.
  if (STAND && STAND.felder && STAND.felder[q.feld_id]) {
    const j = (await jget(`/fall/${FALL}/feld/${q.feld_id}/warum`)).body.justification || {};
    if (j.event_id) { ev.ersetzt = j.event_id; ev.signal.signal_1 = j.event_id; }
  }
  const r = await jpost(`/fall/${FALL}/event`, ev);
  btn.disabled = false;
  if (!okStatus(r.status)) {
    zeigeNetzFehler("Abgewiesen: " + (r.body.fehler || r.status));
    return;
  }
  await refresh();   // Ring + Belegt-Liste ziehen mit; die Seite bleibt vorn (RUECKFRAGEN_OFFEN)
  RF_INDEX += 1;
  zeigeRueckfrage();
}

// „Später beantworten" — und zwar OHNE Merker. Das Feld bleibt unbeantwortet und steht damit von
// selbst wieder in der Fragen-Queue (traverser.naechste_fragen führt jedes unbeantwortete askable
// Feld). Ein eigener Zustand „später" verschwände beim Neuladen und machte dem Nutzer eine Zusage,
// die die Software nicht hält.
function rueckfrageSpaeter() {
  if (RF_INDEX >= RF_LISTE.length) return;
  RF_INDEX += 1;
  zeigeRueckfrage();
}

// Die Runde ist durch: jetzt erst die Bestätigungen (Teil 3 der Reihenfolge), sonst zurück in den
// Fragebogen. refresh() holt in beiden Fällen Ring und Belegt-Liste nach — und schiebt den
// Fragebogen nur dann vor, wenn keine Verstanden-Seite ihn festhält.
function rueckfragenFertig() {
  const n = RF_NACHHER || { vorschlaege: [], konflikte: [] };
  rueckfragenSchliessen();
  if (n.vorschlaege.length || n.konflikte.length) zeigeVerstanden(n.vorschlaege, n.konflikte);
  return refresh();
}

// Die Seite räumen, ohne etwas zu zeigen. Zweiter Aufrufer ist chatSenden(): eine neue KI-Antwort
// überholt die laufende Runde — sie ist zu demselben Gespräch die neuere Auskunft.
// Die zurückgestellten Vorschläge gehen dabei nicht verloren: sie liegen als VORLÄUFIGE Events im
// Fall und begegnen dem Nutzer im Fragebogen erneut, mit Hold-to-confirm (wie bei
// verstandenWeiter). Deshalb braucht es hier kein Zusammenführen zweier Runden.
function rueckfragenSchliessen() {
  RUECKFRAGEN_OFFEN = false;
  RF_LISTE = []; RF_INDEX = 0; RF_NACHHER = null;
  $("rueckfragen").hidden = true;
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

// Die Laufanzeige — bewusst NICHT über beraterZeile(). Genau das war der Fehler (Befund B1):
// „Die KI liest mit …" entstand mit derselben Funktion wie jede Antwort und stand danach als
// unbewegter Absatz im Verlauf. Ein Antwortverlauf ist der eine Ort, an dem stillstehender Text
// als Antwort gelesen wird — und wer ihn für eine Antwort hält, hört auf zu warten.
// Die Bewegung steckt im CSS (.chat-warte-punkte); unter prefers-reduced-motion tritt an ihre
// Stelle das sichtbare Wort „läuft" (.chat-warte-still), damit der Zustand nicht mit der
// Animation verschwindet.
function chatWarteZeile() {
  const p = document.createElement("p");
  p.className = "chat-warte";
  p.setAttribute("role", "status");     // Screen-Reader: Zustandswechsel, keine neue Nachricht
  p.setAttribute("aria-live", "polite");
  const txt = document.createElement("span");
  txt.textContent = "Die KI liest mit …";
  p.appendChild(txt);
  const punkte = document.createElement("span");
  punkte.className = "chat-warte-punkte";
  punkte.setAttribute("aria-hidden", "true");   // reine Optik, nichts zum Vorlesen
  for (let i = 0; i < 3; i++) punkte.appendChild(document.createElement("i"));
  p.appendChild(punkte);
  const still = document.createElement("span");
  still.className = "chat-warte-still";
  still.setAttribute("aria-hidden", "true");
  still.textContent = "läuft";
  p.appendChild(still);
  return p;
}

// Was während eines laufenden KI-Aufrufs gesperrt wird. Julius 2026-08-20: „ansonsten kann der
// nutzer etwas klicken und torpediert u.U. die KI ausgabe" — bis hierher war NUR der Absende-
// knopf gesperrt, der ganze Rest der Oberfläche nicht.
//
// Es sind genau die Bereiche, aus denen heraus sich `AKTUELL` ändern lässt: der Fragebereich
// (bestaetigen → refresh), die Verstanden-Liste (Stimmt/Ändern → refresh bzw. korrigiereBestaetigt),
// die Belegt-Liste (Klick → korrigiereBestaetigt) und die Import-Box (Vorjahr/Kontoauszug →
// refresh). Dazu das Chat-Eingabefeld selbst, dessen Inhalt am Ende ohnehin geleert wird.
//
// `inert` statt `disabled`: die Bereiche bleiben LESBAR — der Nutzer soll seine Frage weiter
// sehen können, während die KI seinen Satz liest — nehmen aber weder Klick noch Tastatur noch
// Vorlesefokus an. `disabled` gibt es auf einem <section> gar nicht, und jedes Kind einzeln zu
// sperren hieße, sich beim Freigeben genau zu merken, welche vorher schon gesperrt waren.
//
// #rueckfragen gehört seit 2026-08-23 dazu, und zwar aus genau demselben Grund wie #wegpunkt:
// „Weiter" dort schreibt ein Event und ruft refresh(), ändert also `AKTUELL`. Dass die Seite
// während eines laufenden Aufrufs ohnehin meist im Hintergrund liegt, ist kein Argument — sie
// bleibt sichtbar, wenn der Nutzer aus ihr heraus etwas in den Chat schreibt.
const CHAT_SPERRE_IDS = ["wegpunkt", "verstanden", "belegt-liste", "rueckfragen"];

function chatSperren(an) {
  for (const id of CHAT_SPERRE_IDS) {
    const el = $(id);
    if (el) el.toggleAttribute("inert", an);
  }
  const box = document.querySelector(".vorjahr-box");
  if (box) box.toggleAttribute("inert", an);
  const t = $("chat-text");
  if (t) t.disabled = an;
  $("chat-send").disabled = an;
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

// --- Die Zwischenstufe, sichtbar gemacht (2026-08-21) ----------------------------------------
// Der KI-Dialog lief bis hierher als EIN Aufruf: Satz rein, Werte raus. Gemessen an einem echten
// Satz (222 Zeichen, FÜNF Fakten) kamen ZWEI Vorschläge zurück — der Prompt trug 301 Feld-
// beschreibungen, 87.000 von 90.000 Zeichen waren Katalog. Die Zerlegung in drei Stufen
// (produkt/haut/api_llm.py) hat für die Oberfläche eine Folge, die schwerer wiegt als die
// Trefferquote: die mittlere Stufe ist ZEIGBAR.
//
// Der Nutzer sieht jetzt, was aus seinem Satz gelesen wurde, BEVOR daraus Werte werden — und vor
// allem sieht er, was NICHT zugeordnet werden konnte. Genau das verschwand vorher ersatzlos: fünf
// Fakten hin, zwei Vorschläge zurück, und über die drei anderen stand nirgends ein Wort. Der
// Nutzer merkte nur, dass etwas fehlt, und hatte keinen Anhaltspunkt, was.
//
// Alles hier hängt sich an den Chat-Verlauf (#chat-body) und NICHT an #wegpunkt / #verstanden /
// #belegt-liste / .vorjahr-box. Das ist kein Zufall: chatSperren() legt während eines laufenden
// Aufrufs `inert` genau auf jene vier. Eine Anzeige, die dort hineinwüchse, sperrte sich selbst —
// und eine Rückfrage, die man während des nächsten Aufrufs nicht mehr anklicken kann, ist keine.
const AUSSAGE_MARKE = {
  vorschlag: "als Vorschlag oben",
  rueckfrage: "Rückfrage",
  kein_feld: "keinem Feld zugeordnet",
};

// Die Antwort auf eine Rückfrage geht durch DENSELBEN Kanal wie jeder andere Satz: sie landet im
// Chat-Eingabefeld, der Nutzer schreibt sie zu Ende und schickt sie ab. Kein zweiter Weg und kein
// eigener Zustand — eine Rückfrage, die nur über ein Sonderformular beantwortbar wäre, eröffnete
// ein zweites Gespräch neben dem Gespräch, und der Verlauf zeigte danach nur noch die Hälfte.
// Die Frage steht im Vorspann mit drin: der Nutzer sieht wörtlich, was er absendet, und die
// nächste Runde bekommt den Bezug mitgeliefert, statt ihn erraten zu müssen.
function rueckfrageBeantworten(rf) {
  const t = $("chat-text");
  // Läuft gerade ein Aufruf, wird `chat-text` am Ende von chatSenden() geleert — der Vorspann
  // wäre dann still weg, und der Nutzer hätte auf einen Knopf gedrückt, der nichts tat.
  if (!t || $("chat-send").disabled) return;
  const vorspann = "Zu deiner Rückfrage „" + (rf.frage || "") + "“: ";
  const bisher = (t.value || "").trim();
  // Schon Getipptes NICHT überschreiben: der Nutzer hat womöglich längst angefangen zu schreiben.
  t.value = (bisher ? bisher + "\n" : "") + vorspann;
  chatHoeheAnpassen();
  t.focus();
  t.setSelectionRange(t.value.length, t.value.length);
  t.scrollIntoView({ block: "nearest" });
}

function rueckfrageBlock(rf) {
  const d = document.createElement("div");
  d.className = "a-rueckfrage-box";
  const f = document.createElement("div");
  f.className = "a-frage";
  f.textContent = rf.frage || "";
  d.appendChild(f);
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "a-antworten";
  btn.textContent = "Antworten";
  btn.addEventListener("click", () => rueckfrageBeantworten(rf));
  d.appendChild(btn);
  return d;
}

function aussagenZeile(a, rueckfragen) {
  const li = document.createElement("li");
  // Nur bekannte Zustände bekommen ihre eigene Klasse. Ein unbekannter darf NICHT in die Optik
  // eines erfolgreichen rutschen — sonst sähe ein Zustand, den diese Oberfläche nicht kennt, aus
  // wie „hat geklappt".
  const bekannt = Object.prototype.hasOwnProperty.call(AUSSAGE_MARKE, a.status);
  li.className = "a-zeile " + (bekannt ? "a-" + a.status : "a-unbekannt");
  li.dataset.status = a.status || "";

  const kopf = document.createElement("div");
  kopf.className = "a-kopf";
  const txt = document.createElement("span");
  txt.className = "a-text";
  txt.textContent = a.text || "(ohne Text)";
  const marke = document.createElement("span");
  marke.className = "a-marke";
  marke.textContent = bekannt ? AUSSAGE_MARKE[a.status] : (a.status || "ohne Angabe");
  kopf.appendChild(txt);
  kopf.appendChild(marke);
  li.appendChild(kopf);

  if (a.beleg) {
    // Dasselbe wörtliche Nutzerzitat wie in der Verstanden-Liste, und aus demselben Grund: die
    // Aussage ist eine Behauptung der KI über den Satz des Nutzers. Erst der Beleg macht sie
    // nachprüfbar — ohne ihn muss man glauben.
    const b = document.createElement("div");
    b.className = "a-beleg";
    b.textContent = "weil du sagtest: „" + a.beleg + "“";
    li.appendChild(b);
  }
  if (a.status === "kein_feld") {
    const h = document.createElement("div");
    h.className = "a-hinweis";
    h.textContent = "Dazu wurde kein passendes Feld gefunden — schreib es etwas genauer, "
      + "oder trag den Wert oben direkt ein.";
    li.appendChild(h);
  }
  for (const rf of rueckfragen) li.appendChild(rueckfrageBlock(rf));
  return li;
}

function aussagenBlock(aussagen, rueckfragen) {
  const box = document.createElement("div");
  box.className = "chat-aussagen";
  const titel = document.createElement("div");
  titel.className = "chat-aussagen-titel";
  titel.textContent = "Das habe ich aus deinem Satz gelesen:";
  box.appendChild(titel);
  const ul = document.createElement("ul");
  ul.className = "a-liste";
  const zugeordnet = new Set();
  aussagen.forEach((a, i) => {
    const meine = rueckfragen.filter(rf => rf.aussage === i);
    for (const rf of meine) zugeordnet.add(rf);
    ul.appendChild(aussagenZeile(a, meine));
  });
  // Eine Rückfrage ohne gültigen Index darf nicht verschwinden — sie ist der eine Ort, an dem die
  // KI zugibt, dass sie sonst raten müsste. Lieber ohne die Aussage darüber als gar nicht.
  for (const rf of rueckfragen) {
    if (!zugeordnet.has(rf)) {
      ul.appendChild(aussagenZeile({ text: "Nachfrage der KI", status: "rueckfrage" }, [rf]));
    }
  }
  box.appendChild(ul);
  return box;
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
// Das Eingabefeld wächst mit dem Text (Julius, 2026-08-21: „das ki input feld muss mitwachsen
// wenn der user viel text eingibt"). Zwei feste Zeilen zwangen sonst jeden längeren Satz in ein
// Guckloch — man sieht beim Tippen nicht mehr, was oben steht, und kann seine eigene Angabe vor
// dem Absenden nicht überprüfen.
//
// `height = "auto"` VOR dem Lesen von scrollHeight ist nicht kosmetisch: scrollHeight meldet
// sonst nie einen Wert UNTER der aktuellen Höhe, das Feld könnte also nur wachsen und nie wieder
// schrumpfen — nach dem Absenden bliebe es in voller Größe stehen.
//
// Die Obergrenze steht in CSS (max-height), nicht hier: darüber übernimmt der eigene Scrollbalken
// des Feldes. Ohne sie schöbe ein langer Text den Absendeknopf aus dem Bild — das Panel ist
// sticky und hat nur die Fensterhöhe.
function chatHoeheAnpassen() {
  const t = $("chat-text");
  if (!t) return;
  t.style.height = "auto";
  t.style.height = t.scrollHeight + "px";
}

async function chatSenden() {
  const sendBtn = $("chat-send");
  if (sendBtn.disabled) return;   // Doppel-Submit-Schutz
  const t = $("chat-text"); const freitext = (t && t.value || "").trim();
  if (!freitext) return;
  chatSperren(true);
  // Der Kontext, für den die Antwort erzeugt wird — EINMAL beim Absenden gelesen. Er geht mit in
  // den Prompt (_erklaer_kontext in api.py) und wird unten gegen den dann aktuellen verglichen.
  const kontextFeld = AKTUELL ? AKTUELL.feld_id : null;
  const body = $("chat-body");
  body.appendChild(beraterZeile("chat-du", "Du: " + freitext));
  const warte = chatWarteZeile();
  body.appendChild(warte);
  body.scrollTop = body.scrollHeight;
  let r;
  try {
    r = await jpost(`/fall/${FALL}/chat`, { text: freitext, feld_id: kontextFeld });
  } finally {
    // Was gesperrt wurde, wird IMMER wieder freigegeben. Vorher stand `sendBtn.disabled = false`
    // allein am Ende des Rumpfs: jede Ausnahme davor ließ den Knopf DAUERHAFT gesperrt und die
    // Wartemeldung für immer stehen — ohne Neuladen der Seite kam der Nutzer da nicht mehr heraus.
    // Nachgemessen 2026-08-20: `jpost` fängt Netzabbrüche bereits selbst ab (catch → status 0), der
    // ursprünglich vermutete Auslöser greift also nicht. Die Zusage hängt trotzdem nicht länger an
    // dieser Messung — und für die Sperren oben ist sie neu und zwingend: sie umfassen jetzt den
    // ganzen Fragebereich, nicht mehr nur einen Knopf.
    warte.remove();
    chatSperren(false);
  }
  // Ab hier ist nichts mehr gesperrt: die Auswertung darf refresh() rufen und die Verstanden-Seite
  // den Fokus nehmen (in einem inerten Bereich liefe focus() ins Leere), und eine Ausnahme hier
  // strandet nichts mehr.
  //
  // Das Eingabefeld wird HIER geleert, nicht am Ende: „Absendeknopf wieder frei" und „Feld wieder
  // leer" müssen derselbe Augenblick sein. Am Ende der Funktion waren sie das nur, solange bis
  // dorthin kein `await` lag — die Auswertung darunter ruft aber refresh() und (seit dem
  // Rückfragen-Schritt) /fragen. Dazwischen stand das Feld sichtbar mit dem eben abgeschickten
  // Satz da, obwohl der Knopf schon wieder ansprach: ein zweiter Klick hätte denselben Satz noch
  // einmal an die KI geschickt. Zwischen dem finally oben und dieser Zeile liegt kein `await`.
  if (t) { t.value = ""; chatHoeheAnpassen(); }
  const kontextGewechselt = (AKTUELL ? AKTUELL.feld_id : null) !== kontextFeld;
  if (r.status === 501) {
    body.appendChild(beraterZeile("chat-vertrag",
      (r.body && r.body.vertrag) || "Der KI-Kanal ist noch nicht verbunden."));
  } else if (r.status === 200 && r.body) {
    const konflikte = r.body.konflikte || [];
    // Die drei neuen Felder sind ADDITIV und dürfen fehlen: der dreistufige Dialog kann später
    // kommen oder ganz ausbleiben, und dann muss hier exakt das Verhalten von vorher stehen —
    // keine leere Überschrift, keine Ausnahme in der Konsole. Array.isArray statt `|| []`, weil
    // ein Nicht-Array (null, ein Objekt, eine Zeichenkette) sonst bis in .forEach durchliefe.
    const aussagen = Array.isArray(r.body.aussagen) ? r.body.aussagen : [];
    const rueckfragen = Array.isArray(r.body.rueckfragen) ? r.body.rueckfragen : [];
    // Auflage: zu einer Rückfrage gibt es KEINEN Vorschlag am selben Feld. Kommt beides, hat die
    // KI gleichzeitig gefragt und geraten — und der Vorschlag IST die Vermutung, nach der sie
    // gerade fragt. Gemessener Anlass (2026-08-21): aus „bis Juni 100k p.a." wurde ein Jahres-
    // brutto von 100.000 EUR, und zwar sicher — 70.000 EUR zu viel, nur durch Julius' Korrektur
    // verhindert. Hier gewinnt deshalb die Rückfrage, und dass ein Wert zurückgehalten wurde,
    // steht sichtbar da: still verschwinden lassen wäre derselbe Fehler in die andere Richtung.
    const gefragteFelder = new Set(rueckfragen.map(rf => rf.feld_id).filter(Boolean));
    const alleVorschlaege = r.body.vorschlaege || [];
    const vorschlaege = alleVorschlaege.filter(v => !gefragteFelder.has(v.feld_id));
    const zurueckgehalten = alleVorschlaege.length - vorschlaege.length;
    if (aussagen.length || rueckfragen.length) {
      body.appendChild(aussagenBlock(aussagen, rueckfragen));
      if (zurueckgehalten) {
        body.appendChild(beraterZeile("chat-unsicher",
          zurueckgehalten === 1
            ? "Zu einem Feld kamen gleichzeitig eine Rückfrage und ein fertiger Wert — der Wert "
              + "wird zurückgehalten, bis die Rückfrage beantwortet ist."
            : zurueckgehalten + " Werte werden zurückgehalten, weil zu denselben Feldern noch "
              + "eine Rückfrage offen ist."));
      }
    }
    if (r.body.antwort) {
      // Die Antwort wurde für die Frage erzeugt, bei der der Nutzer sie abgeschickt hat — der
      // Kontext dafür ging als `feld_id` mit. Steht er inzwischen woanders, ist sie eine Auskunft
      // zu einer ANDEREN Frage, und still darunter gehängt liest sie sich als Antwort auf die
      // jetzt offene. Die Sperre oben verhindert den Wechsel per Klick; ein Wechsel aus einem
      // schon vorher laufenden Vorgang (Kontoauszug-/Vorjahr-Import ruft refresh() und setzt
      // AKTUELL neu) kann sie NICHT verhindern — dann sagt es die Zeile hier.
      // Nur die Antwort braucht das: die Vorschläge tragen ihre eigene `feld_id` bis in den
      // Schreibpfad (verstandenBestaetigen postet v.feld_id, nicht AKTUELL), nachgemessen.
      if (kontextGewechselt) {
        body.appendChild(beraterZeile("chat-kontext-hinweis",
          "Diese Antwort gehört zu der Frage, bei der du sie abgeschickt hast — inzwischen ist "
          + "eine andere Frage offen."));
      }
      body.appendChild(beraterZeile("chat-antwort", r.body.antwort));
      // Das Modell muss im Schema angeben, ob es sich sicher ist. Wird das verschwiegen, sieht
      // eine Vermutung aus wie eine Auskunft — und der Nutzer trägt sie in seine Erklärung ein.
      if (r.body.unsicher) {
        body.appendChild(beraterZeile("chat-unsicher",
          "Die KI ist sich hier nicht sicher — bitte nachprüfen, im Zweifel steuerlich beraten lassen."));
      }
    }
    // Eine neue Antwort überholt eine noch laufende Rückfragen-Runde: sie ist zu demselben
    // Gespräch die neuere Auskunft, und die alten Fragen können durch den eben geschickten Satz
    // längst beantwortet sein. Gemerkt wird nur, DASS eine lief — der Fragebogen muss danach
    // zurückkommen, auch wenn diese Antwort weder Rückfrage noch Vorschlag bringt.
    const rundeLief = RUECKFRAGEN_OFFEN;
    if (rundeLief) rueckfragenSchliessen();
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
    // DIE REIHENFOLGE (Julius 2026-08-23): erst die Rückfragen, dann die Bestätigungen, dann
    // zurück in den Fragebogen. Nie zwei Aufforderungen gleichzeitig — bis hierher erschienen
    // Verstanden-Seite und Rückfrage-Kästen nebeneinander, und der Nutzer musste raten, was
    // zuerst dran ist. Ohne Rückfragen bleibt alles, wie es war.
    if (rueckfragen.length) {
      if (teile.length) {
        body.appendChild(beraterZeile("chat-erklaer",
          "Dazu " + teile.join(" und ") + " — die zeige ich dir gleich nach den Rückfragen."));
      }
      await starteRueckfragen(rueckfragen, vorschlaege, konflikte);
    } else if (vorschlaege.length || konflikte.length) {
      body.appendChild(beraterZeile("chat-erklaer", "Dazu " + teile.join(" und ") + " — oben."));
      // Die Verstanden-Seite tritt vor: der Nutzer sieht alles auf einmal, was aus seinem Satz
      // geworden ist, samt dem Satzteil, auf den es sich stützt. Der Berater bleibt darunter
      // stehen, die Antwort ist also weiter lesbar.
      zeigeVerstanden(vorschlaege, konflikte);
      await refresh();   // Ring/Belegt aktualisieren; zeigeVerstanden hält die Seite vorn
    } else {
      if (!r.body.antwort) {
        // Hier ist `rueckfragen` bereits leer (sonst liefe der Zweig oben): steht eine Rückfrage
        // auf dem Schirm, hat die KI sehr wohl etwas erkannt — sie fragt ja nach. „Weder einen
        // Wert noch eine Frage" wäre dort schlicht falsch und schickte den Nutzer zum
        // Umformulieren, statt zum Antworten.
        body.appendChild(beraterZeile("chat-erklaer",
          "Daraus konnte die KI weder einen Wert ableiten noch eine Frage erkennen. "
          + "Schreib es etwas genauer — oder trag den Wert oben direkt ein."));
      }
      // Die Rückfragen-Seite ist gerade weggeräumt worden und nichts tritt an ihre Stelle: ohne
      // das hier bliebe der Fluss leer, weil #wegpunkt noch versteckt ist.
      if (rundeLief) await refresh();
    }
  } else {
    body.appendChild(beraterZeile("chat-vertrag",
      "Der KI-Kanal antwortete unerwartet (" + r.status + ")."));
  }
  // Das Leeren des Eingabefelds (samt chatHoeheAnpassen, sonst bliebe das leere Feld gross) steht
  // jetzt weiter oben, direkt hinter dem finally — s. die Begründung dort.
  body.scrollTop = body.scrollHeight;   // das Neueste im Blick behalten
  // Kein `sendBtn.disabled = false` mehr: das Freigeben steht jetzt vollständig im finally oben.
  // Zwei Freigabestellen wären eine zu viel — diese hier liefe nach einer Ausnahme nie, und genau
  // darauf beruhte der Dauer-Sperr-Fehler.
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
  rueckfragenSchliessen();   // räumt auch RUECKFRAGEN_OFFEN — sonst bliebe der Fluss danach leer
  $("start").hidden = true;
  $("wegwahl").hidden = true;
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
  OFFEN_ANZAHL = null; GESAMT_VOR = null;
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
// `[data-scheibe]` statt `.kachel`: die Wegwahl benutzt dieselbe Kachel-Optik, hat aber keine
// Scheibe — ohne den Zusatz riefe ihr Klick waehleScheibe(undefined) und legte einen zweiten Fall an.
document.querySelectorAll(".kachel[data-scheibe]").forEach(k => k.addEventListener("click", () => waehleScheibe(k.dataset.scheibe)));
$("weg-fragebogen").addEventListener("click", () => wegWaehlen("fragebogen"));
$("weg-ki").addEventListener("click", () => wegWaehlen("ki"));
$("rf-weiter").addEventListener("click", rueckfrageWeiter);
$("rf-spaeter").addEventListener("click", rueckfrageSpaeter);
$("chat").addEventListener("click", erklaereFeld);
$("chat-send").addEventListener("click", chatSenden);
// `input` statt `keyup`: es feuert auch beim Einfügen aus der Zwischenablage und beim Ziehen von
// Text ins Feld — und gerade der eingefügte Absatz ist der Fall, für den das Mitwachsen gebaut ist.
$("chat-text").addEventListener("input", chatHoeheAnpassen);
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
