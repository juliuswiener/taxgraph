#!/usr/bin/env python
"""ELSTER-VERSAND (`EricBearbeiteVorgang` mit `ERIC_VALIDIERE | ERIC_SENDE`) — Phase 4.4.

NIE automatisch ausgeloest. Der Versand ist eine ausgehende, unwiderrufliche Aktion an eine
Behoerde — dieses Modul stellt den Pfad bereit, ruft ihn aber nirgends selbst auf. Kein
CLI-Default sendet, kein Test sendet real (alle Tests mocken `EricBearbeiteVorgang`). Auslösen
ist Julius' Aufgabe, siehe `elster/VERSAND.md`.

Unterschied zu `checkest_gate.validate()`: dort nur `ERIC_VALIDIERE` (lokal, offline, kein
Zertifikat noetig). Hier zusaetzlich `ERIC_SENDE` — braucht ein Nutzer-Zertifikat + PIN
(`eric_verschluesselungs_parameter_t`, ericapi.h) und geht ins Netz.

Sicherheitsmodell (der eigentliche Zweck dieser Datei):
  - Default ist IMMER Testversand: Testmerker `TESTMERKER` (amtlich `ERIC_TESTMERKER_CLEARINGSTELLE`,
    ericdef.h — "werden in der Clearingstelle aussortiert und verworfen. Es findet keine
    Verarbeitung im Finanzamt statt.").
  - Echtversand (`testmerker=None`) verlangt `echtversand_freigabe` woertlich gleich
    `ECHTVERSAND_FREIGABE` — kein Bool, kein Default, keine Kurzform. Fehlt sie oder stimmt sie
    nicht exakt, wird VOR jedem ERiC-Aufruf abgebrochen (`EchtversandOhneFreigabe`).
  - VOR jedem ERiC-Aufruf wird das XML selbst gegen den behaupteten Modus geprueft
    (`_pruefe_merker_konsistenz`) — die Funktion vertraut dem Aufrufer nicht: behauptet er
    Testversand, muss das XML exakt `TESTMERKER` tragen; behauptet er Echtversand, darf gar
    kein `<Testmerker>`-Element vorhanden sein. Bei Abweichung: `XmlMerkerMismatch`, kein
    ERiC-Aufruf, keine Ausnahme.
  - Ohne Zertifikat (fehlender Pfad, fehlende Datei, fehlende PIN) bricht `sende()` mit
    `ZertifikatFehlt` ab, BEVOR `EricBearbeiteVorgang` ueberhaupt aufgerufen wird.

    from elster.versand import sende, ECHTVERSAND_FREIGABE
    rc, rueckgabe_xml, serverantwort_xml = sende(
        xml, "ESt_2025", zertifikat_pfad=..., pin=...)               # Testversand (Default)
    rc, rueckgabe_xml, serverantwort_xml = sende(
        xml, "ESt_2025", zertifikat_pfad=..., pin=..., testmerker=None,
        echtversand_freigabe=ECHTVERSAND_FREIGABE)                   # ECHTVERSAND

Erfolg heisst rc==0 UND `telenummer(rueckgabe_xml)` liefert eine Telenummer (ericapi.h:
"der Text im Pufferspeicher rueckgabeXmlPuffer enthaelt beim Versand XML-Daten mit generierter
Telenummer"). `serverantwortXmlPuffer` traegt zusaetzlich die Antwort des Annahmeservers.

ERiC-Pfad aus $ERIC_DIR (Default ~/02_Software/eric), nie hart im Code — geteilt mit
checkest_gate ueber `CE._load_and_init()` (eine Instanz pro Prozess).
"""
from __future__ import annotations

import argparse
import ctypes
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import checkest_gate as CE  # noqa: E402  (teilt _load_and_init/ERIC_VALIDIERE/Instanz-Zustand)

ERIC_SENDE = 1 << 2   # eric_types.h: eric_bearbeitung_flag_t.ERIC_SENDE — zusaetzlich zu ERIC_VALIDIERE

# ericdef.h: #define ERIC_TESTMERKER_CLEARINGSTELLE "700000004" — woertlich: "Bei der Verwendung
# dieses Testmerkers werden die Faelle in der Clearingstelle aussortiert und verworfen. Es
# findet keine Verarbeitung im Finanzamt statt." Identisch mit elster_xml.TESTMERKER_ERIC.
TESTMERKER = "700000004"

# Woertliche Freigabe-Phrase fuer den Echtversand. Bewusst lang, bewusst kein Bool/Kurzform —
# ein Aufrufer, der sie nicht ausdruecklich abtippt oder kopiert, bekommt nie einen Echtversand.
ECHTVERSAND_FREIGABE = "JA ICH SENDE ECHT AN DAS FINANZAMT"

_TESTMERKER_TAG_RE = re.compile(rb"<Testmerker>([^<]*)</Testmerker>")
_TELENUMMER_RE = re.compile(r"<Telenummer>([^<]+)</Telenummer>")


class VersandGesperrt(Exception):
    """Basisklasse: sende() hat VOR jedem ERiC-Aufruf abgebrochen (fail-closed)."""


class EchtversandOhneFreigabe(VersandGesperrt):
    """testmerker=None ohne (oder mit falscher) echtversand_freigabe."""


class XmlMerkerMismatch(VersandGesperrt):
    """Das XML traegt nicht den Merker, den der Aufrufer behauptet."""


class ZertifikatFehlt(VersandGesperrt):
    """Zertifikatspfad/-datei/PIN fehlt, oder ERiC lehnt das Zertifikat ab."""


def _merker_in_xml(xml: bytes) -> str | None:
    m = _TESTMERKER_TAG_RE.search(xml)
    return m.group(1).decode() if m else None


def _pruefe_merker_konsistenz(xml: bytes, testmerker: str | None) -> None:
    """Gegenprobe VOR jedem ERiC-Aufruf: das XML muss den behaupteten Merker tragen —
    unabhaengig davon, was der Aufrufer sagt oder was erzeuge_xml() eigentlich gebaut hat.
    Kein Vertrauen in den Aufrufer (siehe Modul-Docstring)."""
    gefunden = _merker_in_xml(xml)
    if testmerker is not None:
        if gefunden != testmerker:
            raise XmlMerkerMismatch(
                f"Testversand angefordert (Merker {testmerker!r}), aber das XML traegt "
                f"{gefunden!r}. Abgebrochen VOR jedem ERiC-Aufruf — kein Versand.")
    else:
        if gefunden is not None:
            raise XmlMerkerMismatch(
                f"Echtversand angefordert (kein Testmerker), aber das XML traegt noch "
                f"<Testmerker>{gefunden}</Testmerker>. Abgebrochen VOR jedem ERiC-Aufruf — "
                f"kein Versand. Das waere ohnehin nur in der Clearingstelle verworfen worden, "
                f"aber der Aufrufer hat offensichtlich das falsche XML gebaut.")


class _VerschluesselungsParameter(ctypes.Structure):
    """ctypes-Abbild von eric_verschluesselungs_parameter_t (eric_types.h:298-318).
    version muss laut Doku derzeit immer 3 sein."""
    _fields_ = [
        ("version", ctypes.c_uint32),
        ("zertifikatHandle", ctypes.c_uint32),
        ("pin", ctypes.c_char_p),
    ]


def _konfiguriere_zertifikat_signaturen(eric) -> None:
    """Signaturen fuer die Zertifikatsfunktionen, die checkest_gate._load_and_init() nicht
    braucht und deshalb nicht setzt (ericapi.h:1208, 487)."""
    eric.EricGetHandleToCertificate.restype = ctypes.c_int
    eric.EricGetHandleToCertificate.argtypes = [
        ctypes.POINTER(ctypes.c_uint32), ctypes.POINTER(ctypes.c_uint32), ctypes.c_char_p]
    eric.EricCloseHandleToCertificate.restype = ctypes.c_int
    eric.EricCloseHandleToCertificate.argtypes = [ctypes.c_uint32]


def _lade_zertifikat(eric, pfad: str, pin: str) -> int:
    """Oeffnet das Zertifikat-Handle. Bricht mit ZertifikatFehlt ab, BEVOR irgendein
    ERiC-Sendeaufruf stattfindet — nie Pfad oder PIN im Fehlertext (nur Existenz/Basisname
    des Ordners waere schon zu viel; wir nennen nur, WAS fehlt, nicht WO)."""
    if not pfad:
        raise ZertifikatFehlt(
            "Kein Zertifikatspfad gesetzt. $ELSTER_ZERTIFIKAT_PFAD exportieren oder "
            "zertifikat_pfad= uebergeben — siehe elster/VERSAND.md.")
    if not os.path.isfile(pfad):
        raise ZertifikatFehlt(
            "Zertifikatsdatei nicht gefunden (Pfad gesetzt, Datei existiert nicht) — "
            "siehe elster/VERSAND.md.")
    if not pin:
        raise ZertifikatFehlt(
            "Keine PIN gesetzt. $ELSTER_ZERTIFIKAT_PIN exportieren oder pin= uebergeben — "
            "siehe elster/VERSAND.md.")
    _konfiguriere_zertifikat_signaturen(eric)
    htoken = ctypes.c_uint32(0)
    info = ctypes.c_uint32(0)
    rc = eric.EricGetHandleToCertificate(
        ctypes.byref(htoken), ctypes.byref(info), pfad.encode())
    if rc != 0:
        raise ZertifikatFehlt(
            f"ERiC lehnt das Zertifikat ab (EricGetHandleToCertificate rc={rc}) — "
            f"Pfad/Format/PIN pruefen, siehe elster/VERSAND.md. Wert nicht geloggt.")
    return htoken.value


def zusammenfassung(*, xml: bytes | str, datenart_version: str, testmerker: str | None,
                     zertifikat_pfad: str) -> dict:
    """Nur unbedenkliche Metadaten fuers Logging/--dry-run. NIE Zertifikatspfad, PIN,
    Steuernummer oder IBAN im Klartext (die stehen im XML-Inhalt, der hier nie ausgegeben wird)."""
    xb = xml.encode("utf-8") if isinstance(xml, str) else xml
    return {
        "datenart_version": datenart_version,
        "modus": "TESTVERSAND (Clearingstelle, wird verworfen)" if testmerker else "ECHTVERSAND",
        "merker_im_xml": _merker_in_xml(xb),
        "merker_konsistent": _merker_konsistent_ohne_wurf(xb, testmerker),
        "xml_bytes": len(xb),
        "zertifikat_vorhanden": bool(zertifikat_pfad) and os.path.isfile(zertifikat_pfad),
    }


def _merker_konsistent_ohne_wurf(xml: bytes, testmerker: str | None) -> bool:
    try:
        _pruefe_merker_konsistenz(xml, testmerker)
        return True
    except XmlMerkerMismatch:
        return False


def telenummer(rueckgabe_xml: str) -> str | None:
    """Telenummer aus dem Rueckgabepuffer — der Erfolgsnachweis (ericapi.h: 'der Text im
    Pufferspeicher rueckgabeXmlPuffer enthaelt beim Versand XML-Daten mit generierter
    Telenummer')."""
    m = _TELENUMMER_RE.search(rueckgabe_xml)
    return m.group(1) if m else None


def sende(xml: bytes | str, datenart_version: str, *, zertifikat_pfad: str, pin: str,
          testmerker: str | None = TESTMERKER,
          echtversand_freigabe: str | None = None) -> tuple[int, str, str]:
    """EricBearbeiteVorgang(ERIC_VALIDIERE | ERIC_SENDE) — der eigentliche Versand.

    testmerker=TESTMERKER (Default): Testversand gegen die Clearingstelle, dort verworfen,
    amtlich KEINE Verarbeitung im Finanzamt.
    testmerker=None: ECHTVERSAND. Verlangt echtversand_freigabe==ECHTVERSAND_FREIGABE woertlich.

    In JEDEM Fall zuerst die Freigabe-Pruefung, dann die XML-Merker-Gegenprobe, dann erst das
    Zertifikat — jede Stufe bricht fail-closed ab, bevor die naechste beginnt bzw. bevor
    EricBearbeiteVorgang aufgerufen wird.

    Rueckgabe: (rc, rueckgabe_xml, serverantwort_xml). rc==0 UND telenummer(rueckgabe_xml)
    gesetzt heisst: wirklich uebermittelt (im Testfall: an die Clearingstelle, dort verworfen).
    """
    xb = xml.encode("utf-8") if isinstance(xml, str) else xml

    if testmerker is None:
        if echtversand_freigabe != ECHTVERSAND_FREIGABE:
            raise EchtversandOhneFreigabe(
                "Echtversand verlangt echtversand_freigabe woertlich gleich "
                f"{ECHTVERSAND_FREIGABE!r} — abgebrochen VOR jedem ERiC-Aufruf.")
    elif testmerker != TESTMERKER:
        raise ValueError(
            f"Unbekannter Testmerker {testmerker!r} — nur {TESTMERKER!r} oder None "
            f"(Echtversand) sind zulaessig.")

    _pruefe_merker_konsistenz(xb, testmerker)

    eric = CE._load_and_init()
    htoken = _lade_zertifikat(eric, zertifikat_pfad, pin)
    try:
        crypto = _VerschluesselungsParameter(version=3, zertifikatHandle=htoken, pin=pin.encode())
        rueck = eric.EricRueckgabepufferErzeugen()
        server = eric.EricRueckgabepufferErzeugen()
        try:
            rc = eric.EricBearbeiteVorgang(
                xb, datenart_version.encode(), CE.ERIC_VALIDIERE | ERIC_SENDE,
                None, ctypes.byref(crypto), rueck, server)
            rueck_raw = eric.EricRueckgabepufferInhalt(rueck)
            server_raw = eric.EricRueckgabepufferInhalt(server)
            rueck_txt = rueck_raw.decode("utf-8", "replace") if rueck_raw else ""
            server_txt = server_raw.decode("utf-8", "replace") if server_raw else ""
        finally:
            eric.EricRueckgabepufferFreigeben(rueck)
            eric.EricRueckgabepufferFreigeben(server)
    finally:
        eric.EricCloseHandleToCertificate(htoken)
    return rc, rueck_txt, server_txt


# --------------------------------------------------------------------------------- CLI ---

def _cli(argv: list[str]) -> int:
    p = argparse.ArgumentParser(
        description="ELSTER-Versand (EricBearbeiteVorgang mit ERIC_SENDE). "
                    "Siehe elster/VERSAND.md VOR der ersten Nutzung.")
    p.add_argument("--xml", required=True, help="Pfad zur bereits erzeugten Submission-XML-Datei")
    p.add_argument("--datenart", required=True, help="z.B. ESt_2025")
    p.add_argument("--zertifikat", default=os.environ.get("ELSTER_ZERTIFIKAT_PFAD", ""),
                   help="Pfad zur Zertifikatsdatei (Default: $ELSTER_ZERTIFIKAT_PFAD)")
    modus = p.add_mutually_exclusive_group(required=True)
    modus.add_argument("--dry-run", action="store_true",
                       help="Nichts senden — nur zeigen, was passieren wuerde")
    modus.add_argument("--testversand", action="store_true",
                       help=f"Versand mit Testmerker {TESTMERKER} (Clearingstelle verwirft)")
    modus.add_argument("--echtversand", action="store_true",
                       help="ECHTER Versand ans Finanzamt — verlangt --freigabe")
    p.add_argument("--freigabe", default=None,
                   help="Nur fuer --echtversand: die woertliche Freigabe-Phrase")
    args = p.parse_args(argv)

    with open(args.xml, "rb") as f:
        xml = f.read()

    if args.dry_run:
        info = zusammenfassung(xml=xml, datenart_version=args.datenart,
                               testmerker=(None if args.echtversand else TESTMERKER),
                               zertifikat_pfad=args.zertifikat)
        print("[versand] DRY-RUN — es wird NICHTS gesendet.")
        for k, v in info.items():
            print(f"[versand]   {k}: {v}")
        if not info["merker_konsistent"]:
            print("[versand]   WARNUNG: XML und Modus passen nicht zusammen — ein echter "
                 "Lauf wuerde HIER mit XmlMerkerMismatch abbrechen, bevor irgendetwas gesendet wird.")
        return 0

    pin = os.environ.get("ELSTER_ZERTIFIKAT_PIN", "")
    if args.echtversand:
        if args.freigabe != ECHTVERSAND_FREIGABE:
            print("[versand] ECHTVERSAND VERWEIGERT — --freigabe muss woertlich sein:")
            print(f"[versand]   {ECHTVERSAND_FREIGABE}")
            return 2
        try:
            bestaetigung = input(
                f"Echtversand ans Finanzamt — tippe zur Bestaetigung erneut ein "
                f"(nicht kopieren):\n{ECHTVERSAND_FREIGABE}\n> ")
        except EOFError:
            print("[versand] ECHTVERSAND VERWEIGERT — keine interaktive Bestaetigung moeglich.")
            return 2
        if bestaetigung != ECHTVERSAND_FREIGABE:
            print("[versand] ECHTVERSAND VERWEIGERT — Bestaetigung stimmte nicht ueberein.")
            return 2
        testmerker, echt_freigabe = None, ECHTVERSAND_FREIGABE
    else:
        testmerker, echt_freigabe = TESTMERKER, None

    try:
        rc, rueck, _server = sende(xml, args.datenart, zertifikat_pfad=args.zertifikat, pin=pin,
                                   testmerker=testmerker, echtversand_freigabe=echt_freigabe)
    except VersandGesperrt as e:
        print(f"[versand] ABGEBROCHEN: {e}")
        return 2

    tn = telenummer(rueck)
    print(f"[versand] rc={rc}")
    print(f"[versand] Telenummer: {tn if tn else '(keine — kein Erfolg)'}")
    if rc != 0 or not tn:
        print("[versand] KEIN ERFOLG. Siehe elster/VERSAND.md, Abschnitt 'Wenn es schiefgeht' — "
             "NICHT blind wiederholen.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(_cli(sys.argv[1:]))
