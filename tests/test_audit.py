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
import pathlib
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
    def test_alle_audit_aufrufer_sind_bekannt(self):
        """Jede Datei, die audit.append() ruft, muss hier eingetragen sein.

        Zweck: die PII-Freiheit eines neuen Aufrufers muss ausdrücklich bestätigt
        werden, statt stillschweigend mitzulaufen. Ein neuer Aufrufer macht diesen
        Test rot — das ist die Absicht.

        Bewusst auf DATEI-Ebene, nicht auf Zeilenebene: Zeilennummern verschieben
        sich bei jeder Änderung darüber, und ein Test, der bei unbeteiligten Edits
        rot wird, wird irgendwann entnervt gelockert. Die Datei ist die Einheit, in
        der jemand die detail-Felder prüft.
        """
        dateien = {s.split(":", 1)[0] for s in self._call_sites()}
        erwartet = {
            "produkt/haut/api.py",       # fall_anlegen, fall_loeschen, zugriff_verweigert
            "produkt/haut/api_llm.py",   # PII-Kategorien beim LLM-Call
            "produkt/haut/server.py",    # login, logout, Fall-Zugriffe
        }
        neu_dazu = dateien - erwartet
        assert not neu_dazu, (
            f"Neue audit.append-Aufrufer in {sorted(neu_dazu)} — prüf ihre detail-Felder "
            f"auf PII (nur Metadaten: Feld-IDs, Status, Kategorien, Scheibe, VZ) und trag "
            f"die Datei hier ein.")
        verschwunden = erwartet - dateien
        assert not verschwunden, (
            f"Erwartete Aufrufer fehlen: {sorted(verschwunden)} — wurde Audit-Protokollierung "
            f"entfernt? Wenn absichtlich, hier austragen.")

    def test_detail_felder_enthalten_nur_metadaten(self):
        """Die detail-Argumente der Aufrufer sind Metadaten, kein Nutzer-Freitext.

        Geprüft wird der Quelltext: ein detail-Ausdruck darf Feld-IDs, Status-Codes,
        Kategorien, Scheibennamen und Zeiträume enthalten — aber keinen Wert, der aus
        einer Nutzereingabe stammt.
        """
        import re as _re
        # Der Nutzerwert selbst ist gefährlich, seine LÄNGE nicht: len(freitext) sagt
        # nichts über den Inhalt. Deshalb nur direkte Interpolation treffen ({wert}),
        # nicht jede Erwähnung des Bezeichners.
        verdaechtig = _re.compile(
            r"audit\.append\([^)]*\{(?!len\()[^}]*\b"
            r"(wert|name|idnr|iban|betrag|freitext|eingabe)\b[^}]*\}",
            _re.IGNORECASE | _re.DOTALL)
        treffer = []
        for rel in ("produkt/haut/api.py", "produkt/haut/api_llm.py", "produkt/haut/server.py"):
            quelle = (pathlib.Path(ROOT) / rel).read_text(encoding="utf-8")
            for m in verdaechtig.finditer(quelle):
                zeile = quelle[:m.start()].count("\n") + 1
                treffer.append(f"{rel}:{zeile}")
        assert not treffer, (
            f"audit.append mit möglichem Nutzerwert im detail: {treffer}. "
            f"Das Audit protokolliert WER WANN WAS getan hat, nie WELCHE Werte.")

    # Verzeichnisse, die kein Aufrufer-Code sind, fuer den Fallback ohne Git (s. u.).
    # Deckt sich mit .gitignore — dort gitignored, hier per Name ausgeschlossen.
    _FALLBACK_AUSSCHLUSS = (
        ".git", "__pycache__", "_build", "_target", "_targets",
        ".venv", ".venv312", "graphify-out", "scratch",
    )

    def _call_sites(self) -> list[str]:
        """Gibt alle Dateien+Zeilen mit audit.append zurück.

        Nutzt `git ls-files` fuer die Dateiliste statt eines Repo-weiten `grep -r`:
        unter oracle/.venv312 liegen allein 6079 fremde .py-Dateien (venv, gitignored),
        macht aus 235 versionierten .py-Dateien 6410 zu durchsuchende — 27x mehr Datei-
        Oeffnungen fuer denselben Befund. Isoliert kaum spuerbar, aber unter Last (mehrere
        parallele Worker) sprengte genau das den 30s-Timeout (gemessen: 48s unter Last,
        1,5s isoliert — 2026-08-10). git ls-files ist von sich aus blind fuer alles
        Ignorierte, kein manueller Ausschluss noetig.

        `--cached --others --exclude-standard` (nicht nur `--cached`): reines `--cached`
        sieht einen frisch angelegten, noch nicht `git add`eten Aufrufer nicht — genau der
        Fall, den die Mutationsprobe dieses Tests prueft. `--others --exclude-standard`
        nimmt neue Dateien mit, respektiert aber weiter .gitignore (kein Rueckfall auf den
        6410er-Wald).

        Fallback ohne Git-Arbeitsbaum (z.B. eine git-archive-Kopie, s. Kommentar bei
        test_k_alle_python_dateien_parsen): repo-weiter grep mit denselben Ausschluessen,
        die ein Arbeitsbaum ueber .gitignore ohnehin haette. Kein Skip — die Invariante
        bleibt scharf, nur der Weg zur Dateiliste ist ein anderer.

        Timeouts grosszuegiger als vorher (30s): der schnelle Pfad braucht jetzt <1s
        (27x weniger Dateien), das erlaubt mehr Toleranz fuer den Lastfall, ohne den
        Normalfall zu verlangsamen.
        """
        import subprocess
        lf = subprocess.run(
            ["git", "ls-files", "--cached", "--others", "--exclude-standard", "--", "*.py"],
            cwd=ROOT, capture_output=True, text=True, timeout=15)
        if lf.returncode == 0:
            dateien = [os.path.join(ROOT, d) for d in lf.stdout.strip().splitlines() if d]
            if not dateien:
                return []
            r = subprocess.run(
                ["grep", "-n", "audit.append(", *dateien],
                capture_output=True, text=True, timeout=30)
        else:
            # Kein Git-Arbeitsbaum -> Fallback statt Absturz.
            ausschluss = [f"--exclude-dir={d}" for d in self._FALLBACK_AUSSCHLUSS]
            r = subprocess.run(
                ["grep", "-rn", "audit.append(", ROOT, "--include=*.py", *ausschluss],
                capture_output=True, text=True, timeout=90)
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
        """Beweis: Wird das Anhängen abgeschaltet, überlebt nur der letzte Eintrag. Der Test
        bestätigt den Schaden, nicht seine Abwesenheit — bei Mutation ASSERTIERT er 1 Eintrag.

        NACHGEZOGEN am 2026-08-18: die alte Fassung mutierte `builtins.open` und ersetzte den
        Modus "a" durch "w". Seit audit.py über `os.open(..., O_APPEND, 0o600)` schreibt (damit
        das Protokoll nicht die umask erbt und für alle lesbar ist, Audit
        sec-users-json-world-readable), griff diese Mutation ins Leere: der Test blieb grün,
        obwohl er nichts mehr bewies. Mutiert wird jetzt die Stelle, die das Anhängen wirklich
        garantiert — O_APPEND raus, O_TRUNC rein.

        Dauerhafter Guard ist der statische Check daneben."""
        _echtes_os_open = os.open

        def mut_open(pfad, flags, mode=0o777, **kwargs):
            if "audit.jsonl" in str(pfad):
                flags = (flags & ~os.O_APPEND) | os.O_TRUNC   # Mutation: anhängen -> überschreiben
            return _echtes_os_open(pfad, flags, mode, **kwargs)

        monkeypatch.setattr(os, "open", mut_open)

        audit.append("erster", "login", None, None)
        audit.append("zweiter", "logout", None, None)

        entries = audit.lies()
        assert len(entries) == 1, \
            f"O_APPEND-Mutation wirkungslos: {len(entries)} Einträge — das Anhängen hängt " \
            f"nicht an der mutierten Stelle, dieser Beweis prüft ins Leere"
        assert entries[0]["user_id"] == "zweiter", \
            f"O_APPEND-Mutation überschrieb nicht: {entries[0]['user_id']}"

    def test_datei_wird_im_append_modus_geoeffnet(self, audit_dir):
        """Statischer Guard. Prüft die Stelle, die das Anhängen HEUTE garantiert.

        Hiess bis 2026-08-18 test_file_append_mode_ist_a und suchte den Modus-String "a". Der
        steht nach dem Umbau auf os.open weiterhin da (in `os.fdopen(fd, "a")`) — nur garantiert
        er nichts mehr, das tut O_APPEND im os.open darüber. Der Test wäre also grün geblieben,
        auch wenn das Anhängen verschwunden wäre: ein Guard, der auf ein Überbleibsel zeigt."""
        audit_path = os.path.join(ROOT, "produkt", "store", "audit.py")
        with open(audit_path, encoding="utf-8") as f:
            content = f.read()
        assert "O_APPEND" in content, \
            "audit.py öffnet das Protokoll nicht mehr mit O_APPEND — ein Eintrag könnte das " \
            "gesamte bisherige Protokoll überschreiben"
        assert "O_TRUNC" not in content, \
            "audit.py verwendet O_TRUNC — das Protokoll wird abgeschnitten"