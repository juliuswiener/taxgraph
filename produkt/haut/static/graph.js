"use strict";
// TaxGraph Desktop-Graph — read-only Abhängigkeits-Übersicht. Redet nur über GET /graph + /warum;
// keine Steuerlogik, keine Schreibaktion. Fall-ID aus der URL (?fall=…).

const FALL = new URLSearchParams(location.search).get("fall");

async function load() {
  const regeln = document.getElementById("regeln");
  if (!FALL) {
    regeln.textContent = "Kein Fall angegeben — Aufruf mit ?fall=<fall_id>.";
    return;
  }
  const r = await fetch(`/fall/${FALL}/graph`);
  if (!r.ok) {
    regeln.textContent = `Fall ${FALL} nicht gefunden (${r.status}).`;
    return;
  }
  render(await r.json());
}

function render(g) {
  const proRegel = {};
  for (const k of g.kanten) (proRegel[k.regel_id] ||= []).push(k);

  const el = document.getElementById("regeln");
  el.innerHTML = "";
  for (const n of g.knoten) {
    const card = document.createElement("div");
    card.className = "regel status-" + n.status;

    const kopf = document.createElement("div");
    kopf.className = "regel-kopf";
    const name = document.createElement("span");
    name.className = "regel-name";
    name.textContent = n.regel_id;
    const badge = document.createElement("span");
    badge.className = "status-badge status-" + n.status;
    badge.textContent = n.status;
    kopf.appendChild(name);
    kopf.appendChild(badge);
    card.appendChild(kopf);

    if (n.gates_offen.length || n.annahmen_offen.length) {
      const offen = document.createElement("p");
      offen.className = "offen";
      const teile = [];
      if (n.gates_offen.length) teile.push("offene Gates: " + n.gates_offen.join(", "));
      if (n.annahmen_offen.length) teile.push("Annahmen: " + n.annahmen_offen.join(", "));
      offen.textContent = teile.join(" · ");
      card.appendChild(offen);
    }

    const ul = document.createElement("ul");
    ul.className = "felder";
    for (const k of (proRegel[n.regel_id] || [])) {
      const li = document.createElement("li");
      li.className = "feld zustand-" + k.zustand;
      li.title = k.fragetext_laie || "";
      li.textContent = `${k.feld_id} · ${k.rolle} · ${k.zustand}`;
      li.addEventListener("click", () => zeigeWarum(k.feld_id));
      ul.appendChild(li);
    }
    card.appendChild(ul);
    el.appendChild(card);
  }
}

async function zeigeWarum(fid) {
  const panel = document.getElementById("warum-panel");
  const r = await fetch(`/fall/${FALL}/feld/${fid}/warum`);
  if (r.status === 404) {
    panel.textContent = `${fid}: noch kein Wert erfasst.`;
  } else {
    const j = (await r.json()).justification;
    const a = j.anker_ref || {};
    panel.textContent = `${fid} = ${JSON.stringify(j.wert)}  (${j.zustand}, Herkunft ${j.herkunft.herkunft})\n`
      + `${a.quelle || ""}\n„${a.zitatanker || ""}"`;
  }
  panel.hidden = false;
}

load();
