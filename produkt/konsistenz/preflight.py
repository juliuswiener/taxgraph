"""K2-Preflight-Orchestrator — Sammelt alle Konsistenz-Prüfungen vor Submission. NULL LLM.

Führt nacheinander aus:
  1. flag_check.flag_widersprueche()  — Abwesenheits-Flag ↔ Einkunftsart
  2. partner_check.partner_ohne_zusammen() + alleinerziehend_mit_zusammen()  — Partner-Feld ↔ Veranlagung
  3. check_pauschalen.pauschal_hinweise()  — Vergessene Pauschalen (soft)
  4. plausibilitaets_widersprueche()  — Betrag ↔ Bezugsgröße (hier im Modul, s. u.)

Ergebnis: dict mit den Ergebnissen + aggregiertem status (RED/AMBER/GREEN).
"""
from __future__ import annotations

import os
import sys

_PRODUKT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(_PRODUKT, "konsistenz"))
sys.path.insert(0, os.path.join(_PRODUKT, "traverser"))
import flag_check       # noqa: E402
import partner_check    # noqa: E402
import check_pauschalen  # noqa: E402
import check_nicht_gerechnet  # noqa: E402
import traverser as TR   # noqa: E402
from _helpers import _bestaetigt_wert  # noqa: E402


# ===================== Kreuz-Plausibilisierung (Betrag ↔ Bezugsgröße) =====================
# Anlass, gemessen am 2026-08-27 an einem echten Durchgang: bei 40.000 € Bruttoarbeitslohn gingen
# 12.123.213 € Lohnsteuer, 243.234 € eigener Rentenversicherungsanteil, 22.222 € Kirchensteuer und
# 234.234 € Schulgeld für ein Kind glatt durch — nichts schlug an. Das ist keine Lücke, die woanders
# aufgefangen würde: bindung_p36_abschlusszahlung.yaml hält zu `p36_lohnsteuer` ausdrücklich fest,
# „checkESt kennt keine Betrags-Plausibilitätsprüfung gegen den Bruttoarbeitslohn". Fällt sie hier
# aus, fällt sie ganz aus.
#
# Drei Eigenschaften, die diese Prüfungen von den übrigen im Modul unterscheiden:
#
#  1. Sie MELDEN, sie sperren nicht. Ein zu hoher Betrag kann in seltenen Fällen richtig sein; nur
#     der Nutzer weiß das. Der Text nennt daher beide Zahlen und bittet, die richtige zu prüfen —
#     ohne Feldnamen, die außerhalb dieses Repos niemand kennt.
#
#  2. Ohne Bezugsgröße bleiben sie STILL. Fehlt der Bruttoarbeitslohn (unbestätigt oder 0), ist
#     „unplausibel" eine Aussage über unser Nichtwissen, nicht über die Eingabe — das wäre Rauschen
#     an genau der Stelle, an der der Nutzer Vertrauen aufbauen soll. Fail-closed heißt hier
#     schweigen, nicht raten.
#
#  3. Die Schwellen sind Größenordnungen, keine Feinjustierung. Sie sind bewusst so weit gesetzt,
#     dass ein echter Grenzfall durchläuft; sie fangen Tippfehler um Faktor 10 und mehr.
#
# Wo eine Schwelle einen gesetzlichen Wert braucht, steht seine Herkunft an der Konstante. Wo im
# Repo kein belegter Wert liegt (allgemeine Beitragsbemessungsgrenze und RV-Beitragssatz sind NICHT
# in params/ — dort steht nur die knappschaftliche Größe für den § 10 Abs. 3-Höchstbetrag), prüfen
# wir bewusst ein gröberes Verhältnis, das ohne den Wert auskommt, statt einen Wert zu erfinden.

# Kirchensteuer ist ein Zuschlag auf die Einkommensteuer, höchstens 8 bzw. 9 v.H. (Bayern und
# Baden-Württemberg 8, übrige Länder 9; statutarische Obergrenze 10 v.H. nach Art. 8 Abs. 1
# KirchStG Bayern — sources/kirchensteuer/kist_hebesatz_2026-07-22.txt). Die Einkommensteuer selbst
# ist kleiner als der Bruttolohn, also liegt die Kirchensteuer normal unter 10 % des Bruttolohns.
# Faktor 3 als Luft nach oben, weil in einem Jahr auch Nachzahlungen für Vorjahre anfallen können.
_KIST_ANTEIL_BRUTTO_MAX = 0.30

# § 10 Abs. 1 Nr. 9 S. 1 EStG, Gesetzeswortlaut: „30 Prozent des Entgelts, höchstens 5 000 Euro"
# (Zitatanker in bindung_p10_1_9_schulgeld_gesamt.yaml, Quelle
# sources/gesetze-im-internet/estg_p10_2026-07-11.txt). Der Satz von 30 % ist auch in
# params/{2024,2025,2026}/schulgeld_p10.yaml hinterlegt und über alle drei VZ identisch — deshalb
# ist er hier als Konstante vertretbar, obwohl preflight() den Veranlagungszeitraum nicht kennt.
# Ab rund 16.700 € Schulgeld ist der Höchstbetrag ausgeschöpft; alles darüber ändert am Abzug nichts
# mehr. Erst das Zehnfache dieser Grenze gilt uns als unplausibel — eine teure Privatschule bleibt
# damit unbeanstandet.
_SCHULGELD_HOECHSTBETRAG_CENT = 5_000_00
_SCHULGELD_ABZUGSSATZ = 0.30
_SCHULGELD_FAKTOR = 10

# Einbehaltene und gezahlte Kirchensteuer beschreiben bei einem reinen Arbeitnehmer dieselbe Zahlung
# aus zwei Blickwinkeln. Sie dürfen auseinanderliegen (Nachzahlung, Kirchgeld, Kapitalertragsteuer),
# aber nicht um eine Größenordnung. Kein gesetzlicher Wert nötig.
_KIST_ABGLEICH_FAKTOR = 10


def _eur(cent: int) -> str:
    """Cent → „12.123.213 €". Alle hier geprüften Beträge sind `typ: cent` laut Bindung."""
    return f"{cent // 100:,}".replace(",", ".") + " €"


def _bestaetigter_betrag(snapshot: dict, feld_id: str):
    """Bestätigter Geldbetrag > 0 in Cent, sonst None.

    `True` ist in Python eine 1 — ohne den bool-Ausschluss würde ein bestätigtes Ja-Feld als
    Betrag von einem Cent durchgehen und die Verhältnisrechnung lautlos verfälschen.
    """
    wert = _bestaetigt_wert(snapshot, feld_id)
    if isinstance(wert, bool) or not isinstance(wert, (int, float)) or wert <= 0:
        return None
    return wert


def _schulgeld_felder(snapshot: dict) -> list:
    """(feld_id, betrag) je Kind. Schulgeld ist `instanz_gruppe: kind`, liegt also flach im
    Snapshot als `schulgeld` (Kind 1) und `schulgeld__2`, `schulgeld__3`, … (Kind 2..N) —
    dieselbe __n-Konvention wie in est_mapping.instanzen. Wer nur die Basis prüft, sieht das
    zweite Kind nicht."""
    treffer = []
    for feld_id in sorted(snapshot):
        ist_instanz = (feld_id.startswith("schulgeld__")
                       and feld_id[len("schulgeld__"):].isdigit())
        if feld_id != "schulgeld" and not ist_instanz:
            continue
        betrag = _bestaetigter_betrag(snapshot, feld_id)
        if betrag is not None:
            treffer.append((feld_id, betrag))
    return treffer


def _aufzaehlung(etikett: str, nummern: list) -> str:
    """[3] -> „Kind 3"; [2, 3] -> „Kind 2 und Kind 3"; [2, 3, 4] -> „Kind 2, Kind 3 und Kind 4".

    Das Etikett bleibt im Singular, weil es aus der Bindung kommt und dort kein Plural steht — und
    einen deutschen Plural zu RATEN geht bei „Objekt/Gerät/Verkauf" reihenweise schief.
    """
    teile = [f"{etikett} {n}" for n in nummern]
    return teile[0] if len(teile) == 1 else ", ".join(teile[:-1]) + " und " + teile[-1]


def _frage_kurz(feld_id: str, bindung: dict) -> str:
    """Der Fragetext bis zum Fragezeichen — der Nutzer erkennt die Frage wieder, ohne dass ihm der
    ganze Erklärsatz noch einmal vorgelegt wird (`schulgeld` trägt drei Zeilen Hinweis mit)."""
    text = (bindung.get(feld_id) or {}).get("fragetext_laie") or ""
    kopf, marke, _ = text.partition("?")
    return (kopf + marke) if marke else text


def unvollstaendige_instanzen(snapshot: dict) -> list:
    """Angekündigt, aber nicht ausgefüllt: „3 Kinder angegeben, 2 Namen eingetragen".

    ANLASS, gemessen am 2026-08-27: wer drei Kinder angibt und nur zwei Namen einträgt, verliert
    den dritten LAUTLOS. Ist die erste Instanz beantwortet, fällt das Feld ganz aus dem Fragebogen
    — der Traverser führt nur das Basisfeld, `kind_vorname__3` steht dort nie als eigene Frage.
    Die Oberfläche versprach an zwei Stellen wörtlich das Gegenteil („die dritte Frage bleibt offen
    und kommt im Fragebogen wieder"); die Zusage galt nie.

    WARUM DIE MELDUNG HIER STEHT UND NICHT DIE FRAGE OFFEN BLEIBT: das Zählfeld ist nach dem
    Beantworten selbst nicht mehr im Fragebogen. Der Nutzer könnte „es sind doch nur zwei" also
    gar nicht mehr sagen, und die Frage würde nie schliessen — eine Sackgasse statt einer Lücke.
    Gemeldet wird sie hier, wo Angabe gegen Angabe steht, wie die IBAN-trotz-„keine Bankverbindung"
    darunter: derselbe Bauart-Fall, ohne Betrag und ohne Bezugsgrösse.

    Welche Instanzen fehlen, rechnet `traverser.fehlende_instanzen` aus — die `__n`-Konvention
    stand am selben Tag an vier Stellen nachgebaut (auch gleich nebenan in `_schulgeld_felder`),
    und eine fünfte hätte sie nicht besser gemacht.
    """
    bindung = TR.lade_bindung()
    return [{
        "feld_id": luecke["feld_id"], "wert": len(luecke["vorhanden"]), "bezug": luecke["anzahl"],
        "grund": f"Angegeben hast du {luecke['anzahl']}, ausgefüllt sind "
                 f"{len(luecke['vorhanden'])}: auf die Frage "
                 f"»{_frage_kurz(luecke['feld_id'], bindung)}« fehlt die Antwort für "
                 f"{_aufzaehlung(luecke['etikett'], luecke['fehlend'])}. Diese Frage kommt im "
                 f"Fragebogen nicht noch einmal — ohne die Antwort steht "
                 f"{_aufzaehlung(luecke['etikett'], luecke['fehlend'])} nicht in deiner "
                 f"Steuererklärung. Bitte trage die fehlende Angabe nach, oder gib die Zahl an, "
                 f"die wirklich in die Erklärung soll."}
        for luecke in TR.fehlende_instanzen(snapshot, bindung)]


def plausibilitaets_widersprueche(snapshot: dict) -> list:
    """{feld_id -> {wert, zustand, ...}} → Liste der Betrag↔Bezugsgröße-Widersprüche.

    Rein deterministisch, nur bestätigte Werte (vorläufig ist kein Beleg). Ohne die jeweilige
    Bezugsgröße wird die betroffene Prüfung übersprungen, nicht geschätzt."""
    widersprueche = []
    brutto = _bestaetigter_betrag(snapshot, "bruttoarbeitslohn")

    if brutto is not None:
        # Lohnsteuer über dem Lohn. Der Spitzensteuersatz beträgt 0,45 (§ 32a Abs. 1 S. 2 Nr. 5
        # EStG, params/<vz>/einkommensteuertarif_p32a.yaml: zone5_faktor) — die Lohnsteuer bleibt
        # also immer deutlich unter dem halben Lohn. Die Schwelle „mehr als der Lohn selbst" liegt
        # weit darüber und meldet nur, was rechnerisch unmöglich ist.
        lohnsteuer = _bestaetigter_betrag(snapshot, "p36_lohnsteuer")
        if lohnsteuer is not None and lohnsteuer > brutto:
            widersprueche.append({
                "feld_id": "p36_lohnsteuer", "wert": lohnsteuer, "bezug": brutto,
                "grund": f"Bei einem Bruttoarbeitslohn von {_eur(brutto)} kann dein Arbeitgeber "
                         f"nicht {_eur(lohnsteuer)} Lohnsteuer einbehalten haben — die Lohnsteuer "
                         f"wird vom Lohn abgezogen und ist deshalb immer kleiner als der Lohn. "
                         f"Bitte prüfe, welche der beiden Zahlen stimmt."})

        # Rentenversicherungsbeiträge über dem Lohn. Beide Anteile sind ein Prozentsatz des Lohns,
        # können ihn also nicht übersteigen — dafür braucht es weder Beitragssatz noch
        # Beitragsbemessungsgrenze (beide liegen für die allgemeine RV nicht belegt im Repo vor).
        for feld_id, name in (("vor_an_anteil_rv", "dein eigener Anteil"),
                              ("vor_ag_anteil_rv", "der Anteil deines Arbeitgebers")):
            beitrag = _bestaetigter_betrag(snapshot, feld_id)
            if beitrag is not None and beitrag > brutto:
                widersprueche.append({
                    "feld_id": feld_id, "wert": beitrag, "bezug": brutto,
                    "grund": f"Bei einem Bruttoarbeitslohn von {_eur(brutto)} können die "
                             f"Rentenversicherungsbeiträge nicht {_eur(beitrag)} betragen "
                             f"({name}). Die Beiträge sind ein Anteil des Lohns und damit immer "
                             f"kleiner als der Lohn. Bitte prüfe, welche der beiden Zahlen stimmt."})

        # Kirchensteuer, gemessen am Lohn.
        for feld_id, name in (("kist_gezahlt", "gezahlte"),
                              ("kirchensteuer_arbeitgeber", "vom Arbeitgeber einbehaltene")):
            kist = _bestaetigter_betrag(snapshot, feld_id)
            if kist is not None and kist > brutto * _KIST_ANTEIL_BRUTTO_MAX:
                widersprueche.append({
                    "feld_id": feld_id, "wert": kist, "bezug": brutto,
                    "grund": f"Bei einem Bruttoarbeitslohn von {_eur(brutto)} sind {_eur(kist)} "
                             f"{name} Kirchensteuer sehr unwahrscheinlich. Die Kirchensteuer "
                             f"beträgt 8 bis 9 Prozent der Einkommensteuer und liegt damit "
                             f"üblicherweise im Bereich einiger hundert Euro. Bitte prüfe, "
                             f"welche der beiden Zahlen stimmt."})

    # Schulgeld je Kind. Braucht den Lohn nicht: Bezugsgröße ist der gesetzliche Höchstbetrag.
    schwelle = int(_SCHULGELD_HOECHSTBETRAG_CENT / _SCHULGELD_ABZUGSSATZ) * _SCHULGELD_FAKTOR
    for feld_id, betrag in _schulgeld_felder(snapshot):
        if betrag > schwelle:
            widersprueche.append({
                "feld_id": feld_id, "wert": betrag,
                "grund": f"{_eur(betrag)} Schulgeld für ein Kind in einem Jahr ist ungewöhnlich "
                         f"hoch. Absetzbar sind 30 Prozent des Schulgelds, höchstens 5.000 € je "
                         f"Kind — dieser Höchstbetrag ist bereits ab rund 16.700 € Schulgeld "
                         f"erreicht. Bitte prüfe den Betrag."})

    # Zwei Angaben zur selben Kirchensteuer, die nie gegeneinander gehalten wurden.
    gezahlt = _bestaetigter_betrag(snapshot, "kist_gezahlt")
    einbehalten = _bestaetigter_betrag(snapshot, "kirchensteuer_arbeitgeber")
    if gezahlt is not None and einbehalten is not None:
        gross, klein = max(gezahlt, einbehalten), min(gezahlt, einbehalten)
        if gross > klein * _KIST_ABGLEICH_FAKTOR:
            widersprueche.append({
                "feld_id": "kist_gezahlt", "wert": gezahlt, "bezug": einbehalten,
                "grund": f"Dein Arbeitgeber hat {_eur(einbehalten)} Kirchensteuer einbehalten, "
                         f"gezahlt hast du nach deiner Angabe {_eur(gezahlt)}. Beide Angaben "
                         f"beschreiben normalerweise dieselbe Zahlung und liegen hier weit "
                         f"auseinander. Bitte prüfe, welche der beiden Angaben stimmt."})

    # Keine Bankverbindung gewollt — trotzdem eine IBAN erfasst. Reiner Angabe-gegen-Angabe-Fall,
    # ohne Betrag und ohne Bezugsgröße. Eine leere Zeichenkette ist keine IBAN.
    keine_bank = _bestaetigt_wert(snapshot, "stammdaten_keine_bankverbindung")
    iban = _bestaetigt_wert(snapshot, "stammdaten_iban")
    if keine_bank is True and isinstance(iban, str) and iban.strip():
        widersprueche.append({
            "feld_id": "stammdaten_iban", "wert": iban,
            "grund": "Du hast angegeben, keine Bankverbindung für eine Erstattung angeben zu "
                     "wollen — trotzdem ist eine Kontonummer (IBAN) erfasst. Ohne Konto kann "
                     "das Finanzamt eine Erstattung nicht überweisen. Bitte prüfe, welche der "
                     "beiden Angaben stimmt."})

    # Kirchensteuer erklärt, Kirchenzugehörigkeit nicht. Angabe gegen FEHLENDE Angabe — der einzige
    # Fall hier, der eine Lücke meldet statt zweier Zahlen, die sich widersprechen.
    #
    # GEMESSEN 2026-08-28 am Fall serie-verheiratet-1kind-handwerker: Bundesland, gezahlte (580 €)
    # und erstattete Kirchensteuer bestätigt, die Konfession nie beantwortet. Alle drei Felder
    # werden laut Bindung NUR gefragt, wenn eine Konfession vorliegt — die Software hat sie also
    # gestellt, weil sie eine annahm, und rechnete danach mit „keine". Ergebnis: 0 € Kirchensteuer
    # gemeldet, wo bei römisch-katholisch 1.053,36 € festzusetzen wären. Die Einkommensteuer blieb
    # dabei richtig (Delta 0 Cent), deshalb MELDEN und nicht sperren: einen Sperrgrund zu setzen
    # nähme dem Nutzer eine korrekte Zahl weg. Unvollständig ist die Erklärung, nicht die Rechnung.
    #
    # Ein Betrag von 0 € zählt hier NICHT als Angabe (_bestaetigter_betrag verlangt > 0): wer bei
    # der erstatteten Kirchensteuer eine Null einträgt, sagt damit nichts über seine Mitgliedschaft.
    # Das Bundesland dagegen zählt schon als gesetzt, sobald es dasteht — es wird ausschließlich
    # gebraucht, um den Hebesatz einer Kirche anzuwenden.
    konfession = _bestaetigt_wert(snapshot, "kist_konfession")
    if not (isinstance(konfession, str) and konfession):
        bundesland = _bestaetigt_wert(snapshot, "kist_bundesland")
        belege = [(f, _bestaetigter_betrag(snapshot, f))
                  for f in ("kist_gezahlt", "kirchensteuer_arbeitgeber", "kist_erstattet")]
        belege = [(f, b) for f, b in belege if b is not None]
        if belege or (isinstance(bundesland, str) and bundesland):
            feld_id, betrag = belege[0] if belege else ("kist_bundesland", None)
            womit = (f"Kirchensteuer gezahlt oder einbehalten ({_eur(betrag)})" if betrag is not None
                     else "angegeben, in welchem Bundesland du Kirchensteuer zahlst")
            widersprueche.append({
                "feld_id": feld_id, "wert": betrag,
                "grund": f"Du hast {womit} — ob du einer Kirche angehörst, die Kirchensteuer "
                         f"erhebt, ist aber noch offen. Ohne diese Angabe wird deine "
                         f"Kirchensteuer nicht berechnet und fehlt in deinem Ergebnis. Bitte gib "
                         f"an, ob du einer solchen Kirche angehörst — und wenn nicht, prüfe die "
                         f"Kirchensteuer-Angaben noch einmal."})

    # Angekündigt, aber nicht ausgefüllt — Angabe gegen Angabe wie die beiden Fälle darüber, nur
    # zählt hier eine Anzahl gegen die Zahl der Antworten. BEWUSST IM SELBEN SCHLÜSSEL
    # (`widersprueche_plausibilitaet`): `api.preflight_check` führt eine fest verdrahtete Liste der
    # ausgelieferten Schlüssel, und ein neuer Schlüssel ohne Eintrag dort wäre totes Wiring — genau
    # das ist an dieser Datei schon einmal passiert.
    widersprueche += unvollstaendige_instanzen(snapshot)

    return widersprueche


def preflight(snapshot: dict) -> dict:
    """Snapshot → Preflight-Ergebnis.

    Rückgabe:
      - widersprueche_flag: Liste (flag_check)
      - widersprueche_partner: Liste (partner_ohne_zusammen)
      - widersprueche_alleinerziehend: Liste (alleinerziehend_mit_zusammen)
      - widersprueche_plausibilitaet: Liste (plausibilitaets_widersprueche)
      - hinweise_pauschalen: Liste (check_pauschalen)
      - hinweise_nicht_gerechnet: Liste (check_nicht_gerechnet)
      - status: "RED" (harte Widersprüche), "AMBER" (nur soft warnings), "GREEN" (clean)
    """
    flag = flag_check.flag_widersprueche(snapshot)
    partner = partner_check.partner_ohne_zusammen(snapshot)
    alleinerziehend = partner_check.alleinerziehend_mit_zusammen(snapshot)
    plausibilitaet = plausibilitaets_widersprueche(snapshot)
    pauschal = check_pauschalen.pauschal_hinweise(snapshot)
    nicht_gerechnet = check_nicht_gerechnet.nicht_gerechnete_angaben(snapshot)

    if flag or partner or alleinerziehend or plausibilitaet:
        status = "RED"
    elif pauschal or nicht_gerechnet:
        # AMBER, nicht GREEN: die Erklärung ist in Ordnung, aber die angezeigte Zahl bildet
        # nicht alles ab, was deklariert wird. GREEN hieße "hier gibt es nichts zu wissen" —
        # und genau das stimmt dann nicht.
        status = "AMBER"
    else:
        status = "GREEN"

    return {
        "widersprueche_flag": flag,
        "widersprueche_partner": partner,
        "widersprueche_alleinerziehend": alleinerziehend,
        "widersprueche_plausibilitaet": plausibilitaet,
        "hinweise_pauschalen": pauschal,
        "hinweise_nicht_gerechnet": nicht_gerechnet,
        "status": status,
    }
