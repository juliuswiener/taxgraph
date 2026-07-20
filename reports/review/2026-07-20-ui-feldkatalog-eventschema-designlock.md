# UI-Runde (Julius #3) — Feld-Katalog + Event-Schema + Sicherheits-Golden — Design-Lock (dev-2, 2026-07-20)

Read-only Deklarations-Seite der UI-Runde (LLM-Chat + Politur + externe Dienste). Der Zwei-Signal-Vertrag =
sicherheits-kritische Hälfte. KEIN Bau. Gegründet auf der EXISTIERENDEN Store/Mapping-Architektur (nicht erfunden).

## 0. ⭐ KERN-BEFUND: der K2-Invariant ist SCHON DOPPELT strukturell enforced
Die UI-Runde fügt WRITER + einen FELD-KATALOG hinzu — NICHT den Kern-Invariant (der steht):
1. **Store-Ebene** (`produkt/store/store.py` append_event): JEDER Vorschlags-Schreiber ist fail-closed auf
   `zustand=vorlaeufig` + `signal_2=null` gezwungen — SCHREIBER-scoped Präfixe: `llm:` (herkunft=llm_vorschlag),
   `import:beleg` (beleg_import), `import:vorjahr` (vorjahr), `import:kontoauszug` (kontoauszug), `berechnet:`
   (berechnet — externe Dienste/Maps). `bestaetigt` braucht STRUKTURELL `signal_2` (Z.175). Die KI/der externe
   Dienst kann NIE direkt bestätigen.
2. **Mapping-Ebene** (`produkt/mapping/est_mapping.py` Z.141/211/227/241): `if zustand != "bestaetigt" →
   unvollstaendig` (nicht deklariert). Ein vorläufiger Wert fließt STRUKTURELL NICHT in die Steuer-Summe.
→ Ein LLM/externer Wert bewegt die Steuer NIE ohne menschliches signal_2 — **doppelt** (Store + Mapping),
defense-in-depth. Die UI-Runde muss diesen Invariant nur ERHALTEN, nicht neu bauen.

## 1. FELD-KATALOG — welche Felder darf ein Vorschlags-Schreiber überhaupt VORSCHLAGEN?
Der Store zwingt vorläufig, restringiert aber NICHT welches FELD ein Schreiber schreibt (nur der
kontoauszug-Writer scoped via `KATEGORIE_FELD`). → NEUES Element: per-Feld-Whitelist, **DEFAULT = HUMAN-ONLY
(fail-closed)**. Ein Feld ist vorschlagbar NUR wenn explizit gelistet. Mechanik-Vorschlag: bindung-Feld
`vorschlagbar_von: [kontoauszug|beleg|maps|llm]` (absent/[] = human-only), Drift-Wächter kreuzprüft.

### ⭐ GENERAL-PRINZIP (Instructor-adjudiziert 2026-07-20, verbindliches Katalog-Kriterium)
**SUGGESTIBLE = ableitbarer DOKUMENT-BETRAG** (Beleg/Auszug/Adresse → Zahl; die KI liest/rechnet, der Mensch
bestätigt). **HUMAN-ONLY = Klassifikation / Wahlrecht / Status / Identität / ABWESENHEITS-Erklärung /
ALLOKATIONS-Urteil.** Ein halluziniertes „du hast keine Kapitaleinkünfte" (Abwesenheit) = Under-Deklaration =
Under-tax + Haftung → KI/Dienst darf eine Abwesenheit NIE behaupten. Eine Rechts-Klassifikation (Gewerbe vs
Selbständig) oder Allokation (welcher §20-Topf) = Urteil, kein Dokument-Betrag → Mensch.

### 1a. SUGGESTIBLE (Betrag/Quantität aus Beleg/Auszug/Adresse ableitbar)
| Feld(er) | Quelle | Begründung |
|---|---|---|
| hh_handwerker_arbeitskosten / hh_dienstleistungen / hh_minijob_aufwendungen (§35a) | kontoauszug/beleg | schon in KATEGORIE_FELD; Rechnungsbetrag |
| spenden_betrag (§10b) | kontoauszug/beleg | Zuwendungsbeleg |
| vor_rv_ausserhalb_lstb (§10 Vorsorge) | kontoauszug/beleg | Beitragsbescheinigung |
| ep_entfernung_km (§9 Entfernung) | **maps (berechnet:maps)** | aus Wohnung+Tätigkeitsstätte via ors_client berechnet |
| bruttoarbeitslohn, kap_kapitalertraege, gewst_messbetrag | beleg (OCR) | Lohnsteuerbesch./Steuerbesch./Messbescheid |
| agb_aufwendungen, berufsausbildung_aufwendungen, betriebseinnahmen/-ausgaben, afa_jahresbetrag | beleg/llm | Beleg-Beträge |
Kriterium: **quantitativer BETRAG, aus einem Dokument/Auszug/einer Adresse plausibel ableitbar** — die KI
liest/rechnet, der Mensch bestätigt.

### 1b. HUMAN-ONLY (nie LLM/extern — Wahlrecht/Rechts-Status/Identität)
| Kategorie | Felder | Begründung |
|---|---|---|
| **Wahlrechte/Anträge** | veranlagung (§26), antrag_ermaessigter_satz (§34 Abs.3), am_gwg_sofortabzug_gewaehlt (§6 Abs.2), kap_zusammenveranlagung | rechtsgestaltende Wahl — nur der Mensch |
| **einmal-im-Leben/Status** | ermaessigung_einmal_genutzt (§34 Abs.3 S.4), dauernd_berufsunfaehig (SV-Status), fam_alleinstehend (§24b) | Rechts-Status/lebenslange Entscheidung |
| **Identität** | geburtsjahr(_partner), person_b_idnr | identitäts-kritisch, KI darf nie raten |
| **Kinder-Zuordnung** | fam_kinder_beruecksichtigt, fam_kinder_im_haushalt, fam_anzahl_kinder | Kindschaftsverhältnis = rechtl. Zuordnung |
| **Abwesenheits-Behauptung** | kein_gewinn/kein_kap/kein_vuv/kein_sonstige | ASSERTION „keine Einkünfte Art X" = rechtl. Erklärung; KI darf Abwesenheit nie behaupten (K2) |
| **Tatbestands-Voraussetzung** | dhf_* (doppelte Haushaltsf.), agb_notwendig_angemessen/agb_zwangslaeufig | rechtl. Subsumtion (zwangsläufig/notwendig) = Mensch |
| **art-Verzweigung** | gewinn_betriebsart, rentner_veraeusserungs_betriebsart | Gewerbe/Selbständig/LuF = rechtl. Qualifikation (steuert Anlage-Kz + §35-Folge) → HUMAN-ONLY (Instructor-adjudiziert 2026-07-20: Rechts-Klassifikation, kein Beleg-Betrag) |
| **§20-Allokation (Töpfe)** | kap_gewinn_aktien(_partner), kap_gewinn_sonstige(_partner), kap_verlust_aktien(_partner), kap_verlust_sonstige(_partner) | die §20 Abs.2-Töpfe (Aktien vs sonstige) + Verlustverrechnung (§20 Abs.6) = ALLOKATIONS-Urteil, kein Dokument-Betrag → HUMAN-ONLY |

✅ **kap_*-ADJUDIKATION (Instructor 2026-07-20): Dokument-Betrag vs Semantik-Allokation trennen.**
- **kap_kapitalertraege(_partner) = SUGGESTIBLE** (beleg-OCR): die TOTAL-Zeile der Steuerbescheinigung (§20 Abs.1,
  wie bruttoarbeitslohn aus der LStB) — ein Dokument-Betrag, vorläufig+Hold-Confirm.
- **kap_gewinn/verlust_aktien/sonstige (+_partner) = HUMAN-ONLY** (s. Tabelle): die Töpfe/Allokation = Urteil.

## 2. EVENT-SCHEMA (existiert — dokumentiert + neue Writer bestätigt)
Zwei-Signal-Event (append_event): `{feld_id, wert, zustand, herkunft:{herkunft,pruef_tiefe,haftung}, schreiber,
signal:{signal_1,signal_2}, ersetzt}`. **SCHREIBER-scoped Provenance-Guard** (^präfix, [[produkt-beleg-writer]]):
| Writer (UI-Runde) | schreiber-Präfix | herkunft-Enum | Status |
|---|---|---|---|
| LLM-Chat | `llm:<rolle>` | llm_vorschlag | EXISTIERT (Store-Gate Z.127) |
| externe Dienste (Entfernung/Maps) | `berechnet:maps` | berechnet | EXISTIERT (Store-Gate Z.167) |
| Kontoauszug | `import:kontoauszug` | kontoauszug | EXISTIERT (Writer + Gate) |
| Beleg-OCR | `import:beleg` | beleg_import | EXISTIERT |
Alle vorläufig-erzwungen. **Neu nur: der Feld-Katalog-Check** (Writer prüft `feld_id ∈ vorschlagbar_von[schreiber-typ]`
VOR append_event; sonst fail-closed „Feld nicht vorschlagbar"). Kein neues herkunft-Enum nötig.

## 3. GOLDEN-TEST-PLAN (Sicherheits-Invariant — der Kern)
| # | Test | Beweist |
|---|---|---|
| S1 | llm:/berechnet:/kontoauszug schreibt spenden_betrag vorläufig → est_mapping: spenden in `unvollstaendig`, NICHT deklariert → Steuer-Summe UNVERÄNDERT | Vorschlag bewegt die Steuer NICHT (Mapping-Ebene) |
| S2 | Mensch bestätigt (signal_2) → zustand=bestaetigt → est_mapping deklariert → Steuer-Summe ÄNDERT sich | Zwei-Signal-Übergang korrekt (der Mensch ist das 2. Signal) |
| S3 | llm:-Writer versucht zustand=bestaetigt ODER signal_2≠null → `ValueError` (Store fail-closed A) | KI kann NIE direkt bestätigen |
| S4 | berechnet:maps/import:kontoauszug dito bestaetigt-Versuch → ValueError | alle Vorschlags-Schreiber symmetrisch fail-closed |
| S5 | LLM/externer Writer versucht HUMAN-ONLY-Feld (veranlagung/antrag_ermaessigter_satz/kein_kap) → fail-closed „nicht vorschlagbar" (Feld-Katalog) | KI schlägt kein Wahlrecht/keine Abwesenheit vor |
| S6 | E2E: llm-Vorschlag spenden 5000 → /ergebnis-est == ohne-spenden-est (unverändert); dann Confirm → est sinkt | End-to-End Steuer-Summen-Invariant unter der echten Ring-Rechnung |
| S7 | Provenance-Guard: import:elster darf beleg_import NICHT schreiben (schreiber-scoped ^import:beleg, nicht herkunft-scoped) — [[produkt-beleg-writer]]-Lehre | Guard-Scope korrekt |

## Fazit
Der K2-Kern (Vorschlag bewegt Steuer nie ohne Mensch-signal_2) steht schon STRUKTURELL doppelt (Store+Mapping) —
die UI-Runde ERHÄLT ihn. NEU: (1) Feld-Katalog `vorschlagbar_von` (default human-only=fail-closed) — verhindert
dass KI Wahlrechte/Status/Abwesenheit/Identität vorschlägt; (2) Feld-Katalog-Check im Writer VOR append_event;
(3) 7 Sicherheits-Goldens (S1-S7) die den Invariant beweisen. Writer-Schema + Provenance-Guard existieren.
Instructor-Grenzfälle: gewinn_betriebsart + kap_*-Suggestibilität. Deklarations-Seite; Haut-Build = dev-1.
