"use strict";
// TaxGraph-Haut — Mobile-Wegpunkt-Fluss (Vanilla-JS, kein Build). Redet NUR über die HTTP-API;
// keine Steuerlogik im Frontend, jede Zahl kommt vom Server. KI erklärt, setzt nie Werte (POST /chat -> 501).

let FALL = null;
let AKTUELL = null;   // aktuelle Frage (aus /fragen)
let STAND = null;     // letzter /stand
let SPANNE0 = null;   // Referenz-Spanne (für den Schrumpf-Anteil)

async function jget(url) { const r = await fetch(url); return { status: r.status, body: await r.json() }; }
async function jpost(url, obj) {
  const r = await fetch(url, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(obj || {}) });
  return { status: r.status, body: await r.json() };
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
  const fid = "demo-" + Date.now();
  const a = await jpost("/fall", { scheibe, veranlagungszeitraum: 2025, fall_id: fid });
  if (a.status >= 400) { alert("Konnte nicht starten: " + (a.body.fehler || a.status)); return; }
  FALL = a.body.fall_id;
  SPANNE0 = null;
  $("start").hidden = true;
  $("flow").hidden = false;
  await refresh();
}

async function refresh() {
  STAND = (await jget(`/fall/${FALL}/stand`)).body;
  zeigeRing(STAND);
  zeigeBelegt(STAND.felder);
  const fragen = (await jget(`/fall/${FALL}/fragen`)).body.fragen;
  if (fragen.length === 0) { $("wegpunkt").hidden = true; await zeigeErgebnis(); }
  else { AKTUELL = fragen[0]; zeigeFrage(AKTUELL, STAND); }
}

// --- Dim 4: der Bescheid als schrumpfender Ring ---
function zeigeRing(stand) {
  const spanneEl = $("spanne"), hintEl = $("spanne-hint"), ringEl = $("ring"), mitteEl = $("ring-mitte");
  const felder = Object.values(stand.felder || {});
  const fest = felder.filter(f => f.zustand === "bestaetigt").length;
  const anteil = felder.length ? fest / felder.length : 0;
  ringEl.style.setProperty("--anteil", anteil);
  mitteEl.textContent = felder.length ? `${fest}/${felder.length}` : "";

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
    const li = document.createElement("li"); li.className = "zeile";
    const bi = badgeInfo(f.herkunft_badge);
    const badge = document.createElement("button");
    badge.className = "badge " + bi.kl + (f.zustand === "vorlaeufig" ? " badge-vorlaeufig" : "");
    badge.textContent = bi.sym + " " + bi.lab;
    badge.title = "Herkunft ansehen"; badge.type = "button";
    badge.addEventListener("click", () => herkunftKette(fid, f));
    li.appendChild(badge);
    const t = document.createElement("span"); t.className = "z-name"; t.textContent = fid;
    const v = document.createElement("span"); v.className = "z-wert"; v.textContent = JSON.stringify(f.wert);
    li.appendChild(t); li.appendChild(v); ul.appendChild(li);
  }
}

// --- Dim 1: Herkunft-Kette (Euro -> Regel -> Norm -> Beleg) im Bestätigungsmoment ---
async function herkunftKette(fid, f) {
  const r = await jget(`/fall/${FALL}/feld/${fid}/warum`);
  const j = (r.status === 200 && r.body.justification) ? r.body.justification : {};
  const bi = badgeInfo(f.herkunft_badge);
  $("kette-titel").textContent = `${fid}: ${JSON.stringify(f.wert)}`;
  const body = $("kette-body"); body.innerHTML = "";
  const step = (dot, titel, txt) => {
    const d = document.createElement("div"); d.className = "step";
    d.innerHTML = `<span class="dot">${dot}</span> <b>${titel}</b> ${txt ? "· " + txt : ""}`;
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
    input = document.createElement("select");
    for (const [t, v] of [["Ja", "true"], ["Nein", "false"]]) {
      const o = document.createElement("option"); o.value = v; o.textContent = t;
      // bool-Prefill (dev-2 #4): beispielwert als Vorauswahl statt fix „Ja".
      if (q.beispielwert !== undefined && q.beispielwert !== null && String(o.value) === String(q.beispielwert)) o.selected = true;
      input.appendChild(o);
    }
  } else if (q.typ === "enum") {
    input = document.createElement("select");
    for (const v of (q.enum_werte || [])) {
      const o = document.createElement("option"); o.value = v; o.textContent = v;
      if (String(v) === String(q.beispielwert)) o.selected = true;
      input.appendChild(o);
    }
  } else {
    input = document.createElement("input"); input.type = "number";
    input.inputMode = q.typ === "cent" ? "decimal" : "numeric";
    if (q.bereich) { if ("min" in q.bereich) input.min = q.bereich.min; if ("max" in q.bereich) input.max = q.bereich.max; }
    input.placeholder = q.typ === "cent" ? "Betrag in Euro" : String(q.beispielwert ?? "");
    if (kiVorschlag && vorhanden.wert !== null) input.value = (q.typ === "cent") ? (vorhanden.wert / 100) : vorhanden.wert;
  }
  input.id = "feld-input"; box.appendChild(input);
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

function leseWert(q) {
  const el = $("feld-input");
  if (q.typ === "enum") return el.value;
  if (q.typ === "bool") { const ja = el.value === "true"; return q.feld_id.startsWith("kein_") ? !ja : ja; }
  if (q.typ === "cent") return Math.round(parseFloat(el.value || "0") * 100);
  return parseInt(el.value || "0", 10);
}

// --- Bestätigen: Zwei-Signal über den EINZIGEN Schreibpfad. kiFeld gesetzt -> ersetzt das vorläufige KI-Event. ---
async function bestaetigen(kiFeld) {
  if (!AKTUELL) return;
  const wert = leseWert(AKTUELL);
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
  }
  const r = await jpost(`/fall/${FALL}/event`, ev);
  if (r.status >= 400) { alert("Abgewiesen: " + (r.body.fehler || r.status)); return; }
  await refresh();
}

// --- Dim 5: Chat als Berater daneben — die KI SCHLÄGT VOR (vorläufig), setzt NIE einen Wert.
//     Vorschläge erscheinen im Fluss mit ✦-Badge + Hold-Confirm (Zwei-Signal). Kein Key -> 501-Erklär-Grenze. ---
async function oeffneChat() {
  $("chat-body").innerHTML = `<p class="chat-erklaer">Beschreib deine Situation — die KI <b>schlägt Werte vor</b>, du bestätigst jeden selbst (die KI setzt <b>nie</b> einen Wert). <span class="chat-grenze">Du entscheidest.</span></p>`;
  const t = $("chat-text"); if (t) t.value = "";
  $("chat-overlay").hidden = false;
}

async function chatSenden() {
  const t = $("chat-text"); const freitext = (t && t.value || "").trim();
  if (!freitext) return;
  const body = $("chat-body");
  body.innerHTML = `<p class="chat-erklaer">Die KI liest deine Beschreibung …</p>`;
  const r = await jpost(`/fall/${FALL}/chat`, { text: freitext });
  if (r.status === 501) {
    body.innerHTML = `<p class="chat-erklaer">Die KI erklärt und verlinkt Paragraph &amp; Beleg — aber sie setzt <b>nie</b> selbst einen Wert. <span class="chat-grenze">Du entscheidest.</span></p>`
      + `<p class="chat-vertrag">${(r.body && r.body.vertrag) ? r.body.vertrag : "Der KI-Kanal ist noch nicht verbunden."}</p>`;
  } else if (r.status === 200 && r.body) {
    const n = (r.body.vorschlaege || []).length;
    if (n) {
      body.innerHTML = `<p class="chat-erklaer">Die KI hat <b>${n} Vorschlag${n === 1 ? "" : "e"}</b> gemacht — sie erscheinen im Fluss mit dem <span class="chat-grenze">✦-Abzeichen</span>. Bitte bestätige jeden selbst (halten zum Bestätigen).</p>`;
      await refresh();   // die vorläufigen KI-Vorschläge im Fluss zeigen (Hold-Confirm = Zwei-Signal)
    } else {
      body.innerHTML = `<p class="chat-erklaer">Die KI konnte daraus keinen konkreten Feld-Wert vorschlagen. Beschreib es genauer oder trag den Wert direkt ein.</p>`;
    }
  } else {
    body.textContent = "Der KI-Kanal antwortete unerwartet (" + r.status + ").";
  }
}

async function zeigeWarum() {
  if (!AKTUELL) return;
  const r = await jget(`/fall/${FALL}/feld/${AKTUELL.feld_id}/warum`);
  const el = $("anker");
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
  const el = $("ergebnis");
  if (r.zahl_cent === null) {
    el.className = "ergebnis ergebnis-guard";
    if (r.grund in GUARD) el.textContent = GUARD[r.grund];
    else if (r.grund === "kein_scheiben_gesamtbescheid") el.textContent = "Alle Angaben erfasst — die Gesamtsteuer wird in einem späteren Schritt berechnet.";
    else if (r.grund === "engine_unavailable") el.textContent = "Alle Angaben bestätigt — die Rechen-Engine ist hier nicht verfügbar.";
    else el.textContent = "Noch offen: " + (r.offen || []).join(", ");
  } else {
    el.className = "ergebnis";
    el.innerHTML = `<span class="erg-zahl">${euro(r.zahl_cent)}</span><span class="erg-label">festzusetzende Einkommensteuer</span>`;
  }
}

// --- Vorjahr-Übernahme: Vorjahres-Fall → vorläufige Vorschläge (herkunft=vorjahr), Nutzer bestätigt ---
async function vorjahrUebernehmen() {
  const vf = $("vorjahr-fid").value.trim(), st = $("vorjahr-status");
  if (!vf) { st.textContent = "Bitte die Vorjahres-Fall-ID angeben."; return; }
  st.textContent = "Übernehme …";
  const r = await jpost(`/fall/${FALL}/vorjahr`, { vorjahr_fall_id: vf });
  if (r.status === 200) {
    st.textContent = `${r.body.uebernommen} Feld(er) aus dem Vorjahr als Vorschlag übernommen — bitte im Fluss bestätigen.`;
    await refresh();
  } else {
    st.textContent = "Übernahme fehlgeschlagen: " + ((r.body && r.body.fehler) || r.status);
  }
}

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

// --- Verdrahtung ---
document.querySelectorAll(".kachel").forEach(k => k.addEventListener("click", () => waehleScheibe(k.dataset.scheibe)));
$("chat").addEventListener("click", oeffneChat);
$("chat-send").addEventListener("click", chatSenden);
$("warum").addEventListener("click", zeigeWarum);
$("kette-zu").addEventListener("click", () => $("kette-overlay").hidden = true);
$("chat-zu").addEventListener("click", () => $("chat-overlay").hidden = true);
$("vorjahr-toggle").addEventListener("click", () => { const p = $("vorjahr-panel"); p.hidden = !p.hidden; });
$("vorjahr-go").addEventListener("click", vorjahrUebernehmen);
$("konto-toggle").addEventListener("click", () => { const p = $("konto-panel"); p.hidden = !p.hidden; });
$("konto-file").addEventListener("change", (e) => kontoauszugHochladen(e.target.files[0]));
