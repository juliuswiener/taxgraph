"""Besondere Kategorien nach Art. 9 DSGVO verlassen das Gerät nicht — und ein NEUES solches Feld
fällt auf, statt vergessen zu werden.

DER FUND (Audit 2026-08-16, gdpr-art9-und-drittdaten-an-llm), am 2026-08-18 nachgerechnet:
`_erklaer_kontext()` hängt bis zu 40 bestätigte Feldwerte an jeden /chat-Prompt, und
`pii_filter.filtere()` davor maskiert nur KENNUNGEN. Gemessen gingen unverändert hinaus:

    - Welchen Grad der Behinderung hat dein Schwerbehindertenausweis? → 80
    - Bist du hilflos, blind oder taubblind (Merkzeichen H, Bl oder TBl)? → ja
    - Welchen Pflegegrad hat die Person, die du zu Hause unentgeltlich pflegst? → 3

`filtere()` gab die Probe BYTE-IDENTISCH zurück und meldete keine Kategorie. Bei der gepflegten
Person wurden Anschrift und Geburtsdatum zu [PII] — ihr NAME blieb stehen, weil die Anrede-Regel
nur bei „Herr/Frau“ greift. Name plus Pflegegrad plus Verwandtschaftsverhältnis einer Person,
die nie mit dem System zu tun hatte, an einen Auftragsverarbeiter ohne Art.-28-Vertrag.

WARUM DER TEXTFILTER DAS NICHT KONNTE, und zwar prinzipiell: „80" ist als Zeichenfolge nicht von
einem Betrag zu unterscheiden. Ob eine Angabe eine Behinderung offenbart, weiss allein das FELD,
aus dem sie stammt. Die Sperre gehört deshalb auf die Feld-Ebene, an die Quelle.

WAS DIESE DATEI VOR ALLEM LEISTET, ist nicht die Sperre selbst (die sind zehn Zeilen), sondern
ihr Schutz vor dem Verrotten. Der Audit nennt genau das als Schwäche der Denylist-Variante:
„a newly added sensitive field can be forgotten; same drift class returns". Deshalb prüft
test_kein_neues_feld_rutscht_durch die GANZE Bindungstabelle gegen dieselbe Regel — ein neues
Feld mit einem solchen Merkmal macht den Test rot, bevor es jemand in einen Prompt schreibt.
Dieselbe Mechanik wie bei den Ratschen und Ausnahmelisten anderswo im Repo.

NULL LLM: es wird kein Anbieter angesprochen, geprüft wird der Text VOR dem Absenden.
"""
from __future__ import annotations

import os
import sys
import tempfile

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/import", "produkt/bescheid",
             "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api as API          # noqa: E402
import audit               # noqa: E402
import pii_filter as PII   # noqa: E402
import traverser as TR     # noqa: E402


# Felder, die eine besondere Kategorie offenbaren — gemessen, nicht geraten: die Regel in
# pii_filter._ART9 auf alle 307 Bindungsfelder angewandt ergab am 2026-08-19 diese 21.
def _art9_felder() -> list[str]:
    b = TR.lade_bindung()
    return sorted(f for f, e in b.items()
                  if PII.ist_besondere_kategorie(f, str(e.get("fragetext_laie") or "")))


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    API.fall_anlegen({"fall_id": "a9", "scheibe": "rentner_gesamt",
                      "veranlagungszeitraum": 2025})
    return "a9"


def _setze(fall_id: str, feld_id, wert) -> bool:
    try:
        API.event(fall_id, {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
                            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                         "haftung": "nutzer"},
                            "schreiber": "ui:laie",
                            "signal": {"signal_1": None, "signal_2": f"k@{feld_id}"}})
        return True
    except Exception:                      # noqa: BLE001 — Feld nicht in dieser Scheibe o.ä.
        return False


# ------------------------------------------------------------------ die Sperre selbst

def test_gesundheitsangaben_stehen_nicht_im_prompt(fall):
    """Der Kern, über den echten Kontextbau — nicht über die Regex allein."""
    gesetzt = [f for f, w in [("rentner_grad_der_behinderung", 80),
                              ("rentner_hilflos_blind_taubblind", True),
                              ("rentner_pflegegrad", 3)] if _setze(fall, f, w)]
    assert gesetzt, "keine der Angaben liess sich setzen — der Test prüft dann nichts"

    store = API.lade_fall(fall)
    kontext = API._erklaer_kontext(store, API._scheibe_bindung(store), None)

    for verraeterisch in ("Grad der Behinderung", "hilflos", "Pflegegrad"):
        for zeile in kontext.splitlines():
            if verraeterisch.lower() in zeile.lower() and "→" in zeile:
                pytest.fail(f"Der Wert einer besonderen Kategorie steht im Prompt: {zeile!r}")


def test_der_name_einer_dritten_person_bleibt_daheim(fall):
    """Die schärfste Stelle des Fundes: `rentner_gepflegter_angaben` fragt nach Name, Anschrift
    und Verhältnis der gepflegten Person. Anschrift und Datum fing der PII-Filter, den NAMEN
    nicht — er greift nur bei „Herr/Frau“. Jetzt geht das Feld gar nicht erst hinaus."""
    if not _setze(fall, "rentner_gepflegter_angaben",
                  "Maria Musterfrau, Beispielweg 3, 80331 Muenchen, meine Mutter"):
        pytest.skip("Feld nicht in dieser Scheibe")

    store = API.lade_fall(fall)
    kontext = API._erklaer_kontext(store, API._scheibe_bindung(store), None)
    assert "Musterfrau" not in kontext, (
        "Der Name der gepflegten Person steht im Prompt — genau die Lücke, die der Textfilter "
        "nicht schliessen konnte")


def test_die_kuerzung_wird_genannt_statt_verschwiegen(fall):
    """Eine stille Kürzung wäre der falsche Fix. Weiss die KI nicht, dass Angaben fehlen, hält
    sie den Kontext für vollständig und fragt nach Dingen, die längst beantwortet sind — der
    Nutzer erlebt das als schlechte Antwort, ohne den Grund zu sehen."""
    if not _setze(fall, "rentner_grad_der_behinderung", 80):
        pytest.skip("Feld nicht in dieser Scheibe")
    store = API.lade_fall(fall)
    kontext = API._erklaer_kontext(store, API._scheibe_bindung(store), None)
    assert "dürfen dir aber nicht übermittelt werden" in kontext, (
        f"Die zurückgehaltenen Angaben werden nicht erwähnt:\n{kontext[-400:]}")


def test_unverfaengliche_angaben_gehen_weiter_hinaus(fall):
    """Der Normalfall muss bleiben. Ohne diesen Test wäre die dichteste Sperre die beste — und
    ein Chat ohne jede Fallkenntnis ist wertlos (dieselbe Falle wie ein Deckel von 0)."""
    if not _setze(fall, "rentner_jahresrente", 2000000):
        pytest.skip("Feld nicht in dieser Scheibe")
    store = API.lade_fall(fall)
    kontext = API._erklaer_kontext(store, API._scheibe_bindung(store), None)
    assert "Das hat der Nutzer bereits bestätigt" in kontext, (
        "gar kein Kontext mehr — die Sperre greift zu weit")


# ------------------------------------------------------------------ Schutz vor dem Verrotten

def test_kein_neues_feld_rutscht_durch():
    """Der eigentliche Wert dieser Datei.

    Der Audit nennt als Schwäche jeder Denylist: „a newly added sensitive field can be
    forgotten; same drift class returns." Diese Prüfung schliesst das, indem sie die Sperre
    nicht gegen eine gepflegte Liste hält, sondern gegen die GANZE Bindungstabelle: jedes Feld,
    dessen feld_id oder Fragetext ein Art.-9-Merkmal nennt, MUSS von der Regel erfasst sein.

    Wer ein neues solches Feld anlegt, wird hier rot — bevor sein Wert je in einem Prompt steht.
    Die Liste unten ist bewusst KEINE Aufzählung der 21 heutigen Felder: eine Aufzählung müsste
    bei jedem neuen Feld gepflegt werden und verrottet."""
    b = TR.lade_bindung()
    # Begriffe, die eine besondere Kategorie ankündigen — unabhängig von der Regel in
    # pii_filter formuliert, damit der Test nicht bloss die Regel gegen sich selbst hält.
    verdaechtig = ("behinder", "konfession", "kirche", "pflege", "hilflos", "blind",
                   "taubblind", "merkzeichen", "berufsunfähig", "berufsunfaehig",
                   "erwerbsunfähig", "krankheitskosten", "heilbehandlung", "schwerbehind")

    # GEPRÜFT, KEIN ART. 9 — jeder Eintrag mit dem Grund, warum das Suchwort hier danebengreift.
    # Die Liste ist der ehrliche Teil dieses Tests: ohne sie müsste entweder das Suchwort so
    # eng werden, dass es echte Fälle verpasst, oder die Sperre so weit, dass sie den Chat
    # verstümmelt. Beim Bau am 2026-08-19 hat genau dieser Durchlauf zwei ECHTE Lücken gezeigt
    # (kind_pb_nicht_selbst_genutzt, rentner_pflege_durch) — die stehen nicht hier, sondern
    # sind in die Regel gewandert.
    geprueft_unverfaenglich = {
        # „Pflegeversicherung" ist ein Beitrag, den jeder zahlt — er offenbart keine
        # Pflegebedürftigkeit. Aus „PV-Beitrag 1.234 €" folgt nichts über den Gesundheitszustand.
        "basis_pv": "Beitrag zur Pflegeversicherung, kein Merkmal",
        "basis_pv_partner": "Beitrag zur Pflegeversicherung, kein Merkmal",
        "kind_pv": "Beitrag zur Pflegeversicherung des Kindes, kein Merkmal",
        "p33a_unterhalt_kv_pv": "Beiträge für eine unterhaltsberechtigte Person, kein Merkmal",
        "realsplitting_empfaenger_kv_pv": "Beiträge für den Ex-Partner, kein Merkmal",
        "realsplitting_empfaenger_kv_krankengeld": "Anteil eines Beitrags, kein Merkmal",
        "versicherungsart": "gesetzlich/privat — sagt nichts über Gesundheit",
        "versicherungsart_partner": "gesetzlich/privat — sagt nichts über Gesundheit",
        # „Pflegekind" ist ein Verwandtschaftsverhältnis, kein Gesundheitsdatum.
        "kind_kindschaftsverhaeltnis_a": "Verwandtschaft (leiblich/Adoptiv/Pflegekind)",
        "kind_kindschaftsverhaeltnis_b": "Verwandtschaft (leiblich/Adoptiv/Pflegekind)",
        "kind_anderer_elternteil_kindschaftsverhaeltnis": "Verwandtschaft, kein Merkmal",
        "kind_unter_14_haushaltszugehoerig": "Alter und Haushaltszugehörigkeit, kein Merkmal",
        # Freitext für eine Haushaltsleistung — das Suchwort trifft „Gartenpflege".
        "hh_dienstleistung_art": "Art der Haushaltsleistung, z.B. 'Gartenpflege'",
        # § 34 Abs. 3 setzt Alter >= 55 ODER Berufsunfähigkeit voraus; der Wert ist ein Ja/Nein
        # zum ermässigten Steuersatz und lässt offen, welche der beiden Voraussetzungen greift.
        # Die Berufsunfähigkeit selbst steht in `dauernd_berufsunfaehig` — das IST gesperrt.
        "antrag_ermaessigter_satz": "Antrag auf ermässigten Satz; die Voraussetzung selbst "
                                    "steht in dauernd_berufsunfaehig und ist gesperrt",
    }

    durchgerutscht = []
    for fid, e in sorted(b.items()):
        frage = str(e.get("fragetext_laie") or "")
        text = f"{fid} {frage}".lower()
        if (any(w in text for w in verdaechtig)
                and not PII.ist_besondere_kategorie(fid, frage)
                and fid not in geprueft_unverfaenglich):
            durchgerutscht.append(f"{fid}: „{frage[:70]}…")
    assert not durchgerutscht, (
        "Bindungsfelder nennen ein Merkmal nach Art. 9 DSGVO, werden von pii_filter._ART9 aber "
        "nicht erfasst — ihr Wert ginge an den LLM-Anbieter:\n  " + "\n  ".join(durchgerutscht)
        + "\n\nEntweder die Regel in pii_filter._ART9 erweitern, oder das Feld in "
          "geprueft_unverfaenglich eintragen — MIT dem Grund, warum das Suchwort danebengreift.")

    # Tote Einträge: ein Feld, das die Regel inzwischen erfasst, gehört nicht mehr in die
    # Unverfänglich-Liste — sonst deckt der Eintrag beim nächsten Mal ein echtes Merkmal mit ab.
    zombies = sorted(f for f in geprueft_unverfaenglich
                     if f in b and PII.ist_besondere_kategorie(
                         f, str(b[f].get("fragetext_laie") or "")))
    assert not zombies, (
        f"{zombies} stehen als 'geprüft unverfänglich' drin, werden von der Regel aber "
        f"inzwischen erfasst — Eintrag streichen.")
    fehlend = sorted(f for f in geprueft_unverfaenglich if f not in b)
    assert not fehlend, f"{fehlend} gibt es in der Bindung nicht mehr — Eintrag streichen."


def test_die_regel_findet_die_bekannten_felder():
    """Gegenprobe gegen die bequemste Art, den Test oben grün zu bekommen: die Regel so weit
    fassen, dass sie alles trifft — oder so eng, dass sie nichts trifft. Bei der Messung am
    2026-08-19 waren es 21 von 307 Feldern."""
    treffer = _art9_felder()
    assert 12 <= len(treffer) <= 60, (
        f"{len(treffer)} von {len(TR.lade_bindung())} Feldern als besondere Kategorie erkannt "
        f"(erwartet 12–60; bei der Messung 21). Zu wenige heisst: die Regel greift nicht mehr. "
        f"Zu viele heisst: sie sperrt den Chat aus, ohne dass jemand es gemerkt hat.\n"
        f"{treffer[:20]}")
    for pflicht in ("kist_konfession", "rentner_grad_der_behinderung",
                    "rentner_hilflos_blind_taubblind", "kind_grad_der_behinderung"):
        assert pflicht in treffer, f"{pflicht} wird nicht als besondere Kategorie erkannt"


def test_betraege_werden_nicht_mitgesperrt():
    """Die Gegenrichtung, bewusst festgehalten: aus „Kirchensteuer gezahlt: 412 €" folgt keine
    Konfession, und die Zahl steht auf jeder Lohnsteuerbescheinigung. Sie zu sperren würde den
    Chat verstümmeln, ohne etwas zu schützen — eine Sperre, die zu viel nimmt, wird irgendwann
    ganz abgeschaltet."""
    for harmlos in ("kist_gezahlt", "kist_erstattet", "basis_kv", "basis_pv"):
        assert not PII.ist_besondere_kategorie(harmlos, ""), (
            f"{harmlos} ist ein Betrag, kein Merkmal — er nennt die besondere Kategorie nicht")


def test_die_regel_erkennt_ihren_eigenen_fehlerfall():
    """Ohne diese Probe wäre nicht belegt, dass ist_besondere_kategorie überhaupt anschlägt."""
    assert PII.ist_besondere_kategorie("irgendein_feld", "Welchen Pflegegrad hat die Person?")
    assert PII.ist_besondere_kategorie("kind_grad_der_behinderung", "")
    assert not PII.ist_besondere_kategorie("bruttoarbeitslohn", "Wie hoch war dein Bruttolohn?")
