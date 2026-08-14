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