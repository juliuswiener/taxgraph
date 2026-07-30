# 09 — Production Readiness: What's Missing

## Scope

Diese Analyse bewertet, was fehlt, damit TaxGraph als **echtes Produkt** (Steuerberechnung für Endnutzer, Einreichung ans Finanzamt) live gehen kann. Sie geht über die Code-Qualität hinaus und betrachtet Infrastruktur, Sicherheit, Compliance, Betrieb und User Experience.

## Die 7 Säulen der Production Readiness

Für jede Säule: aktueller Stand + was fehlt.

---

## 1. Berechnungskorrektheit (Steuer-Mathe)

**Stand**: 85% der MVP-Szenarien korrekt. Guard-System fängt unrechenbare Fälle fail-closed ab. 61 Accessor-Funktionen, 15 davon Pure-Python (verifizierbar), 46 Catala-generiert (vertrauensbasiert).

**Was fehlt für Production:**

| # | Lücke | Schwere |
|---|-------|---------|
| 1.1 | **DBA Freistellung funktioniert nicht** — Guard-Konflikt blockiert AT/US | CRITICAL |
| 1.2 | **Kein Cross-Check gegen GETTSIM für alle Scheiben** — nur Stichproben | HIGH |
| 1.3 | **Kein Golden-Corpus mit amtlich verifizierten Musterfällen** — Tests prüfen Δ, nicht absolute Werte | HIGH |
| 1.4 | **p32b Progressionsvorbehalt ungetestet** — kein Ring-Diff beweist Wirksamkeit | HIGH |
| 1.5 | **§32d Abgeltungsteuer + DBA + §35 Interaktion** — keine Kombinationstests | MEDIUM |
| 1.6 | **Mehrere Veranlagungszeiträume** — VZ-Wechsel (2024→2025→2026) ungetestet | MEDIUM |

---

## 2. ELSTER-Submission (Finanzamt-Interface)

**Stand**: `elster_writer.py` generiert XML. `elster/validate_mapping.py` prüft Feld-Mapping. ERiC 44.2.4.0 installiert (`~/02_Software/eric`). `checkESt` läuft mit rc=0.

**Was fehlt für Production:**

| # | Lücke | Schwere |
|---|-------|---------|
| 2.1 | **Kein End-to-End ELSTER-Test** — XML wird generiert aber nie an ERiC übergeben und validiert | CRITICAL |
| 2.2 | **Keine ERiC-Fehlercode-Behandlung** — was passiert bei ERiC-Ablehnung? | HIGH |
| 2.3 | **Keine ELSTER-Authentifizierung** — Zertifikat/HERSTELLER_ID nur als env-Var | HIGH |
| 2.4 | **Kein ELSTER-Rückkanal** — Bescheid-Parsing nach Einreichung fehlt komplett | HIGH |
| 2.5 | **Keine elektronische Bekanntgabe (§ 87a AO)** — Zustellung digitaler Bescheide | MEDIUM |
| 2.6 | **§150 Abs.7 AO Auto-Bestätigung** — eDaten werden nie als `bestaetigt` markiert | MEDIUM |

---

## 3. Sicherheit & Datenschutz

**Stand**: Keine sichtbare Authentifizierung. Store = JSON-Dateien im Dateisystem. `person_b_idnr` wird gespeichert. LLM-Chat routed über OpenRouter (Drittanbieter).

**Was fehlt für Production:**

| # | Lücke | Schwere |
|---|-------|---------|
| 3.1 | **Keine Authentifizierung** — Jeder kann jede fall_id erraten und Daten lesen | CRITICAL |
| 3.2 | **Keine Autorisierung** — Kein Fall-Besitzer-Konzept, kein Multi-Tenant | CRITICAL |
| 3.3 | **Steuerdaten im Klartext auf Dateisystem** — keine Verschlüsselung at rest | CRITICAL |
| 3.4 | **Steuer-ID (person_b_idnr) ungeschützt** — höchstsensibles Datum nach § 139b AO | CRITICAL |
| 3.5 | **LLM-Chat sendet Steuerdaten an OpenRouter** — kein PII-Filter, kein Audit-Log | HIGH |
| 3.6 | **Keine Transportverschlüsselung** — HTTP (nicht HTTPS) im Test-Setup | HIGH |
| 3.7 | **Kein Löschkonzept** — DSGVO Art. 17 Recht auf Löschung nicht implementiert | HIGH |
| 3.8 | **Keine Zugriffslogs** — kein Audit-Trail wer wann welche Daten sah | MEDIUM |

---

## 4. Datenhaltung & Persistenz

**Stand**: Store = `json.load()` / `json.dump()` auf Einzeldateien pro Fall. `lru_cache` für `lade_bindung()`. Keine Datenbank.

**Was fehlt für Production:**

| # | Lücke | Schwere |
|---|-------|---------|
| 4.1 | **Keine Datenbank** — JSON-Dateien sind nicht transaktional, nicht concurrent-safe | CRITICAL |
| 4.2 | **Kein Backup** — Festplattendefekt = alle Fälle verloren | CRITICAL |
| 4.3 | **Keine Migration-Strategie** — Schema-Änderungen brechen alte Fälle | HIGH |
| 4.4 | **Kein Concurrent-Request-Handling** — Zwei parallele Events auf denselben Fall → Race Condition | HIGH |
| 4.5 | **Store-Größe unbegrenzt** — ein Fall kann beliebig viele Events akkumulieren | LOW |

---

## 5. Betrieb & Observability

**Stand**: `server.py` ist ein einfacher FastAPI/HTTP-Server. Kein Logging-Framework. Keine Metriken. `daemon_threads=False` für Catala (Race-Condition-Fix).

**Was fehlt für Production:**

| # | Lücke | Schwere |
|---|-------|---------|
| 5.1 | **Kein strukturiertes Logging** — `print()` statements in Produktion | HIGH |
| 5.2 | **Keine Health-Checks** — kein `/health` Endpoint, kein Readiness-Probe | HIGH |
| 5.3 | **Keine Metriken** — keine Request-Dauer, Fehlerrate, catala_engine-Latenz | MEDIUM |
| 5.4 | **Kein Rate-Limiting** — LLM-Chat, ELSTER-Submission ungeschützt gegen Überlastung | MEDIUM |
| 5.5 | **Kein Graceful-Shutdown** — laufende Requests brechen bei SIGTERM ab | MEDIUM |
| 5.6 | **Catala-Engine-Isolation** — `daemon_threads` Fix ist ein Workaround, keine Lösung | LOW |

---

## 6. User Experience & Frontend

**Stand**: `produkt/haut/` ist reines Backend (API). Kein Frontend-Code im Repository. Die API-Endpunkte (`/fragen`, `/stand`, `/ergebnis`, `/graph`) sind auf ein hypothetisches UI ausgelegt.

**Was fehlt für Production:**

| # | Lücke | Schwere |
|---|-------|---------|
| 6.1 | **Kein Frontend** — Endnutzer können die API nicht direkt nutzen | CRITICAL |
| 6.2 | **Fragen-Flow nicht benutzergetestet** — Sind die Laien-Fragen verständlich? | HIGH |
| 6.3 | **Keine Fehlerbehandlung im UI** — Was sieht der Nutzer bei `grund=dba_freistellung_offen`? | HIGH |
| 6.4 | **Kein Onboarding-Flow** — Wie legt ein Neunutzer seinen ersten Fall an? | MEDIUM |
| 6.5 | **Keine Mobile-Responsivität** — geplant aber nicht gebaut | LOW |

---

## 7. Recht & Compliance

**Stand**: Quellen in `sources/` dokumentiert. `anker_ref` verweist auf Gesetzesstellen. `clerk.toml` definiert Catala-Regeln.

**Was fehlt für Production:**

| # | Lücke | Schwere |
|---|-------|---------|
| 7.1 | **Keine Rechtsberatungs-Abgrenzung** — Art. 1 § 1 RBerG: darf das System überhaupt Steuern berechnen ohne Steuerberater-Lizenz? | CRITICAL |
| 7.2 | **Keine AGB/Datenschutzerklärung** — DSGVO Art. 13 Informationspflichten | CRITICAL |
| 7.3 | **Keine Auftragsverarbeitung (Art. 28 DSGVO)** — OpenRouter als Dritt-Anbieter für LLM-Chat | HIGH |
| 7.4 | **Kein Haftungsausschluss** — Was passiert bei Rechenfehler → falscher Steuerbescheid? | HIGH |
| 7.5 | **Keine Barrierefreiheit** — EU-Richtlinie 2016/2102, BITV 2.0 | MEDIUM |
| 7.6 | **Keine GoBD-Compliance** — Grundsätze ordnungsmäßiger Buchführung (Archivierung, Nachvollziehbarkeit) | MEDIUM |

---

## Zusammenfassung: Die Top-10 Blocker für Production

| Rang | Blocker | Säule |
|------|---------|-------|
| 1 | **Authentifizierung + Autorisierung** — jeder kann jeden Fall lesen | Sicherheit |
| 2 | **Steuer-ID im Klartext** — DSGVO-Höchststrafe (Art. 83 Abs. 5) | Sicherheit |
| 3 | **Rechtsberatungs-Abgrenzung** — RBerG §1: droht Abmahnung/Unterlassung | Compliance |
| 4 | **ELSTER-End-to-End-Test** — XML nie gegen ERiC validiert | Submission |
| 5 | **Datenbank statt JSON-Dateien** — Datenverlust, Race Conditions | Daten |
| 6 | **DBA Freistellung funktioniert nicht** — AT/US-Kunden bekommen Fehler | Berechnung |
| 7 | **Golden-Corpus mit Musterfällen** — Tests prüfen Δ, nicht absolute Richtigkeit | Berechnung |
| 8 | **DSGVO: Löschkonzept + Informationspflichten** — Art. 13, 17 | Compliance |
| 9 | **LLM-Chat: Steuerdaten an OpenRouter** — PII-Leak an US-Anbieter | Sicherheit |
| 10 | **Frontend fehlt komplett** — kein benutzbares Produkt | UX |

## Das Urteil

TaxGraph ist ein **beeindruckend korrekter Berechnungskern** mit exzellenter Fail-Closed-Architektur. Die Steuer-Mathematik ist auf dem Niveau eines guten Steuerrechners.

**Aber**: Ein Berechnungskern ist kein Produkt. Es fehlen ALLE Schichten, die aus einem Algorithmus ein sicheres, rechtskonformes, benutzbares Produkt machen — Authentifizierung, Verschlüsselung, Datenbank, ELSTER-Integration, Frontend, Rechtssicherheit.

**Roadmap-Vorschlag:**
1. **Monat 1-2**: Sicherheit (Auth, Verschlüsselung, Datenbank-Migration)
2. **Monat 3**: ELSTER-End-to-End (XML→ERiC→Bescheid-Parsing)
3. **Monat 4**: Rechts-Compliance (AGB, DSGVO, RBerG-Klärung)
4. **Monat 5-6**: Frontend MVP (Laien-Fragen-Flow)
5. **Parallel**: DBA-Fix, Golden-Corpus, Test-Suite bereinigen
