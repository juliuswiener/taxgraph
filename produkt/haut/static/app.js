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

function zeigeSpanne(stand) {
  const el = document.getElementById("spanne");
  if (stand.engine === "unavailable" || !stand.intervall) {
    el.textContent = "Bescheid-Spanne: (Rechen-Engine nicht verfügbar)";
    return;
  }
  const iv = stand.intervall;
  const lo = euro(iv.min_cent), hi = euro(iv.max_cent);
  const off = (iv.min_offen || iv.max_offen) ? " (noch offen)" : "";
  el.textContent = (iv.min_cent === iv.max_cent)
    ? `Bescheid: ${lo}`
    : `Bescheid zwischen ${lo} und ${hi}${off}`;
  // Ring: Anteil bestätigter Felder
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
  if (q.typ === "bool") return el.value === "true";
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
  if (r.zahl_cent === null) {
    el.textContent = r.grund === "engine_unavailable"
      ? "Alle Angaben bestätigt — die Rechen-Engine ist hier nicht verfügbar."
      : "Noch offen: " + (r.offen || []).join(", ");
  } else {
    el.textContent = "Deine Entfernungspauschale: " + euro(r.zahl_cent);
  }
}

document.getElementById("bestaetigen").addEventListener("click", bestaetigen);
document.getElementById("warum").addEventListener("click", zeigeWarum);
start();
