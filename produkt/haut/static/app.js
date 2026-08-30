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
// Seit der Umkehrung der Reihenfolge (2026-08-25) der Normalfall: erst bestätigen, DANN nachfragen.
// Hier liegen die Rückfragen, solange die Verstanden-Seite steht — sie werden danach neu gegen die
// dann offenen Fragen geprüft, weil eine Bestätigung ganze Blöcke abschalten kann.
let VERSTANDEN_DANACH = null;   // {rueckfragen, vorschlaege, konflikte, zurueckgestellt}
// „Ändern" führt in den Fragebogen und MUSS danach zurück in die Liste. Gemerkt wird, aus welcher
// Zeile es kam — die Liste selbst bleibt im DOM stehen, samt der Häkchen, die schon gesetzt sind.
let VERSTANDEN_ZURUECK = null;  // {feld_id, li}
let RF_BEANTWORTET = 0;         // wie viele Rückfragen dieser Runde einen Wert bekommen haben
let RF_SPAETER = 0;             // und wie viele der Nutzer zurückgestellt hat
let SCREENING_OFFEN = false;    // wie VERSTANDEN_OFFEN: refresh() darf die Seite nicht wegschieben
let SCREENING_LISTE = [];       // die Fragen mit `screening: true`, aus /fragen

// Fluss-Mitschnitt: läuft er? Einmal beim Start aus /health gelesen (dort steht `flow`). Der
// Server sieht nur die fertigen /event-Aufrufe — die Ankreuzliste, die Nachfragen und die
// KI-Prüfliste sind Bildschirme HIER, und was VORGELEGT wurde, weiss nur diese Seite. Eine
// übersprungene Nachfrage hinterlässt im Fall überhaupt nichts.
//
// Julius, 2026-08-27: „du hast in dem verlauf nicht die nachfragen, ki überprüfungsfragen und die
// checkliste drin!!!"
let FLOW_AN = false;

// Melden und NICHT darauf warten: der Mitschnitt ist Beobachtung, er darf den Ablauf nicht
// aufhalten und erst recht nicht anhalten, wenn er scheitert. Deshalb ohne `await` gerufen und mit
// eigenem catch — ein Protokoll darf nicht kaputtmachen, was es beschreibt.
function meldeFluss(art, inhalt) {
  if (!FLOW_AN || !FALL) return;
  jpost(`/fall/${FALL}/flow`, { art, inhalt }).catch(() => {});
}

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
// KI-FOKUS: der Chat wird vom Begleiter zum Arbeitsbereich und bekommt die breite Spalte
// (Julius 2026-08-23: „die kis sidebar ist zu klein wenn das aber der bereich ist in dem der user
// gerade arbeitet … es sollte den main fokus bekommen").
//
// Eine Klasse am Rahmen, sonst nichts — kein gespeicherter Modus, keine zweite Wahrheit darüber,
// „wo man ist". Sie hält über den GANZEN KI-Weg: Antwort, Rückfragen, Bestätigungen. Sie fällt
// erst weg, wenn der Nutzer im Fragebogen selbst antwortet (bestaetigen). Genau deshalb muss sie
// kein Neuladen überleben — wer neu lädt, steht nicht mehr am Anfang, und dann ist die
// Seitenleiste wieder die richtige Form.
function kiFokus(an) {
  const f = $("flow");
  if (f) f.classList.toggle("ki-fokus", !!an);
}

async function wegWaehlen(weg) {
  meldeFluss("weg_gewaehlt", { weg });   // steht sonst nirgends — der Fall kennt nur das Ergebnis
  $("wegwahl").hidden = true;
  $("flow").hidden = false;
  kiFokus(weg === "ki");
  await refresh();   // setzt AKTUELL und zeigt den Wegpunkt — auch auf dem KI-Weg
  // Der Fragebogen beginnt mit der Ankreuzliste: zehn Kreuze nehmen 147 der 321 Fragen weg.
  // Auf dem KI-Weg NICHT — dort erhebt die KI dieselben Dinge aus dem Satz des Nutzers, und die
  // Liste käme erst, wenn er später in den Fragebogen wechselt (s. zumFragebogen).
  if (weg !== "ki") { await zeigeScreening(); return; }
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
  if (VERSTANDEN_OFFEN || RUECKFRAGEN_OFFEN || SCREENING_OFFEN) return;
  if (!fragen) return;   // ohne Queue keine neue Frage — die bisherige bleibt stehen
  if (fragen.length === 0) {
    $("wegpunkt").hidden = true;
    // Keine offene Frage mehr: der KI-Weg ist an seinem Ziel. Ohne das bliebe der KI-Fokus
    // stehen, und der blendet ALLES ausser dem Panel aus — auch das Ergebnis, auf das der
    // Nutzer die ganze Zeit hingearbeitet hat.
    kiFokus(false);
    await zeigeErgebnis();
  }
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
    // „Vereinfachter Bescheid hier nicht möglich / siehe Ergebnis unten" — Julius 2026-08-25:
    // „was soll das heißen?" Zu Recht: der Satz nennt eine Einschränkung, die der Nutzer nie
    // verlangt hat („vereinfacht"?), und verweist auf ein Ergebnis, das es noch gar nicht gibt.
    // Gemeint ist: eine deiner Angaben braucht eine Rechnung, die die schnelle Schätzung hier
    // oben nicht leisten kann — die vollständige kommt am Ende.
    spanneEl.textContent = "Schätzung hier nicht möglich";
    hintEl.textContent = "eine deiner Angaben braucht die volle Rechnung — die steht am Ende";
    ringEl.style.setProperty("--schrumpf", 1); return;
  }
  const iv = stand.intervall;
  // GEMESSEN 2026-08-24: hier stand „Bescheid: —" mit dem Untertitel „steht". Beides zusammen ist
  // eine Behauptung über einen Wert, den es nicht gibt: `euro(null)` ist „—", und die Bedingung
  // `min === max` ist für zwei null-Grenzen erfüllt. Der Nutzer las also „steht" an einer Stelle,
  // an der die Rechnung noch gar nichts hergibt. Ein fehlender Wert muss als fehlend dastehen.
  if (iv && (iv.min_cent === null || iv.max_cent === null)) {
    spanneEl.textContent = "Noch keine Zahl";
    hintEl.textContent = "sobald genug beantwortet ist, steht hier deine Steuer";
    ringEl.style.setProperty("--schrumpf", 1);
    return;
  }
  if (iv) {
    const breite = Math.max(0, iv.max_cent - iv.min_cent);
    if (SPANNE0 === null || breite > SPANNE0) SPANNE0 = breite || 1;
    ringEl.style.setProperty("--schrumpf", SPANNE0 ? breite / SPANNE0 : 0);
    // „steht" allein sagte einem Laien nichts — es ist die Aussage, dass die Spanne zu einem Punkt
    // geschrumpft ist und keine offene Frage den Betrag mehr bewegt.
    if (iv.min_cent === iv.max_cent) {
      spanneEl.textContent = `Deine Steuer: ${euro(iv.min_cent)}`;
      hintEl.textContent = "steht fest — keine offene Frage ändert diesen Betrag mehr";
    }
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

// --- Die Ankreuzliste am Anfang -------------------------------------------------------------
//
// Zehn Fragen erheben je die EXISTENZ eines ganzen Themas (Kinder, Vermietung, Kapitalerträge,
// Behinderung, …). Einzeln durch den Fragebogen verteilt standen sie auf den Positionen 2, 4, 5,
// 8, 9, 18, 19, 27, 33 und 38 — dazwischen die Detailfragen genau der Themen, nach denen noch
// gar nicht gefragt war. Gemessen 2026-08-25: zehn Kreuze nehmen 147 der 321 Fragen weg.
//
// WELCHE Fragen das sind, sagt die Bindung (`screening: true`), nicht diese Datei. Ein Filter über
// den Feldnamen („kein_…") wäre dieselbe Heuristik, die hier schon einmal zwei Feldern das
// Gegenteil der Nutzerantwort entlockt hat — dafür gibt es `frage_invertiert`, und aus demselben
// Grund gibt es jetzt `screening`.
//
// EIN KREUZ HEISST „JA, GAB ES". Der Speicherwert ist bei diesen Feldern das Gegenteil
// (`kein_kap` = true heisst „keine Kapitalerträge"), deshalb läuft die Antwort durch dieselbe
// Umkehr wie im Fragebogen: boolAntwort(q, angekreuzt).
async function zeigeScreening(danach) {
  const fr = await jget(`/fall/${FALL}/fragen`);
  if (fr.status === 401) return false;      // Anmeldemaske hat übernommen
  const offen = (Array.isArray(fr.body.fragen) ? fr.body.fragen : []).filter(q => q.screening);
  if (!offen.length) return false;          // alle schon beantwortet: nichts zu zeigen

  SCREENING_LISTE = offen;
  const ul = $("screening-liste");
  ul.innerHTML = "";
  for (const q of offen) {
    const li = document.createElement("li");
    li.className = "sc-zeile";
    const lab = document.createElement("label");
    lab.className = "sc-label";
    const box = document.createElement("input");
    box.type = "checkbox";
    box.className = "sc-box";
    box.dataset.feld = q.feld_id;
    const txt = document.createElement("span");
    txt.className = "sc-text";
    txt.textContent = q.fragetext_laie || q.feld_id;
    lab.append(box, txt);
    li.appendChild(lab);
    if (q.hilfe_kurz) {
      const h = document.createElement("div");
      h.className = "sc-hilfe";
      h.textContent = q.hilfe_kurz;
      li.appendChild(h);
    }
    ul.appendChild(li);
  }
  SCREENING_DANACH = danach || null;
  SCREENING_OFFEN = true;
  $("wegpunkt").hidden = true;
  $("fertig").hidden = true;
  $("screening").hidden = false;
  $("screening").focus({ preventScroll: true });
  return true;
}

let SCREENING_DANACH = null;   // was nach dem Weiter dran ist (Funktion oder null = Fragebogen)

// „Weiter": JEDE Frage bekommt eine Antwort — die angekreuzten ein Ja, die übrigen ein Nein.
//
// Das ist die eigentliche Entscheidung dieser Seite, und sie ist nicht selbstverständlich: ein
// leeres Kästchen könnte auch „noch nicht gelesen" heissen. Dann müsste man jede Frage einzeln
// stellen — also genau das, was die Seite abschaffen soll. Deshalb sagt die Seite ausdrücklich
// „wozu du nichts ankreuzt, fragen wir gar nicht erst" UND nennt den Rückweg: die Felder bleiben
// änderbar, und mit ihnen kommen die Folgefragen zurück.
async function screeningWeiter() {
  const btn = $("screening-weiter");
  if (btn.disabled) return;                 // Doppel-Submit-Schutz
  btn.disabled = true;
  const alt = btn.textContent;
  btn.textContent = "Wird gespeichert …";
  // KEINE Meldung für die Ankreuzliste: jede ihrer Antworten trägt `screening@…` als zweites
  // Signal, und das Kreuz hinter dem gespeicherten Wert liefert `frage_invertiert` aus der
  // Bindung. Sie ist damit aus dem Fall vollständig rekonstruierbar (nachgemessen 2026-08-27) —
  // eine zusätzliche Anfrage brächte nichts Neues.
  try {
    for (const q of SCREENING_LISTE) {
      const box = $("screening-liste").querySelector(`.sc-box[data-feld="${q.feld_id}"]`);
      const wert = boolAntwort(q, !!(box && box.checked));
      const r = await jpost(`/fall/${FALL}/event`, {
        feld_id: q.feld_id, wert, zustand: "bestaetigt",
        herkunft: { herkunft: "laie", pruef_tiefe: "ungeprueft", haftung: "nutzer" },
        schreiber: "ui:laie",
        signal: { signal_1: null, signal_2: "screening@" + q.feld_id },
      });
      if (!okStatus(r.status)) {
        // Abbrechen statt weiterlaufen: die übrigen Felder blieben sonst unbeantwortet, während
        // der Nutzer die Seite verlässt — und er hielte sie für erledigt.
        zeigeNetzFehler("Abgewiesen: " + (r.body.fehler || r.status));
        btn.disabled = false; btn.textContent = alt;
        return;
      }
    }
  } finally {
    btn.textContent = alt;
    btn.disabled = false;
  }
  SCREENING_OFFEN = false;
  $("screening").hidden = true;
  SCREENING_LISTE = [];
  const danach = SCREENING_DANACH;
  SCREENING_DANACH = null;
  if (danach) return danach();
  await refresh();
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
  // Eine Überschrift ohne Inhalt ist eine Zusage, die nichts einlöst: „Schon beantwortet" stand
  // im Bild, als noch nichts beantwortet war (gemessen 2026-08-24, KI-Weg vor der ersten
  // Antwort). Gilt für beide Wege — leer ist leer.
  const sektion = ul.closest(".belegt");
  if (sektion) sektion.hidden = !Object.keys(felder || {}).length;
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
    // Die FRAGE und der LESBARE Wert, nicht Kennung und Rohwert (2026-08-24). Vorher stand hier
    // „bruttoarbeitslohn 2500000" — beides für einen Laien unlesbar, und die Zahl liest sich als
    // sein Betrag. Dieselben Formatierer wie in der Verstanden-Liste; die Metadaten liefert
    // /stand seit demselben Tag mit (api._anzeige_metadaten).
    // Fällt zurück auf Kennung + Rohwert, wenn die Metadaten fehlen (Feld nicht mehr in der
    // Bindung, etwa nach einem Scheiben-Wechsel) — lieber technisch als leer.
    const a = belegtAnzeige(fid, f, felder);
    const t = document.createElement("span"); t.className = "z-name";
    t.textContent = a.frage || fid;
    t.title = fid;                       // die Kennung bleibt erreichbar, nur nicht im Weg
    const v = document.createElement("span"); v.className = "z-wert";
    v.textContent = a.typ ? verstandenWertText(a) : JSON.stringify(f.wert);
    li.appendChild(t); li.appendChild(v); ul.appendChild(li);
  }
}

// Anzeige-Daten einer Belegt-Zeile. Fuer fast jedes Feld ist das die Zeile selbst; fuer ein
// INSTANZ-Feld nicht.
//
// GEMESSEN 2026-08-27, zwei Kinder eingetragen — untereinander standen:
//     Wie heisst dein Kind mit Vornamen?    Anna
//     kind_vorname__2                       "Ben"
// Die zweite Zeile zeigt Kennung und Rohwert, also genau die zwei Dinge, die diese Liste dem Laien
// ersparen soll. Ursache: die Anzeige-Metadaten kommen aus der Bindung (api._anzeige_metadaten),
// und dort steht nur das Basisfeld — ein Nachschlagen unter `kind_vorname__2` findet nichts, alle
// Werte bleiben None.
//
// Der Rueckgriff aufs Basisfeld passiert deshalb HIER, nicht serverseitig: /stand liefert beide
// Zeilen im selben Aufruf, die noetigen Angaben stehen also schon im Bild. Die Nummer kommt dazu,
// sonst stuenden zwei Zeilen mit derselben Frage untereinander und niemand wuesste, welche welche
// ist. Das gilt fuer die KLICKBARE Zeile — eine, die der Nutzer nicht als seine Frage erkennt,
// klickt er auch nicht an, und der Korrekturweg dahinter bliebe unerreichbar.
//
// Fehlt auch das Basisfeld (Instanz 1 leer gelassen), bleibt es beim bisherigen Rueckfall auf
// Kennung + Rohwert — lieber technisch als leer.
function belegtAnzeige(fid, f, felder) {
  if (f.typ) return f;
  const { basis, instanz } = basisFeldId(fid);
  const b = basis !== fid ? (felder || {})[basis] : null;
  if (!b || !b.typ) return f;
  return { ...f, typ: b.typ, einheit: b.einheit, enum_labels: b.enum_labels,
           frage_invertiert: b.frage_invertiert,
           frage: b.frage ? `${b.frage} (Nr. ${instanz})` : null };
}

// --- Dim 1: Herkunft-Kette (Euro -> Regel -> Norm -> Beleg) im Bestätigungsmoment ---
async function herkunftKette(fid, f) {
  const r = await jget(`/fall/${FALL}/feld/${fid}/warum`);
  if (r.status === 401) return;   // Anmeldemaske hat übernommen — kein Overlay mehr darüberlegen
  const j = (r.status === 200 && r.body.justification) ? r.body.justification : {};
  const bi = badgeInfo(f.herkunft_badge);
  $("kette-titel").textContent = (f.frage || fid) + ": "
    + (f.typ ? verstandenWertText(f) : JSON.stringify(f.wert));
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
  step("◗", "Wert", `${f.typ ? verstandenWertText(f) : JSON.stringify(f.wert)} `
                    + `(${bi.lab}, ${f.zustand})`);
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

  // Bei einer Instanz-Achse kommt die Vorbelegung JE INSTANZ aus dem Stand, nicht als ein Wert.
  // Sonst laesst sich ein Instanz-Feld nicht korrigieren, ohne alle anderen Zeilen neu zu tippen —
  // und wer eine vorbelegte Zeile leer vorfindet, muss annehmen, seine Antwort sei weg.
  baueEingabe(q, $("eingabe"), "feld-input", "frage",
              q.instanz_anzahl > 1
                ? instanzVorbelegungen(q, stand)
                : ((kiVorschlag && vorhanden.wert !== null) ? vorhanden.wert : null));

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
// N Eingabefelder statt N-mal derselben Frage (Julius 2026-08-25: „wie hier mehrere angeben bei
// nur einem feld?? … oder man gibt direkt n inputfelder anstatt jedesmal einen neue frage").
//
// 69 Felder tragen eine `instanz_gruppe`, 31 davon für Kinder. Store, ELSTER-Mapping und Bescheid
// kennen die Achse seit langem (`base`, `base__2`, `base__3`; est_mapping.parse_instanz ist die
// EINE Enumerations-Wahrheit) — der Fragebogen fragte trotzdem einmal. Wer zwei Kinder hatte,
// konnte einen Vornamen eintragen; für das zweite gab es kein Feld.
//
// Der Traverser bleibt dabei unangetastet: er liefert das Basisfeld wie bisher einmal, und die
// Zahl steht als `instanz_anzahl` daneben. Diese Funktion baut daraus die Felder.
// Was fuer dieses Feld schon im Fall steht, je Instanz: `{1: "Anna", 2: "Ben"}`. Die Werte liegen
// in `stand.felder` als EIGENE Schluessel nebeneinander (`kind_vorname`, `kind_vorname__2`), nicht
// als Liste unter dem Basisfeld — deshalb wird je Instanz nachgeschlagen. Kein eigener Endpunkt:
// /stand fuehrt sie laengst, sie wurden hier nur nie gelesen.
//
// Eine Instanz ohne Wert bleibt WEG statt als `null` einzuziehen: `baueEingabe` unterscheidet
// "nicht vorbelegt" an genau diesem Unterschied, und eine leere Zeile mit Platzhalter liest sich
// richtig — als noch nicht beantwortet.
function instanzVorbelegungen(q, stand) {
  const felder = (stand && stand.felder) || {};
  const out = {};
  for (let i = 1; i <= q.instanz_anzahl; i++) {
    const f = felder[instanzFeldId(q.feld_id, i)];
    if (f && f.wert !== null && f.wert !== undefined) out[i] = f.wert;
  }
  return out;
}

// Die Vorbelegung genau EINER Instanz. Nur ein Objekt `{1: …, 2: …}` traegt Werte je Instanz.
//
// EIN SKALAR IST KEINER, und das war nicht bloss wirkungslos: Zeichenketten lassen sich indizieren.
// GEMESSEN 2026-08-27, zwei Kinder und die Vorbelegung "Anna": beide Zeilen standen mit dem
// Buchstaben "n" da ("Anna"[1] und "Anna"[2]). Erreichbar, sobald ein KI-Vorschlag auf einem Feld
// mit Achse liegt — zeigeFrage reichte den Wert bis hierher als einzelnen Wert durch. Ein
// Buchstabe im Feld sieht dabei wie eine Antwort aus, nicht wie ein Fehler.
function vorbelegungJe(vorbelegungen, i) {
  if (!vorbelegungen || typeof vorbelegungen !== "object") return null;
  const w = vorbelegungen[i];
  return w === undefined ? null : w;
}

function baueInstanzEingaben(q, box, id, labelId, vorbelegungen) {
  box.innerHTML = "";
  const wrap = document.createElement("div");
  wrap.className = "instanzen";
  wrap.id = id;                       // damit leseInstanzWerte() den Block wiederfindet
  for (let i = 1; i <= q.instanz_anzahl; i++) {
    const zeile = document.createElement("div");
    zeile.className = "instanz-zeile";
    const marke = document.createElement("span");
    marke.className = "instanz-marke";
    marke.textContent = `${q.instanz_etikett || "Nr."} ${i}`;
    const feld = document.createElement("div");
    feld.className = "eingabe instanz-feld";
    zeile.append(marke, feld);
    wrap.appendChild(zeile);
    // Jede Instanz bekommt EIN normales Eingabefeld — derselbe Bau wie sonst, damit Typ, Format,
    // Standardwert-Knopf und Auswertung überall gleich sind.
    const einzeln = { ...q, instanz_anzahl: 1 };
    const el = baueEingabe(einzeln, feld, `${id}__${i}`, labelId,
                           vorbelegungJe(vorbelegungen, i));
    el.dataset.instanz = String(i);
  }
  box.appendChild(wrap);
  return wrap;
}

function baueEingabe(q, box, id, labelId, vorbelegung) {
  if (q.instanz_anzahl > 1) return baueInstanzEingaben(q, box, id, labelId, vorbelegung);
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
  } else if (q.typ === "text" && q.muster) {
    // Feld MIT zugesagtem Format (Bindung `muster`, seit 2026-08-25). `pattern` lässt den Browser
    // selbst warnen, bevor abgeschickt wird; leseWert() prüft zusätzlich, und der Store setzt es
    // fail-closed durch. Drei Ebenen, weil ein formal falscher Wert erst beim Finanzamt auffiele:
    // Julius' Durchgang speicherte „01.01-31.122" (eine 2 zu viel) anstandslos.
    input = document.createElement("input");
    input.type = "text";
    input.pattern = q.muster.replace(/^\^|\$$/g, "");   // HTML verankert ohnehin am ganzen Wert
    input.placeholder = String(q.standardwert ?? q.beispielwert ?? "");
    if (vorbelegung !== null && vorbelegung !== undefined) input.value = String(vorbelegung);
  } else if (q.typ === "text") {
    // GEMESSEN 2026-08-24, im Live-Lauf des KI-Wegs: bis hierher fielen `text` und `datum` in den
    // Zahlen-Zweig unten. Die Rückfrage „Wie heißen deine Kinder mit Vornamen?" stand über einem
    // <input type="number"> mit dem Platzhalter „Anna" — ein Feld, das den eigenen Beispielwert
    // nicht annimmt. 56 text- und 5 datum-Felder der Bindung waren damit nicht beantwortbar,
    // im Fragebogen genauso wie im Wizard (beide rufen diese Funktion).
    input = document.createElement("input");
    input.type = "text";
    input.placeholder = String(q.beispielwert ?? "");
    // Vorbelegung wie im Zahlen-Zweig: beim Korrigieren eines schon belegten Feldes
    // (korrigiereBestaetigt) stünde der Name sonst nicht mehr da, und der Nutzer müsste ihn
    // neu tippen, um eine Kleinigkeit zu ändern.
    if (vorbelegung !== null && vorbelegung !== undefined) input.value = String(vorbelegung);
  } else if (q.typ === "datum") {
    // Kalenderfeld statt Tippen — aber ACHTUNG bei der Auswertung: `.value` ist hier IMMER ISO
    // (2025-07-15), unabhängig davon, wie der Browser es anzeigt. Der Store verlangt TT.MM.JJJJ
    // und weist ISO mit 422 ab (store.py `_typ_konform`, mit Begründung: amtliches ELSTER-Format,
    // nichts in der Pipeline konvertiert). Die Umrechnung steht in leseWert().
    input = document.createElement("input");
    input.type = "date";
    // Der gespeicherte Wert ist TT.MM.JJJJ, das Feld will ISO — dieselbe Umrechnung wie in
    // leseWert(), nur andersherum. Ohne sie stünde das Feld beim Korrigieren leer da.
    const d = /^(\d{2})\.(\d{2})\.(\d{4})$/.exec(String(vorbelegung ?? ""));
    if (d) input.value = `${d[3]}-${d[2]}-${d[1]}`;
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

  // „Üblichen Wert übernehmen" — nur wo die Bindung einen `standardwert` ZUSAGT (2026-08-25).
  // Julius: „wenn in 95% der default fall eintritt sollte man evtl einen button ergänzen der
  // diesen default wert übernimmt." Es sind die Zeitraum-Felder, deren Hilfe selbst sagt „Meist
  // das ganze Jahr": den Nutzer 01.01-31.12 abtippen zu lassen ist Arbeit ohne Ertrag.
  //
  // AUSDRÜCKLICH NICHT aus `beispielwert` abgeleitet, obwohl der bei fast jedem Feld dasteht: der
  // ist laut Bindungs-Schema ein BEISPIEL, kein Standard. Ein Knopf „Üblich: 62.000 €" unter der
  // Frage nach dem Bruttoarbeitslohn wäre eine Vorgabe, die der Nutzer womöglich übernimmt.
  if (q.standardwert !== null && q.standardwert !== undefined && q.standardwert !== "") {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "btn-link standardwert";
    b.textContent = "Üblichen Wert übernehmen: " + q.standardwert;
    b.addEventListener("click", () => {
      input.value = String(q.standardwert);
      input.focus();
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    box.appendChild(b);
  }
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

// Die Werte ALLER Instanzen, in der Reihenfolge 1..n. Eine leere Instanz liefert `undefined` und
// wird vom Aufrufer NICHT geschrieben — dieselbe Regel wie bei einem einzelnen leeren Feld
// (Stille-Null-Fix). Wer nur das erste Kind kennt, trägt eben nur das erste ein.
function leseInstanzWerte(q, id) {
  const wrap = $(id);
  if (!wrap) return [];
  const out = [];
  for (let i = 1; i <= q.instanz_anzahl; i++) {
    const el = wrap.querySelector(`#${CSS.escape(id + "__" + i)}`);
    out.push(el ? leseWert(q, el) : undefined);
  }
  return out;
}

// Wie das Feld der i-ten Instanz im Store heisst. `base` ist Instanz 1, ab 2 mit Suffix — genau
// das Format, das est_mapping.parse_instanz liest. Kein zweites Regex, keine Drift.
function instanzFeldId(basis, i) {
  return i <= 1 ? basis : `${basis}__${i}`;
}

// Der Rueckweg: aus `kind_vorname__2` wird `{basis: "kind_vorname", instanz: 2}`, aus einem Feld
// ohne Achse `{basis: <es selbst>, instanz: 1}`.
//
// KEIN eigenes Muster, und das ist hier der ganze Punkt: die Zerlegung wird gegen instanzFeldId
// GEGENGEPROBT. Nur was der Hinweg genau so gebaut haette, gilt als Instanz. Damit koennen Hin-
// und Rueckweg nicht auseinanderlaufen — ein zweites Regex daneben muesste bei jeder Aenderung
// mitgezogen werden, und die `__n`-Konvention steht ohnehin schon an drei Stellen
// (est_mapping.parse_instanz, instanzFeldId hier, der Typpruefer im Store).
//
// Die Gegenprobe faengt nebenbei die Faelle, die ein handgeschriebenes Regex gern durchlaesst:
// `x__02` (fuehrende Null), `x__0`, `x__` und `x__2x` sind KEINE Instanzen — instanzFeldId haette
// sie nie so geschrieben.
function basisFeldId(fid) {
  const p = String(fid || "").lastIndexOf("__");
  if (p < 0) return { basis: fid, instanz: 1 };
  const basis = fid.slice(0, p);
  const n = Number(fid.slice(p + 2));
  return (Number.isInteger(n) && n >= 2 && instanzFeldId(basis, n) === fid)
    ? { basis, instanz: n } : { basis: fid, instanz: 1 };
}

// „Bitte einen gültigen Wert eingeben" sagt einem Nutzer, der gerade „01.01-31.122" getippt hat,
// nichts — er sieht ja einen Wert dastehen. Wo die Bindung ein Format zusagt, wird es genannt,
// mit dem üblichen Wert als Beispiel.
function formatHinweis(q) {
  if (!q || !q.muster) return "";
  const bsp = q.standardwert || q.beispielwert;
  return "Der Wert passt nicht zum erwarteten Format"
         + (bsp ? ` — er muss aussehen wie „${bsp}“` : "");
}

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
  if (q.typ === "text") {
    // Passt der Wert nicht zum zugesagten Format, wird NICHTS geschrieben — dieselbe Regel wie
    // bei leer/ungültig sonst. Der Aufrufer meldet es dem Nutzer; ohne das nähme der Store ihn an
    // (typ text = beliebiger String) und der Fehler fiele erst beim Finanzamt auf.
    if (q.muster && !new RegExp(q.muster).test(roh)) return undefined;
    return roh;
  }
  if (q.typ === "datum") {
    // Ein <input type="date"> liefert ISO, der Store verlangt TT.MM.JJJJ (store.py `_typ_konform`:
    // amtliches ELSTER-Format, im XSD verankert, nichts konvertiert unterwegs). Beides wird
    // angenommen — falls das Feld doch einmal ein Textfeld ist, tippt der Nutzer deutsch.
    const iso = /^(\d{4})-(\d{2})-(\d{2})$/.exec(roh);
    if (iso) return `${iso[3]}.${iso[2]}.${iso[1]}`;
    // Kein erkennbares Datum -> undefined statt einer Zeichenkette, die der Store mit 422
    // abweist: leer/ungültig schreibt NICHTS (dieselbe Regel wie beim Stille-Null-Fix).
    return /^\d{2}\.\d{2}\.\d{4}$/.test(roh) ? roh : undefined;
  }
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
  // Die event_id wird fuer DAS ANGEKLICKTE Feld geholt, auch wenn es eine Instanz ist:
  // `kind_vorname__2` ist ein eigenes Feld im Store mit eigenem aktivem Event (gemessen
  // 2026-08-27). Nur seine event_id belegt, dass da wirklich etwas zu korrigieren ist — die des
  // Basisfelds sagt ueber Instanz 2 nichts.
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

  // Die Frage zu DIESEM Feld — nicht die Suche in /fragen, die hier bis zum 2026-08-27 stand.
  //
  // /fragen ist die Queue der UNBEANTWORTETEN Felder (traverser.naechste_fragen: `_unbeantwortet`).
  // Ein bestaetigtes Feld faellt heraus, ein vorlaeufiges bleibt drin — also fand die Suche hier das
  // zu korrigierende Feld NIE, sobald es bestaetigt war, und schickte den Nutzer mit „durch eine
  // andere Antwort entfallen" weg. Gemessen an einem Feld ganz ohne Instanz-Achse:
  // korrigiereBestaetigt('fam_anzahl_kinder') -> false, obwohl er die Frage gerade selbst
  // beantwortet hatte. Dass „Ändern" auf der Pruefliste trotzdem lief, lag allein daran, dass
  // KI-Vorschlaege vorlaeufig sind und damit in der Queue bleiben.
  //
  // Der Endpunkt loest `__n` selbst aufs Basisfeld auf und legt `instanz_anzahl` daneben — deshalb
  // wird `fid` unveraendert uebergeben und die Basis aus der ANTWORT genommen. Eine zweite
  // Aufloesung hier waere eine zweite Wahrheit ueber dieselbe Konvention.
  const frage_r = await jget(`/fall/${FALL}/feld/${fid}/frage`);
  if (frage_r.status !== 200 || !frage_r.body.frage) {
    // 404 heisst: das Feld gehoert nicht (mehr) zu dieser Scheibe — nach einem Scheiben-Wechsel
    // etwa. Die Kennung gehoert nicht in die Meldung, der Laie hat sie nie gesehen (gemessen
    // 2026-08-25: „Feld kind_wohnsitz_inland_zeitraum ist nicht mehr im Fragenfluss.").
    //
    // Der ANDERE Fall — Frage durch eine andere Antwort abgeschaltet — steht unten und hat seine
    // eigene Meldung. Beides in einen Satz zu legen war der urspruengliche Fehler.
    zeigeNetzFehler("Diese Frage gehört nicht mehr zu deiner Erklärung und lässt sich deshalb "
                    + "nicht ändern.");
    return false;
  }
  const frage = frage_r.body.frage;

  // Ist die Frage durch eine ANDERE ANTWORT abgeschaltet? Dann ist sie wirklich entfallen — und
  // genau dafuer war dieser Hinweis gedacht. Bis der Endpunkt eine `regel_id` mitgab, war dieser
  // Fall von einem bloss beantworteten Feld nicht zu trennen (gemessen 2026-08-27: Kinder
  // eingetragen, dann „keine Kinder" geantwortet -> `kind_vorname` bleibt bestaetigt im Stand,
  // faellt aus /fragen, und der Endpunkt antwortet 200 mit voller Frage). Die Oberflaeche legte dem
  // Nutzer dann die Frage nach dem Vornamen seines Kindes vor, kurz nachdem er gesagt hatte, er
  // habe keine.
  //
  // NUR `ausgeschlossen` SPERRT, und das ist keine Vorsicht, sondern gemessen: `relevanz` kennt
  // DREI Werte, und im selben Fall standen 39 Regeln auf `ausgeschlossen`, 24 auf `relevant` und
  // 13 auf `unentschieden` — darunter die Kinderfreibetraege selbst, solange noch Gates offen
  // waren. Eine Sperre auf `!== "relevant"` haette also den Normalfall dieses Feldes mitgesperrt
  // und die Korrektur genau dort verhindert, wo sie gerade erst moeglich wurde.
  //
  // Fehlt die Relevanz ganz, wird NICHT gesperrt. Alle 89 Fragen tragen heute eine regel_id, die
  // in /stand steht (gemessen) — faellt das eines Tages auseinander, ist die Korrektur zu erlauben
  // die harmlose Richtung: sie schreibt einen Wert, den der Nutzer sieht. Sie faelschlich zu
  // sperren ist der Fehler, der hier gerade behoben wurde.
  const rel = (STAND.relevanz || {})[frage.regel_id];
  if (rel && rel.status === "ausgeschlossen") {
    zeigeNetzFehler("Diese Frage ist durch eine andere Antwort entfallen und lässt sich nicht mehr "
                    + "ändern. Willst du sie zurückholen, ändere die Antwort, die sie abgeschaltet "
                    + "hat.");
    return false;
  }

  KORREKTUR_FID = frage.feld_id;   // die Basis, wie der Endpunkt sie aufgeloest hat
  AKTUELL = frage;  // Jetzt ist dieses Feld die "aktuelle" Frage
  zeigeFrage(AKTUELL, STAND);
  return true;
}

// --- Bestätigen: Zwei-Signal über den EINZIGEN Schreibpfad. kiFeld gesetzt -> ersetzt das vorläufige KI-Event. ---
async function bestaetigen(kiFeld) {
  if (!AKTUELL) return;
  const btn = $("bestaetigen");
  if (btn.disabled) return;   // Doppel-Submit-Schutz
  kiFokus(false);   // der Nutzer arbeitet im Fragebogen — der Chat ist wieder Begleiter
  btn.disabled = true;
  const altLabel = kiFeld ? null : btn.textContent;
  if (!kiFeld) btn.textContent = "Wird gespeichert …";

  // Feld mit Instanz-Achse: N Werte, N Events (`base`, `base__2`, …). Eigener Zweig, weil die
  // Fehlerbehandlung eine andere ist — bei fünf Kindern soll nicht die ganze Antwort verworfen
  // werden, weil eines leer blieb.
  if (AKTUELL.instanz_anzahl > 1) {
    const ok = await bestaetigeInstanzen(kiFeld, btn, altLabel);
    if (!ok) return;
    await weiterNachDemSchreiben();
    return;
  }

  const wert = leseWert(AKTUELL);
  if (wert === undefined) {
    // Stille-Null-Fix (Befund B): leer/ungültig -> KEIN Event, kein "0" -- Nutzer wird informiert
    // statt dass eine falsche Zahl bestätigt in den Store wandert (derselbe Fehler-Anzeige-Stil
    // wie jede andere Ablehnung in dieser Funktion, s. unten).
    zeigeNetzFehler(formatHinweis(AKTUELL) || "Bitte einen gültigen Wert eingeben.");
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
  await weiterNachDemSchreiben();
}

// Was nach einem erfolgreichen Schreiben immer passiert — einmal statt zweimal, seit der
// Instanz-Zweig daneben steht.
async function weiterNachDemSchreiben() {
  $("bestaetigen").disabled = false;   // refresh() klont den Knopf (rüsteBestaetigen); das
                                       // Attribut wäre sonst dauerhaft übernommen
  // Kam die Korrektur aus der Bestätigungsliste, geht es dorthin zurück. Verglichen wird das
  // GESCHRIEBENE Feld, nicht bloss „es lief eine Korrektur": KORREKTUR_FID bleibt auch stehen,
  // wenn der Nutzer die vorgelegte Frage überspringt und eine andere beantwortet — dann gehört er
  // nicht in die Liste zurückgeworfen. AKTUELL zeigt hier noch auf das eben geschriebene Feld;
  // refresh() darunter setzt es weiter.
  //
  // Verglichen wird gegen das BASISFELD, weil korrigiereBestaetigt seit 2026-08-27 aufloest: in
  // AKTUELL liegt danach das Basisfeld, in VERSTANDEN_ZURUECK die Kennung, mit der der Aufrufer
  // kam. Das sind seither zwei verschiedene Raeume, und dieser Vergleich stellt sie wieder in
  // denselben — es ist die Naht meiner eigenen Aenderung, nicht Vorsorge fuer einen fremden Fall.
  //
  // HEUTE NICHT AUSLOESBAR, und das gehoert dazugesagt: VERSTANDEN_ZURUECK wird nur von
  // verstandenAendern gesetzt, die Pruefliste zeigt nur vorlaeufige KI-Vorschlaege, und die KI darf
  // kein `__n` schreiben — der Store weist es ab („fail-closed (Katalog): llm:chat darf
  // kind_vorname__2 nicht vorschlagen", gemessen 2026-08-27). Fuer jedes Feld ohne Achse gibt
  // basisFeldId die Kennung unveraendert zurueck, die Zeile verhaelt sich also wie vorher. Sollte
  // die Pruefliste eines Tages Instanzen fuehren, bricht der Rueckweg hier nicht still.
  const zurueck = (VERSTANDEN_ZURUECK && AKTUELL
                   && basisFeldId(VERSTANDEN_ZURUECK.feld_id).basis === AKTUELL.feld_id)
                  ? VERSTANDEN_ZURUECK : null;
  KORREKTUR_FID = null;                // Korrektur abgeschlossen
  await refresh();
  if (zurueck) {
    VERSTANDEN_ZURUECK = null;
    verstandenZurueck(zurueck);
  }
}

// N Instanzen schreiben: `base`, `base__2`, `base__3` … DER EINE SCHREIBWEG FÜR DIE ACHSE, für den
// Fragebogen wie für den Rückfragen-Schritt.
//
// Dass er beiden gehört, ist seit 2026-08-27 keine Vorsorge mehr, sondern der behobene Befund.
// Julius' Mitschnitt vom selben Tag, zwei Zeilen untereinander:
//
//     10:12:19  rueckfrage@kind_vorname   kind_vorname = "Anna"    (Frage im PLURAL, er hat zwei)
//     10:19:17  klick@kind_geburtsdatum   kind_geburtsdatum + kind_geburtsdatum__2
//
// Der Fragebogen bediente die Achse, der Rückfragen-Schritt nicht — er schrieb genau EIN Ereignis
// für `q.feld_id`. Danach galt das Feld als beantwortet und kam nie wieder: das zweite Kind hatte
// keinen Vornamen mehr, und niemand fragte je danach. Die Anzeige war dabei nicht das Problem
// (baueEingabe verzweigt selbst auf die Achse und baute die N Felder auch in der Rückfrage) — der
// Wert wurde nur nie gelesen: `leseWert(q, $("rf-input"))` traf den Wrapper-<div> statt eines
// Eingabefelds und lieferte `undefined`. Zwei Felder dastehen und eines schreiben wäre derselbe
// Verlust mit besserer Optik gewesen, deshalb misst der Test den STORE.
//
// KNOPF UND MELDUNG MACHT DER AUFRUFER. Genau darin unterscheiden sich die beiden: der Fragebogen
// setzt die Beschriftung seines Knopfes zurück, der Rückfragen-Schritt verweist auf „Später
// beantworten". Das Feldformat (instanzFeldId), die Stille-Null-Regel und `ersetzt` dagegen dürfen
// nicht zweimal stimmen müssen — die letzte Naht dieser Art hat einen ganzen Vornamen gekostet.
//
// LEERE INSTANZEN WERDEN ÜBERSPRUNGEN, nicht als Fehler behandelt. Wer drei Kinder angegeben hat,
// aber nur zwei Namen zur Hand, soll die zwei speichern können — die dritte Frage bleibt offen und
// kommt im Fragebogen wieder. Nur wenn ALLE leer sind, ist es keine Antwort (`{leer: true}`): dann
// meldet der Aufrufer dasselbe wie beim einzelnen leeren Feld (Stille-Null-Fix).
//
// Rückgabe: {ok: true} · {leer: true} · {fehler, instanz} — bei letzterem bleibt geschrieben, was
// davor durchging, und die Nummer sagt, in welcher Zeile der Nutzer suchen muss.
async function schreibeInstanzen(q, id, signal2) {
  const werte = leseInstanzWerte(q, id);
  const gefuellt = werte.map((w, i) => [i + 1, w]).filter(([, w]) => w !== undefined);
  if (!gefuellt.length) return { leer: true };
  for (const [i, wert] of gefuellt) {
    const fid = instanzFeldId(q.feld_id, i);
    const ev = {
      feld_id: fid, wert, zustand: "bestaetigt",
      herkunft: { herkunft: "laie", pruef_tiefe: "ungeprueft", haftung: "nutzer" },
      schreiber: "ui:laie",
      signal: { signal_1: null, signal_2: signal2 + fid },
    };
    // Trägt die Instanz schon einen Wert (Korrektur, KI-Vorschlag), verlangt Auflage B ein
    // `ersetzt` — sonst weist der Store sie ab und die Antwort wäre verloren.
    if (STAND && STAND.felder && STAND.felder[fid]) {
      const j = (await jget(`/fall/${FALL}/feld/${fid}/warum`)).body.justification || {};
      if (j.event_id) { ev.ersetzt = j.event_id; ev.signal.signal_1 = j.event_id; }
    }
    const r = await jpost(`/fall/${FALL}/event`, ev);
    if (!okStatus(r.status)) return { fehler: r.body.fehler || r.status, instanz: i };
  }
  return { ok: true };
}

// Der Fragebogen-Aufrufer: Knopf freigeben und die Beschriftung zurücksetzen, wenn nichts
// geschrieben wurde. Gibt false zurück, wenn der Fluss nicht weitergehen darf.
async function bestaetigeInstanzen(kiFeld, btn, altLabel) {
  const s = await schreibeInstanzen(AKTUELL, "feld-input", kiFeld ? "hold@" : "klick@");
  if (s.ok) return true;
  zeigeNetzFehler(s.leer
    ? (formatHinweis(AKTUELL)
       || `Bitte mindestens einen Wert eingeben (${AKTUELL.instanz_etikett} 1).`)
    // Abbrechen und SAGEN, welche Instanz — sonst sucht der Nutzer den Fehler in der falschen Zeile.
    : `${AKTUELL.instanz_etikett} ${s.instanz} abgewiesen: ${s.fehler}`);
  btn.disabled = false;
  if (!kiFeld) btn.textContent = altLabel;
  return false;
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
  aendern.addEventListener("click", () => verstandenAendern(v, aendern));
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
  // Der Knopf muss sagen, wohin er führt. Folgen noch Rückfragen, ist „Weiter zu den Fragen"
  // falsch — der Nutzer landet dann bei der nächsten Nachfrage, nicht im Fragebogen.
  const folgt = VERSTANDEN_DANACH && VERSTANDEN_DANACH.rueckfragen
                && VERSTANDEN_DANACH.rueckfragen.length;
  $("verstanden-weiter").textContent = folgt ? "Weiter zu den Nachfragen" : "Weiter zu den Fragen";
  VERSTANDEN_OFFEN = true;
  // Auch hier KEIN kiFokus(false) — dieselbe falsche Annahme wie in starteRueckfragen(). Was die
  // KI vorgeschlagen hat, zu bestätigen, ist die letzte Stufe des KI-Wegs, nicht Arbeit im
  // Fragebogen. Der Fokus endet erst, wenn der Nutzer dort wirklich antwortet (bestaetigen()).
  $("wegpunkt").hidden = true;
  $("fertig").hidden = true;
  $("verstanden").hidden = false;
  $("verstanden").focus({ preventScroll: true });   // Screen-Reader: Wechsel des Screens ansagen
  // Wie beim Wizard: das Panel sitzt im Berater, bei langem Verlauf stünde die Liste sonst
  // unterhalb des sichtbaren Bereichs.
  $("verstanden").scrollIntoView({ block: "nearest" });
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

async function verstandenAendern(v, btn) {
  // ERST die Zielfrage laden, DANN die Liste wegräumen. Bis zum 2026-08-27 stand es umgekehrt,
  // und dazwischen liegen ZWEI Netzaufrufe (korrigiereBestaetigt holt /warum und /fragen). In
  // dieser Lücke war die Verstanden-Seite weg und der Fragebogen stand offen — mit der Frage, die
  // vorher aktuell war.
  //
  // GEMESSEN 2026-08-27 mit 350 ms künstlicher Verzögerung je Netzaufruf: nach dem Klick auf
  // „Ändern" bei `ep_arbeitstage` stand `veranlagung` da. Der Nutzer bekommt also für die Dauer
  // einer langsamen Verbindung die FALSCHE Frage vorgelegt — und kann in sie hineinschreiben,
  // bevor sie unter ihm wechselt.
  //
  // In dieser Reihenfolge braucht es auch die Rücknahme nicht mehr (Seite zurückholen, wenn die
  // Frage nicht ladbar war) und den Sonderfall der Anmeldemaske nicht: was nie versteckt wurde,
  // muss nicht zurückgeholt werden.
  // Die Korrektur kommt gleich als gewöhnliches `klick@…` im Fall an und ist dort von jeder
  // anderen Antwort nicht mehr zu unterscheiden. Dass sie aus der Prüfliste kam, steht nur hier.
  meldeFluss("pruefliste_aendern", { feld_id: v.feld_id, war: v.wert });
  if (btn) { btn.disabled = true; btn.textContent = "Wird geladen …"; }
  try {
    if (!await korrigiereBestaetigt(v.feld_id)) return;   // Meldung steht, Liste bleibt stehen
  } finally {
    if (btn) { btn.disabled = false; btn.textContent = "Ändern"; }
  }
  // GEMERKT, WOHER: ohne das endete EIN „Ändern" den ganzen Bestätigungsbogen. Julius im Live-Lauf
  // 2026-08-27: „habe bei einer nachfrage anstatt ‚Stimmt‘ ‚Ändern‘ geklickt, das hat den ganzen
  // bestaetigungsbogen beendet."
  //
  // Verloren war dabei nichts — die übrigen Zeilen bleiben vorläufig und stehen weiter im
  // Fragefluss. Weg war die ARBEIT: der Nutzer war dabei, eine Liste durchzugehen, und traf ihre
  // Reste danach einzeln und ohne Zusammenhang wieder. Genau davor warnt der Kopf von
  // tests/test_ui_verstanden_seite.py seit jeher unter Punkt 3 — für „Stimmt" ist es durch
  // VERSTANDEN_OFFEN abgesichert, und „Ändern" schaltete diese Sperre ab, ohne sie je
  // zurückzunehmen.
  VERSTANDEN_ZURUECK = { feld_id: v.feld_id, li: btn ? btn.closest(".v-zeile") : null };
  VERSTANDEN_OFFEN = false;
  $("verstanden").hidden = true;
}

// Zurück in die Bestätigungsliste, nachdem die Korrektur im Fragebogen geschrieben wurde. Die
// Liste steht noch im DOM — mit allen Häkchen, die vorher gesetzt wurden; sie muss nur wieder
// sichtbar werden. Die korrigierte Zeile wird dabei als erledigt markiert: der Wert, über den sie
// spricht, ist jetzt bestätigt, und ein „Stimmt" daneben wäre eine Frage nach etwas Entschiedenem.
function verstandenZurueck(z) {
  if (z.li) {
    z.li.classList.add("v-fertig");
    const ok = z.li.querySelector(".v-ok");
    if (ok) ok.remove();
    const ae = z.li.querySelector(".v-aendern");
    if (ae) { ae.textContent = "✓ geändert"; ae.disabled = true; }
  }
  // Ist nichts mehr offen, wäre die Liste eine leere Aufforderung — dann bleibt der Fragebogen
  // stehen, den refresh() gerade vorgelegt hat.
  if (!$("verstanden-liste").querySelectorAll(".v-zeile:not(.v-fertig)").length) return;
  VERSTANDEN_OFFEN = true;
  $("wegpunkt").hidden = true;
  $("fertig").hidden = true;
  $("verstanden").hidden = false;
  $("verstanden").focus({ preventScroll: true });
  $("verstanden").scrollIntoView({ block: "nearest" });
}

async function verstandenWeiter() {
  // Nicht bestätigte Zeilen bleiben vorläufig und stehen damit weiter im Fragefluss — dort
  // begegnen sie dem Nutzer erneut, mit Hold-to-confirm. Nichts geht verloren, nichts zählt
  // ungefragt. Welche das waren, steht danach nirgends: im Fall liegt nur ein vorläufiges
  // KI-Event, und das lag dort auch schon, bevor die Liste überhaupt erschien.
  meldeFluss("pruefliste_weiter", {
    unbestaetigt: [...document.querySelectorAll("#verstanden-liste .v-zeile:not(.v-fertig)")]
                    .map(li => li.dataset.feld),
  });
  VERSTANDEN_OFFEN = false;
  $("verstanden").hidden = true;

  // Stehen noch Rückfragen an, sind SIE der nächste Schritt — nicht der Fragebogen (Reihenfolge
  // seit 2026-08-25: erst bestätigen, dann nachfragen). starteRueckfragen() prüft dabei gegen die
  // JETZT offenen Fragen: was die eben bestätigten Werte abgeschaltet haben, wird gar nicht mehr
  // gefragt.
  const n = VERSTANDEN_DANACH;
  VERSTANDEN_DANACH = null;
  if (n && n.rueckfragen && n.rueckfragen.length) {
    await starteRueckfragen(n.rueckfragen, [], [], n.zurueckgestellt);
    return;
  }

  // Sonst ist die Kette zu Ende: „Weiter zu den Fragen" ist der Moment, in dem der Nutzer den
  // KI-Weg verlässt. Ohne kiFokus(false) bliebe der Fragebogen ausgeblendet (auf diesem Weg ist
  // NUR das Panel da) und der Knopf führte ins Leere.
  kiFokus(false);
  chatVerlaufAufraeumen();
  if (await zeigeScreening()) return;   // der Fragebogen beginnt mit der Ankreuzliste
  await refresh();
}

// Derselbe Übergang, nur jederzeit erreichbar: der Nutzer muss nicht erst Vorschläge bestätigen,
// um in den Fragebogen zu kommen.
async function zumFragebogen() {
  rueckfragenSchliessen();
  VERSTANDEN_OFFEN = false;
  VERSTANDEN_DANACH = null;
  $("verstanden").hidden = true;
  kiFokus(false);
  chatVerlaufAufraeumen();
  // Der Fragebogen beginnt mit der Ankreuzliste — auch wenn man über den KI-Weg dorthin kommt.
  // Was die KI schon geklärt hat, steht nicht mehr drin (zeigeScreening liest die OFFENEN Fragen).
  if (await zeigeScreening()) return;
  await refresh();
}

// Was die KI aus einem Satz gelesen hat, ist erledigt, sobald der Nutzer im Fragebogen steht: die
// Vorschläge sind bestätigt oder verworfen, die Rückfragen beantwortet oder verschoben.
//
// Julius 2026-08-25, mit Screenshot: „hier sind wir weiter zum fragebogen geführt worden (korrekt)
// aber die KI spalte zeigt immernoch die alten infos an (jetzt nicht mehr relevant). hier sollte
// einfach nur das inputfeld sein für nachfragen und weitere textangaben."
//
// Weg müssen die Aussagen-Kästen und die Ablaufmeldungen dazu („Dazu 4 Vorschläge …"). Der eigene
// Satz („Du: …") und die ANTWORTEN der KI bleiben: das eine ist, was der Nutzer gesagt hat, das
// andere eine Auskunft, die auch später noch gilt.
function chatVerlaufAufraeumen() {
  for (const e of $("chat-body").querySelectorAll(".chat-aussagen, .chat-erklaer")) e.remove();
}

// --- Der Rückfragen-Schritt: eine Frage, ein Feld, „Weiter" ----------------------------------
//
// DIE ANTWORT GEHT NICHT AN DIE KI ZURÜCK. Die Rückfrage nennt ihr `feld_id` — die Antwort gehört
// direkt dorthin, über denselben /event-Pfad wie jede Fragebogen-Antwort (schreiber "ui:laie",
// zustand "bestaetigt", Klick als signal_2). Der Rückweg über das Modell kostete drei Stufen und
// könnte die Angabe erneut falsch deuten: genau das, was die Rückfrage verhindern soll.
//
// Der TYP kommt aus /fragen, nicht aus der Rückfrage — die trägt nur {frage, feld_id, aussage}.
// Nennt die Rückfrage gar kein Feld, gibt es keine Bauart für ein Eingabefeld; dann bleibt der Chat
// der Weg.
//
// NENNT SIE EIN FELD, DAS NICHT MEHR OFFEN IST, FÄLLT SIE WEG (2026-08-25). Das ist der Gewinn der
// umgekehrten Reihenfolge, und Julius' Beispiel dafür ist genau dieser Fall: „wenn ich hier angebe
// dass ich nicht mit dem auto gefahren bin erübrigt sich die nachfrage an wievielen tagen das war."
// Nach der Bestätigung „kein eigenes Auto" steht `ep_arbeitstage` nicht mehr in /fragen — die
// Rückfrage danach ist gegenstandslos. Vorher landete sie im Chat („Im Berater beantworten"), und
// der Nutzer beantwortete eine Frage, die es nicht mehr gab.
async function starteRueckfragen(rueckfragen, vorschlaege, konflikte, zurueckgestellt) {
  // Die Typen frisch holen: die eben bestätigten Werte haben die offene Fragenliste verändert.
  const fr = await jget(`/fall/${FALL}/fragen`);
  if (fr.status === 401) return;   // Anmeldemaske hat übernommen — keine Seite darüberlegen
  const katalog = {};
  for (const q of (Array.isArray(fr.body.fragen) ? fr.body.fragen : [])) katalog[q.feld_id] = q;
  const entfallen = rueckfragen.filter(rf => rf.feld_id && !katalog[rf.feld_id]);
  // Was hier WEGFÄLLT, hinterlässt im Fall nichts — und sieht danach genau so aus wie eine, die
  // der Nutzer übersprungen hat. Nur an dieser Stelle sind die beiden noch zu unterscheiden.
  meldeFluss("nachfragen_gestartet", {
    gestellt: rueckfragen.filter(rf => !rf.feld_id || katalog[rf.feld_id])
                         .map(rf => ({ feld_id: rf.feld_id, frage: (rf.frage || "").slice(0, 120) })),
    entfallen: entfallen.map(rf => rf.feld_id),
    zurueckgestellt: zurueckgestellt || 0,
  });
  RF_LISTE = rueckfragen
    .filter(rf => !rf.feld_id || katalog[rf.feld_id])
    .map(rf => ({ ...rf, meta: rf.feld_id ? katalog[rf.feld_id] : null }));
  RF_INDEX = 0;
  RF_NACHHER = { vorschlaege: vorschlaege || [], konflikte: konflikte || [] };

  // Was weggefallen ist, SAGEN — sonst wirkt es, als hätte die KI die Frage vergessen. Und die
  // gebündelten erst hier: vorher stünde eine Zahl da, die sich gleich noch ändert.
  const body = $("chat-body");
  if (entfallen.length) {
    body.appendChild(beraterZeile("chat-erklaer",
      entfallen.length === 1
        ? "Eine Nachfrage hat sich durch deine Bestätigung erledigt."
        : entfallen.length + " Nachfragen haben sich durch deine Bestätigungen erledigt."));
  }
  if (zurueckgestellt) {
    body.appendChild(beraterZeile("chat-erklaer",
      zurueckgestellt === 1
        ? "Eine weitere Nachfrage habe ich in den Fragebogen verschoben — hier stelle ich je "
          + "Angabe nur die eine, die wirklich offen ist."
        : zurueckgestellt + " weitere Nachfragen habe ich in den Fragebogen verschoben — hier "
          + "stelle ich je Angabe nur die eine, die wirklich offen ist."));
  }
  body.scrollTop = body.scrollHeight;
  RUECKFRAGEN_OFFEN = true;
  VERSTANDEN_OFFEN = false;
  // KEIN kiFokus(false) hier. Das stand bis 2026-08-24 da, mit der Begründung „die Aufgabe steht
  // jetzt auf der Rückfragen-Seite" — und war falsch: Rückfragen sind derselbe KI-Weg. Der Nutzer
  // hat „erst von der KI ausfüllen lassen" gewählt und wartet auf die KI; der Fragebogen ist in
  // diesem Moment Kulisse. Julius: „der user wartet auf die rückfragen bzw zustimmungen."
  // Gemessene Wirkung des alten Standes: der Spaltentausch war genau im wichtigsten Moment wieder
  // aus — Fragebogen breit, KI schmal.
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
  // preventScroll oben verhindert den Sprung des DOKUMENTS; hier wird nur das Panel selbst in
  // seinen sichtbaren Bereich geholt — bei langem Verlauf stünde die Frage sonst unterhalb.
  $("rueckfragen").scrollIntoView({ block: "nearest" });
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

  // Feld mit Instanz-Achse: N Eingabefelder stehen da, also müssen N Ereignisse geschrieben werden.
  // Dieser Zweig fehlte bis 2026-08-27 — der Fragebogen hatte ihn, der Rückfragen-Schritt nicht,
  // und unter „Wie heißen deine KINDER mit Vornamen?" kam genau ein Vorname an. Näheres bei
  // schreibeInstanzen(), dem gemeinsamen Schreibweg beider Wege.
  if (q.instanz_anzahl > 1) {
    btn.disabled = true;
    const s = await schreibeInstanzen(q, "rf-input", "rueckfrage@");
    btn.disabled = false;
    if (!s.ok) {
      // „Später beantworten“ mitnennen: es ist hier der einzige Weg an einer Frage vorbei, und
      // ohne den Hinweis säße der Nutzer vor einem Knopf, der nichts tut.
      zeigeNetzFehler(s.leer
        ? ((formatHinweis(q) || `Bitte mindestens einen Wert eingeben (${q.instanz_etikett} 1)`)
           + " — oder „Später beantworten“.")
        : `${q.instanz_etikett} ${s.instanz} abgewiesen: ${s.fehler}`);
      return;
    }
  } else {
    // Dieselbe Lesart wie im Fragebogen: cent rechnet Euro in Cent, bool dreht `frage_invertiert`
    // zurück in den Speicherwert. leer/ungültig -> undefined, und dann wird NICHTS geschrieben
    // (Stille-Null-Fix): eine stille 0 wäre hier so falsch wie dort.
    const wert = leseWert(q, $("rf-input"));
    if (wert === undefined) {
      zeigeNetzFehler((formatHinweis(q) || "Bitte einen gültigen Wert eingeben")
                      + " — oder „Später beantworten“.");
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
    // ohne Event antwortet er mit 404, und ein 404 je beantworteter Rückfrage wäre eine
    // Fehlermeldung in der Konsole für den Normalfall. Liegt STAND einmal daneben, weist der Store
    // die Antwort ab und der Nutzer SIEHT es (Banner unten) — kein stiller Verlust.
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
  }
  RF_BEANTWORTET += 1;
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
  // Schreibt bewusst nichts in den Fall (s. den Kommentar darüber) — deshalb ist DAS hier die
  // einzige Stelle, an der ein Übersprungenes überhaupt festgehalten werden kann.
  const rf = RF_LISTE[RF_INDEX];
  meldeFluss("nachfrage_spaeter", { feld_id: rf && rf.feld_id,
                                    frage: ((rf && rf.frage) || "").slice(0, 120) });
  RF_SPAETER += 1;
  RF_INDEX += 1;
  zeigeRueckfrage();
}

// Die Runde ist durch: jetzt erst die Bestätigungen (Teil 3 der Reihenfolge), sonst zurück in den
// Fragebogen. refresh() holt in beiden Fällen Ring und Belegt-Liste nach — und schiebt den
// Fragebogen nur dann vor, wenn keine Verstanden-Seite ihn festhält.
// Die Runde ist durch — und das muss DASTEHEN.
//
// Julius 2026-08-25, mit Screenshot: „nachdem alle nachfragen beantwortet wurden sehen wir das …
// hier sollte eher eine erfolgsnachricht mit zusammenfassung stehen." Der Wizard verschwand, und
// zurück blieb der Verlauf von vorhin — inklusive vier Marken „frage ich gleich" an Aussagen,
// deren Fragen längst beantwortet waren. Nichts sagte, dass etwas fertig ist.
//
// Zwei Dinge passieren hier deshalb: die abgearbeiteten Aussagen-Kästen werden geräumt (ihre
// Marken beschreiben einen Zustand, den es nicht mehr gibt), und an ihre Stelle tritt eine
// Bilanz — was angekommen ist, was zurückgestellt wurde, und wie es weitergeht.
async function rueckfragenFertig() {
  const n = RF_NACHHER || { vorschlaege: [], konflikte: [] };
  const beantwortet = RF_BEANTWORTET, spaeter = RF_SPAETER;
  rueckfragenSchliessen();
  if (n.vorschlaege.length || n.konflikte.length) {
    // Alter Pfad (Bestätigungen NACH den Rückfragen). Seit der Umkehr der Reihenfolge am
    // 2026-08-25 läuft er nicht mehr — RF_NACHHER ist leer, weil zeigeVerstanden vorher kam.
    zeigeVerstanden(n.vorschlaege, n.konflikte);
    return refresh();
  }
  await refresh();
  if (beantwortet || spaeter) zeigeRundenBilanz(beantwortet, spaeter);

  // Und dann der nächste Schritt (Julius 2026-08-25: „die checkboxen kommen nicht nach den
  // nachfragen"). Die KI-Kette ist hier zu Ende: Text gelesen, bestätigt, nachgefragt. Was folgt,
  // ist der Fragebogen — und der beginnt mit der Ankreuzliste. Sie zeigt nur noch die Themen, die
  // offen geblieben sind; was die KI aus dem Satz geklärt hat, steht nicht mehr drin.
  kiFokus(false);
  await zeigeScreening();
}

// Was aus der Runde geworden ist, in Zahlen, die der Nutzer selbst nachzählen kann.
function zeigeRundenBilanz(beantwortet, spaeter) {
  chatVerlaufAufraeumen();      // die Kästen von vorhin sind abgearbeitet
  const body = $("chat-body");

  const teile = [];
  if (beantwortet) {
    teile.push(beantwortet === 1 ? "eine Angabe übernommen"
                                 : beantwortet + " Angaben übernommen");
  }
  if (spaeter) {
    teile.push(spaeter === 1 ? "eine Frage zurückgestellt"
                             : spaeter + " Fragen zurückgestellt");
  }
  const p = beraterZeile("chat-bilanz", "✓ Fertig — " + teile.join(", ") + ".");
  body.appendChild(p);

  // Wie es weitergeht, und wo das Zurückgestellte geblieben ist. Ohne diesen Satz liest sich
  // „zurückgestellt" wie „verworfen".
  const offen = (OFFEN_ANZAHL === null || OFFEN_ANZAHL === undefined) ? null : OFFEN_ANZAHL;
  const wie = spaeter
    ? "Was du zurückgestellt hast, steht im Fragebogen wieder da."
    : "";
  const rest = (offen === null) ? "" : (offen === 0
    ? "Es sind keine Fragen mehr offen."
    : `Im Fragebogen sind noch ${offen} Fragen offen.`);
  const satz = [wie, rest].filter(Boolean).join(" ");
  if (satz) body.appendChild(beraterZeile("chat-erklaer", satz));
  body.scrollTop = body.scrollHeight;
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
  // „Die KI liest mit …" klang nach jemandem, der über die Schulter schaut (Julius 2026-08-25:
  // „auch eine eigenartige formulierung"). Sie liest nicht MIT, sie liest den Satz, den er ihr
  // gerade geschickt hat.
  txt.textContent = "Ich lese deinen Satz …";
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
// „Weiter" dort schreibt ein Event und ruft refresh(), ändert also `AKTUELL`. Seit dem Umzug ins
// Berater-Panel (2026-08-24) ist das keine Randbedingung mehr, sondern der Normalfall: der Wizard
// steht direkt über dem Eingabefeld, aus dem heraus der Aufruf gestartet wird, und bleibt dabei
// vollständig sichtbar. Gesperrt wird das Panel, NICHT der Berater — der Verlauf bleibt lesbar.
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
// ALLE Status, die api_llm setzt — vollständig, nicht nur die drei häufigen. Gemessen im
// Live-Lauf 2026-08-25: bei einem Ausfall von Stufe 2 stand im Chat wörtlich die Marke
// „themen_ausgefallen". Ein unbekannter Status fällt auf `a-unbekannt` und zeigt seinen ROHEN
// Namen — das ist als Notnagel richtig (ein unbekannter Zustand darf nicht wie ein erfolgreicher
// aussehen), aber für die Status, die es WIRKLICH gibt, ist es schlicht eine vergessene Zeile.
const AUSSAGE_MARKE = {
  vorschlag: "als Vorschlag oben",
  // Die Frage steht auf der Rückfragen-Seite, nicht hier. Stünde da bloss „Rückfrage", suchte der
  // Nutzer sie in diesem Kasten — die Marke sagt deshalb, wo sie hinkommt.
  rueckfrage: "frage ich gleich",
  kein_feld: "keinem Feld zugeordnet",
  kein_thema: "kein passendes Thema gefunden",
  ohne_beleg: "nicht belegt — verworfen",
  // Die beiden sagen etwas über UNS, nicht über den Satz des Nutzers.
  themen_ausgefallen: "bei mir ausgefallen",
  werte_ausgefallen: "bei mir ausgefallen",
};

// Genau die Status, bei denen die Ursache auf unserer Seite liegt. Wichtig für die Meldung
// darunter: „schreib es genauer" wäre dann eine Fehldiagnose zu Lasten des Nutzers.
const AUSGEFALLEN = ["themen_ausgefallen", "werte_ausgefallen"];

// Rückfragen OHNE `feld_id` haben kein Eingabefeld, in das sich etwas schreiben ließe — für sie
// bleibt der Chat der Weg. Aufgerufen wird das aus rueckfrageWeiter() („Im Berater beantworten"),
// nicht mehr aus dem Verlauf: siehe den Block über aussagenBlock().
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

function aussagenZeile(a) {
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
  return li;
}

// NUR die Aussagen — die Rückfragen stehen auf der Rückfragen-SEITE, und zwar dort allein.
//
// ANLASS, wörtlich (Julius, 2026-08-24, aus dem echten Durchgang): „gleiche rückfrage links und
// rechts. das ist quatsch. wir wollen einen dialog fenster dass der nutzer durchklickt."
// Bis hierher zeichnete dieser Block dieselben Fragen ein zweites Mal in den Verlauf, mit einem
// eigenen „Antworten"-Knopf. Der Nutzer sah zwei Aufforderungen und musste raten, welche gilt —
// und die beiden Wege schrieben verschieden: die Seite direkt ins Feld (/event), der Knopf schickte
// die Antwort noch einmal durch alle drei Modellstufen, die sie erneut falsch deuten konnten.
// Genau das sollte die Rückfrage verhindern.
//
// Die Aussagen bleiben: sie sind Information über den eigenen Satz, keine Aufforderung. Ihre Marke
// sagt, dass gleich nachgefragt wird — die Frage selbst kommt auf der Seite.
function aussagenBlock(aussagen) {
  const box = document.createElement("div");
  box.className = "chat-aussagen";
  const titel = document.createElement("div");
  titel.className = "chat-aussagen-titel";
  titel.textContent = "Das habe ich aus deinem Satz gelesen:";
  box.appendChild(titel);
  const ul = document.createElement("ul");
  ul.className = "a-liste";
  for (const a of aussagen) ul.appendChild(aussagenZeile(a));
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
  // „Der Knopf ist wieder frei" heisst NICHT „der Vorgang ist durch". Die Sperre fällt bewusst
  // früh (s. das finally weiter unten und die Begründung dahinter), und erst danach entscheidet
  // sich, ob die Verstanden- oder die Rückfragen-Seite kommt — die Rückfragen-Seite holt dafür
  // sogar noch /fragen. Zwischen beidem liegt also ein Zustand, den nichts nach aussen anzeigte.
  //
  // GEMESSEN 2026-08-27, 1 von 6 Läufen: die Rückfragen-Seite stand noch nicht, als der Knopf
  // schon wieder frei war. Genau daran hingen zwei Tests, die unter Last zufällig rot wurden —
  // und, leiser, mindestens einer, der zu früh mass und deshalb GRÜN war.
  //
  // Diese Marke ist deshalb kein Test-Anhängsel, sondern die fehlende Aussage: läuft gerade eine
  // KI-Runde zu Ende? Sie steht am <body>, ist per CSS erreichbar (die Seite verbietet
  // script-src eval, ein Ausdruck im Test ginge nicht) und wird im finally IMMER abgeräumt.
  document.body.dataset.chatLaeuft = "1";
  try {
    await chatSendenLauf();
  } finally {
    delete document.body.dataset.chatLaeuft;
  }
}

async function chatSendenLauf() {
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
  // SOFORT leeren, im selben Augenblick, in dem der Satz in den Verlauf wandert. Julius 2026-08-25:
  // „hier steht meine eingabe 2 mal … das ist quatsch." Vorher stand `t.value = ""` hinter dem
  // `await` — während des ganzen Modellaufrufs (Sekunden bis Minuten) stand der Satz doppelt da:
  // einmal als „Du: …" im Verlauf, einmal noch im Feld.
  // Bei einem Fehlschlag kommt er zurück (s. unten): der Satz ist Nutzerarbeit und darf nicht
  // verloren gehen, nur weil der Aufruf nicht durchkam.
  if (t) { t.value = ""; chatHoeheAnpassen(); }
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
  // Der Satz ist Nutzerarbeit. Kam der Aufruf nicht durch, gehört er zurück ins Feld, statt dass
  // der Nutzer ihn neu tippt — 501 ist KEIN Fehlschlag in diesem Sinn (die Erklär-Grenze ist eine
  // Antwort), 200 ohnehin nicht.
  if (t && r.status !== 200 && r.status !== 501 && !t.value.trim()) {
    t.value = freitext;
    chatHoeheAnpassen();
  }
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
    if (aussagen.length) body.appendChild(aussagenBlock(aussagen));
    // Steht ausserhalb des Aussagen-Kastens: die Meldung gehört zu den Rückfragen, und die kommen
    // auch ohne Aussagen vor (dann wäre der Kasten leer und die Meldung mit ihm verschwunden).
    if (zurueckgehalten) {
      body.appendChild(beraterZeile("chat-unsicher",
        zurueckgehalten === 1
          ? "Zu einem Feld kamen gleichzeitig eine Rückfrage und ein fertiger Wert — der Wert "
            + "wird zurückgehalten, bis die Rückfrage beantwortet ist."
          : zurueckgehalten + " Werte werden zurückgehalten, weil zu denselben Feldern noch "
            + "eine Rückfrage offen ist."));
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
    // DIE REIHENFOLGE — UMGEDREHT AM 2026-08-25. Hier stand seit dem 2026-08-23: erst die
    // Rückfragen, dann die Bestätigungen. Julius aus dem echten Durchgang:
    //
    //   „diese bestätigungen kamen nach den nachfragen dazu. das sollte andersrum sein. wenn ich
    //    hier angebe dass ich nicht mit dem auto gefahren bin erübrigt sich die nachfrage an
    //    wievielen tagen das war."
    //
    // Das ist zwingend, nicht Geschmack: eine Bestätigung kann einen ganzen Frageblock
    // ABSCHALTEN. „Kein eigenes Auto" nimmt der Frage nach den Arbeitstagen die Grundlage — sie
    // steht danach nicht mehr in /fragen. In der alten Reihenfolge hat der Nutzer sie trotzdem
    // beantwortet, und zwar bevor irgendetwas entschieden war.
    //
    // Die Regel „nie zwei Aufforderungen gleichzeitig" bleibt: es ist weiter eine Kette, nur
    // andersherum. Und starteRueckfragen() wirft danach jede Rückfrage weg, deren Feld nicht mehr
    // offen ist — genau das ist der Gewinn dieser Umkehrung.
    const zurueckgestellt = Number(r.body.rueckfragen_zurueckgestellt) || 0;
    if (vorschlaege.length || konflikte.length) {
      body.appendChild(beraterZeile("chat-erklaer",
        "Dazu " + teile.join(" und ") + (rueckfragen.length
          ? " — bitte erst bestätigen, danach frage ich das Offene nach." : " — oben.")));
      // Die zurückgestellten NACH den Bestätigungen melden hiesse: der Nutzer liest von Fragen,
      // die er vielleicht nie sieht (weil sie entfallen). Deshalb erst, wenn feststeht, welche
      // Rückfragen wirklich kommen — s. starteRueckfragen().
      RF_NACHHER = null;
      VERSTANDEN_DANACH = { rueckfragen, vorschlaege, konflikte, zurueckgestellt };
      zeigeVerstanden(vorschlaege, konflikte);
      await refresh();   // Ring/Belegt aktualisieren; zeigeVerstanden hält die Seite vorn
    } else if (rueckfragen.length) {
      await starteRueckfragen(rueckfragen, [], [], zurueckgestellt);
    } else {
      if (!r.body.antwort) {
        // Hier ist `rueckfragen` bereits leer (sonst liefe der Zweig oben): steht eine Rückfrage
        // auf dem Schirm, hat die KI sehr wohl etwas erkannt — sie fragt ja nach. „Weder einen
        // Wert noch eine Frage" wäre dort schlicht falsch und schickte den Nutzer zum
        // Umformulieren, statt zum Antworten.
        //
        // Dieselbe Fehldiagnose in schwerer: fällt eine STUFE des Dialogs aus, liegt die Ursache
        // bei uns. Gemessen 2026-08-25 im Live-Lauf (Stufe 2 nach 188s ausgefallen): der Nutzer
        // las „schreib es etwas genauer", obwohl sein Satz einwandfrei war und die gelesene
        // Aussage direkt darüber stand. Ihn für unseren Ausfall zum Umformulieren zu schicken,
        // ist die unfreundlichste Art, einen Fehler zu verschweigen.
        const ausgefallen = aussagen.some(a => AUSGEFALLEN.includes(a.status));
        body.appendChild(beraterZeile(ausgefallen ? "chat-unsicher" : "chat-erklaer",
          ausgefallen
            ? "Bei mir ist gerade ein Schritt ausgefallen — dein Satz ist angekommen, ich konnte "
              + "ihn nur nicht zu Ende verarbeiten. Schick ihn gleich noch einmal ab."
            : "Daraus konnte die KI weder einen Wert ableiten noch eine Frage erkennen. "
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

// --- Absendeknopf: löst NUR die lokale checkESt-Prüfung aus (POST /fall/{id}/einreichen), sendet
// nichts ans Finanzamt — der echte Versand bleibt CLI-only (elster/versand.py) und ist hier bewusst
// nicht verdrahtet. Server ist fail-closed: der Knopf ist immer klickbar, `vollstaendig` und
// `preflight.status` gaten hier nichts, weil beide Signale ELSTER-Pflichtfelder übersehen können.
//
// Fail-closed in der ANDEREN Richtung als sonst üblich: nicht eine Liste bekannter
// "nicht geprüft"-Gründe pflegen (ein neuer, unbekannter Grund würde sonst lautlos durchrutschen),
// sondern umgekehrt — NUR der ausdrückliche Erfolg (200, kein `grund`-Schlüssel) gilt als
// "in Ordnung"; jeder andere Fall (bekannt oder nicht) landet in "nicht geprüft/nicht bestanden".
// Einzige Ausnahme: `grund === "plausibilitaet_verletzt"` bekommt einen eigenen Text, weil das der
// einzige Fall ist, in dem checkESt wirklich ein Urteil über die Erklärung gefällt hat.
// Kein Ursachentext zu rc-Codes (Instructor 2026-08-30): rc=610301200 heißt laut ERiC-Header
// (eric_fehlercodes.h) ERIC_IO_READER_SCHEMA_VALIDIERUNGSFEHLER — im Repo seit 7c0a725
// RC_IO_SCHEMA_VALIDIERUNGSFEHLER (elster/checkest_gate.py:48), zuvor RC_IO_KEIN_TICKET, ein
// Fehlschluss aus einer einzelnen Fuzz-Probe. Belegt (Commit cebb228): derselbe rc entsteht
// auch aus einem ganz anderen Fehler (Namensraum-Präfix). EIN Code, MINDESTENS zwei unverwandte
// Ursachen — daraus lässt sich keine Ursache ableiten, also steht hier keine.
async function einreichenPruefen() {
  const btn = $("einreichen-btn");
  if (btn.disabled) return;   // Doppel-Submit-Schutz
  btn.disabled = true;
  const status = $("einreichen-status");
  status.hidden = true;
  status.replaceChildren();
  const r = await jpost(`/fall/${FALL}/einreichen`, {});
  const kopf = document.createElement("p");
  kopf.className = "einreichen-kopf";
  const detail = document.createElement("p");
  detail.className = "einreichen-detail";
  if (r.status === 200 && r.body && !("grund" in r.body)) {
    kopf.textContent = "Geprüft und in Ordnung.";
    detail.textContent = r.body.hinweis || "";
  } else if (r.body && r.body.grund === "plausibilitaet_verletzt") {
    kopf.textContent = "Geprüft und beanstandet.";
    detail.textContent = "Die Prüfung hat Einwände gegen die Erklärung gefunden. Bitte Angaben " +
                          "prüfen, bevor erneut eingereicht wird.";
  } else {
    kopf.textContent = "Nicht geprüft.";
    detail.textContent = "Aus der Prüfung liegt kein Ergebnis vor. Der Fall gilt als offen.";
  }
  status.replaceChildren(kopf, detail);
  status.hidden = false;
  btn.disabled = false;
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
  VERSTANDEN_ZURUECK = null;   // sonst zöge die Liste des ABGEMELDETEN Falls den nächsten hinein
  OFFEN_ANZAHL = null; GESAMT_VOR = null;
  btn.disabled = false;
  aktualisiereKontoLeiste();
  zeigeAnmeldemaske();
}

// Start-Sequenz: entscheidet, ob die Login-Maske oder der Start-Screen zuerst erscheint — OHNE
// vorher einen Fall anzulegen (POST /fall prüft nie den Besitz, ein so entstandener herrenloser
// Fall wäre nach einem Login für niemanden mehr lesbar, s. api.py _fall_owner_check).
async function initAuth() {
  // Läuft der Fluss-Mitschnitt? Einmal, öffentlich, ohne Fall — ohne diese Frage müsste die Seite
  // ihre Bildschirme immer melden, auch wenn niemand mitschreibt, und das sind auf einem
  // einfädigen Server Anfragen, die echte blockieren.
  jget("/health").then(r => { FLOW_AN = !!(r.body && r.body.flow); }).catch(() => {});
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
$("zum-fragebogen").addEventListener("click", zumFragebogen);
$("screening-weiter").addEventListener("click", screeningWeiter);
$("kette-zu").addEventListener("click", () => $("kette-overlay").hidden = true);
$("vorjahr-toggle").addEventListener("click", () => { const p = $("vorjahr-panel"); p.hidden = !p.hidden; });
$("vorjahr-go").addEventListener("click", vorjahrUebernehmen);
$("einreichen-btn").addEventListener("click", einreichenPruefen);
$("konto-toggle").addEventListener("click", () => { const p = $("konto-panel"); p.hidden = !p.hidden; });
$("konto-file").addEventListener("change", (e) => kontoauszugHochladen(e.target.files[0]));
$("login-go").addEventListener("click", loginGo);
$("login-umschalten").addEventListener("click", loginModusUmschalten);
$("logout-btn").addEventListener("click", abmelden);
initAuth();
