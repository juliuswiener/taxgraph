# TaxGraph v3 → Marktreifes Produkt — Umfassende Roadmap

**Stand:** 2026-07-24, HEAD `8d904b4`, 854/105 tests passing
**Scope:** Vom aktuellen Berechnungskern zum launchbaren Steuerprodukt
**Quellen:** Autopsy docs/01-11, externe Reviews, Product-Gap-Analyse, Production-Readiness

---

## Roadmap-Philosophie

**Pakete sind unabhängig** — jedes Paket kann von einem eigenen Worker/Team parallel bearbeitet werden, solange die angegebenen Abhängigkeiten (→) erfüllt sind.

**Milestones liefern inkrementellen Wert** — nach jedem Milestone ist das System in einem besseren Zustand als vorher. Kein "Big Bang" Deployment.

**Risiko-First-Priorisierung** — Sicherheit und Compliance kommen VOR Feature-Expansion. Ein unsicheres Produkt darf nicht live gehen, egal wie gut es rechnet.

---

## Paket-Übersicht

```
                         ┌─────────────────┐
                         │  P0: Foundation  │  ← SOFORT (1-2 Wochen)
                         │  Bugfixes + Tests│
                         └────────┬────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
  ┌───────▼──────┐     ┌─────────▼────────┐     ┌───────▼──────┐
  │ P1: Security │     │ P2: Legal +      │     │ P3: ELSTER   │
  │ Auth/Crypto  │     │     Compliance   │     │ Production   │
  │ (Month 1-2)  │     │ (Month 1-2)      │     │ (Month 2-3)  │
  └───────┬──────┘     └─────────┬────────┘     └───────┬──────┘
          │                       │                       │
          └───────────────────────┼───────────────────────┘
                                  │
                         ┌────────▼────────┐
                         │ P4: Database    │
                         │ Migration       │
                         │ (Month 2)       │
                         └────────┬────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
  ┌───────▼──────┐     ┌─────────▼────────┐     ┌───────▼──────┐
  │ P5: UX +     │     │ P6: Data Import  │     │ P7: Coverage │
  │ Interview    │     │ (eDaten/OCR/     │     │ Expansion    │
  │ (Month 3-5)  │     │  Vorjahr)        │     │ (Month 4-8)  │
  │              │     │ (Month 3-4)      │     │              │
  └───────┬──────┘     └─────────┬────────┘     └───────┬──────┘
          │                       │                       │
          └───────────────────────┼───────────────────────┘
                                  │
                         ┌────────▼────────┐
                         │ P8: Operations  │
                         │ CI/CD, Monitor, │
                         │ VZ-Update       │
                         │ (Month 4+)      │
                         └─────────────────┘
```

---

## Milestone 0: Foundation — JETZT (Woche 1-2)

**Ziel:** Alle bekannten Bugs fixen, Test-Suite auf Grün bringen, Build automatisieren. Kein neues Feature — nur Reparatur.

### Paket P0: Bugfixes & Test-Recovery

| ID | Task | Schwere | Aufwand | Abhängigkeit |
|----|------|---------|---------|-------------|
| P0.1 | **`kapitalertraege` NameError fixen** (`api.py:1301`): `kapitalertraege` → `g2["est_gesamt"]` | CRITICAL | 1h | — |
| P0.2 | **DBA-Guard-Konflikt lösen** (`api.py:1792`): Guard entfernen ODER Routing-Code konsolidieren. Empfehlung: Guard entfernen + p32b-Test schreiben | CRITICAL | 4h | P0.1 |
| P0.3 | **`ergebnis.json` Schema-Grund-Enum aktualisieren**: 6 fehlende `grund`-Werte hinzufügen (`dba_freistellung_offen`, `dba_multi_country_offen`, `dba_kapital_offen`, `p32b_kombi_offen`, `progression_gehoert_in_gesamt`, `p16_4_gate_offen`) | HIGH | 2h | — |
| P0.4 | **`make build-python` in CI automatisieren**: `pkg`-Modul-Bau als Pre-Test-Schritt. Ohne dies können 14 Testdateien nicht gesammelt werden | HIGH | 2h | — |
| P0.5 | **`bindung_n_vor_gwg.yaml` Merge-Konflikt bereinigen**: `git checkout --theirs` gefolgt von manueller Prüfung | HIGH | 1h | — |
| P0.6 | **`p7_1_lineare_afa` fehlende Slots binden**: `anschaffungs_herstellungskosten` und `anzurechnende_monate` zu Lücken-Einträgen hinzufügen oder binden | MEDIUM | 2h | — |
| P0.7 | **VENV312-Hardcoding im Makefile fixen**: `VENV312 ?= oracle/.venv312/bin/activate` mit Fallback | MEDIUM | 30m | — |
| P0.8 | **Exception-Schema-Normalisierung**: `{"fehler": "..."}` → schema-konformes `{"zahl_cent": None, "grund": "engine_unavailable", "offen": [...]}` | MEDIUM | 2h | — |
| P0.9 | **Vollsuite-Gate: 959/0 als Baseline setzen**: `pytest tests/ -q` muss 0 failures haben vor jedem Commit | HIGH | 1h | P0.1-P0.8 |
| P0.10 | **Tote Code-Kommentare bereinigen**: `api.py:51` "AfA>800 ist ungebunden" → aktualisieren | LOW | 30m | — |

**P0 Deliverable:** `pytest tests/ -q` = 959 passed, 0 failed. Kein Commit ohne grünes Gate.

**Gate:** `pytest tests/ -q` exit code 0 + `make build-python` exit code 0 + `pytest tests/test_bindungstabelle.py` exit code 0.

---

## Milestone 1: Secure Foundation (Monat 1-2)

**Ziel:** Das System ist sicher genug um sensible Steuerdaten zu speichern. Auth, Crypto, Datenbank. Kein neues Feature — nur Härtung.

### Paket P1: Security (parallel zu P2)

| ID | Task | Schwere | Aufwand | Abhängigkeit |
|----|------|---------|---------|-------------|
| P1.1 | **Authentifizierung**: JWT-basierte Login/Logout, Passwort-Hashing (bcrypt/argon2), Session-Management | CRITICAL | 3d | — |
| P1.2 | **Autorisierung**: Fall-Besitzer-Konzept (user_id → fall_id), Multi-Tenant-Isolation. Kein User kann einen fremden Fall lesen | CRITICAL | 3d | P1.1 |
| P1.3 | **Encryption at Rest**: AES-256-GCM für alle Store-Daten. `person_b_idnr` separat verschlüsselt mit eigenem Key. Key-Management via env-var oder KMS | CRITICAL | 3d | P4.1 (Datenbank) |
| P1.4 | **Encryption in Transit**: HTTPS/TLS für alle API-Endpunkte. Cert-Management (Let's Encrypt oder Cloud-LB) | HIGH | 1d | — |
| P1.5 | **LLM-Chat PII-Filter**: Vor dem Senden an OpenRouter: Steuer-ID, Name, Adresse, Geburtsdatum, IBAN filtern. Audit-Log was gesendet wurde | HIGH | 2d | — |
| P1.6 | **Zugriffs-Audit-Log**: Wer (user_id) hat wann (timestamp) auf welchen Fall (fall_id) zugegriffen? Immutable append-only Log | MEDIUM | 2d | P1.1 |
| P1.7 | **DSGVO-Löschkonzept**: Endpunkt `DELETE /fall/{id}` mit kaskadierender Löschung aller Events + Audit-Log-Eintrag. 30-Tage Soft-Delete → Hard-Delete | HIGH | 2d | P4.1 |

**P1 Deliverable:** Kein unauthentifizierter Zugriff möglich. Alle Steuerdaten verschlüsselt. PII-Filter aktiv. Löschbar.

### Paket P2: Legal + Compliance (parallel zu P1)

| ID | Task | Schwere | Aufwand | Abhängigkeit |
|----|------|---------|---------|-------------|
| P2.1 | **StBerG-Gutachten einholen**: Fachanwalt für Steuerrecht prüft Produktarchitektur auf § 2 StBerG-Konformität. Insbesondere: Automatisierte Handlungsempfehlungen, Optimierungsvorschläge, LLM-Chat-Einordnung | CRITICAL | Extern (2-4 Wo) | — |
| P2.2 | **AGB & Datenschutzerklärung**: Juristisch geprüfte Texte für Endnutzer. Klare Trennung: Deklaration (erlaubt) vs. Beratung (nicht erlaubt ohne StBerG-Lizenz) | CRITICAL | 1 Wo (extern) | P2.1 |
| P2.3 | **Disclaimer-Architektur**: An jedem potenziell beratenden UI-Element ("Das könnte Ihre Steuerlast senken...") muss ein juristisch geprüfter Disclaimer stehen | HIGH | 2d | P2.1 |
| P2.4 | **DSGVO Art. 9 Folgenabschätzung**: Datenschutz-Folgenabschätzung (DPIA) für besondere Kategorien personenbezogener Daten (Gesundheit §33, Religion KiSt) | HIGH | 3d | P2.1 |
| P2.5 | **Auftragsverarbeitungsvertrag (AVV) mit OpenRouter**: Art. 28 DSGVO — falls LLM-Chat bestehen bleibt, muss ein AVV mit dem US-Anbieter geschlossen werden (problematisch: Privacy Shield 2.0 unsicher) | HIGH | 1 Wo (extern) | — |
| P2.6 | **Impressum + Barrierefreiheit**: Impressumspflicht (§ 5 TMG), Barrierefreiheitserklärung (BITV 2.0) | MEDIUM | 1d | — |

**P2 Deliverable:** StBerG-konforme Architektur bestätigt. AGB/Datenschutz live. DPIA dokumentiert. LLM-Chat entweder konform oder deaktiviert.

### Paket P3: ELSTER Production (startet Monat 2, nach P0)

| ID | Task | Schwere | Aufwand | Abhängigkeit |
|----|------|---------|---------|-------------|
| P3.1 | **ELSTER-Herstellerregistrierung**: Offizielle Registrierung bei der Konsens-Gruppe, Beantragung Hersteller-ID, Einbindung der ERiC-C-Bibliothek mit eigener ID | CRITICAL | 2-4 Wo (extern) | — |
| P3.2 | **ERiC-Submission-Endpunkt**: `POST /fall/{id}/einreichen` → ERiC `ERIC_ENCRYPT_AND_SEND`. XML-Generierung → Signatur → Verschlüsselung → Übertragung an Finanzamt-Rechenzentrum | CRITICAL | 5d | P3.1 |
| P3.3 | **Zertifikats-Management**: Nutzer-Zertifikate (.pfx) oder ElsterSecure hochladen/speichern. Zertifikats-PIN-Verwaltung (nie im Klartext loggen) | CRITICAL | 3d | P3.1 |
| P3.4 | **DIVA-Bescheidabholung**: Automatische Abholung des elektronischen Steuerbescheids. Polling-Endpoint oder Webhook für DIVA-Notifications | HIGH | 5d | P3.2 |
| P3.5 | **Soll-Ist-Abgleich**: TaxGraph-Berechnung vs. amtlicher Bescheid. Differenzen-Report: welche Positionen weichen ab, warum, um wie viel | HIGH | 3d | P3.4 |
| P3.6 | **ERiC-Fehlercode-Mapping**: Alle ERiC-Fehlercodes (20.000+) auf menschenlesbare deutsche Fehlermeldungen mappen. Priorisierte TOP-100 Fehler mit Handlungsempfehlung | HIGH | 3d | P3.2 |
| P3.7 | **ELSTER-End-to-End-Test**: Mindestens 3 reale Testszenarien (AN, AN+Kapital, Rentner) durch die komplette Pipeline: Eingabe → Berechnung → XML → ERiC → Submission-Simulation | CRITICAL | 2d | P3.2-P3.6 |

**P3 Deliverable:** Mindestens ein ELSTER-Testfall erfolgreich an ERiC-Testumgebung übermittelt und validiert.

### Paket P4: Database Migration (Monat 2)

| ID | Task | Schwere | Aufwand | Abhängigkeit |
|----|------|---------|---------|-------------|
| P4.1 | **PostgreSQL-Schema**: Tabellen für users, faelle, events, audit_log. Foreign-Key-Constraints (user_id → faelle, fall_id → events). Index auf fall_id + timestamp | CRITICAL | 3d | — |
| P4.2 | **Store-Abstraktion**: `produkt/store/` Interface so umbauen, dass PostgreSQL und JSON-Dateien dieselbe API bedienen (Storage-Backend-Swap). Existierende Tests müssen mit beiden Backends laufen | HIGH | 3d | P4.1 |
| P4.3 | **Migration existierender Fälle**: JSON→SQL-Migrationsskript. Alle existierenden Test-Fälle in der Test-Suite müssen migriert werden können | HIGH | 2d | P4.2 |
| P4.4 | **Concurrent-Request-Safety**: Transaktionen für Event-Schreiben. `SELECT ... FOR UPDATE` für Race-Condition-Prävention. Optimistic Locking via Event-Version | HIGH | 2d | P4.1 |
| P4.5 | **Backup-Strategie**: pg_dump Cronjob + WAL-Archivierung. Point-in-Time Recovery. Backup-Restore-Test als CI-Schritt | CRITICAL | 2d | P4.1 |

**P4 Deliverable:** Alle Store-Operationen transaktional via PostgreSQL. Migration getestet. Backup aktiv.

**Gate:** Existierende Test-Suite läuft mit PostgreSQL-Backend (959 passed).

---

## Milestone 2: User-Facing Product (Monat 3-5)

**Ziel:** Das System wird benutzbar für echte Menschen. Interview-Flow, Datenimport, Erklärbarkeit.

### Paket P5: UX + Interview Layer (Monat 3-5)

| ID | Task | Schwere | Aufwand | Abhängigkeit |
|----|------|---------|---------|-------------|
| P5.1 | **Dynamic Interview Engine**: Regelgraph-basierte Relevanzpropagation. Frage nur, was für diesen Fall relevant ist. Abhängigkeiten: `bindung/graph` → `askable` Felder → Filter auf `kein_gewinn`/`kein_kap`/etc. | CRITICAL | 10d | P4.2 |
| P5.2 | **Laien-Übersetzung**: Jedes `feld_id` → menschenlesbare Frage + Hilfetext + Beispiel. Die `hilfe_kurz`-Felder aus den Bindungen sind der Start, müssen aber auf Laien-Niveau gebracht werden | HIGH | 5d | — |
| P5.3 | **Interview-UI (Web)**: Responsive Single-Page-App (React/Vue/Svelte). Schritt-für-Schritt-Flow mit Fortschrittsbalken. Zurück-Button. Zwischenspeichern. | CRITICAL | 15d | P5.1, P5.2 |
| P5.4 | **Catala Explain-View**: "Warum zahle ich X € Steuer?" — Grafische Aufbereitung des Berechnungswegs. `/graph`-Endpunkt-Daten visualisieren. Sankey-Diagramm: Brutto → zvE → tarifliche ESt → festzusetzende ESt | HIGH | 8d | — |
| P5.5 | **Pre-Flight Plausibilitäts-Check**: Vor dem ELSTER-Versand: Automatische Hinweise auf vergessene Pauschalen, widersprüchliche Angaben, ungewöhnliche Werte. Regelbasiert (kein LLM) | MEDIUM | 5d | — |
| P5.6 | **Mobile-Responsive**: Alle UI-Komponenten müssen auf Smartphone (< 400px Breite) benutzbar sein | MEDIUM | 3d | P5.3 |

**P5 Deliverable:** Ein Mensch ohne Steuerkenntnisse kann einen einfachen Arbeitnehmerfall vollständig erfassen, berechnen lassen und das Ergebnis verstehen.

### Paket P6: Data Import (Monat 3-4, parallel zu P5)

| ID | Task | Schwere | Aufwand | Abhängigkeit |
|----|------|---------|---------|-------------|
| P6.1 | **VaSt/eDaten-Abruf**: ERiC `ERIC_VaSt_ABHOLEN` → eDaten-Parsing → Store-Integration. Lohnsteuerbescheinigung, Rentenbezugsmitteilung, KV/PV-Beiträge automatisch übernehmen | HIGH | 5d | P3.2 |
| P6.2 | **eDaten-Auto-Bestätigt**: Julius-Cap umsetzen: eDaten mit `zustand="bestaetigt"` schreiben (statt `vorlaeufig`). Nutzer-Override bleibt möglich (§ 150 Abs. 7 S. 2 AO) | MEDIUM | 2d | P6.1 |
| P6.3 | **Beleg-OCR-Pipeline**: Upload → Tesseract OCR (deutsch) → Feld-Klassifikation → `vorlaeufig`-Event mit `herkunft="beleg"`. Start mit: Spendenbescheinigung, Handwerkerrechnung, Nebenkostenabrechnung | MEDIUM | 8d | — |
| P6.4 | **Vorjahresübernahme**: `POST /fall/{id}/vorjahr-uebernehmen` → Stammdaten (Name, Adresse, Geburtsdatum) + Verlustvorträge + AfA-Verläufe aus Vorjahresfall kopieren | MEDIUM | 3d | P4.2 |
| P6.5 | **Kontoauszug-Import**: CSV-Upload → Feld-Matching → `vorlaeufig`-Events. Bestehender `kontoauszug_writer.py` als Basis | LOW | 3d | — |

**P6 Deliverable:** Nutzer kann eDaten abrufen, Belege fotografieren und Vorjahresdaten übernehmen. Manuelle Eingabe nur noch für Sonderfälle.

### Paket P7: Coverage Expansion (Monat 4-8)

| ID | Task | Schwere | Aufwand | Abhängigkeit |
|----|------|---------|---------|-------------|
| P7.1 | **DBA per-Einkunftsart-Routing**: `DBA_METHOD_MAP` von `{staat: methode}` auf `{(staat, einkunftsart): methode}` erweitern. Art. 6-22 für alle 11 Länder. PL als Template (9 entries bereits analysiert) | HIGH | 5d | P0.2 |
| P7.2 | **§32b Progressionsvorbehalt End-to-End**: Ring-Diff-Test der beweist dass `p32b_progressionseinkuenfte` tatsächlich die Steuer beeinflusst. Test-Szenario: DBA-Freistellung AT → 30k ausländisch → tarifliche ESt höher als ohne | HIGH | 3d | P0.2 |
| P7.3 | **§3 Nr. 72 Photovoltaik**: Steuerfreiheit für kleine PV-Anlagen. Neue Geltungsbedingung + Freibetrag. Relativ einfach (binäre Bedingung) | MEDIUM | 3d | — |
| P7.4 | **§13 Land- und Forstwirtschaft**: Spezielle Bewertungsregeln, Teilwert, Nutzungswertbesteuerung. Separate Scheibe `lu` oder Integration in gesamt? | MEDIUM | 15d | — |
| P7.5 | **Mehrere Veranlagungszeiträume**: VZ-übergreifende Tests (2024→2025→2026). Parameter-Layer-Versionierung. `params/2024/`, `params/2025/`, `params/2026/` | MEDIUM | 5d | — |
| P7.6 | **Multi-Country DBA**: `dba_multi_country_offen` Guard entfernen → Mehrfach-DBA-Routing. Pro Land: `dba_staat_1`, `dba_staat_2`, ... (Multi-Instanz wie §23) | MEDIUM | 8d | P7.1 |
| P7.7 | **Gewerbesteuer & E-Bilanz** (optional, nur bei Vollprodukt): Bilanz-Taxonomie, § 4 Abs. 1/§ 5 EStG, E-Bilanz-XML. Nur relevant wenn Selbstständige/Gewerbe voll bedient werden | LOW | 20d+ | — |

**P7 Deliverable:** DBA per-Einkunftsart für alle 11 Länder. §32b End-to-End verifiziert. §3 Nr. 72 live.

### Paket P8: Operations (Monat 4+, kontinuierlich)

| ID | Task | Schwere | Aufwand | Abhängigkeit |
|----|------|---------|---------|-------------|
| P8.1 | **CI/CD-Pipeline**: GitHub Actions: `make build-python` → `pytest tests/` → `make snapshot-verify` → Docker-Build → Deploy-Staging. Blockiert Merge bei rotem Gate | HIGH | 5d | P0.4, P0.9 |
| P8.2 | **Pre-Commit Snapshot-Verification**: `.git/hooks/pre-commit`: `python3 pipeline/snapshot.py verify --changed` → blockiert Commit wenn Catala-Source ohne Snapshot-Update editiert wurde | MEDIUM | 2d | — |
| P8.3 | **Health-Check + Monitoring**: `/health` (liveness), `/ready` (readiness: DB+ERiC+Catala). Prometheus-Metriken: Request-Dauer, Fehlerrate, ERiC-Latenz. Grafana-Dashboard | HIGH | 3d | P4.1 |
| P8.4 | **Rate-Limiting**: LLM-Chat: 10 Requests/Minute/User. ERiC-Submission: 1 Request/Minute/User. ELSTER-schreibt vor: max 100 Submissions/Tag global | MEDIUM | 2d | — |
| P8.5 | **Graceful-Shutdown**: SIGTERM-Handler: laufende Requests zu Ende führen (max 30s), dann DB-Verbindung schließen, dann exit | MEDIUM | 1d | — |
| P8.6 | **Jährlicher VZ-Update-Prozess**: Dokumentierte Checkliste für VZ-Wechsel (z.B. 2025→2026). Parameter-Layer-Update (`params/2026/`), Catala-Regel-Update (neue Gesetze), Golden-Corpus-Erweiterung, ERiC-Version-Update | HIGH | Prozess-Definition: 2d; Durchführung: 5d/Jahr | P7.5 |
| P8.7 | **Type-Annotation-Coverage**: `golden/runner.py`: `s: dict` → `TypedDict` für alle 61 Accessoren. Verhindert Key-Name-Mismatches (wie beim `kapitalertraege`-Bug) | LOW | 3d | — |

**P8 Deliverable:** CI/CD-Pipeline aktiv. Monitoring-Dashboard live. VZ-Update-Prozess dokumentiert und einmal durchgespielt.

---

## Milestone 3: Launch Readiness (Monat 6)

**Ziel:** Alle kritischen Pfade sind gehärtet, getestet und compliant. Produkt kann in Closed Beta gehen.

| ID | Task | Schwere | Aufwand | Abhängigkeit |
|----|------|---------|---------|-------------|
| M3.1 | **End-to-End-Produkttest**: 10 reale Steuerfälle (repräsentativ für Zielgruppe) durch komplette Pipeline: Onboarding → Interview → Berechnung → ELSTER-Submission → DIVA-Abgleich | CRITICAL | 5d | P1-P7 |
| M3.2 | **Penetrationstest**: Externer Security-Audit. OWASP Top 10. Fokus: Auth-Bypass, IDOR (Insecure Direct Object Reference), PII-Leakage | CRITICAL | 2 Wo (extern) | P1 |
| M3.3 | **Lasttest**: 1000 gleichzeitige Nutzer, 100 Berechnungen/Minute. Engpass-Identifikation (DB? Catala? ERiC?). Stress-Test: 10.000 Nutzer-Spitze | HIGH | 3d | P4, P8 |
| M3.4 | **Closed-Beta-Infrastruktur**: Staging-Umgebung mit echten (anonymisierten) Steuerdaten. Beta-Nutzer-Onboarding. Feedback-Kanal (Intercom/Email). Bug-Tracker | HIGH | 5d | P1-P8 |
| M3.5 | **Rechtliche finale Freigabe**: StBerG-Gutachten liegt vor. AGB/Datenschutz finale Version. DPIA abgeschlossen. Externer Datenschutzbeauftragter bestellt | CRITICAL | 2 Wo (extern) | P2 |

**M3 Deliverable:** Closed Beta mit 50-100 echten Nutzern. Keine kritischen Security-Issues. StBerG-Freigabe liegt vor.

---

## Milestone 4: Public Launch (Monat 7-8)

| ID | Task | Aufwand | Abhängigkeit |
|----|------|---------|-------------|
| M4.1 | Beta-Feedback einarbeiten | 10d | M3.4 |
| M4.2 | Skalierung auf Produktionslast | 5d | M3.3 |
| M4.3 | Payment-Integration (Stripe) | 5d | P1.1 |
| M4.4 | Marketing-Website + SEO | 10d | — |
| M4.5 | Support-Team-Training | 3d | — |
| M4.6 | Go-Live-Entscheidung | — | M3.5, M4.1-M4.5 |

**M4 Deliverable:** Öffentlicher Launch. Produkt unter steuerapp.de (oder ähnlich) erreichbar.

---

## Zusammenfassung: Zeitlinie

```
Woche 1-2   │ P0: Bugfixes ────────────────────────► Gate: 959/0 grün
             │
Monat 1-2   │ P1: Security ───────────────────────► Auth + Crypto live
             │ P2: Legal ──────────────────────────► StBerG-Gutachten beauftragt
             │
Monat 2     │ P4: Database ────────────────────────► PostgreSQL live
             │ P3: ELSTER ─────────────────────────► Erste ERiC-Testsubmission
             │
Monat 3-5   │ P5: UX ─────────────────────────────► Dynamic Interview live
             │ P6: Data Import ────────────────────► eDaten + OCR live
             │
Monat 4-8   │ P7: Coverage ────────────────────────► DBA-Einkunftsart + PV + §32b
             │ P8: Operations ─────────────────────► CI/CD + Monitoring
             │
Monat 6     │ M3: Launch Readiness ────────────────► Closed Beta
             │
Monat 7-8   │ M4: Public Launch ───────────────────► PRODUKT LIVE
```

## Unabhängige Pakete — Maximale Parallelisierung

Diese Pakete können **gleichzeitig** von verschiedenen Teams bearbeitet werden:

| Gruppe | Pakete | Team-Size | Parallel möglich? |
|--------|--------|-----------|-------------------|
| **Sofort** | P0 (Bugfixes) | 1-2 devs | — |
| **Welle 1** | P1 (Security) + P2 (Legal) | 2-3 devs + Extern | ✅ Parallel zueinander |
| **Welle 2** | P4 (Database) → P3 (ELSTER) | 1-2 devs + Extern | P4 vor P3 (ELSTER braucht DB) |
| **Welle 3** | P5 (UX) + P6 (Import) + P7 (Coverage) | 3-5 devs | ✅ Alle drei parallel |
| **Welle 4** | P8 (Operations) | 1 dev | Kontinuierlich ab Monat 4 |
| **Launch** | M3 (Beta) → M4 (Launch) | Alle + Extern | Sequentiell |

## Kostenschätzung

| Phase | Intern (Personentage) | Extern (€) | Was |
|-------|----------------------|------------|-----|
| P0 Foundation | 5-8 PT | — | Bugfixes, Tests, Schema |
| P1 Security | 15-20 PT | — | Auth, Crypto, Audit-Log |
| P2 Legal | 3-5 PT | 8-15 k€ | StBerG-Gutachten, AGB, DSGVO |
| P3 ELSTER | 25-30 PT | 2-5 k€ | Herstellerregistrierung, ERiC-Integration |
| P4 Database | 12-15 PT | — | PostgreSQL, Migration, Backup |
| P5 UX | 40-50 PT | — | Interview, UI, Explain |
| P6 Import | 18-25 PT | — | eDaten, OCR, Vorjahr |
| P7 Coverage | 35-50 PT | — | DBA, §32b, PV, §13 |
| P8 Operations | 15-20 PT | — | CI/CD, Monitoring, VZ-Update |
| M3 Beta | 15-20 PT | 20-30 k€ | E2E-Test, Pentest, Lasttest |
| M4 Launch | 20-30 PT | 5-10 k€ | Beta-Feedback, Payment, Marketing |
| **Total** | **200-280 PT** | **35-60 k€** | 6-8 Monate bis Launch |

**PT = Personentage** (1 PT = 1 Entwickler·Tag). Bei 3-5 parallelen Entwicklern: ~6 Monate bis Launch.
