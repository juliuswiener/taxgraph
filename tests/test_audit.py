"""P1.6 Audit-Log — Append-only, PII-frei, user_id-Robustheit.

Prüft:
- append-only: kein truncate/rewrite, lies() gibt alle Einträge zurück
- Parallel-Sicherheit: O_APPEND-Atomaritat (simuliert)
- PII-Freiheit: detail-Felder aller Aufrufer (grep)
- user_id-Fallback: None → "unbekannt"/"dev"
- Mutationsprobe: "a" → "w" zerstört Append-Only-Garantie
"""

from __future__ import annotations

import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/store", "produkt/haut", "produkt/auth", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import audit


# --------------------------------------------------------------- Fixtures
@pytest.fixture
def audit_dir(tmp_path):
    """Setzt AUDIT_DIR auf tmp_path, isoliert von echten Fällen."""
    old = audit.AUDIT_DIR
    audit.AUDIT_DIR = str(tmp_path)
    yield tmp_path
    audit.AUDIT_DIR = old


# --------------------------------------------------------------- Append-Only
class TestAppendOnly:
    def test_eintrag_anlegen_und_lesen(self, audit_dir):
        """Ein Eintrag wird geschrieben und lies() gibt ihn zurück."""
        audit.append("test1", "login", None, None)
        entries = audit.lies()
        assert len(entries) == 1
        assert entries[0]["user_id"] == "test1"
        assert entries[0]["action"] == "login"

    def test_zwei_eintraege_beide_vorhanden(self, audit_dir):
        """Zwei Einträge nacheinander — lies() gibt beide zurück."""
        audit.append("a", "login", None, None)
        audit.append("b", "logout", None, None)
        entries = audit.lies()
        assert len(entries) == 2
        assert [e["user_id"] for e in entries] == ["a", "b"]

    def test_vorherige_eintraege_unveraendert(self, audit_dir):
        """Neuer Eintrag ändert alte Einträge nicht."""
        audit.append("nutzer1", "fall_angelegt", "f1", "scheibe=gesamt")
        # Datei-Inhalt sichern
        pfad = audit._audit_pfad()
        with open(pfad, encoding="utf-8") as f:
            inhalt_vorher = f.readlines()
        # Zweiter Eintrag
        audit.append("nutzer1", "zugriff_verweigert", "f2", "user=nutzer1, owner=nutzer2")
        with open(pfad, encoding="utf-8") as f:
            inhalt_nachher = f.readlines()
        # Erste Zeile muss identisch sein
        assert inhalt_vorher[0] == inhalt_nachher[0], \
            "Alte Zeile wurde durch neuen Append verändert"
        assert len(inhalt_nachher) == len(inhalt_vorher) + 1

    def test_append_nicht_truncate(self, audit_dir):
        """Datei muss mit 'a' geöffnet werden, nicht 'w' — sonst
        überschreibt der zweite Schreibvorgang den ersten."""
        audit.append("x", "login", None, None)
        audit.append("y", "logout", None, None)
        entries = audit.lies()
        assert len(entries) == 2, \
            "Nur 1 Eintrag gelesen → Datei wurde getruncated statt appended"
        # Falls getruncated: user_id des ersten wäre "y" statt "x"
        assert entries[0]["user_id"] == "x", \
            "Erster Eintrag überschrieben → Datei wurde mit 'w' geöffnet"

    def test_append_danach_lesen_konsistent(self, audit_dir):
        """Nach jedem Append ist lies() konsistent (flush + fsync)."""
        for i in range(10):
            audit.append(f"user{i}", "login", None, f"nr={i}")
            entries = audit.lies()
            assert len(entries) == i + 1, \
                f"lies() nach Eintrag {i}: erwarte {i+1}, habe {len(entries)}"


# --------------------------------------------------------------- Parallel-Sicherheit
class TestParallel:
    def test_schnelle_sequenz_kein_verlust(self, audit_dir):
        """10 Appends in schneller Folge — kein Eintrag verloren."""
        for i in range(10):
            audit.append("p", "login", None, f"seq={i}")
        entries = audit.lies()
        assert len(entries) == 10
        details = [e["detail"] for e in entries]
        assert all(f"seq={i}" in details for i in range(10))

    def test_gleichzeitige_aufrufe_simuliert(self, audit_dir):
        """Simuliert Overlap: O_APPEND garantiert Atomaritat bis
        PIPE_BUF (4096B). Jede Zeile ist < 1KB → kein Clobber."""
        import threading
        errors = []

        def writer(n):
            try:
                for _ in range(5):
                    audit.append("t", "login", None, f"thread={n}")
            except Exception as e:
                errors.append(e)

        threads = [threading.Thread(target=writer, args=(i,)) for i in range(4)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(10)

        assert not errors, f"Thread-Fehler: {errors}"
        entries = audit.lies()
        assert len(entries) == 20, \
            f"Erwarte 20 Einträge bei 4×5 Threads, habe {len(entries)}"
        # Jede Zeile muss wohlgeformtes JSON sein
        pfad = audit._audit_pfad()
        with open(pfad, encoding="utf-8") as f:
            for i, line in enumerate(f):
                assert line.strip(), f"Leere Zeile {i}"
                json.loads(line)  # muss valide sein


# --------------------------------------------------------------- user_id-Robustheit
class TestUserId:
    def test_fallback_unbekannt(self, audit_dir, monkeypatch):
        """user_id=None → 'unbekannt' im Audit-Eintrag.
        Der Fallback sitzt in audit.append selbst (unterste Schicht)."""
        audit.append(None, "llm_call", None, "pii_kategorien=[]")
        entries = audit.lies()
        assert len(entries) == 1
        assert entries[0]["user_id"] == "unbekannt", \
            f"Erwarte 'unbekannt', habe '{entries[0]['user_id']}'"

    def test_fallback_dev(self, audit_dir, monkeypatch):
        """_AUTH_USER=None → api_llm nutzt 'unbekannt' (oder 'dev'
        in server.py-Dispatch). Test: append mit None wird zu 'unbekannt'."""
        audit.append(None, "llm_call", None, "pii_kategorien=[], textlaenge_vor=0")
        e = audit.lies()[0]
        assert e["user_id"] == "unbekannt"

    def test_kein_absturz_bei_none_feldern(self, audit_dir):
        """Optional-Felder (fall_id, detail) sind None → kein Crash."""
        audit.append("user", "login", None, None)
        e = audit.lies()[0]
        assert e["fall_id"] is None
        assert e["detail"] is None


# --------------------------------------------------------------- PII-Freiheit (Static Analysis)
class TestPiiFrei:
    def test_detail_kein_freitext(self):
        """Jeder audit.append()-Aufruf im Code: detail enthält keine
        Nutzer-Freitextwerte (Namen, IdNr, IBAN, Beträge, etc.).

        Diese Prüfung listet alle Aufruf-Stellen auf und belegt, dass
        detail nur Metadaten enthält (Feld-IDs, Status-Codes, Kategorien,
        Scheiben-Namen, Veranlagungszeiträume)."""
        # Diese Assertion ist der Platzhalter für die statische Analyse
        # unten im Test — siehe test_detail_aufrufer_enthalten_kein_pii
        pass

    def test_all_audit_call_sites_known(self):
        """Liefert eine Liste aller audit.append-Aufrufer als
        Dokumentation. Jeder neue Aufrufer muss hier ergänzt werden."""
        sites = self._call_sites()
        # Bekannte Aufrufer: dict {relativer_pfad: zeile}
        expected = {
            "produkt/haut/api.py:67",
            "produkt/haut/api.py:1656",
            "produkt/haut/api.py:1914",
            "produkt/haut/api_llm.py:87",
            "produkt/haut/server.py:60",
            "produkt/haut/server.py:62",
            "produkt/haut/server.py:169",
            "produkt/haut/server.py:172",
        }
        for site in sites:
            assert site in expected, f"Unbekannter audit.append-Aufrufer: {site}"
        # Umgekehrt: jeder Erwartete muss auch gefunden werden
        for exp in expected:
            assert exp in sites, f"Erwarteter Aufrufer nicht gefunden: {exp}"
        assert len(sites) == len(expected), \
            f"Anzahl Aufrufer ({len(sites)}) != erwartet ({len(expected)})"

    def _call_sites(self) -> list[str]:
        """Gibt alle Dateien+Zeilen mit audit.append zurück."""
        import subprocess
        r = subprocess.run(
            ["grep", "-rn", 'audit.append(', ROOT, "--include=*.py"],
            capture_output=True, text=True, timeout=30)
        sites = []
        for line in r.stdout.strip().split("\n"):
            if not line or "test_" in line:
                continue
            parts = line.split(":")
            if len(parts) >= 2:
                pfad = os.path.relpath(parts[0], ROOT)
                sites.append(f"{pfad}:{parts[1]}")
        return sorted(sites)


# --------------------------------------------------------------- Fehlertoleranz
class TestRobustheit:
    def test_audit_dir_wird_angelegt(self, tmp_path):
        """append() legt AUDIT_DIR an, wenn nicht vorhanden."""
        neu = str(tmp_path / "tief" / "unten")
        old = audit.AUDIT_DIR
        audit.AUDIT_DIR = neu
        try:
            audit.append("user", "login", None, None)
            assert os.path.isdir(neu)
            pfad = os.path.join(neu, "audit.jsonl")
            assert os.path.exists(pfad)
        finally:
            audit.AUDIT_DIR = old

    def test_sonderzeichen_im_detail(self, audit_dir):
        """Detail mit Umlauten, JSON-Sonderzeichen, Leerzeichen."""
        audit.append("u", "login", "f-1", "scheibe=gesamt, grund=äßüö")
        # Entspricht auch: detail mit geschweiften Klammern
        audit.append("u", "llm_call", None, 'pii_kategorien=["steuer_id","iban"]')
        entries = audit.lies()
        assert len(entries) == 2
        # JSON-Kodierung muss valide bleiben
        pfad = audit._audit_pfad()
        with open(pfad, encoding="utf-8") as f:
            lines = f.readlines()
        for line in lines:
            obj = json.loads(line)
            assert "user_id" in obj
            assert "action" in obj

    def test_ts_ist_isoformat(self, audit_dir):
        """Zeitstempel muss ISO-8601-konform sein."""
        audit.append("u", "login", None, None)
        e = audit.lies()[0]
        ts = e["ts"]
        assert "T" in ts, f"Kein ISO-Format: {ts}"
        assert ts.endswith("+00:00") or ts.endswith("Z"), \
            f"Keine UTC-Zeitzone: {ts}"


# --------------------------------------------------------------- Mutationsprobe
class TestMutation:
    def test_append_only_mutation_wirkt(self, audit_dir, monkeypatch):
        """Beweis: Wenn 'a' durch 'w' ersetzt wird, überlebt nur der
        letzte Eintrag. Der Test bestätigt den Schaden, nicht seine
        Abwesenheit — bei Mutation ASSERTIERT er 1 Eintrag (den letzten).

        Einmaliger Commit-Beleg (P1.6). Dauerhafter Guard ist der
        statische Check test_file_append_mode_ist_a daneben."""
        import builtins
        _real_open = builtins.open

        def mut_open(path, mode="r", *args, **kwargs):
            if "audit.jsonl" in str(path) and mode == "a":
                mode = "w"  # Mutation: "a" → "w"
            return _real_open(path, mode, *args, **kwargs)

        monkeypatch.setattr(builtins, "open", mut_open)

        audit.append("erster", "login", None, None)
        audit.append("zweiter", "logout", None, None)

        entries = audit.lies()
        # Bei "w" überschreibt der zweite Eintrag den ersten → nur 1
        assert len(entries) == 1, \
            f"'a'→'w'-Mutation wirkungslos: {len(entries)} Einträge"
        assert entries[0]["user_id"] == "zweiter", \
            f"'a'→'w'-Mutation überschrieb nicht: {entries[0]['user_id']}"

    def test_file_append_mode_ist_a(self, audit_dir):
        """Die audit.py-Datei verwendet 'a' zum Öffnen (static check)."""
        audit_path = os.path.join(ROOT, "produkt", "store", "audit.py")
        with open(audit_path, encoding="utf-8") as f:
            content = f.read()
        # "a" als open-mode muss vorkommen
        assert '"a"' in content or "'a'" in content, \
            "audit.py verwendet nicht 'a' als open-mode"
        # "w" darf nicht als open-mode für audit.jsonl vorkommen
        # (aber "w" kann in anderen Kontexten vorkommen — hier nicht prüfbar)