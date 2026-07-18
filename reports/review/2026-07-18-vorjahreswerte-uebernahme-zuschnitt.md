# Vorjahreswerte-Übernahme — Stufe-A-Zuschnitt (dev-2)

**Status:** concept-first, KEIN Bau (Read-only während Bash-Outage). Julius' Roadmap, meine Zone (lokal,
kein externer Dienst → kein Cap). Non-idle ohne unverifizierte Builds. LLM-frei.

## Kern: Vorjahr-Writer = symmetrischer Store-Writer zum Beleg-Writer
Der Store hat schreiber-scoped Fail-closed-Guards (store.py:127/134): `^llm:` und `^import:beleg` müssen
herkunft-passend + **vorlaeufig + signal_2=null** tragen (nie Direkt-Bestätigung). herkunft=`vorjahr`
existiert schon im Enum (store/schema.json). Die Vorjahres-Übernahme ist der DRITTE Writer desselben
Musters: `import:vorjahr` liest einen Vorjahres-Store + schreibt Werte als VORSCHLAG.

## (1) Wie liest der Store einen Vorjahres-Fall + überträgt
- Ein Vorjahres-Fall = ein persistierter Store (faelle/<id>.json, VZ n−1). materialisiere() liefert dessen
  {feld_id → wert, zustand, herkunft}.
- Übernahme = pro übertragbarem Feld ein `append_event(feld_id, wert=vorjahr_wert, zustand="vorlaeufig",
  herkunft={"herkunft":"vorjahr", …}, schreiber="import:vorjahr", signal={signal_1: vorjahr-Ref, signal_2: null})`
  in den NEUEN Store (VZ n). Wie beim Beleg-Writer: der Wert ist ein Vorschlag, der Mensch bestätigt/
  aktualisiert (Zwei-Signal, signal_2).
- **NAHT-GAP (Store): ein `import:vorjahr`-Guard fehlt** (analog import:beleg). Ohne ihn könnte ein
  Vorjahr-Writer bestaetigt schreiben → K2-Verletzung. VORSCHLAG (Store-Zone):
  ```
  if schreiber.startswith("import:vorjahr"):
      if herkunft.get("herkunft") != "vorjahr" or zustand != "vorlaeufig" or signal.get("signal_2") is not None:
          raise ValueError("fail-closed: import:vorjahr = vorlaeufig + signal_2=null, Vorjahr bestätigt nie direkt")
  ```

## (2) Welche Felder sinnvoll übertragbar
| Kategorie | Beispiele | Übernahme |
|---|---|---|
| **Stammdaten / kohorten-fix (hoch stabil)** | person_b_idnr, veranlagung, renten_art(_partner), renten_beginn_jahr(_partner), alter_bei_rentenbeginn, grad_der_behinderung(_partner), hilflos/hinterbliebenen, rentenfreibetrag(_partner), betriebsart | JA — ändern sich selten, starker Vorschlag (trotzdem vorlaeufig) |
| **Jahres-spezifische Beträge** | bruttoarbeitslohn(_partner), kap_*(_partner), vv_*, jahresrente(_partner), vor_*, agb/hh/spenden, EP/Verpflegung | JA als VORSCHLAG — Vorjahres-Betrag als Startwert, Nutzer aktualisiert (vorlaeufig zwingt die Aktualisierung) |
| **Struktur-Flags** | kein_gewinn/kap/vuv/sonstige | JA (Einkunftsstruktur meist stabil), aber vorlaeufig re-bestätigen |
| **NICHT übertragen** | VZ-abhängige Params/Konstanten (Sparer-PB, Pauschbeträge, Kohorten-%) | NEIN — das sind params/<vz>, keine Nutzer-Felder; der Ring zieht sie je VZ frisch |
| **NIE Zustand mit** | jeder übertragene Wert | NIE bestaetigt — alles vorlaeufig, Mensch bestätigt neu |

Mechanik-Vorschlag: ein Bindungs-Flag `vorjahr_uebernehmbar: true|false` je Feld (Default true für askable,
false für DERIVED/params-nahe), ODER eine Kategorie-Regel (askable + nicht-DERIVED → übertragbar). Bindungs-
Flag ist expliziter (auditierbar, wie herkunft_slots).

## (3) K2 / fail-closed
- **Vorjahr bewegt keine Zahl bis bestätigt:** alle übernommenen Werte = vorlaeufig. catala_gesamt/die Ringe
  rechnen nur über bestaetigt-Kegel (meet_zustand: Aggregat bestaetigt nur wenn ALLE Inputs bestaetigt) →
  ein vorlaeufiger Vorjahres-Wert produziert KEINEN festen Bescheid, nur die /stand-Spanne. Korrekt fail-closed.
- **Provenance sauber:** herkunft=vorjahr (nicht laie/beleg) → die UI-Badge zeigt „Vorjahr" (Enum existiert),
  der Nutzer sieht die Herkunft + kann gezielt aktualisieren. signal_1 = Vorjahr-Fall-Ref (welcher VZ/Fall).

## Bau-Umfang nach OK (meine Zone + 1 Store-Naht)
1. produkt/store/store.py: import:vorjahr-Guard (Store-Zone — meine oder dev-1? flag).
2. produkt/import/vorjahr_writer.py (NEU, analog beleg_writer): liest Vorjahres-Store + append_event je
   übertragbarem Feld (herkunft=vorjahr, vorlaeufig, signal_2=null).
3. bindung: vorjahr_uebernehmbar-Flag je Feld (falls Flag-Mechanik statt Kategorie-Regel).
4. Tests: Übernahme→alle vorlaeufig, kein bestaetigt-Durchschlag, Guard feuert bei Direkt-Bestätigung,
   nicht-übertragbare Felder ausgelassen.

## Zur Abnahme
(1) Flag-Mechanik (`vorjahr_uebernehmbar` je Feld) oder Kategorie-Regel (askable+nicht-DERIVED)?
(2) import:vorjahr-Guard = meine Store-Zone oder dev-1? (3) Struktur-Flags (kein_*) + veranlagung
übertragen oder je Jahr frisch fragen? (4) signal_1-Vorjahr-Ref-Form (VZ+Fall-id)? → dann Bau.
