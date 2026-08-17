"""Round-Trip-Test fuer `make backup` / `make restore` (Review 2026-08-17, fuenf Maengel).

Der Plan hinter dieser Arbeit sagt ausdruecklich "kein Schritt 2 ohne Schritt 1" — eine Sicherung
ohne erprobte Wiederherstellung ist keine. Dieser Test sichert wirklich (echter `tar czf` via
Makefile-Subprozess), zerstoert/veraendert den simulierten Live-Stand, stellt wieder her und
vergleicht per SHA-256 gegen den Stand VOR der Sicherung.

Fasst NIE die echten Pfade an: FAELLE_ROOT/AUTH_USERS/BACKUP_DIR zeigen ausschliesslich auf
tmp_path. `make` laeuft als eigener Subprozess — der conftest-Audit-Wachhund (der ECHTE
audit.jsonl-Schreibzugriffe innerhalb DIESES Pytest-Prozesses abfaengt) greift hier nicht und
ist auch nicht noetig, weil der Subprozess nie in der Naehe von produkt/haut/faelle/audit.jsonl
schreibt.

Deckt konkret drei der fuenf Review-Punkte messbar ab:
  - Punkt 1 (audit.jsonl-Operand traf nie): audit.jsonl liegt in der Fixture INNERHALB von
    faelle/ (wie in der Produktion) — der Checksummen-Vergleich schliesst es ein.
  - Punkt 3 (Restore mergt statt zu ersetzen): eine Datei, die NACH der Sicherung im Ziel
    entsteht bzw. veraendert wird, MUSS nach dem Restore wieder wie zum Sicherungszeitpunkt
    aussehen (Datei-fuer-Datei-Vergleich, kein Teilmengen-Check).
  - Punkt 5 (users.json fehlte im Archiv): eigene Assertion auf den Auth-Store.
Punkt 2 (Sekundenaufloesung) und Punkt 4 (liver Default + fehlende Vorher-Sicherung) sind
Makefile-Mechanik ohne Datenverlust-Symptom im Round-Trip selbst; Punkt 4 wird unten trotzdem
mitgeprueft (CONFIRM=yes-Bypass + automatische Vorher-Sicherung + Ablehnungspfad).
"""
from __future__ import annotations

import glob
import hashlib
import json
import os
import shutil
import subprocess

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def _verzeichnis_hashes(root: str) -> dict[str, str]:
    """relativer Pfad -> sha256(Inhalt), rekursiv. Datei-genau statt EIN Gesamt-Hash, damit ein
    Fehlschlag sagt WELCHE Datei abweicht, nicht nur DASS etwas abweicht."""
    out = {}
    for dirpath, _dirs, files in os.walk(root):
        for name in files:
            p = os.path.join(dirpath, name)
            rel = os.path.relpath(p, root)
            with open(p, "rb") as f:
                out[rel] = hashlib.sha256(f.read()).hexdigest()
    return out


def _make(*args: str, eingabe: str | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(["make", *args], cwd=ROOT, input=eingabe,
                          capture_output=True, text=True, timeout=60)


def _fixture_anlegen(tmp_path) -> tuple[str, str, str, str]:
    """Baut einen Mini-Fall-Bestand + Auth-Store nach echtem Layout (produkt/haut/faelle/*.json +
    audit.jsonl, produkt/auth/users.json) — alles unter tmp_path, nie unter den echten Pfaden.
    Gibt (faelle_root, auth_users_pfad, backup_dir, faelle_dir) zurueck."""
    faelle_root = tmp_path / "haut"
    faelle_dir = faelle_root / "faelle"
    faelle_dir.mkdir(parents=True)
    (faelle_dir / "fall-a.json").write_text(
        json.dumps({"version": 1, "veranlagungszeitraum": 2025, "fall_id": "fall-a",
                    "events": [], "snapshots": []}), encoding="utf-8")
    (faelle_dir / "fall-b.json").write_text(
        json.dumps({"version": 1, "veranlagungszeitraum": 2025, "fall_id": "fall-b",
                    "events": [], "snapshots": []}), encoding="utf-8")
    (faelle_dir / "audit.jsonl").write_text(
        '{"ts": "2026-08-17T10:00:00+00:00", "user_id": "u", "action": "login", '
        '"fall_id": null, "detail": null}\n', encoding="utf-8")

    auth_dir = tmp_path / "auth"
    auth_dir.mkdir()
    auth_users = auth_dir / "users.json"
    auth_users.write_text(
        json.dumps({"users": {"tester": {"created_at": "2026-08-17T10:00:00+00:00",
                                          "password_hash": "nicht-echt"}}}), encoding="utf-8")

    backup_dir = tmp_path / "backups"
    return str(faelle_root), str(auth_users), str(backup_dir), str(faelle_dir)


def test_backup_restore_roundtrip_stellt_exakten_stand_wieder_her(tmp_path):
    faelle_root, auth_users, backup_dir, faelle_dir = _fixture_anlegen(tmp_path)
    stand_bei_sicherung = _verzeichnis_hashes(faelle_dir)
    auth_hash_original = hashlib.sha256(open(auth_users, "rb").read()).hexdigest()

    r = _make("backup", f"FAELLE_ROOT={faelle_root}", f"AUTH_USERS={auth_users}",
              f"BACKUP_DIR={backup_dir}")
    assert r.returncode == 0, f"make backup fehlgeschlagen:\n{r.stdout}\n{r.stderr}"
    archive_nach_backup = sorted(glob.glob(os.path.join(backup_dir, "*.tar.gz")))
    assert len(archive_nach_backup) == 1, f"erwartet genau 1 Archiv, gefunden: {archive_nach_backup}"
    archiv = archive_nach_backup[0]

    # Drift NACH der Sicherung simulieren — genau der Fall, der "restore" von einem blossen
    # Entpacken unterscheidet (Review-Punkt 3): eine neue Datei UND eine veraenderte bestehende
    # Datei duerfen den restaurierten Stand nicht ueberleben.
    with open(os.path.join(faelle_dir, "orphan-nach-backup.json"), "w", encoding="utf-8") as f:
        f.write('{"nicht": "im archiv"}')
    with open(os.path.join(faelle_dir, "fall-a.json"), "a", encoding="utf-8") as f:
        f.write("MANIPULIERT")
    with open(auth_users, "w", encoding="utf-8") as f:
        f.write('{"users": {}}')

    # Ablehnungspfad zuerst: "n" auf die interaktive Rueckfrage -> Ziel bleibt UNVERAENDERT.
    r_abgelehnt = _make("restore", f"ARCHIV={archiv}", f"FAELLE_ROOT={faelle_root}",
                        f"AUTH_USERS={auth_users}", f"BACKUP_DIR={backup_dir}", eingabe="n\n")
    assert r_abgelehnt.returncode != 0, "Ablehnung haette restore abbrechen muessen (rc=0 wäre falsch)"
    assert os.path.exists(os.path.join(faelle_dir, "orphan-nach-backup.json")), (
        "Drift-Datei nach ABGELEHNTEM restore verschwunden — der Ablehnungspfad hat trotzdem geschrieben"
    )

    # Jetzt der echte Restore, non-interaktiv (CONFIRM=yes — Review-Punkt 4, Skript-Pfad).
    r = _make("restore", f"ARCHIV={archiv}", f"FAELLE_ROOT={faelle_root}",
              f"AUTH_USERS={auth_users}", f"BACKUP_DIR={backup_dir}", "CONFIRM=yes")
    assert r.returncode == 0, f"make restore fehlgeschlagen:\n{r.stdout}\n{r.stderr}"

    stand_nach_restore = _verzeichnis_hashes(faelle_dir)
    assert stand_nach_restore == stand_bei_sicherung, (
        "Restore weicht vom Sicherungsstand ab (Diff je Datei-Hash):\n"
        f"nur vorher: {sorted(set(stand_bei_sicherung) - set(stand_nach_restore))}\n"
        f"nur nachher (Merge-Rest — Punkt 3!): {sorted(set(stand_nach_restore) - set(stand_bei_sicherung))}\n"
        f"inhaltlich abweichend: {sorted(k for k in stand_bei_sicherung if k in stand_nach_restore and stand_bei_sicherung[k] != stand_nach_restore[k])}"
    )
    assert not os.path.exists(os.path.join(faelle_dir, "orphan-nach-backup.json")), (
        "Merge statt Ersetzen (Punkt 3): die Nach-Sicherung angelegte Datei hat den restore ueberlebt"
    )

    auth_hash_restauriert = hashlib.sha256(open(auth_users, "rb").read()).hexdigest()
    assert auth_hash_restauriert == auth_hash_original, (
        "users.json nach restore nicht identisch zum Sicherungsstand (Punkt 5)"
    )

    # Punkt 4, zweite Haelfte: restore muss VOR dem Ueberschreiben automatisch eine eigene
    # Sicherheits-Sicherung angelegt haben -- der abgelehnte Lauf oben legte keine an (er brach
    # vor dem Backup-Schritt ab), also muss die Archivzahl jetzt genau um 1 gestiegen sein
    # (die automatische Vorher-Sicherung des ECHTEN restore-Laufs) statt nur um den urspruenglichen.
    archive_danach = sorted(glob.glob(os.path.join(backup_dir, "*.tar.gz")))
    assert len(archive_danach) == 2, (
        f"restore haette VOR dem Ueberschreiben automatisch eine Sicherheits-Sicherung anlegen "
        f"muessen (Punkt 4) -- erwartet 2 Archive (Ursprung + Vorher-Sicherung), gefunden: {archive_danach}"
    )


def test_restore_auf_leeres_ziel_stellt_wieder_her(tmp_path):
    """Der Katastrophenfall — und der einzige, fuer den `restore` ueberhaupt existiert: frische
    Maschine, `faelle/` gibt es noch gar nicht.

    Der Test oben stellt IMMER ueber einen bestehenden Bestand wieder her und hat deshalb genau
    diese Route nie beruehrt. Gemessen 2026-08-17: sie war blockiert. `restore` fuhr vor dem
    Loeschen unbedingt `make backup`, und `backup` bricht ohne Bestand mit
    `tar: faelle: Cannot stat` ab -- der Fehler riss das ganze Ziel mit, das Ziel blieb leer.
    Ein gruener Round-Trip-Test hat das nicht bemerkt, weil er die Voraussetzung selbst mitbrachte.
    """
    faelle_root, auth_users, backup_dir, faelle_dir = _fixture_anlegen(tmp_path)
    stand_bei_sicherung = _verzeichnis_hashes(faelle_dir)

    r = _make("backup", f"FAELLE_ROOT={faelle_root}", f"AUTH_USERS={auth_users}",
              f"BACKUP_DIR={backup_dir}")
    assert r.returncode == 0, f"make backup fehlgeschlagen:\n{r.stdout}\n{r.stderr}"
    archiv = sorted(glob.glob(os.path.join(backup_dir, "*.tar.gz")))[0]

    # Totalverlust: der ganze Bestand ist weg, nicht nur veraendert.
    shutil.rmtree(faelle_dir)
    assert not os.path.exists(faelle_dir)

    r = _make("restore", f"ARCHIV={archiv}", f"FAELLE_ROOT={faelle_root}",
              f"AUTH_USERS={auth_users}", f"BACKUP_DIR={backup_dir}", "CONFIRM=yes")
    assert r.returncode == 0, (
        f"restore auf leeres Ziel fehlgeschlagen -- genau der Katastrophenfall:\n{r.stdout}\n{r.stderr}")
    assert _verzeichnis_hashes(faelle_dir) == stand_bei_sicherung, (
        "Wiederherstellung nach Totalverlust weicht vom Sicherungsstand ab")


def test_gescheiterte_vorher_sicherung_verhindert_das_loeschen(tmp_path):
    """Die Gegenprobe zur Lockerung von oben, und sie ist der eigentliche Grund fuer die
    Unterscheidung `nichts da` gegen `ging schief`.

    Damit der Katastrophenfall durchkommt, darf eine fehlende Vorher-Sicherung `restore` nicht
    mehr abbrechen. Ein pauschales `-`/`|| true` vor dem Backup-Schritt haette das erledigt --
    und dabei den Schutz mitgenommen, fuer den der Schritt da ist. Hier wird belegt, dass er
    weiterhin greift: ist ein Bestand DA und die Sicherung SCHEITERT, bricht restore ab, BEVOR
    irgendetwas geloescht wird. Ein Restore darf nie die letzte Kopie sein.
    """
    if os.geteuid() == 0:
        pytest.skip("als root sind Schreibrechte nicht durchsetzbar — die Sicherung wuerde gelingen")

    faelle_root, auth_users, backup_dir, faelle_dir = _fixture_anlegen(tmp_path)
    r = _make("backup", f"FAELLE_ROOT={faelle_root}", f"AUTH_USERS={auth_users}",
              f"BACKUP_DIR={backup_dir}")
    assert r.returncode == 0
    archiv = sorted(glob.glob(os.path.join(backup_dir, "*.tar.gz")))[0]
    vorher = _verzeichnis_hashes(faelle_dir)

    # Sicherung zum Scheitern bringen, ohne den Bestand anzufassen: das Zielverzeichnis der
    # Vorher-Sicherung liegt unter einem nicht beschreibbaren Elternteil, `mkdir -p` scheitert.
    gesperrt = tmp_path / "gesperrt"
    gesperrt.mkdir()
    os.chmod(gesperrt, 0o500)
    try:
        r = _make("restore", f"ARCHIV={archiv}", f"FAELLE_ROOT={faelle_root}",
                  f"AUTH_USERS={auth_users}", f"BACKUP_DIR={gesperrt / 'darunter'}", "CONFIRM=yes")
    finally:
        os.chmod(gesperrt, 0o700)

    assert r.returncode != 0, (
        "restore lief trotz gescheiterter Vorher-Sicherung weiter — der Bestand haette dabei "
        "geloescht werden koennen, ohne dass eine Kopie existiert")
    assert _verzeichnis_hashes(faelle_dir) == vorher, (
        "restore hat trotz gescheiterter Vorher-Sicherung schon geschrieben/geloescht")
