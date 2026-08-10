"""Tests fuer elster/versand.py — der Echtversand-Pfad. ALLES gemockt, nie ein echter
ERiC-Sendeaufruf (EricBearbeiteVorgang wird hier immer durch _FakeEric ersetzt, nie die
echte .so aufgerufen). Siehe elster/versand.py Modul-Docstring fuer das Sicherheitsmodell."""
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "elster"))
import versand as V  # noqa: E402

XML_TESTVERSAND = (
    b'<?xml version="1.0"?><Elster><TransferHeader>'
    b'<Testmerker>700000004</Testmerker></TransferHeader></Elster>')
XML_FALSCHER_MERKER = (
    b'<?xml version="1.0"?><Elster><TransferHeader>'
    b'<Testmerker>700000001</Testmerker></TransferHeader></Elster>')
XML_ECHT = b'<?xml version="1.0"?><Elster><TransferHeader></TransferHeader></Elster>'
ANTWORT_ERFOLG = "<Elster><Erfolg><Telenummer>N552026081012345</Telenummer></Erfolg></Elster>"


class _FakeEric:
    """Ersetzt die ctypes-CDLL komplett. Kein Netz, kein Zertifikat, keine .so — nur
    Python-Callables, die aufzeichnen, was mit welchen Flags aufgerufen wurde."""

    def __init__(self, *, rc=0, rueckgabe=ANTWORT_ERFOLG, server="", zert_rc=0):
        self.calls = []
        self._rc = rc
        self._rueckgabe = rueckgabe.encode()
        self._server = server.encode()
        self._zert_rc = zert_rc
        self._next_handle = 0

        # Plain Funktionsobjekte statt gebundener Methoden: versand.py setzt darauf
        # .restype/.argtypes wie auf eine echte ctypes-Funktion — gebundene Methoden
        # erlauben das nicht (kein __dict__), Funktionsobjekte schon.
        def get_handle(htoken_ptr, info_ptr, pfad):
            self.calls.append(("EricGetHandleToCertificate", pfad))
            return self._zert_rc

        def close_handle(htoken):
            self.calls.append(("EricCloseHandleToCertificate", htoken))
            return 0

        def puffer_erzeugen():
            self._next_handle += 1
            return self._next_handle

        def puffer_inhalt(handle):
            return self._rueckgabe if handle == 1 else self._server

        def puffer_freigeben(handle):
            self.calls.append(("EricRueckgabepufferFreigeben", handle))

        def bearbeite(xml, datenart, flags, druck, crypto, rueck, server):
            self.calls.append(("EricBearbeiteVorgang", flags, xml))
            return self._rc

        self.EricGetHandleToCertificate = get_handle
        self.EricCloseHandleToCertificate = close_handle
        self.EricRueckgabepufferErzeugen = puffer_erzeugen
        self.EricRueckgabepufferInhalt = puffer_inhalt
        self.EricRueckgabepufferFreigeben = puffer_freigeben
        self.EricBearbeiteVorgang = bearbeite


def _patch(monkeypatch, fake):
    monkeypatch.setattr(V.CE, "_load_and_init", lambda: fake)


# --------------------------------------------------------------- Freigabe-Sperre ---

def test_echtversand_ohne_freigabe_wird_verweigert(monkeypatch):
    fake = _FakeEric()
    _patch(monkeypatch, fake)
    with pytest.raises(V.EchtversandOhneFreigabe):
        V.sende(XML_ECHT, "ESt_2025", zertifikat_pfad="/x", pin="1234",
                testmerker=None, echtversand_freigabe=None)
    assert fake.calls == [], "kein ERiC-Aufruf ohne Freigabe erlaubt"


def test_echtversand_mit_falscher_freigabe_wird_verweigert(monkeypatch):
    fake = _FakeEric()
    _patch(monkeypatch, fake)
    with pytest.raises(V.EchtversandOhneFreigabe):
        V.sende(XML_ECHT, "ESt_2025", zertifikat_pfad="/x", pin="1234",
                testmerker=None, echtversand_freigabe="ja klar")
    assert fake.calls == []


# ------------------------------------------------------------- XML-Merker-Gegenprobe ---

def test_testversand_mit_falschem_marker_wird_verweigert(monkeypatch):
    fake = _FakeEric()
    _patch(monkeypatch, fake)
    with pytest.raises(V.XmlMerkerMismatch):
        V.sende(XML_FALSCHER_MERKER, "ESt_2025", zertifikat_pfad="/x", pin="1234")
    assert fake.calls == [], "kein ERiC-Aufruf bei Merker-Mismatch"


def test_testversand_ohne_marker_im_xml_wird_verweigert(monkeypatch):
    fake = _FakeEric()
    _patch(monkeypatch, fake)
    with pytest.raises(V.XmlMerkerMismatch):
        V.sende(XML_ECHT, "ESt_2025", zertifikat_pfad="/x", pin="1234")  # testmerker=Default
    assert fake.calls == []


def test_echtversand_mit_marker_noch_im_xml_wird_verweigert(monkeypatch):
    """Selbst mit korrekter Freigabe: traegt das XML noch den Testmerker, obwohl Echtversand
    behauptet wird, bricht sende() ab — die Funktion vertraut dem Aufrufer nicht."""
    fake = _FakeEric()
    _patch(monkeypatch, fake)
    with pytest.raises(V.XmlMerkerMismatch):
        V.sende(XML_TESTVERSAND, "ESt_2025", zertifikat_pfad="/x", pin="1234",
                testmerker=None, echtversand_freigabe=V.ECHTVERSAND_FREIGABE)
    assert fake.calls == []


# ------------------------------------------------------------------- Zertifikat ---

def test_ohne_zertifikatspfad_bricht_ab_vor_eric_aufruf(monkeypatch):
    fake = _FakeEric()
    _patch(monkeypatch, fake)
    with pytest.raises(V.ZertifikatFehlt, match="Zertifikatspfad"):
        V.sende(XML_TESTVERSAND, "ESt_2025", zertifikat_pfad="", pin="1234")
    assert fake.calls == []


def test_zertifikatsdatei_nicht_gefunden_bricht_ab(monkeypatch):
    fake = _FakeEric()
    _patch(monkeypatch, fake)
    with pytest.raises(V.ZertifikatFehlt, match="nicht gefunden"):
        V.sende(XML_TESTVERSAND, "ESt_2025",
                zertifikat_pfad="/nicht/existent/zertifikat.pfx", pin="1234")
    assert fake.calls == []


def test_ohne_pin_bricht_ab(monkeypatch, tmp_path):
    zert = tmp_path / "zert.pfx"
    zert.write_bytes(b"dummy")
    fake = _FakeEric()
    _patch(monkeypatch, fake)
    with pytest.raises(V.ZertifikatFehlt, match="PIN"):
        V.sende(XML_TESTVERSAND, "ESt_2025", zertifikat_pfad=str(zert), pin="")
    assert fake.calls == []


def test_eric_lehnt_zertifikat_ab(monkeypatch, tmp_path):
    zert = tmp_path / "zert.pfx"
    zert.write_bytes(b"dummy")
    fake = _FakeEric(zert_rc=610201106)  # ERIC_CRYPT_E_PIN_WRONG
    _patch(monkeypatch, fake)
    with pytest.raises(V.ZertifikatFehlt, match="ERiC lehnt"):
        V.sende(XML_TESTVERSAND, "ESt_2025", zertifikat_pfad=str(zert), pin="falsch")
    assert ("EricBearbeiteVorgang" not in [c[0] for c in fake.calls]), \
        "kein Sendeaufruf, wenn schon das Zertifikat scheitert"


# ------------------------------------------------------------------ Erfolgspfad ---

def test_erfolgreicher_testversand_gemockt(monkeypatch, tmp_path):
    zert = tmp_path / "zert.pfx"
    zert.write_bytes(b"dummy")
    fake = _FakeEric(rc=0, rueckgabe=ANTWORT_ERFOLG)
    _patch(monkeypatch, fake)
    rc, rueck, _server = V.sende(XML_TESTVERSAND, "ESt_2025",
                                  zertifikat_pfad=str(zert), pin="1234")
    assert rc == 0
    assert V.telenummer(rueck) == "N552026081012345"
    bearbeite_calls = [c for c in fake.calls if c[0] == "EricBearbeiteVorgang"]
    assert len(bearbeite_calls) == 1
    flags = bearbeite_calls[0][1]
    assert flags & V.ERIC_SENDE, "ERIC_SENDE-Flag muss beim Versand gesetzt sein"
    assert flags & V.CE.ERIC_VALIDIERE


def test_erfolgreicher_echtversand_gemockt(monkeypatch, tmp_path):
    zert = tmp_path / "zert.pfx"
    zert.write_bytes(b"dummy")
    fake = _FakeEric(rc=0, rueckgabe=ANTWORT_ERFOLG)
    _patch(monkeypatch, fake)
    rc, rueck, _server = V.sende(XML_ECHT, "ESt_2025", zertifikat_pfad=str(zert), pin="1234",
                                  testmerker=None, echtversand_freigabe=V.ECHTVERSAND_FREIGABE)
    assert rc == 0
    assert V.telenummer(rueck) == "N552026081012345"


def test_misserfolg_ohne_telenummer(monkeypatch, tmp_path):
    zert = tmp_path / "zert.pfx"
    zert.write_bytes(b"dummy")
    fake = _FakeEric(rc=610101271, rueckgabe="<Elster><Fehler/></Elster>")  # ERIC_TRANSFER_ERR_SEND
    _patch(monkeypatch, fake)
    rc, rueck, _server = V.sende(XML_TESTVERSAND, "ESt_2025",
                                  zertifikat_pfad=str(zert), pin="1234")
    assert rc != 0
    assert V.telenummer(rueck) is None


# ----------------------------------------------------------------- telenummer() ---

def test_telenummer_fehlt_liefert_none():
    assert V.telenummer("<Elster><Fehler/></Elster>") is None


def test_telenummer_extrahiert_wert():
    assert V.telenummer(ANTWORT_ERFOLG) == "N552026081012345"


# --------------------------------------------------------------- zusammenfassung() ---

def test_zusammenfassung_leakt_kein_zertifikatspfad(tmp_path):
    zert = tmp_path / "geheim" / "mein_zertifikat.pfx"
    zert.parent.mkdir()
    zert.write_bytes(b"dummy")
    info = V.zusammenfassung(xml=XML_TESTVERSAND, datenart_version="ESt_2025",
                              testmerker=V.TESTMERKER, zertifikat_pfad=str(zert))
    dump = repr(info)
    assert str(zert) not in dump
    assert "geheim" not in dump
    assert info["zertifikat_vorhanden"] is True


def test_zusammenfassung_nennt_modus_und_merker():
    info = V.zusammenfassung(xml=XML_TESTVERSAND, datenart_version="ESt_2025",
                              testmerker=V.TESTMERKER, zertifikat_pfad="")
    assert info["merker_im_xml"] == "700000004"
    assert info["merker_konsistent"] is True
    assert "TESTVERSAND" in info["modus"]
    assert info["zertifikat_vorhanden"] is False


def test_zusammenfassung_meldet_inkonsistenz_ohne_wurf():
    info = V.zusammenfassung(xml=XML_ECHT, datenart_version="ESt_2025",
                              testmerker=V.TESTMERKER, zertifikat_pfad="")
    assert info["merker_konsistent"] is False


# ------------------------------------------------------------------------- CLI ---

def test_cli_dry_run_ruft_niemals_load_and_init(monkeypatch, tmp_path, capsys):
    def _boom():
        raise AssertionError("dry-run darf ERiC nie laden")
    monkeypatch.setattr(V.CE, "_load_and_init", _boom)
    xmlf = tmp_path / "fall.xml"
    xmlf.write_bytes(XML_TESTVERSAND)
    rc = V._cli(["--xml", str(xmlf), "--datenart", "ESt_2025", "--dry-run"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "DRY-RUN" in out
    assert "NICHTS gesendet" in out


def test_cli_echtversand_ohne_freigabe_flag_verweigert(monkeypatch, tmp_path, capsys):
    def _boom():
        raise AssertionError("ohne Freigabe darf ERiC nie geladen werden")
    monkeypatch.setattr(V.CE, "_load_and_init", _boom)
    xmlf = tmp_path / "fall.xml"
    xmlf.write_bytes(XML_ECHT)
    rc = V._cli(["--xml", str(xmlf), "--datenart", "ESt_2025", "--echtversand"])
    assert rc == 2
    assert "VERWEIGERT" in capsys.readouterr().out
