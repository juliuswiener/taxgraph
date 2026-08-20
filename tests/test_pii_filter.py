"""PII-Filter-Tests (P1.5).

- Leck-Test: Freitext mit IdNr + IBAN → gefiltert im LLM-Call
- Audit-Test: Audit-Eintrag vorhanden, Freitext NICHT drin
- Fehlalarm-Test: Steuer-Freitext ohne PII bleibt wortgleich (Cent-Beträge intakt)
- Naht-Mutationsprobe: Filter im echten /chat-Pfad verdrahtet
"""

from __future__ import annotations

import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser",
             "produkt/import", "produkt/mapping", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api_llm
import audit as AUDIT
from pii_filter import filtere


# --------------------------------------------------------------- Filter-Einheit
class TestFiltere:
    def test_leer(self):
        assert filtere("") == ("", [])
        assert filtere(None) == (None, [])  # noqa

    def test_steuer_id_wird_ersetzt(self):
        t, k = filtere("Meine IdNr ist 12345678901")
        assert "[PII]" in t
        assert "steuer_id" in k

    def test_steuer_id_mit_leerzeichen(self):
        t, k = filtere("IdNr 12 345 678 901")
        assert "[PII]" in t
        assert "steuer_id" in k

    def test_iban_wird_ersetzt(self):
        t, k = filtere("IBAN: DE12500105170648489890")
        assert "[PII]" in t
        assert "iban" in k

    def test_iban_vor_steuer_id(self):
        """IBAN enthält 11 Ziffern — IBAN muss vor steuer_id prüfen."""
        t, k = filtere("DE12500105170648489890")
        assert "iban" in k
        # steuer_id darf NICHT getroffen sein (die 11 Ziffern sind Teil der IBAN)
        # Nachdem IBAN ersetzt wurde, ist der Match-Weg für steuer_id weg
        assert "steuer_id" not in k, "IBAN enthält 11 Ziffern — steuer_id darf nicht separat treffen"

    def test_datum_wird_ersetzt(self):
        t, k = filtere("Geboren am 15.03.1985")
        assert "[PII]" in t
        assert "datum" in k

    def test_plz_ort_wird_ersetzt(self):
        t, k = filtere("Wohnhaft in 12345 Berlin")
        assert "[PII]" in t
        assert "plz_ort" in k

    def test_strasse_wird_ersetzt(self):
        t, k = filtere("Musterstraße 12")
        assert "[PII]" in t
        assert "strasse" in k

    def test_strasse_mit_abkuerzung(self):
        t, k = filtere("Hauptstr. 7")
        assert "[PII]" in t
        assert "strasse" in k

    def test_anrede_name_wird_ersetzt(self):
        t, k = filtere("Mein Name ist Herr Müller")
        assert "[PII]" in t
        assert "anrede_name" in k

    def test_euro_betrag_bleibt(self):
        """Cent-Vorschläge: Geldbeträge dürfen nicht gefiltert werden."""
        t, k = filtere("ich habe 500 Euro für Kinderbetreuung gezahlt")
        assert "500" in t and "Euro" in t
        assert not k, f"unerwartete Kategorien: {k}"

    def test_paragraph_bleibt(self):
        t, k = filtere("nach §35a EStG")
        assert "§35a" in t
        assert not k

    def test_steuerbegriff_riester_bleibt(self):
        """Eigennamen wie Riester dürfen nicht als Name gefiltert werden."""
        t, k = filtere("Riester-Beitrag 2100 Euro")
        assert "Riester" in t
        assert not k


# ------------------------------------------------------- Fehlalarm PLZ_ORT
class TestPlzOrt:
    def test_fuenfstelliger_betrag_mit_euro_bleibt(self):
        """45000 Euro darf nicht als PLZ+Ort gefiltert werden."""
        t, k = filtere("Ich verdiene 45000 Euro im Jahr")
        assert "45000" in t and "Euro" in t
        assert "plz_ort" not in k, f"plz_ort fälschlich getroffen: {k}"

    def test_betrag_mit_eur_bleibt(self):
        """12000 EUR (all caps) darf nicht als PLZ+Ort gefiltert werden."""
        t, k = filtere("Spende 12000 EUR an Verein")
        assert "12000" in t and "EUR" in t
        assert "plz_ort" not in k

    def test_betrag_mit_eurozeichen_bleibt(self):
        """55000 € darf nicht als PLZ+Ort gefiltert werden."""
        t, k = filtere("Bruttolohn 55000 €")
        assert "55000" in t and "€" in t
        assert "plz_ort" not in k

    def test_betrag_mit_kilometer_bleibt(self):
        """50000 Kilometer — Kilometer ist groß, darf nicht als PLZ+Ort
        gefiltert werden (keine Währung, aber kein Ort)."""
        t, k = filtere("Ich bin 50000 Kilometer gefahren")
        assert "50000" in t and "Kilometer" in t
        assert "plz_ort" not in k

    def test_echte_plz_wird_weiterhin_gefiltert(self):
        """80331 München muss weiterhin erkannt werden."""
        t, k = filtere("Wohnhaft in 80331 München")
        assert "[PII]" in t
        assert "plz_ort" in k


# --------------------------------------------------------------- Leck-Test: PII im ausgehenden Call
class TestLeak:
    def test_pii_entfernt_vor_llm_call(self, monkeypatch, tmp_path):
        """Monkeypatch llm_client.complete, Argument einfangen, Freitext MIT IdNr + IBAN
        durch _llm_dialog schicken, assert Roh-IdNr im messages-Objekt NICHT vorkommt."""
        monkeypatch.setattr(AUDIT, "AUDIT_DIR", str(tmp_path))
        captured = {}

        def fake_complete(role, messages, fixture_id=None, schema=None):
            captured["role"] = role
            captured["messages"] = messages
            from llm_client import Completion
            return Completion(text='[]')

        # llm_client.complete monkeypatchen (nicht api_llm.llm_client)
        import llm_client
        monkeypatch.setattr(llm_client, "complete", fake_complete)

        # _llm_dialog aufrufen — muss PII rausfiltern VOR complete
        freitext = "Meine IdNr ist 12345678901 und IBAN DE12500105170648489890"
        result = api_llm._llm_dialog(freitext, [], user_id="test")

        assert "messages" in captured
        msgs = captured["messages"]
        msg_text = json.dumps(msgs)

        # Roh-IdNr darf nicht vorkommen
        assert "12345678901" not in msg_text, \
            f"Roh-IdNr im Call-Argument gefunden: {msg_text}"
        # Roh-IBAN darf nicht vorkommen
        assert "DE12500105170648489890" not in msg_text, \
            f"Roh-IBAN im Call-Argument gefunden: {msg_text}"
        # Platzhalter MUSS vorkommen
        assert "[PII]" in msg_text, \
            f"Platzhalter nicht im Call-Argument: {msg_text}"


# --------------------------------------------------------------- Audit-Test
class TestAudit:
    def test_audit_eintrag_vorhanden(self, monkeypatch, tmp_path):
        """Nach dem Call steht ein Eintrag in audit.jsonl, Freitext NICHT drin."""
        # Audit in tmp_path umleiten
        monkeypatch.setattr(AUDIT, "AUDIT_DIR", str(tmp_path))

        import llm_client
        monkeypatch.setattr(llm_client, "complete",
                            lambda role, messages, fixture_id=None, schema=None: llm_client.Completion(text='[]'))

        freitext = "Meine IdNr ist 12345678901"
        api_llm._llm_dialog(freitext, [], user_id="audit_test")

        # Audit-Eintrag prüfen
        eintraege = AUDIT.lies()
        assert len(eintraege) >= 1

        chat_calls = [e for e in eintraege if e.get("action") == "llm_call"]
        assert len(chat_calls) >= 1, f"Kein llm_call-Eintrag: {eintraege}"

        e = chat_calls[-1]
        # Freitext (roh oder gefiltert) darf NICHT im Detail sein
        assert "12345678901" not in json.dumps(e), "Roh-IdNr im Audit-Eintrag"
        assert "[PII]" not in json.dumps(e), "Gefilterter Text im Audit-Eintrag"
        # Detail muss Metadaten enthalten
        d = e.get("detail", "")
        assert "pii_kategorien" in d, f"pii_kategorien fehlt: {d}"
        assert "textlaenge_vor" in d, f"textlaenge_vor fehlt: {d}"
        assert "12345678901" not in d, "Roh-IdNr im Detail"


# --------------------------------------------------------------- Fehlalarm-Test
class TestNoFalsePositives:
    def test_steuertext_ohne_pii(self, monkeypatch):
        """Steuer-Freitext ohne PII bleibt wortgleich durch _llm_dialog."""
        import llm_client
        monkeypatch.setattr(llm_client, "complete",
                            lambda role, messages, fixture_id=None, schema=None: llm_client.Completion(text='[]'))

        freitext = "Ich habe 500 Euro für Kinderbetreuung gezahlt und 1200 Euro für ein Handwerkerleistung nach §35a"
        _, kategorien = filtere(freitext)
        assert not kategorien, f"Fehlalarm: {kategorien}"

        # Durch den ganzen Pfad — der Prompt-Bau ändert den Text nicht
        # (Chat-Prompt wickelt system + user, aber der user-Teil bleibt original)
        gefiltert, k = filtere(freitext)
        assert k == [], f"Fehlalarm im Pfad: {k}"
        assert "500" in gefiltert
        assert "1200" in gefiltert
        assert "§35a" in gefiltert


# --------------------------------------------------------------- Naht-Mutationsprobe
class TestNaht:
    def test_filter_ist_im_echten_pfad_verdrahtet(self, monkeypatch):
        """Setzt raise RuntimeError('NAHT') am Anfang von filtere — der Test
        muss über den echten /chat-Pfad (api.chat) fallen, nicht nur pii_filter.py.
        Fällt nur test_pii_filter.py, ist die Naht ungetestet."""
        # api_llm importiert filtere direkt (from pii_filter import filtere)
        # → api_llm.filtere monkeypatchen, nicht pii_filter.filtere
        def kaputt(text):
            raise RuntimeError("NAHT — Filter im Pfad bestätigt")

        monkeypatch.setattr(api_llm, "filtere", kaputt)

        # _llm_dialog muss filtere aufrufen → NAHT
        import llm_client
        monkeypatch.setattr(llm_client, "complete",
                            lambda role, messages, fixture_id=None, schema=None: llm_client.Completion(text='[]'))

        with pytest.raises(RuntimeError, match="NAHT"):
            api_llm._llm_dialog("Hallo Welt", [], user_id="test")


# --------------------------------------------------------------- Kategorien-Reihenfolge
class TestReihenfolge:
    def test_iban_und_steuer_id_im_gemischten_text(self):
        """IBAN + separate Steuer-ID: beide werden korrekt erkannt."""
        t, k = filtere("IBAN DE12500105170648489890 und Steuer-ID 12345678901")
        assert "iban" in k
        assert "steuer_id" in k


# =============================================================================================
# PROBENTABELLEN (Audit pii-filter-chat-pfad-dieselben-loecher, 2026-08-20)
#
# WARUM ZUSÄTZLICH ZU DEN EINZELTESTS OBEN: die Einzeltests prüfen je eine SCHREIBWEISE pro
# Klasse — die, die beim Bau des Musters im Kopf war. Sie waren am 2026-08-20 alle grün, während
# ZEHN von siebzehn Proben unmaskiert an den LLM-Provider gingen: jede IBAN in Kleinschrift, jede
# ausländische IBAN mit Buchstaben im Rumpf, die Bindestrich-Schreibweise, die 13-stellige
# Steuernummer in beiden Formen, und Konto- wie Kartennummern, für die es hier gar keine Regel
# gab. Ein grüner Einzeltest belegt die Schreibweise, die er nennt, und nichts sonst.
#
# Die TABELLE ist der Punkt: sie wächst beim nächsten Muster-Zusatz mit, statt dass ein neues
# Muster einen neuen Einzeltest bekommt und die alten Klassen ungeprüft danebenliegen. Bauart
# übernommen von tests/test_kontoauszug_maskierung.py, das am selben Tag für den anderen Pfad
# entstand.
#
# WAS DIESER PFAD VERLÄSST: `_llm_dialog` gibt gefilterten Freitext UND gefilterten Kontext (die
# schon bestätigten Angaben) an einen externen LLM-Provider. Ein Loch ist kein Rechenfehler, den
# eine spätere Korrektur heilt — es ist ein Datenabfluss an einen Dritten (DSGVO Art. 6, ein
# Auftragsverarbeitungsvertrag mit dem Anbieter existiert nicht).
#
# KEINE ECHTEN DATEN, KEIN LLM-AUFRUF. Alle Werte erfunden, die IBAN ist die öffentlich bekannte
# Testnummer. `filtere` ist reine Textverarbeitung.
# =============================================================================================

# (bezeichnung, klartext, geheimnisse die NICHT überleben dürfen)
MASKIERUNGS_PROBEN = [
    # ---- IBAN (öffentliche Testnummer DE89 3704 0044 0532 0130 00) ----
    ("iban_gross_kompakt", "Zahlung an DE89370400440532013000 Malermeister",
     ["370400440532013000"]),
    ("iban_gross_gruppiert", "Miete DE89 3704 0044 0532 0130 00 Wohnung",
     ["0044", "0532", "0130"]),
    # Wer in einen Chat tippt, tippt klein — und Bank-Exporte mehrerer Institute ebenso.
    ("iban_klein_kompakt", "Zahlung an de89370400440532013000 Malermeister",
     ["370400440532013000"]),
    ("iban_klein_gruppiert", "Miete de89 3704 0044 0532 0130 00 Wohnung",
     ["0044", "0532", "0130"]),
    ("iban_gemischt", "Miete De89 3704 0044 0532 0130 00 Wohnung",
     ["0044", "0532", "0130"]),
    # NL/GB führen Buchstaben IM Rumpf — die alte `\d`-Regel brach daran ab.
    ("iban_ausland_buchstaben", "Rechnung NL91 ABNA 0417 1643 00 Beratung",
     ["ABNA", "0417", "1643"]),
    ("iban_ausland_klein", "Rechnung nl91 abna 0417 1643 00 Beratung",
     ["abna", "0417", "1643"]),
    ("iban_bindestrich", "Zweck DE89-3704-0044-0532-0130-00 Miete",
     ["0044", "0532", "0130"]),

    # ---- IdNr (11-stellig, § 139b AO) ----
    ("idnr_kompakt", "Meine IdNr ist 12345678901", ["12345678901"]),
    ("idnr_gruppiert", "IdNr 12 345 678 901", ["345", "678", "901"]),

    # ---- Steuernummer, Landesform 11-stellig ----
    ("stnr_slash", "Steuernummer 151/815/08154 Finanzamt", ["151/815/08154", "08154"]),
    ("stnr_leerzeichen", "Steuernummer 151 815 08154 Finanzamt", ["08154"]),
    ("stnr_kompakt", "Steuernummer 15181508154 Finanzamt", ["15181508154"]),

    # ---- Steuernummer, bundeseinheitlich 13-stellig ----
    # Die kompakte Form entkam der alten `{10}`-Regel nicht trotz, sondern WEGEN ihrer Länge:
    # ein Muster für genau 11 Ziffern findet in einem 13-stelligen Lauf keine Wortgrenze.
    ("stnr_bund_kompakt", "Steuernummer 3012081543211 Finanzamt", ["3012081543211"]),
    ("stnr_bund_gruppiert", "Steuernummer 3012 0815 4321 1 Finanzamt", ["0815", "4321"]),

    # ---- Lange Ziffernläufe: dieser Pfad hatte dafür GAR KEINE Regel ----
    ("kontonummer", "Lastschrift Konto 1234567890 Beitrag", ["1234567890"]),
    ("kartennummer", "Karte 4111111111111111 belastet", ["4111111111111111"]),
]


# (bezeichnung, klartext, was überleben MUSS)
#
# Ohne diese Tabelle ist das Gate trivial erfüllbar — `return "[PII]", []` bestünde jede Zeile
# oben. Hier wiegt das schwerer als im Kontoauszug-Pfad: `api_llm._beleg_geprueft` prüft die
# Zitate des Modells gegen den GEFILTERTEN Text. Was hier zu viel maskiert wird, kann das Modell
# nicht mehr wörtlich zitieren, und sein Vorschlag fällt belegfrei durch — der Nutzer sieht
# nicht "maskiert", er sieht "kein Vorschlag".
DURCHLASS_PROBEN = [
    ("betrag_klein", "ich habe 500 Euro für Kinderbetreuung gezahlt", ["500", "Euro"]),
    ("betrag_gross", "Ich verdiene 45000 Euro im Jahr", ["45000"]),
    ("betrag_dezimal", "Rechnung über 1.234,56 EUR", ["1.234,56"]),
    ("betrag_cent_getrennt", "Spende 12000 EUR an Verein", ["12000"]),
    ("paragraph", "nach §35a EStG", ["35a", "EStG"]),
    ("kilometer", "Ich bin 50000 Kilometer gefahren", ["50000"]),
    ("kurze_zahl", "Rechnung 4711 Position 12 Heizung", ["4711", "Heizung"]),
    ("steuerbegriff", "Riester-Beitrag 2100 Euro", ["Riester", "2100"]),
]


class TestProbentabellen:
    @pytest.mark.parametrize("bezeichnung,klartext,geheimnisse", MASKIERUNGS_PROBEN,
                             ids=[p[0] for p in MASKIERUNGS_PROBEN])
    def test_kein_geheimnis_ueberlebt(self, bezeichnung, klartext, geheimnisse):
        gefiltert, kategorien = filtere(klartext)
        for geheim in geheimnisse:
            assert geheim not in gefiltert, (
                f"[{bezeichnung}] '{geheim}' steht unmaskiert im Text, der an den externen "
                f"LLM-Provider geht: {gefiltert!r}")
        assert kategorien, (
            f"[{bezeichnung}] keine Kategorie gemeldet — das Audit protokolliert dann eine "
            f"saubere Weste für einen Text, der PII trug: {gefiltert!r}")

    @pytest.mark.parametrize("bezeichnung,klartext,kontext", DURCHLASS_PROBEN,
                             ids=[p[0] for p in DURCHLASS_PROBEN])
    def test_uebermaskierung_frisst_den_sachverhalt_nicht(self, bezeichnung, klartext, kontext):
        """Übermaskierung ist kein sicherer Zustand, sondern ein anderer Fehler."""
        gefiltert, kategorien = filtere(klartext)
        for k in kontext:
            assert k in gefiltert, (
                f"[{bezeichnung}] '{k}' wurde mitmaskiert — das Modell kann den Wert nicht mehr "
                f"zitieren, sein Vorschlag fällt am Beleg-Gate durch: {gefiltert!r}")
        assert not kategorien, f"[{bezeichnung}] Fehlalarm {kategorien}: {gefiltert!r}"

    def test_mehrere_geheimnisse_in_einer_zeile(self):
        """Eine echte Nachricht trägt oft beides. Die Muster dürfen sich nicht gegenseitig
        aufheben — IBAN muss VOR der Ziffernregel greifen, sonst frisst diese den IBAN-Rumpf und
        hinterlässt einen Rest, der wie ein unverdächtiges Fragment aussieht."""
        gefiltert, kategorien = filtere(
            "Überweisung DE89 3704 0044 0532 0130 00 Steuernummer 151/815/08154, "
            "davon 500 Euro Spende")
        assert "0532" not in gefiltert and "0130" not in gefiltert
        assert "08154" not in gefiltert
        assert "iban" in kategorien and "steuer_id" in kategorien
        assert "500" in gefiltert and "Spende" in gefiltert, (
            f"der Sachverhalt ist mitmaskiert worden: {gefiltert!r}")


# --------------------------------------------------------------------------------------------
# DIE NAHT. Alles oben prüft `filtere()` für sich. `TestNaht` oben belegt, dass die Funktion im
# Pfad GERUFEN wird — nicht, was am anderen Ende ankommt. Hier wird das Argument gemessen, das
# `llm_client.complete` tatsächlich sieht, und zwar für BEIDE Eingänge: Freitext und Kontext.
# Der Kontext ist der unauffälligere von beiden — er trägt die schon bestätigten Angaben, also
# genau die Stammdaten, die der Nutzer nie in den Chat getippt hat.
# --------------------------------------------------------------------------------------------
_SCHMUTZIG = "de89 3704 0044 0532 0130 00"          # Kleinschrift: die Klasse, die durchging


class TestNahtGegenDenProvider:
    def _gesehener_prompt(self, monkeypatch, tmp_path, freitext, kontext=""):
        import llm_client
        monkeypatch.setattr(AUDIT, "AUDIT_DIR", str(tmp_path))   # nie ins echte audit.jsonl
        gesehen = []

        def stub(role, messages, fixture_id=None, schema=None):
            gesehen.append(json.dumps(messages, ensure_ascii=False))
            return llm_client.Completion(text="[]")

        monkeypatch.setattr(llm_client, "complete", stub)
        api_llm._llm_dialog(freitext, [], kontext=kontext, user_id="naht_test")
        assert gesehen, "llm_client.complete wurde gar nicht gerufen"
        return gesehen[-1]

    def test_freitext_verlaesst_das_haus_maskiert(self, monkeypatch, tmp_path):
        prompt = self._gesehener_prompt(monkeypatch, tmp_path, f"Meine IBAN ist {_SCHMUTZIG}")
        assert "0532" not in prompt and "0130" not in prompt, (
            f"die IBAN geht unmaskiert an den externen Provider: {prompt!r}")

    def test_kontext_verlaesst_das_haus_maskiert(self, monkeypatch, tmp_path):
        """Der Kontext trägt bestätigte Stammdaten — er ist der zweite, eigene Ausgang.
        Einen von beiden zu heilen und den anderen zu vergessen ist der Normalfall des Fehlers."""
        prompt = self._gesehener_prompt(monkeypatch, tmp_path, "Was fehlt noch?",
                                        kontext=f"Bankverbindung: {_SCHMUTZIG}")
        assert "0532" not in prompt and "0130" not in prompt, (
            f"die IBAN geht über den KONTEXT unmaskiert an den externen Provider: {prompt!r}")


# --------------------------------------------------------------------------------------------
# DOKUMENTIERTE GRENZE — kein Versehen, eine Entscheidung.
# --------------------------------------------------------------------------------------------

def test_grenze_freie_personennamen_werden_nicht_erkannt():
    """FREIE PERSONENNAMEN WERDEN NICHT MASKIERT — nur „Herr/Frau + Wort".

    Ein Namensmuster gibt es nicht. Jede Heuristik, die "Anna Musterfrau" fängt, lässt
    "MUSTERFRAU, Anna" und "A. Musterfrau" durch und trifft zugleich Steuerbegriffe
    ("Riester", "Rürup", "Ehegatte") sowie Firmennamen, aus denen das Modell den Sachverhalt
    liest. Sie würde gleichzeitig zu wenig schützen und zu viel zerstören — und dabei aussehen,
    als wäre das Problem gelöst. Diese Vortäuschung ist teurer als die offene Flanke, weil sie
    die Frage von der Tagesordnung nimmt.

    Gemessen 2026-08-19 (Audit gdpr-art9-und-drittdaten-an-llm): bei der gepflegten Person
    wurden Anschrift und Geburtsdatum maskiert, der NAME blieb stehen. Die belastbare Abhilfe
    liegt eine Ebene höher: Auftragsverarbeitungsvertrag, oder lokales Modell.

    WIRD DIESER TEST ROT, weil jemand eine Namenserkennung gebaut hat: das ist erlaubt. Dann
    diesen Test durch echte Proben ersetzen (Vor-/Nachname, umgestellt, Grossschrift, abgekürzt)
    und die Gegenprobe mitliefern, dass Steuerbegriffe stehen bleiben. Nicht die Zeile anpassen.
    """
    gefiltert, _ = filtere("Ich pflege meine Mutter Anna Musterfrau")
    assert "Musterfrau" in gefiltert, (
        "Es gibt jetzt offenbar eine Namenserkennung — siehe Docstring, dieser Test will durch "
        "echte Proben ersetzt werden, nicht angepasst.")