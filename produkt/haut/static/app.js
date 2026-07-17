"use strict";
// TaxGraph-Haut — Mobile-Wegpunkt-Prototyp (Vanilla-JS, kein Build). Redet nur über die HTTP-API;
// keine Steuerlogik im Frontend, jede Zahl kommt vom Server. Bewusst schlicht (erste Scheibe: EP).

let FALL = null;
let AKTUELL = null;   // aktuelle Frage (aus /fragen)

async function jget(url) {
  const r = await fetch(url);
  return { status: r.status, body: await r.json() };
}
async function jpost(url, obj) {
  const r = await fetch(url, {
    method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify(obj || {}),
  });
  return { status: r.status, body: await r.json() };
}

function euro(cent) {
  if (cent === null || cent === undefined) return "—";
  return (cent / 100).toLocaleString("de-DE", { style: "currency", currency: "EUR" });
}

// --- Fall anlegen + erste Frage ---
async function start() {
  const fid = "demo-" + Date.now();
  const a = await jpost("/fall", { scheibe: "ep", veranlagungszeitraum: 2025, fall_id: fid });
  FALL = a.body.fall_id;
  await refresh();
}

// --- Stand (Spanne, Ring, belegte Felder) + nächste Frage laden ---
async function refresh() {
  const stand = (await jget(`/fall/${FALL}/stand`)).body;
  zeigeSpanne(stand);
  zeigeBelegt(stand.felder);

  const fragen = (await jget(`/fall/${FALL}/fragen`)).body.fragen;
  if (fragen.length === 0) {
    document.getElementById("wegpunkt").hidden = true;
    await zeigeErgebnis();
  } else {
    AKTUELL = fragen[0];
    zeigeFrage(AKTUELL);
  }
}

function spanneText(label, iv) {
  const lo = euro(iv.min_cent), hi = euro(iv.max_cent);
  const off = (iv.min_offen || iv.max_offen) ? " (noch offen)" : "";
  return (iv.min_cent === iv.max_cent) ? `${label}: ${lo}` : `${label} ${lo}–${hi}${off}`;
}

function zeigeSpanne(stand) {
  const el = document.getElementById("spanne");
  if (stand.ring_gesperrt) {
    el.textContent = "Vereinfachter Bescheid hier nicht möglich — siehe Ergebnis unten.";
    document.getElementById("ring").style.setProperty("--anteil", 0);
    return;
  }
  if (stand.intervall) {
    // Gesamt-Bescheid-Ring (Scheibe mit Gesamt-Accessor, z.B. EP allein)
    el.textContent = spanneText("Bescheid", stand.intervall);
  } else if (stand.teil_ringe && stand.teil_ringe.length) {
    // Ehrliche Teil-Ringe: kein Gesamt-Bescheid, nur einzelne Abzugs-Familien
    el.textContent = stand.teil_ringe.map(t => spanneText(t.familie, t.intervall)).join(" · ")
      + " — noch kein Gesamt-Bescheid";
  } else {
    el.textContent = "Bescheid-Spanne: (Rechen-Engine nicht verfügbar)";
  }
  // Ring: Anteil bestätigter Felder (unabhängig vom Bescheid-Status)
  const felder = Object.values(stand.felder || {});
  const fest = felder.filter(f => f.zustand === "bestaetigt").length;
  const anteil = felder.length ? fest / felder.length : 0;
  document.getElementById("ring").style.setProperty("--anteil", anteil);
}

function zeigeBelegt(felder) {
  const ul = document.getElementById("belegt-liste");
  ul.innerHTML = "";
  for (const [fid, f] of Object.entries(felder || {})) {
    const li = document.createElement("li");
    const badge = document.createElement("span");
    badge.className = "badge badge-" + f.herkunft_badge;
    badge.title = f.herkunft_badge === "schimmernd" ? "KI-Vorschlag" : "Beleg / selbst bestätigt";
    badge.textContent = f.herkunft_badge === "schimmernd" ? "KI" : "✓";
    li.appendChild(badge);
    li.appendChild(document.createTextNode(` ${fid}: ${JSON.stringify(f.wert)} (${f.zustand})`));
    ul.appendChild(li);
  }
}

function zeigeFrage(q) {
  document.getElementById("wegpunkt").hidden = false;
  document.getElementById("fertig").hidden = true;
  document.getElementById("frage").textContent = q.fragetext_laie || q.feld_id;
  document.getElementById("hilfe").textContent = q.hilfe_kurz || "";
  document.getElementById("anker").hidden = true;

  const box = document.getElementById("eingabe");
  box.innerHTML = "";
  let input;
  if (q.typ === "bool") {
    input = document.createElement("select");
    for (const [t, v] of [["Ja", "true"], ["Nein", "false"]]) {
      const o = document.createElement("option"); o.value = v; o.textContent = t; input.appendChild(o);
    }
  } else if (q.typ === "enum") {
    input = document.createElement("select");
    for (const v of (q.enum_werte || [])) {
      const o = document.createElement("option"); o.value = v; o.textContent = v; input.appendChild(o);
    }
  } else {
    input = document.createElement("input");
    input.type = "number";
    input.inputMode = q.typ === "cent" ? "decimal" : "numeric";
    if (q.bereich) { if ("min" in q.bereich) input.min = q.bereich.min; if ("max" in q.bereich) input.max = q.bereich.max; }
    input.placeholder = q.typ === "cent" ? "Betrag in Euro" : String(q.beispielwert ?? "");
  }
  input.id = "feld-input";
  box.appendChild(input);
  if (q.einheit) { const s = document.createElement("span"); s.className = "einheit"; s.textContent = " " + q.einheit; box.appendChild(s); }
}

function leseWert(q) {
  const el = document.getElementById("feld-input");
  if (q.typ === "enum") return el.value;
  if (q.typ === "bool") {
    const ja = el.value === "true";
    // Abwesenheits-Flags: positive Frage ("Hattest du X?"), gespeichert wird die Abwesenheit kein_X.
    return q.feld_id.startsWith("kein_") ? !ja : ja;
  }
  if (q.typ === "cent") return Math.round(parseFloat(el.value || "0") * 100);
  return parseInt(el.value || "0", 10);
}

// --- Bestätigen: Zwei-Signal-Event über den EINZIGEN Schreibpfad ---
async function bestaetigen() {
  if (!AKTUELL) return;
  const wert = leseWert(AKTUELL);
  const r = await jpost(`/fall/${FALL}/event`, {
    feld_id: AKTUELL.feld_id, wert: wert, zustand: "bestaetigt",
    herkunft: { herkunft: "laie", pruef_tiefe: "ungeprueft", haftung: "nutzer" },
    schreiber: "ui:laie",
    signal: { signal_1: null, signal_2: "klick@" + AKTUELL.feld_id },
  });
  if (r.status >= 400) { alert("Abgewiesen: " + (r.body.fehler || r.status)); return; }
  await refresh();
}

async function zeigeWarum() {
  if (!AKTUELL) return;
  const r = await jget(`/fall/${FALL}/feld/${AKTUELL.feld_id}/warum`);
  const el = document.getElementById("anker");
  if (r.status === 404) {
    // Feld noch ohne Event -> zeig den Anker aus der Frage selbst
    el.textContent = AKTUELL.anker_ref
      ? `${AKTUELL.anker_ref.quelle}\n„${AKTUELL.anker_ref.zitatanker}"` : "(kein Anker)";
  } else {
    const a = r.body.justification.anker_ref || {};
    el.textContent = `${a.quelle || ""}\n„${a.zitatanker || ""}"`;
  }
  el.hidden = false;
}

async function zeigeErgebnis() {
  const r = (await jget(`/fall/${FALL}/ergebnis`)).body;
  document.getElementById("fertig").hidden = false;
  const el = document.getElementById("ergebnis");
  const guardTexte = {
    werbungskosten_nicht_ring_faehig:
      "Du hast weitere Werbungskosten (z.B. doppelte Haushaltsführung) — der vereinfachte Bescheid gilt nur für den reinen Pendlerfall.",
    sonderausgaben_nicht_ring_faehig:
      "Du hast Sonderausgaben (z.B. Altersvorsorge) — der vereinfachte Bescheid gilt nur ohne gesondert erfasste Sonderausgaben (folgt).",
    einkunftsart_nicht_ring_faehig:
      "Du hast weitere Einkunftsarten — dafür ist die vollständige Berechnung nötig (folgt).",
    dhf_tatbestand_offen:
      "Zur doppelten Haushaltsführung fehlt noch eine Angabe (z.B. beruflicher Anlass, eigener Hausstand) — bitte vervollständigen.",
    ausland_dhf_nicht_ring_faehig:
      "Deine Zweitwohnung liegt im Ausland — dafür gelten andere Grenzen (folgt).",
    partner_kegel_offen:
      "Für die gemeinsame Erklärung fehlen noch Angaben zu deinem Partner (Bruttolohn, Identifikationsnummer).",
    partner_vor_offen:
      "Die gemeinsame Vorsorge-Berechnung folgt — der vereinfachte Splitting-Bescheid gilt vorerst ohne Vorsorgeaufwendungen.",
    verpflegung_reduktion_offen:
      "Verpflegungspauschale bei mehr als 3 Monaten am selben Ort oder gestellten Mahlzeiten reduziert — bitte gib an, ob das zutrifft (die Reduktion folgt).",
  };
  if (r.zahl_cent === null) {
    if (r.grund in guardTexte)
      el.textContent = guardTexte[r.grund];
    else if (r.grund === "kein_scheiben_gesamtbescheid")
      el.textContent = "Alle Angaben erfasst — die Gesamtsteuer wird in einem späteren Schritt berechnet.";
    else if (r.grund === "engine_unavailable")
      el.textContent = "Alle Angaben bestätigt — die Rechen-Engine ist hier nicht verfügbar.";
    else
      el.textContent = "Noch offen: " + (r.offen || []).join(", ");
  } else {
    el.textContent = "Dein Ergebnis: " + euro(r.zahl_cent);
  }
}

document.getElementById("bestaetigen").addEventListener("click", bestaetigen);
document.getElementById("warum").addEventListener("click", zeigeWarum);
start();
