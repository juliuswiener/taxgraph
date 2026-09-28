"""Deklarations-Sperrgruende und Ring-Werte: was einer Abgabe im Weg steht, und welche gerechneten Groessen in die Deklaration zurueckfliessen.

Aufgeteilt am 2026-08-19 aus bescheid.py (2610 Zeilen). Kein Logik-Edit — die
Rümpfe sind byte-identisch übernommen. bescheid.py ist die Fassade geblieben und
re-exportiert alles; die Aufrufer (api.py, Tests) merken den Schnitt nicht.
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
ROOT = os.path.dirname(PRODUKT)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/eingang", "produkt/engine", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import intervall as IV      # noqa: E402
import est_mapping as EM    # noqa: E402
import flag_check as FC     # noqa: E402  (Flag↔Einkunftsart-Widersprüche)
import partner_check as PC  # noqa: E402  (Partner-Behinderungsfeld↔Zusammenveranlagung)
from api_constants import (  # noqa: E402
    AN_GESAMT_FLAGS,
    AN_GESAMT_PARTNER,
    ARBEITSMITTEL_KOSTEN,
    DHF_BEDINGUNGEN,
    DHF_KOSTEN,
    GESAMT_PARTNER_19,
    GESAMT_PARTNER_KAP,
    GEWINN_QUELLEN_MENGEN,
    KAP_ERTRAEGE,
    KAP_ERTRAEGE_PARTNER,
    KAP_TOEPFE,
    KAP_TOEPFE_PARTNER,
    MITU_FELDER,
    RENTNER_22,
    RENTNER_22_PARTNER,
    RENTNER_AA_ARTEN,
    RING_BETRAGSFELDER,
    UEBERNACHTUNG_BEDINGUNGEN,
    UEBERNACHTUNG_KOSTEN,
    VERPFLEGUNG_TAGE,
    VERPFLEGUNG_TAGE_NACH_FRIST,
    VOR_FELDER,
    VOR_PARTNER_FELDER,
    VV_GESAMT_FELDER,
    dba_methode_fuer,
)


# Aus den Blatt-Modulen. Namentlich, nicht per Star-Import (tests/test_split_naht_gate.py).
from bescheid_abzuege import (  # noqa: E402
    _abs3_eligible,
)

def _mit_ring_werten(felder: dict, vz: int) -> dict:
    """Hängt berechnete Ring-Werte als fertige Events in felder ein.

    (1) E0205508 (Kürzungsbetrag wegen Mahlzeitengestellung). Der Ring
    (runner._verpflegung_kuerzung_cent) rechnet den CENT-Wert aus den
    Rohdaten (tage_24h, frühstücke, etc.). Inert: ohne Verpflegungs-Felder
    kein Eintrag (auch kein Wert 0).

    (2) E1900401 (Antrag Günstigerprüfung § 32d Abs. 6) + (3) E1901401
    (genutzter Sparer-Pauschbetrag § 20 Abs. 9): NICHT an den vom Ring beim
    Rechnen gewählten Zweig gekoppelt (kap_st < abgeltung) — gemessen
    2026-08-10 gegen checkESt: der Antrag löst nur eine Prüfung aus (§ 32d
    Abs. 6 S. 1 "wenn dies zu einer niedrigeren Einkommensteuer ... führt"),
    er kann also nie schlechterstellen, aber die alte Kopplung ließ den
    HÄUFIGEREN Fall (Abgeltung günstiger) uneinreichbar (rc=610001002).
    Stattdessen: gesetzt, sobald irgendein KAP-Betragsfeld (eigene Töpfe/
    Aggregat ODER — bei Zusammenveranlagung — die des Ehegatten, § 32d
    Abs. 6 S. 4) erklärt wird UND der Ring den Betrag tatsächlich liefert.
    Beide Kz sind ZUSAMMEN Pflicht (ohne E1901401 bleibt rc=610001002
    trotz Antrag, siehe bindung_kap_vv_familie.yaml) — deshalb ein
    gemeinsames Gate: beide Felder entstehen zusammen, NACH dem Ring-
    Aufruf, oder keins von beiden (2026-08-31: ein Ausfall im Ring durfte
    vorher E1900401 allein stehen lassen, mit einer aus der Exception
    geerbten Fake-Null bei E1901401 — nicht mehr).
    Töpfe-XOR-Aggregat-Auswahl 1:1 zur SINGLE-SOURCE in _bescheid_fn
    (api.py Z. 1019-1047/1462-1483) — bei Änderung dort nachziehen.
    Direkt auf `felder` gerechnet (kein _feste_zahl/Meet-Gate): der Wert
    hängt nur an KAP-Feldern, nicht an unverwandten Kegel-Feldern.

    (4) § 35a Haushaltsnahe Sum-Kz (E0104109/E0107208/E0111215): seit der
    Einzelaufstellung (Anlass 2026-08-10, checkESt rc=610001002 ohne Einz-Kz)
    sind diese drei Felder askable:false — ohne diese Injektion bliebe die
    Sum-Kz auf der Scheibe leer und _scheibe_bindung deklariert sie NICHT
    (dieselbe Naht wie E0205508/E1900401 oben). Wert = STUMPFE Σ über die
    Einz-Instanzen (hh_minijob_betrag/hh_dienstleistung_betrag/
    hh_handwerker_betrag, instanz_gruppe hh_minijob/hh_dienstleistung/
    hh_handwerker) direkt aus `felder` (bereits materialisiert, Instanz-
    Suffixe __n flach enthalten — keine store/bindung-Naht nötig hier).
    Inert wie (1): keine Instanz-Σ > 0 → kein Eintrag (kein Wert-0-Kz).

    Alle Injektionen sind fertige Events (zustand=bestaetigt, schreiber=
    engine, herkunft=berechnet/amtlich/system — fail-closed, Haftung System).
    """
    # (1) Verpflegungskürzung
    verpflegungs_felder = {"tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig"}
    if verpflegungs_felder & set(felder):
        try:
            import runner
            s = {fid: e["wert"] if isinstance(e, dict) else e
                 for fid, e in felder.items()}
            kuerzung_cent = runner._verpflegung_kuerzung_cent(s, vz)
        except Exception:
            kuerzung_cent = 0
        if kuerzung_cent > 0:
            felder["p9_4a_kuerzung_nach_entgelt"] = {
                "wert": kuerzung_cent,  # CENT — _cent_nach_kz wandelt in EURO
                "zustand": "bestaetigt",
                "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
                "schreiber": "engine",
                "signal": {"signal_1": None, "signal_2": None},
            }

    # (2)+(3) Anlage KAP: Antrag Günstigerprüfung + genutzter Sparer-Pauschbetrag
    def _kap_positiv(fid):
        w = (felder.get(fid) or {}).get("wert")
        return isinstance(w, (int, float)) and not isinstance(w, bool) and w > 0

    zusammen = (felder.get("veranlagung") or {}).get("wert") == "zusammen"
    kap_erklaert = (any(_kap_positiv(t) for t in KAP_TOEPFE) or _kap_positiv(KAP_ERTRAEGE)
                    or (zusammen and (any(_kap_positiv(t) for t in KAP_TOEPFE_PARTNER)
                                       or _kap_positiv(KAP_ERTRAEGE_PARTNER))))
    if kap_erklaert:
        # Zustand=bestaetigt erst NACH erfolgreicher Berechnung setzen (nicht vorher, wie bis
        # 2026-08-31): ein Ausfall im try darf nie einen echten Antrag samt Fake-0-Betrag als
        # bestaetigt hinterlassen — sonst sieht kein Waechter den Unterschied zu einer echten
        # Null (gemessen: die Null erreicht unveraendert das abgesendete XML). Bei Ausfall bleibt
        # KEIN Eintrag stehen (identisch zum Inert-Vertrag von (1) oben) — ohne E1901401 waere ein
        # gesetztes E1900401 ohnehin nur ein neuer rc=610001002 (s. Docstring), also kein Gewinn.
        try:
            import runner

            def _c2(fid):
                return int((felder.get(fid) or {}).get("wert") or 0)

            if any(_c2(t) != 0 for t in KAP_TOEPFE):
                verrechnete = runner.catala_kapital_verrechnung({
                    "gewinn_aktien": _c2("kap_gewinn_aktien") // 100,
                    "verlust_aktien": _c2("kap_verlust_aktien") // 100,
                    "gewinn_sonstige": _c2("kap_gewinn_sonstige") // 100,
                    "verlust_sonstige": _c2("kap_verlust_sonstige") // 100})
            else:
                verrechnete = _c2(KAP_ERTRAEGE) // 100
            if zusammen:
                if any(_c2(t) != 0 for t in KAP_TOEPFE_PARTNER):
                    verrechnete += runner.catala_kapital_verrechnung({
                        "gewinn_aktien": _c2("kap_gewinn_aktien_partner") // 100,
                        "verlust_aktien": _c2("kap_verlust_aktien_partner") // 100,
                        "gewinn_sonstige": _c2("kap_gewinn_sonstige_partner") // 100,
                        "verlust_sonstige": _c2("kap_verlust_sonstige_partner") // 100})
                else:
                    verrechnete += _c2(KAP_ERTRAEGE_PARTNER) // 100
            kapitaleinkuenfte = runner.catala_sparer_pb({
                "veranlagungszeitraum": vz, "kapitalertraege": verrechnete,
                "zusammenveranlagung": zusammen})
            pb_genutzt_cent = max(0, verrechnete - kapitaleinkuenfte) * 100
        except Exception:
            pb_genutzt_cent = None
        if pb_genutzt_cent is not None:
            felder["kap_antrag_guenstigerpruefung"] = {
                "wert": True,
                "zustand": "bestaetigt",
                "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
                "schreiber": "engine",
                "signal": {"signal_1": None, "signal_2": None},
            }
            felder["kap_sparer_pauschbetrag_genutzt"] = {
                "wert": pb_genutzt_cent,  # CENT, Vordruck erlaubt ausdruecklich "(ggf. 0)"
                "zustand": "bestaetigt",
                "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
                "schreiber": "engine",
                "signal": {"signal_1": None, "signal_2": None},
            }

    # (4) § 35a Haushaltsnahe: Sum-Kz aus der Σ der Einz-Instanzen (Instanz-Reuse, Basis-feld_id
    # ohne Suffix = Instanz 1 — dieselbe Konvention wie EM.instanzen, hier ohne store/bindung
    # direkt auf dem bereits materialisierten `felder` gerechnet).
    def _instanz_summe(basis_fid):
        total = 0
        for fid, ev in felder.items():
            parsed = EM.parse_instanz(fid)
            if (parsed[0] if parsed else fid) != basis_fid:
                continue
            if not isinstance(ev, dict) or ev.get("zustand") != "bestaetigt":
                continue
            v = ev.get("wert")
            total += int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
        return total
    for sum_fid, betrag_fid in (
        ("hh_minijob_aufwendungen", "hh_minijob_betrag"),
        ("hh_dienstleistungen", "hh_dienstleistung_betrag"),
        ("hh_handwerker_arbeitskosten", "hh_handwerker_betrag"),
    ):
        summe_cent = _instanz_summe(betrag_fid)
        if summe_cent > 0:
            felder[sum_fid] = {
                "wert": summe_cent,
                "zustand": "bestaetigt",
                "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
                "schreiber": "engine",
                "signal": {"signal_1": None, "signal_2": None},
            }

    # (5) § 35c: E0240902 fragt UMGEKEHRT zu unserem Gate. Das amtliche Feld lautet "Ich habe /
    # Wir haben für die energetischen Maßnahmen beantragt / in Anspruch genommen", unser Gate
    # heißt p35c_keine_doppelfoerderung. Die Umkehrung steht hier als eigenes Feld statt im
    # Writer: dort wäre sie unsichtbar, und eine still gedrehte Ja/Nein-Antwort ist genau die
    # Sorte Fehler, die niemand im XML nachrechnet.
    # (6) § 35c-Einzelzeile: derselbe Betrag wie die Summe, nur in der Zeile der gewaehlten
    # Massnahmenart (est_mapping VERZWEIGUNG). Ohne eine Einzelzeile ist die Anlage unvollstaendig.
    # (7) Anlage V: die Summenzeile der Wohnungs-Mieteinnahmen (E0700206). checkESt verlangt
    # Einzelbetrag UND Summe ("Es wurden Mieteinnahmen fuer Wohnungen aus den einzelnen
    # Wohneinheiten angegeben, die Summe wurde jedoch nicht erklaert"). Solange nur EINE
    # Wohneinheit erklaert wird, ist die Summe gleich dem Einzelbetrag.
    _vv_einn = felder.get("vv_einnahmen")
    if isinstance(_vv_einn, dict) and _vv_einn.get("zustand") == "bestaetigt" \
            and isinstance(_vv_einn.get("wert"), int) and _vv_einn["wert"] > 0:
        # Werbungskosten-Summe und Ergebniszeile: checkESt verlangt beide, sobald Einnahmen
        # erklaert sind ("die Summe der Einnahmen ... erklaert, der Ueberschuss jedoch nicht").
        _wk = 0
        for _f in ("vv_gebaeude_afa", "vv_schuldzinsen", "vv_erhaltungsaufwand", "vv_sonstige_wk"):
            _e = felder.get(_f) or {}
            _w = _e.get("wert") if _e.get("zustand") == "bestaetigt" else 0
            _wk += _w if isinstance(_w, int) and not isinstance(_w, bool) else 0
        _vv_uml = felder.get("vv_nebenkosten_umgelegt") or {}
        _uml = _vv_uml.get("wert") if _vv_uml.get("zustand") == "bestaetigt" else 0
        _uml = _uml if isinstance(_uml, int) and not isinstance(_uml, bool) else 0
        # E0701401 ist die Summe ALLER Einnahmen des Objekts (Mieten + Umlagen + Sonstiges),
        # E0700206 nur die der Wohnungs-Mieten. Zwei Zeilen, zwei Bedeutungen.
        felder["vv_einnahmen_summe_gesamt"] = {
            "wert": _vv_einn["wert"] + _uml,
            "zustand": "bestaetigt",
            "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
            "schreiber": "engine",
            "signal": {"signal_1": None, "signal_2": None},
        }
        _vv_ueberschuss = _vv_einn["wert"] + _uml - _wk
        for _fid, _wert in (("vv_summe_werbungskosten", _wk),
                            ("vv_ueberschuss", _vv_ueberschuss),
                            # Zurechnung: bei Alleineigentum voll auf Person A. Die Aufteilung
                            # auf Person B (E0701802) braucht die Miteigentums-Angaben — Nachtrag.
                            ("vv_ueberschuss_person_a", _vv_ueberschuss)):
            felder[_fid] = {
                "wert": _wert,
                "zustand": "bestaetigt",
                "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
                "schreiber": "engine",
                "signal": {"signal_1": None, "signal_2": None},
            }
        felder["vv_mieteinnahmen_summe"] = {
            "wert": _vv_einn["wert"],
            "zustand": "bestaetigt",
            "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
            "schreiber": "engine",
            "signal": {"signal_1": None, "signal_2": None},
        }

    _p35c_sum = felder.get("p35c_sanierungsaufwendungen")
    if isinstance(_p35c_sum, dict) and _p35c_sum.get("zustand") == "bestaetigt" \
            and isinstance(_p35c_sum.get("wert"), int) and _p35c_sum["wert"] > 0:
        felder["p35c_massnahme_einzelbetrag"] = {
            "wert": _p35c_sum["wert"],
            "zustand": "bestaetigt",
            "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
            "schreiber": "engine",
            "signal": {"signal_1": None, "signal_2": None},
        }

    # § 35 / § 16 Abs. 1 GewStG: die zu zahlende Gewerbesteuer ist Messbetrag mal Hebesatz.
    # checkESt verlangt sie neben den beiden Faktoren ("... sind gemeinsam anzugeben",
    # "Der Hebesatz wurde angegeben, die zu zahlende Gewerbesteuer jedoch nicht", 2026-08-19).
    #
    # Der Hebesatz ist eine Prozentzahl (typ int, z. B. 450), der Messbetrag steht in Cent.
    # Gerechnet wird mit dem auf volle Euro ABGERUNDETEN Messbetrag, also mit genau der Zahl, die
    # als E0801606 in der Erklärung steht (_cent_nach_kz rundet ab); Euro mal Prozent ergibt Cent.
    # Grund: ELSTER nimmt E0801606 und E0801704 nur in vollen Euro an (Jahresdokumentation E10
    # 2025, Blatt "G - Felder": GeldBetragOhneCent) und prüft mit Regel 100800013 (Blatt
    # "G - Regeln", Typ Fehler), ob E0801704 bis auf 1 EUR gleich E0801606 * E0801705 / 100 ist.
    # Getrennt abgerundet verfehlte das die Toleranz: 20.247,50 EUR bei 400 % ergab 20247 und
    # 80990, 2 EUR daneben. Die § 35-Rechnung liest dieses Feld nicht (bescheid_zweige.py rechnet
    # aus Messbetrag und Hebesatz selbst).
    # ponytail: abgerundet, obwohl die Anleitung ESt 1 A Runden zu Gunsten des Nutzers erlaubt
    # (Wirkung höchstens knapp 4 EUR § 35-Obergrenze); Aufrunden hieße, diese Stelle und die
    # E0801606-Rundung gemeinsam zu ändern (Entscheidung
    # gewerbesteuer-kennzahlen-aus-dem-abgerundeten-messbetrag).
    for _mb, _hs, _ziel in (("gewst_messbetrag", "gewst_hebesatz", "gewst_zu_zahlen"),
                            ("gewst_messbetrag_partner", "gewst_hebesatz_partner",
                             "gewst_zu_zahlen_partner")):
        _m, _h = felder.get(_mb), felder.get(_hs)
        if all(isinstance(f, dict) and f.get("zustand") == "bestaetigt"
               and isinstance(f.get("wert"), int) and f["wert"] > 0 for f in (_m, _h)):
            felder[_ziel] = {
                "wert": _m["wert"] // 100 * _h["wert"],
                "zustand": "bestaetigt",
                "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
                "schreiber": "engine",
                "signal": {"signal_1": None, "signal_2": None},
            }

    # § 22 Nr. 3: das Formular verlangt drei Zahlen, der Nutzer kennt zwei. Er nennt die
    # Einnahmen (brutto) und die Einkünfte (netto, das ältere Feld fragt ausdrücklich "Einnahmen
    # minus der Kosten"). Daraus folgen der Einzelposten und die Werbungskosten.
    #
    # ERiC rechnet nach (gemessen 2026-08-19): "Der bei den sonstigen Einkünften (Leistungen)
    # erklärte Betrag für die Einkünfte entspricht nicht den erklärten Einnahmen abzüglich der
    # erklärten Werbungskosten". Ein leeres Werbungskosten-Feld erfüllt diese Probe nicht.
    _p22_einn = felder.get("p22_nr3_einnahmen")
    _p22_eink = felder.get("p22_nr3_einkuenfte")
    if all(isinstance(f, dict) and f.get("zustand") == "bestaetigt"
           and isinstance(f.get("wert"), int) for f in (_p22_einn, _p22_eink)) \
            and _p22_einn["wert"] > 0:
        _berechnet = {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"}
        felder["p22_nr3_einnahmen_einzelbetrag"] = {
            "wert": _p22_einn["wert"], "zustand": "bestaetigt", "herkunft": _berechnet,
            "schreiber": "engine", "signal": {"signal_1": None, "signal_2": None},
        }
        # NUR bei echt positiver Differenz. E0305201 ist GanzzahlPos, eine 0 waere unzulässig;
        # und eine negative Differenz (Einnahmen < Einkünfte) ist eine widersprüchliche Eingabe,
        # die hier nicht glattgebügelt wird — dann fehlt das Feld, checkESt beanstandet, und der
        # Widerspruch wird sichtbar statt auf 0 geklemmt. Eine Prüfung schon im Dialog wäre
        # besser, aber die Bindung kennt keine feldübergreifenden Grenzen (bereich: ist überall
        # statisch), das wäre ein eigener Bau.
        _wk = _p22_einn["wert"] - _p22_eink["wert"]
        if _wk > 0:
            felder["p22_nr3_werbungskosten"] = {
                "wert": _wk, "zustand": "bestaetigt", "herkunft": _berechnet,
                "schreiber": "engine", "signal": {"signal_1": None, "signal_2": None},
            }

    # § 10 Abs. 1 Nr. 7: dieselbe Bauart wie § 35c darüber. Die Anlage führt eine Summe
    # (E0108202, vom Nutzer erfragt) und eine Einzelzeile (E0108002); ohne die Einzelzeile
    # lehnt checkESt ab ("Es wurde die Summe der Aufwendungen für die eigene Berufsausbildung
    # angegeben, bitte geben Sie auch die Bezeichnung der Ausbildung und die Art und Höhe der
    # einzelnen Aufwendungen an", gemessen 2026-08-19). Bei EINEM Posten — der MVP-Grenze, s.
    # bindung_sonder_agb_35a.yaml — ist die Einzelzeile betragsgleich mit der Summe. Der
    # Nutzer tippt sie deshalb nicht ein zweites Mal.
    _ausb_sum = felder.get("berufsausbildung_aufwendungen")
    if isinstance(_ausb_sum, dict) and _ausb_sum.get("zustand") == "bestaetigt" \
            and isinstance(_ausb_sum.get("wert"), int) and _ausb_sum["wert"] > 0:
        felder["berufsausbildung_einzelbetrag"] = {
            "wert": _ausb_sum["wert"],
            "zustand": "bestaetigt",
            "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
            "schreiber": "engine",
            "signal": {"signal_1": None, "signal_2": None},
        }

    _p35c_gate = felder.get("p35c_keine_doppelfoerderung")
    if isinstance(_p35c_gate, dict) and _p35c_gate.get("zustand") == "bestaetigt" \
            and isinstance(_p35c_gate.get("wert"), bool):
        felder["p35c_foerderung_in_anspruch"] = {
            "wert": not _p35c_gate["wert"],
            "zustand": "bestaetigt",
            "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
            "schreiber": "engine",
            "signal": {"signal_1": None, "signal_2": None},
        }

    return felder


# Zu jedem Grund, mit dem /ergebnis ohne Zahl zurueckkommt, ein Satz, den ein Laie lesen kann.
# ZWEI Quellen speisen ihn, und beide muessen hier vertreten sein — `ergebnis()` (api.py) setzt den
# Klartext fuer JEDEN grund ausser None/"bestaetigt", ohne zu unterscheiden, woher er stammt:
#   (1) die 36 Rueckgaben von _an_gesamt_sperrgrund unten,
#   (2) die drei, die _ergebnis_roh (api.py) selbst setzt, wenn gar keine Zahl zustande kam.
#
# Anlass 2026-08-27: ein vollstaendig ausgefuellter Fragebogen (134 Antworten, 0 offene Fragen)
# endete mit `grund="flag_konsistenz_offen", offen=[]` — der Nutzer sah eine Maschinen-Kennung und
# eine leere Liste und wusste nicht, was er tun soll. Der Grund SAGT etwas, aber nur nach innen;
# hier bekommt er eine Stimme nach aussen.
#
# Was ein Satz leisten muss: WAS die Software nicht entscheiden kann, und WAS der Nutzer tun soll.
# Keine Feld-Kennungen, keine Regel-Ids, keine Paragraphen — der Nutzer hat diese Namen nie gesehen
# (tests/test_sperrgrund_klartext.py haelt das fest). Kein Vorwurf: eine fehlende Angabe ist ein
# Zustand, kein Fehler des Nutzers.
#
# Vier Sorten Grund, vier Tonlagen:
#   (0) noch nicht fertig  -> sagen, dass nichts kaputt ist, und auf die Liste daneben zeigen
#   (1) etwas fehlt        -> die Frage benennen und sagen, wo die Antwort steht
#   (2) zwei Angaben       -> sagen, DASS sie sich widersprechen und wo beide stehen; nie, welche
#       widersprechen sich    "falsch" ist — das weiss nur der Nutzer
#   (3) nicht gerechnet    -> ehrlich sagen, dass die Software diesen Fall noch nicht rechnet,
#                             statt es nach einem Formfehler des Nutzers klingen zu lassen
#
# Bei den drei Widerspruchs-Gruenden (flag/partner/alleinerziehend) sind diese Texte die ALLGEMEINE
# Fassung. flag_check.flag_widersprueche() und partner_check.partner_ohne_zusammen()/
# alleinerziehend_mit_zusammen() liefern zum selben Fall bereits einen KONKRETEN Satz mit Feldnamen
# und Betrag ("... bei den GWG-Anschaffungen wurden aber 111 EUR erfasst"); _an_gesamt_sperrgrund
# verwirft ihn und behaelt nur den Namen. Wo die Haut an diese Listen herankommt, ist deren Satz der
# bessere — dieser hier ist der Boden, der immer traegt.
SPERRGRUND_KLARTEXT: dict[str, str] = {
    # ---- (3) Fälle, die die Software noch nicht rechnet ----------------------------------------
    "abs3_ueber_5mio_offen":
        "Du hast den ermäßigten Steuersatz für den Verkauf oder die Aufgabe deines Betriebs "
        "beantragt, und der Gewinn liegt über fünf Millionen Euro. Der ermäßigte Satz gilt nur "
        "bis zu dieser Grenze; wie der Teil darüber zu versteuern ist, rechnet die Software noch "
        "nicht. Dieser Fall braucht steuerliche Beratung.",
    "ausland_dhf_nicht_ring_faehig":
        "Deine zweite Wohnung am Arbeitsort liegt im Ausland. Dafür gelten eigene Obergrenzen, die "
        "die Software noch nicht rechnet. Dieser Fall braucht steuerliche Beratung.",
    "dba_kapital_offen":
        "Du hast Kapitalerträge angegeben und zugleich ausländische Einkünfte. Ob und wie eine im "
        "Ausland gezahlte Steuer auf deine Kapitalerträge angerechnet wird, rechnet die Software "
        "noch nicht. Dieser Fall braucht steuerliche Beratung.",
    "dba_multi_country_offen":
        "Du hast Einkünfte aus mehr als einem ausländischen Staat. Jedes Land hat ein eigenes "
        "Abkommen mit Deutschland darüber, wo besteuert wird; mehrere Länder zugleich rechnet die "
        "Software noch nicht. Dieser Fall braucht steuerliche Beratung.",
    "einkunftsart_nicht_ring_faehig":
        "Du hast angegeben, dass du eine Einkunftsart hast, die in dieser Berechnung noch nicht "
        "mitgerechnet werden kann — je nach Fall Renten und andere sonstige Einkünfte, Einnahmen "
        "aus Vermietung, Kapitalerträge, Gewinn aus einem Betrieb oder der Verkauf eines "
        "Grundstücks oder eines anderen Vermögensgegenstands. Ein Ergebnis ohne diese "
        "Einkünfte wäre zu niedrig, deshalb rechnet die Software hier nicht weiter.",
    "kinder_gehoeren_in_gesamt":
        "Du hast Kinder angegeben. Ob Kindergeld oder die Kinderfreibeträge günstiger sind, wird "
        "gegeneinander abgewogen, und diese Abwägung ist in der gerade laufenden Berechnung nicht "
        "enthalten. Ohne sie wäre deine Steuer zu hoch, deshalb rechnet die Software hier nicht "
        "weiter.",
    "luf_euer_offen":
        "Du hast einen land- oder forstwirtschaftlichen Betrieb angegeben und dazu Einnahmen und "
        "Ausgaben einzeln erfasst. Für die Land- und Forstwirtschaft gelten eigene Arten der "
        "Gewinnermittlung, die die Software noch nicht rechnet. Dieser Fall braucht steuerliche "
        "Beratung.",
    "p32b_kombi_offen":
        "Du hast Lohnersatzleistungen wie Eltern-, Kranken- oder Arbeitslosengeld angegeben und "
        "zusätzlich einen Betriebsverkauf, Gewerbesteuer oder ausländische Einkünfte. Diese "
        "Kombination rechnet die Software noch nicht: Lohnersatzleistungen erhöhen den Steuersatz, "
        "und wie sich das mit den anderen Ermäßigungen verzahnt, ist offen. Dieser Fall braucht "
        "steuerliche Beratung.",
    "partner_vor_offen":
        "Du hast eine gemeinsame Veranlagung gewählt und Beiträge zur Rentenversicherung "
        "angegeben. Die Altersvorsorgebeiträge beider Partner rechnet die Software in dieser "
        "Zusammenstellung noch nicht. Dieser Fall wird derzeit nicht berechnet.",
    "progression_gehoert_in_gesamt":
        "Du hast Lohnersatzleistungen wie Eltern-, Kranken- oder Arbeitslosengeld angegeben. Diese "
        "Leistungen sind steuerfrei, erhöhen aber den Steuersatz auf dein übriges Einkommen. "
        "Dieser Effekt ist in der gerade laufenden Berechnung nicht enthalten, deshalb rechnet die "
        "Software hier nicht weiter.",
    "uebernachtung_zeitraum_offen":
        "Du hast Übernachtungskosten angegeben, aber die Angabe fehlt, in wie vielen Monaten dieses "
        "Jahres du auswärts übernachtet hast und seit wie vielen Monaten du schon an diesem Ort "
        "arbeitest. Davon hängt ab, ob deine Kosten nach 48 Monaten noch begrenzt sind. Bitte "
        "beantworte diese Fragen.",
    "verlustvortrag_gehoert_in_gesamt":
        "Du hast einen Verlustvortrag aus einem früheren Jahr angegeben. Seine Verrechnung mit dem "
        "Einkommen dieses Jahres ist in der gerade laufenden Berechnung nicht enthalten. Ohne sie "
        "wäre deine Steuer zu hoch, deshalb rechnet die Software hier nicht weiter.",

    # ---- (0) Nicht aus _an_gesamt_sperrgrund, sondern aus _ergebnis_roh (api.py) -----------------
    # Diese drei laufen NIE durch den Sperr-Guard: api.py setzt sie selbst, wenn gar keine Zahl
    # zustande kam. Sie muessen trotzdem hier stehen, weil `ergebnis()` den Klartext fuer JEDEN
    # grund ausser None/"bestaetigt" setzt — ohne Eintrag saehe der Nutzer den Ersatztext, und der
    # sagt "woran es liegt, laesst sich hier nicht in Worte fassen". Fuer den haeufigsten Zustand
    # ueberhaupt (input_kegel_nicht_bestaetigt, jeder frische Fall) waere das aktiv irrefuehrend:
    # es liegt an konkreten Feldern, die in derselben Antwort unter "offen" danebenstehen.
    "input_kegel_nicht_bestaetigt":
        "Für ein Ergebnis fehlen noch Angaben. Welche das sind, ist hier aufgeführt — sobald sie "
        "beantwortet sind, geht es weiter. Es ist nichts schiefgegangen: du bist noch mitten in "
        "der Erklärung.",
    "kein_scheiben_gesamtbescheid":
        "Für diesen Ausschnitt deiner Erklärung gibt es bewusst keine Gesamtsumme. Die einzelnen "
        "Regeln werden hier gerechnet, aber eine belastbare Gesamtsteuer daraus zu bilden kann die "
        "Software an dieser Stelle noch nicht — und sie zeigt lieber keine Zahl als eine falsche.",
    "engine_unavailable":
        "Alle nötigen Angaben liegen vor, aber der Rechenkern liefert für diesen Fall gerade kein "
        "Ergebnis. Das liegt an der Software, nicht an deinen Angaben. Bitte versuche es später "
        "noch einmal, und melde den Fall, wenn er bestehen bleibt.",

    # ---- (2) Zwei Angaben widersprechen sich -----------------------------------------------------
    "alleinerziehend_konsistenz_offen":
        "Zwei Angaben passen nicht zusammen: Du hast angegeben, allein stehend zu sein, und zugleich "
        "eine gemeinsame Veranlagung mit Ehe- oder Lebenspartner gewählt. Den Entlastungsbetrag "
        "für Alleinerziehende gibt es nur, wenn du nicht gemeinsam veranlagt wirst. Bitte sieh dir "
        "beide Angaben noch einmal an.",
    "flag_konsistenz_offen":
        "Zwei Angaben passen nicht zusammen: Bei einer Einkunftsart hast du angegeben, dass du sie "
        "nicht hast, und an anderer Stelle trotzdem einen Betrag dazu eingetragen. Es geht um eine "
        "der vier Fragen, ob du Gewinneinkünfte, Kapitalerträge, Einnahmen aus Vermietung oder "
        "sonstige Einkünfte wie Renten hast. Bitte sieh dir an, welche der beiden Angaben stimmt.",
    "gewinn_quelle_offen":
        "Deinen Gewinn hast du auf zwei Wegen angegeben: einmal als fertigen Betrag und einmal "
        "aufgeteilt in Betriebseinnahmen, Betriebsausgaben und Abschreibungen. Welcher der beiden "
        "gilt, kann die Software nicht raten. Bitte lass einen der beiden Wege stehen.",
    "kapital_semantik_offen":
        "Deine Kapitalerträge hast du auf zwei Wegen angegeben: einmal als Gesamtsumme und einmal "
        "aufgeteilt in einzelne Gewinne und Verluste. Ob die Einzelbeträge in der Summe schon "
        "enthalten sind oder dazukommen, kann die Software nicht raten. Bitte lass einen der beiden "
        "Wege stehen.",
    "partner_konsistenz_offen":
        "Zwei Angaben passen nicht zusammen: Du hast etwas zu deinem Ehe- oder Lebenspartner "
        "eingetragen — etwa dessen Behinderung, Kapitalerträge oder Rente — aber keine gemeinsame "
        "Veranlagung gewählt. Angaben zum Partner zählen nur in einer gemeinsamen Erklärung. "
        "Bitte sieh dir beide Angaben noch einmal an.",

    # ---- (1) Eine Angabe oder Antwort fehlt noch -------------------------------------------------
    "berufsunfaehigkeit_offen":
        "Du hast den ermäßigten Steuersatz für den Verkauf oder die Aufgabe deines Betriebs "
        "beantragt. Vor dem 55. Geburtstag steht er dir nur zu, wenn du dauernd berufsunfähig bist. "
        "Bitte beantworte diese Frage, auch wenn die Antwort „nein“ ist.",
    # Keine Richtungsaussage ("zu niedrig", "passt nicht"): derselbe Satz gilt fuer einen
    # vorlaeufigen agB-Abzug (senkt die Steuer) und eine vorlaeufige Lohnsteuer-Anrechnung (hebt
    # die Abschlusszahlung) -- jede Richtung waere fuer die Haelfte der Felder falsch. Und kein
    # Verweis auf eine Liste: der Klartext erscheint genau dann, wenn zahl_cent None ist, waehrend
    # die Liste der offenen Angaben im Erfolgs-Zweig daneben steht und hier nie gezeichnet wird
    # (app.js, zwei sich ausschliessende Zweige). tests/test_klasse_c_vorlaeufiger_betrag_sperrt.py
    # haelt beides fest.
    "ring_betrag_vorlaeufig":
        "Ein Betrag, den du genannt hast, ist noch nicht bestätigt. Solange das so ist, zeigt die "
        "Software keine Steuer an; sie könnte einen genannten Betrag sonst nicht mitrechnen. Bitte "
        "sieh dir die Angabe noch einmal an und bestätige sie; danach rechnet die Software die Zahl.",
    "arbeitsmittel_afa_ueber_gwg_offen":
        "Zu deinen angeschafften Arbeitsmitteln fehlt noch, wie die Kosten abgesetzt werden sollen. "
        "Bei Anschaffungen bis 800 Euro ist das die Frage, ob du den Betrag sofort in voller Höhe "
        "absetzen willst; bei teureren Geräten die Nutzungsdauer und — wenn du sie in diesem Jahr "
        "gekauft hast — der Anschaffungsmonat. Bitte beantworte die Rückfragen zu deinen "
        "Arbeitsmitteln.",
    "behinderungsbedingte_aufwendungen_wahlrecht_offen":
        "Du hast eine Behinderung angegeben und zusätzlich Kosten, die dadurch entstanden sind. "
        "Hier hast du die Wahl: entweder der Pauschbetrag ohne Nachweis oder deine tatsächlichen "
        "Kosten mit Belegen. Welcher Weg günstiger ist, hängt an der Höhe deiner Kosten — "
        "deshalb kann die Software das nicht für dich entscheiden. Bitte beantworte die Frage nach "
        "dem Pauschbetrag.",
    "behinderungsbedingte_aufwendungen_wahlrecht_partner_offen":
        "Für deinen Ehe- oder Lebenspartner ist eine Behinderung angegeben und zusätzlich Kosten, "
        "die dadurch entstanden sind. Auch hier gibt es die Wahl zwischen dem Pauschbetrag ohne "
        "Nachweis und den tatsächlichen Kosten mit Belegen. Welcher Weg günstiger ist, hängt an "
        "der Höhe der Kosten — deshalb kann die Software das nicht entscheiden. Bitte beantworte "
        "die Frage nach dem Pauschbetrag für deinen Partner.",
    "dhf_tatbestand_offen":
        "Du hast Kosten für eine zweite Wohnung am Arbeitsort angegeben. Ob sie absetzbar sind, "
        "hängt an drei Voraussetzungen: dass die zweite Wohnung beruflich veranlasst ist, dass du "
        "an deinem Hauptwohnsitz einen eigenen Hausstand führst und dass du dich dort finanziell an "
        "den Kosten beteiligst. Bitte beantworte diese drei Fragen.",
    "gewst_hebesatz_offen":
        "Zu deinem Gewerbebetrieb fehlt der Hebesatz deiner Gemeinde. Ohne ihn lässt sich nicht "
        "berechnen, wie viel Gewerbesteuer auf deine Einkommensteuer angerechnet wird. Den Hebesatz "
        "findest du auf deinem Gewerbesteuerbescheid oder auf der Internetseite deiner Gemeinde.",
    "handwerker_foerderung_offen":
        "Zu deinen Handwerkerkosten fehlt noch die Antwort, ob du dafür öffentliche Fördermittel "
        "bekommen hast — etwa einen zinsverbilligten Kredit oder einen steuerfreien Zuschuss. Für "
        "geförderte Maßnahmen gibt es die Steuerermäßigung nicht. Bitte beantworte diese Frage, "
        "auch wenn du keine Förderung bekommen hast.",
    "haushalt_eu_ewr_offen":
        "Für deine Kosten für Handwerker, Haushaltshilfe oder haushaltsnahe Dienstleistungen fehlt "
        "noch die Antwort, ob der Haushalt in der Europäischen Union oder im Europäischen "
        "Wirtschaftsraum liegt. Nur dann gibt es die Steuerermäßigung. Bitte beantworte diese "
        "Frage — bei einem Haushalt in Deutschland ist sie automatisch mit Ja beantwortet.",
    "p16_4_gate_offen":
        "Es ist ein Gewinn aus dem Verkauf oder der Aufgabe eines Betriebs angegeben — bei dir oder "
        "bei deinem Partner. Dafür gibt es einen Freibetrag, aber nur unter zwei Bedingungen: Die "
        "betreffende Person ist mindestens 55 Jahre alt oder dauernd berufsunfähig, und sie hat "
        "diesen Freibetrag noch nie in Anspruch genommen. Bitte beantworte beide Fragen.",
    "p35c_doppelfoerderung_offen":
        "Zu deiner energetischen Sanierung fehlt noch die Antwort, ob du dafür schon anderweitig "
        "gefördert wurdest — etwa durch öffentliche Zuschüsse oder weil du dieselben Kosten "
        "bereits als Handwerkerleistung geltend machst. In diesen Fällen entfällt die "
        "Steuerermäßigung ganz. Bitte beantworte diese Frage, auch wenn keine andere Förderung "
        "vorliegt.",
    "partner_kegel_offen":
        "Zu deinem Ehe- oder Lebenspartner fehlen noch Angaben, die die gemeinsame Berechnung "
        "braucht — je nach Fall der Bruttoarbeitslohn, die Kapitalerträge oder die Art der "
        "Krankenversicherung. Ein Ergebnis für nur eine der beiden Personen wäre falsch. Bitte "
        "ergänze die offenen Angaben zu deinem Partner.",
    "gwg_tatbestand_offen":
        "Zu einem als Sofortabzug erfassten Gerät fehlt noch eine Antwort zu einer der Voraussetzungen — "
        "ob es allein benutzbar ist, ob der Betrag den Vorsteuerabzug schon abgezogen hat, oder (ab 250 "
        "Euro) ob du dazu eine Liste geführt hast oder es aus deiner Buchführung ersichtlich ist. Bitte "
        "beantworte die offene Frage zu diesem Gerät.",
    "rechnung_unbar_offen":
        "Zu deinen Handwerker- oder Haushaltsdienstleistungen fehlt noch die Antwort, ob du eine "
        "Rechnung erhalten und sie überwiesen hast. Barzahlungen erkennt das Finanzamt hier nicht "
        "an. Bitte beantworte diese Frage.",
    "kinderbetreuung_reine_betreuung_offen":
        "Zu deinen Betreuungskosten fehlt noch die Antwort, ob der Betrag reine Betreuung ist. "
        "Nachhilfe, Musik- oder Sportunterricht und Freizeitkurse sind keine Betreuung und werden "
        "nicht abgezogen. Hast du beides in einem Betrag gezahlt, trage bitte nur den "
        "Betreuungsteil ein und antworte dann mit Ja.",
    "kinderbetreuung_zahlung_offen":
        "Zu deinen Betreuungskosten fehlt noch die Antwort, ob du eine Rechnung erhalten und per "
        "Überweisung bezahlt hast. Das Finanzamt erkennt nur Betreuungskosten an, die auf das "
        "Konto des Betreuers überwiesen wurden. Bar bezahlte Beträge zählen nicht. Bitte "
        "beantworte diese Frage oder trage nur den überwiesenen Teil ein.",
    "rente_instanz_offen":
        "Zu einer deiner Renten oder zu einer Rente deines Partners sind die Angaben unvollständig. "
        "Für jede einzelne Rente braucht die Berechnung vier Dinge: die Art der Rente, den "
        "Jahresbetrag, das Jahr des Rentenbeginns und das Alter der beziehenden Person zu diesem "
        "Zeitpunkt. Bitte ergänze die fehlenden Angaben.",
    "rentenbeginn_offen":
        "Zu deiner Rente fehlt das Jahr, in dem die Rentenzahlung begonnen hat. Die Berechnung "
        "braucht dieses Jahr, um den steuerfreien Teil der Rente richtig festzulegen. Bitte trage "
        "das Jahr des Rentenbeginns ein.",
    "rentenfreibetrag_fixierung_offen":
        "Die Rente hat vor diesem Jahr begonnen. Dann ist der steuerfreie Teil der Rente ein fester "
        "Eurobetrag, der im Jahr nach dem Rentenbeginn einmal festgelegt wurde und sich seither "
        "nicht mehr ändert. Diesen Betrag findest du in einem früheren Steuerbescheid. Bitte trage "
        "ihn ein.",
    "uebernachtung_tatbestand_offen":
        "Du hast Übernachtungskosten auf Auswärtstätigkeit angegeben. Ob sie absetzbar sind, "
        "hängt an mehreren Fragen: ob die Unterkunft im Inland liegt, ob die Übernachtung wirklich "
        "auswärts stattfand, ob du die Unterkunft allein genutzt hast und ob die Tätigkeit ohne "
        "lange Unterbrechung lief. Bitte beantworte diese Fragen.",
    "verpflegung_dreimonatsfrist_aufteilung_offen":
        "Du warst länger als drei Monate am selben auswärtigen Ort tätig. Die "
        "Verpflegungspauschale gibt es nur für die ersten drei Monate, danach entfällt sie. "
        "Deshalb braucht die Berechnung zu jeder Art von Abwesenheitstag zusätzlich die Zahl der "
        "Tage, die nach diesen drei Monaten lagen. Bitte ergänze diese Angabe.",
    "verpflegung_dreimonatsfrist_unterbrechung_offen":
        "Du warst länger als drei Monate am selben auswärtigen Ort tätig, hast aber für die Zeit "
        "nach Ablauf der drei Monate keine Abwesenheitstage angegeben. Das ist möglich, wenn du die "
        "Tätigkeit dort mindestens vier Wochen unterbrochen hast — dann beginnt die Frist neu. "
        "Bitte beantworte die Frage, ob es eine solche Unterbrechung gab.",
    "verpflegung_reduktion_offen":
        "Zu deinen Auswärtstätigkeiten fehlt noch die Antwort, ob dir dabei Mahlzeiten gestellt "
        "wurden — also Frühstück, Mittag- oder Abendessen von deinem Arbeitgeber oder auf dessen "
        "Veranlassung. Jede gestellte Mahlzeit kürzt die Verpflegungspauschale. Bitte beantworte "
        "diese Frage, auch wenn keine Mahlzeiten gestellt wurden.",
    "versorgungsfreibetrag_offen":
        "Du hast Versorgungsbezüge angegeben — etwa eine Betriebsrente oder eine Beamtenpension. "
        "Für den Freibetrag darauf braucht die Berechnung zwei Angaben: das Jahr, in dem die "
        "Versorgung begann, und den Betrag, aus dem der Freibetrag berechnet wird. Beides findest du "
        "in deiner Lohnsteuerbescheinigung oder in der Mitteilung deiner Versorgungsstelle.",
    "vv_instanz_offen":
        "Zu einer deiner vermieteten Immobilien sind die Angaben unvollständig. Jedes weitere "
        "Objekt braucht dieselben Angaben wie das erste: Mieteinnahmen, Gebäudeabschreibung, "
        "Schuldzinsen, Erhaltungsaufwand, sonstige Werbungskosten und den Anteil, der entgeltlich "
        "vermietet ist. Bitte ergänze die fehlenden Angaben.",
}


# Ein Grund ohne Text ist ein Maschinenstring auf dem Schirm — genau der Zustand, den diese
# Zuordnung beendet. tests/test_sperrgrund_klartext.py hält jeden Rückgabewert von
# _an_gesamt_sperrgrund gegen SPERRGRUND_KLARTEXT, ein neuer Grund ohne Satz fällt dort sofort auf.
# Kommt trotzdem einer durch (etwa ein Grund aus einer anderen Quelle als dieser Funktion), ist ein
# ehrlicher Satz besser als die rohe Kennung: der Nutzer erfährt, dass es an der Software liegt und
# nicht an ihm.
UNBEKANNTER_SPERRGRUND = (
    "Die Berechnung kann an dieser Stelle nicht fortgesetzt werden, und woran genau es liegt, lässt "
    "sich hier nicht in Worte fassen. Das liegt an der Software, nicht an deinen Angaben. Bitte "
    "melde diesen Fall — damit lässt sich nachvollziehen, was gefehlt hat."
)


def sperrgrund_klartext(grund: str | None) -> str | None:
    """Der Satz zu einem Sperrgrund, den ein Laie lesen kann.

    None (= keine Sperre) bleibt None: es gibt nichts zu erklaeren. Jeder andere Wert — auch ein
    unbekannter — liefert einen Satz, nie None und nie die rohe Kennung.
    """
    if grund is None:
        return None
    return SPERRGRUND_KLARTEXT.get(grund, UNBEKANNTER_SPERRGRUND)


def sperrgrund_felder(grund: str | None, felder: dict) -> list:
    """Die Angaben, die einen Widerspruchs-Sperrgrund ausloesen — leer bei allen anderen.

    GEGENSTUECK ZU `sperrgrund_klartext`, und aus demselben Grund: wer den Grund liefert, muss
    auch sagen koennen, WORAN er haengt. Hier stand bis 2026-09-26 nur

        if PC.partner_ohne_zusammen(felder):
            return "partner_konsistenz_offen"

    — der Rueckgabewert wurde als Wahrheitswert verbraucht und weggeworfen, obwohl er je
    Widerspruch `feld_id`, `wert` und einen fertigen Satz traegt (partner_check.py:66). Der Nutzer
    las damit, DASS zwei Angaben sich widersprechen, aber nicht, WELCHE, und hatte keinen Weg zu
    der Angabe, die die Sperre aufhebt (backlog
    partnerangaben-nach-umstellung-auf-einzel-sackgasse, gemessen 2026-09-26).

    HIER, NICHT IN DER HAUT: der Widerspruch entsteht in `partner_check`, erkannt wird er in
    `_an_gesamt_sperrgrund`. Ein zweiter Aufruf in api.py waere eine zweite Wahrheit ueber
    denselben Widerspruch — genau die Bauform, die uns beim p16_4-Gate den Rueckweg gekostet hat.

    `partner_ohne_zusammen` zaehlt nur BESTAETIGTE Werte (s. dort): ein vorlaeufiger Wert ist kein
    Beleg und loest auch die Sperre nicht aus. Die beiden bleiben damit deckungsgleich."""
    if grund == "partner_konsistenz_offen":
        return PC.partner_ohne_zusammen(felder)
    return []


def _rentenbeginn_offen_stand(felder: dict, cfg: dict | None = None) -> str | None:
    """§ 22 aa Rentenfreibetrag (K2, /stand-spezifisch): /ergebnis faengt eine fehlende
    rentner_renten_beginn_jahr ueber die Kegel-Vollstaendigkeitspruefung (input_kegel_nicht_bestaetigt);
    /stand faehrt diese Pruefung NICHT mit (gemessen: ein normaler, drei-von-zwanzig-Fragen-Fall würde
    sonst sofort komplett sperren -- das zerstoert genau den Zwischenstand-Zweck). Ohne dieses Gate lief
    eine fehlende Jahresangabe hier still als 0-EUR-Rente durch (HTTP 200, min=max=0 fuer 20.000 EUR
    Jahresrente) statt als Absturz oder Sperre sichtbar zu werden -- deshalb eng auf DIESES eine Feld
    begrenzt, nicht als Ersatz fuer die Kegel-Pruefung. NUR von stand() gerufen, nicht von
    _an_gesamt_sperrgrund (die bedient auch /ergebnis und /einreichen, wo dieselbe Luecke schon die
    Kegel-Vollstaendigkeitspruefung deckt -- ein zweiter Treffer dort wuerde deren grund verfaelschen)."""
    if not cfg or not cfg.get("rentner"):
        return None
    jahresrente = felder.get("rentner_jahresrente", {}).get("wert")
    if (isinstance(jahresrente, (int, float)) and not isinstance(jahresrente, bool)
            and jahresrente > 0
            and not isinstance(felder.get("rentner_renten_beginn_jahr", {}).get("wert"), int)):
        return "rentenbeginn_offen"
    return None


def _vorlaeufige_ring_betraege(felder: dict, cfg: dict, bindung: dict) -> list:
    """Die vorlaeufigen Betragsfelder DIESER Scheibe, die der Ring liest (Klasse C).

    Warum es diese Funktion gibt
    ----------------------------
    _bescheid_fn (bescheid_zweige.py) filtert die flachen felder auf `zustand == "bestaetigt"`.
    Ein vorlaeufiger Betrag ist darin ABSENT, die slot_fn liest `_c(fid) == 0` und rechnet ohne ihn
    weiter. Das ist als over-tax-safe gewollt und richtig -- falsch war nur, dass die Zahl weiter
    "bestaetigt" hiess. Gemessen (5f5cbfd, Scheibe rentner_gesamt): 100.000 EUR vorlaeufiger
    Veraeusserungsgewinn ergaben 59.170,00 EUR statt 82.270,00 EUR, ohne Signal und mit
    gruenem /preflight. Der Nutzer hatte den Betrag genannt.

    Der Kegel deckt das NICHT ab: er ist die Pflicht-Seite, und die betroffenen Felder sind
    gerade die optionalen (agB, Spenden, § 16-vg, § 19-Versorgung, ...) -- genau die, die
    laut Kommentar an _bescheid_fn nie eine Zahl bewegen duerfen, bevor der Mensch bestaetigt.

    Warum generisch und nicht je Feld
    ---------------------------------
    Die Zustandsachse wird sonst nur an den Stellen geprueft, die der Guard einzeln kennt --
    § 16 Abs. 4 prueft die zwei Bedingungs-Bools statt den Betrag, § 19 Abs. 2 Beginnjahr und
    Bemessungsgrundlage statt versorgung_jahresrente. Jedes neue Betragsfeld haette dieselbe
    Luecke neu geerbt. Die Menge kommt deshalb aus api_constants.RING_BETRAGSFELDER (dort steht,
    wie sie hergeleitet ist) und wird hier nur noch auf die Scheibe geschnitten.

    Was NICHT gesperrt wird: Felder, die der Ring gar nicht liest. Ihr Fehlen aendert die Zahl
    nicht, eine Sperre waere ein Fehlalarm. Und Felder aus Instanzgruppen (gwg/kind/p23/...):
    die haben ihren eigenen, schon vorhandenen Filter und eine eigene Hinweis-Behandlung
    (api._ergebnis_roh, offen_c).
    """
    kegel = set(cfg.get("kegel") or ())
    treffer = []
    for fid in RING_BETRAGSFELDER:
        if fid in kegel:
            continue        # Pflicht-Seite: deckt _feste_zahl / der Kegel-Meet bereits ab
        if fid not in (cfg.get("felder") or ()):
            continue        # gehoert nicht zu dieser Scheibe
        if (bindung.get(fid) or {}).get("typ") not in ("cent", "int"):
            continue        # kein Betrag (bool/str/enum) -- eigene Gates, s. die _b-Zweige
        ev = felder.get(fid)
        if ev is not None and ev.get("zustand") != "bestaetigt":
            treffer.append(fid)
    return treffer


def _an_gesamt_sperrgrund(felder: dict, cfg: dict | None = None, vz: int | None = None,
                          store: dict | None = None, bindung: dict | None = None):
    """K2-Guard: nicht-ring-fähige Werbungskosten/Einkunftsarten sperren den Ring GANZ (nie Fake-0).
    Ein dHf-/Verpflegung-/AM-Feld mit Wert > 0 (vorläufig ODER bestätigt) sperrt (kein Catala-Modul);
    ein fremd_arten-Flag = false (Nutzer HAT eine NICHT von dieser Scheibe gerechnete Art) macht den
    §2-Ring unzulässig. VOR (§ 10) ist seit Stufe 1a ring-fähig — kein Guard mehr."""
    def _positiv(f):
        v = felder.get(f)
        w = v and v.get("wert")
        return isinstance(w, (int, float)) and not isinstance(w, bool) and w > 0
    def _dhf_vpf_grund():
        # dHf/Verpflegung §9-WK-Tatbestand — fail-closed (K2). Gilt für JEDE Scheibe, die diese Felder
        # ring-verdrahtet: an_gesamt (catala_est) UND der gesamt/rentner-WK-Pfad (B1, catala_werbungskosten_n).
        # Ausland-dHf → nicht ring-fähig; offene Geltungsbedingung → offen; offene Reduktion (§9 Abs.4a) → offen.
        if _positiv(DHF_KOSTEN):
            _dhf_inland = felder.get("dhf_im_inland") or {}
            if _dhf_inland.get("wert") is False:
                return "ausland_dhf_nicht_ring_faehig"
            # Naht-Fix (gate-naht-guard-liest-zustand): NICHT nur "is True" auf dem Rohwert — ein
            # vorläufiger Wert las hier durch (der Ring filtert auf bestätigt und sieht das Feld
            # dann als fehlend, silent-drop statt Sperre). Beide Bedingungen zusammen.
            if _dhf_inland.get("wert") is not True or _dhf_inland.get("zustand") != "bestaetigt":
                return "dhf_tatbestand_offen"
            if any((felder.get(b) or {}).get("zustand") != "bestaetigt" for b in DHF_BEDINGUNGEN):
                return "dhf_tatbestand_offen"
        if sum((felder.get(t, {}).get("wert") or 0) for t in VERPFLEGUNG_TAGE) > 0:
            # Naht-Fix: ohne bestätigten vpf_monate_am_ort weiss der Guard nicht, ob die 3-Monats-
            # Aufteilung (S.6, unten) greift — ein fehlender/vorläufiger Wert liess die Prüfung dort
            # (isinstance(None, int) ist False) einfach AUS, der Ring bekam die volle Pauschale wie
            # bei ≤3 Monaten, ohne dass "≤3 Monate" je bestätigt war.
            _mon_feld = felder.get("vpf_monate_am_ort") or {}
            _mon_wert = _mon_feld.get("wert")
            if not (isinstance(_mon_wert, int) and not isinstance(_mon_wert, bool)
                    and _mon_feld.get("zustand") == "bestaetigt"):
                return "verpflegung_dreimonatsfrist_aufteilung_offen"
            # Verpflegungspauschale (§ 9 Abs. 4a S. 3): Jahres-Pauschale summiert aus Tage-Kategorien.
            # S. 6 (3-Monats-Frist): Reduktion — wenn vpf_monate_am_ort > 3, MUSS die Aufteilung
            #   (Tage_gesamt vs. Tage_nach_Frist) angegeben sein, ABER NUR FÜR KATEGORIEN MIT TAGEN > 0.
            # S. 8-11 (Mahlzeitenkürzung + steuerfreie Erstattung): RECHENBAR, wenn Felder bestätigt.
            # Prüfung Frist (S. 6): kategorie-weise — nur wenn Tage_i > 0, muss NACH_FRIST_i bestätigt sein.
            _mon = felder.get("vpf_monate_am_ort", {}).get("wert")
            if isinstance(_mon, int) and not isinstance(_mon, bool) and _mon > 3:
                # > 3 Monate: Prüfe jede Kategorie separat
                # VERPFLEGUNG_TAGE[i] <-> VERPFLEGUNG_TAGE_NACH_FRIST[i] sind positionsgleich
                for i, (tage_feld, nach_frist_feld) in enumerate(zip(VERPFLEGUNG_TAGE, VERPFLEGUNG_TAGE_NACH_FRIST)):
                    tage_wert = (felder.get(tage_feld, {}).get("wert") or 0)
                    if isinstance(tage_wert, (int, float)) and not isinstance(tage_wert, bool) and tage_wert > 0:
                        # Diese Kategorie hat Tage > 0 → NACH_FRIST-Feld ist Pflicht
                        nach_frist_bestaetigt = (felder.get(nach_frist_feld) or {}).get("zustand") == "bestaetigt"
                        if not nach_frist_bestaetigt:
                            # Kategorie mit Tagen aber ohne NACH_FRIST-Angabe → fail-closed
                            return "verpflegung_dreimonatsfrist_aufteilung_offen"
            # Plausibilitäts-Frage: monate > 3 + Tage > 0 aber ALLE NACH_FRIST = 0
            # → unplausibel (0 Tage nach Frist bei mehrmonatiger Tätigkeit), aber legitim möglich
            # (Unterbrechung ≥4 Wochen setzt Frist zurück per S.7). Rückfrage nötig.
            _mon = felder.get("vpf_monate_am_ort", {}).get("wert")
            if isinstance(_mon, int) and not isinstance(_mon, bool) and _mon > 3:
                # Nur bestätigte Tage zählen (Konsistenz mit kategorie-weise Prüfung oben)
                tage_gesamt = sum((felder.get(t, {}).get("wert") or 0)
                                  for t in VERPFLEGUNG_TAGE
                                  if (felder.get(t) or {}).get("zustand") == "bestaetigt")
                tage_nach_frist_gesamt = sum((felder.get(t, {}).get("wert") or 0)
                                             for t in VERPFLEGUNG_TAGE_NACH_FRIST
                                             if (felder.get(t) or {}).get("zustand") == "bestaetigt")
                if tage_gesamt > 0 and tage_nach_frist_gesamt == 0:
                    # monate > 3 + Tage insgesamt > 0 + ALLE nach_frist = 0 → Rückfrage
                    # Nur der ZUSTAND zaehlt, nicht der Wert: die Antwort ist eine Erklaerung,
                    # kein Rechenparameter. Deshalb aendert die Polaritaet des Feldes die
                    # Steuer nicht — sie steuert nur den Traverser (Gate, siehe Bindung).
                    frist_erklaert = (felder.get("vpf_frist_nicht_unterbrochen") or {}).get("zustand") == "bestaetigt"
                    if not frist_erklaert:
                        # Rückfrage nicht beantwortet
                        return "verpflegung_dreimonatsfrist_unterbrechung_offen"
            # Mahlzeitenkürzung (S. 8-11): fail-closed auf Eingabe. Die Frage muss beantwortet sein:
            # "Wurden dir Mahlzeiten gestellt?" — wenn JA, dann Anzahlen + Entgelt; wenn NEIN, dann 0 bestätigt.
            # Mahlzeitenzahlen-Felder (fruehstuecke/mittag/abendessen_gestellt_anzahl) sind die Antwort:
            # - Alle UNSET (fehlend) = unbeantwortet → SPERRE
            # - Alle auf 0 bestätigt = beantwortet mit "nein" → OK (keine Kürzung)
            # - >= 1 bestätigt = beantwortet mit "ja" → OK (Kürzung rechnet)
            # Mahlzeitenfrage: neue Semantik (Anzahl-Felder, S. 8-11 Kürzung rechenbar) +
            # alte Semantik (Fallback für an_gesamt-TEST: vpf_keine_mahlzeitengestellung bool).
            mahlzeitenzahl_felder = (
                "vpf_fruehstuecke_gestellt_anzahl",
                "vpf_mittagessen_gestellt_anzahl",
                "vpf_abendessen_gestellt_anzahl",
            )
            # Neu: mindestens ein Anzahl-Feld bestätigt?
            zahlen_bestaetigt = any(
                (felder.get(f) or {}).get("zustand") == "bestaetigt"
                for f in mahlzeitenzahl_felder
            )
            # Alt: vpf_keine_mahlzeitengestellung (bool) bestätigt?
            # Nur "True" (="keine gestellt") ist vollständige Antwort.
            # "False" (="doch gestellt") OHNE Anzahlen → Sperre (unvollständig).
            keine_mahlz_feld = felder.get("vpf_keine_mahlzeitengestellung") or {}
            keine_mahlz_bestaetigt = keine_mahlz_feld.get("zustand") == "bestaetigt"
            keine_mahlz_wert_true = keine_mahlz_bestaetigt and keine_mahlz_feld.get("wert") is True

            # Frage beantwortet, wenn:
            # (neu: ≥1 Anzahl bestätigt) ODER (alt: bool=True bestätigt, "keine gestellt")
            mahlzeiten_beantwortet = zahlen_bestaetigt or keine_mahlz_wert_true
            if not mahlzeiten_beantwortet:
                # Keine Angabe zu gestellten Mahlzeiten, oder bool=False ohne Anzahlen — fail-closed.
                return "verpflegung_reduktion_offen"
        # Übernachtung Auswärtstätigkeit (§ 9 Abs. 1 Nr. 5a): Kosten > 0 → Ring nur fähig, wenn der
        # Tatbestand bestätigt ist (Inland/Ausland als Ortsangabe, die 3 Bedingungen) UND die
        # Zeitraum-Angaben bestätigte int sind.
        # KEINE Sperre mehr für Ausland: die Sätze 1-3 unterscheiden nicht nach dem Ort, und Satz 4
        # begrenzt nur die HÖHE (Ring kappt Inland nach 48 Monaten, Ausland erst ab VZ 2026).
        # KEINE Sperre mehr für den überspannenden Zeitraum: die Schwelle ist ein Zeitpunkt, der Ring
        # teilt monatsweise (siehe runner._uebernachtung_abzug). Beides nahm dem Nutzer den GANZEN
        # Abzug, wofür die Norm keine Grundlage hat.
        if _positiv(UEBERNACHTUNG_KOSTEN):
            _ueb_inland = felder.get("uebernachtung_im_inland") or {}
            # Naht-Fix: Rohwert reichte hier bisher (vorläufig las durch) — jetzt zusätzlich bestätigt
            # verlangt, sonst filtert der Ring das Feld weg und rechnet blind weiter. BEIDE Werte sind
            # gültig: True = Inland (Ring kappt nach 48), False = Ausland (kappt erst ab VZ 2026). Nur
            # eine fehlende oder vorläufige Ortsangabe sperrt — sonst wäre Ausland wieder blockiert,
            # nur unter anderem Namen.
            if (not isinstance(_ueb_inland.get("wert"), bool)
                    or _ueb_inland.get("zustand") != "bestaetigt"
                    or any((felder.get(b) or {}).get("zustand") != "bestaetigt" for b in UEBERNACHTUNG_BEDINGUNGEN)):
                return "uebernachtung_tatbestand_offen"
            _bisher_feld = felder.get("uebernachtung_monate_bisher") or {}
            _monate_feld = felder.get("uebernachtung_monate") or {}
            bisher = _bisher_feld.get("wert")
            monate = _monate_feld.get("wert")
            # Naht-Fix: bisher/monate müssen BESTÄTIGTE int sein — sonst sieht der Guard hier (auf
            # Rohdaten) einen Wert, den der Ring (bestätigt-only) gar nicht kennt, und rechnet mit
            # dem impliziten Default 0 weiter (unbegrenzt statt gekappt, oder WK ganz verschluckt).
            if not (isinstance(bisher, int) and not isinstance(bisher, bool)
                    and _bisher_feld.get("zustand") == "bestaetigt"
                    and isinstance(monate, int) and not isinstance(monate, bool)
                    and _monate_feld.get("zustand") == "bestaetigt"):
                return "uebernachtung_zeitraum_offen"
        # Arbeitsmittel (§ 9 Abs. 1 Nr. 6/7 i.V.m. § 6 Abs. 2 GWG / § 7 AfA): AK > 0 → Ring nur fähig für den
        # GWG-Sofortabzug (AK ≤ 800 EUR mit ausgeübtem Wahlrecht). AK > 800 → mehrjährige § 7-AfA (A6-L2),
        # die ring-fähig ist sobald die Nutzungsdauer gesetzt ist (arbeitsmittel_nutzungsdauer > 0).
        # Schwelle in CENT (80000): 800,01 EUR floort sonst fälschlich auf 800 (Under-tax).
        _am = felder.get(ARBEITSMITTEL_KOSTEN, {}).get("wert")
        if isinstance(_am, (int, float)) and not isinstance(_am, bool) and _am > 0:
            if _am <= 80000:
                _gwg_feld = felder.get("am_gwg_sofortabzug_gewaehlt") or {}
                # Naht-Fix: "is not True" auf dem Rohwert liess ein vorläufiges True durch — der
                # Ring filtert auf bestätigt, sieht das Feld dann fehlend und rechnet ohne Abzug.
                if not (_gwg_feld.get("wert") is True and _gwg_feld.get("zustand") == "bestaetigt"):
                    return "arbeitsmittel_afa_ueber_gwg_offen"
            else:  # _am > 80000 → § 7 Abs. 1 lineare AfA
                nd = felder.get("arbeitsmittel_nutzungsdauer", {}).get("wert")
                monat = felder.get("am_anschaffung_monat", {}).get("wert")
                ist_aj_feld = felder.get("am_afa_ist_anschaffungsjahr") or {}
                ist_aj = ist_aj_feld.get("wert")
                # § 7 Abs. 1: Nutzungsdauer MUSS beantwortet sein (fail-closed).
                # Anschaffungsmonat + Zustand-Flag: nur wenn Anschaffungsjahr=true.
                # Flag unbeantwortet → Folgejahr angenommen (voller Jahresbetrag, Monat egal).
                # Grund: S. 4 Zwölftelung gilt NUR im Anschaffungsjahr; Folgejahre voller Betrag.
                if not isinstance(nd, int) or isinstance(nd, bool) or nd <= 0:
                    return "arbeitsmittel_afa_ueber_gwg_offen"
                # Wenn Anschaffungsjahr=true: Monat MUSS beantwortet sein. Naht-Fix: ein vorläufiges
                # True fiel sonst durch die "unbeantwortet → Folgejahr"-Annahme unten — der Ring
                # filtert das Feld auf bestätigt weg und rechnet den vollen Jahresbetrag statt 1/12.
                if ist_aj is True:
                    if ist_aj_feld.get("zustand") != "bestaetigt":
                        return "arbeitsmittel_afa_ueber_gwg_offen"
                    if (not isinstance(monat, int) or isinstance(monat, bool) or monat < 1 or monat > 12):
                        return "arbeitsmittel_afa_ueber_gwg_offen"
                # ist_aj == false oder None (bestätigt oder gar nicht beantwortet) → Folgejahr:
                # voller Jahresbetrag, OK — unverändert, das ist der akzeptierte Default.
        return None
    # Partner-Behinderungsfeld (§ 33b Person B) ohne Zusammenveranlagung: benannte Inkonsistenz
    # (dev-2s partner_check, Spiegel zu partner_kegel_offen). Universell VOR der Scheiben-Verzweigung —
    # feuert live, sobald eine Scheibe (rentner_gesamt) die rentner_*_partner-Felder führt.
    if PC.partner_ohne_zusammen(felder):
        return "partner_konsistenz_offen"
    # § 24b Alleinerziehend ↔ Zusammenveranlagung (D-Fix, K2, Under-tax): fam_alleinstehend bestätigt True UND
    # veranlagung=zusammen → Widerspruch (§ 24b Abs. 1/3 verlangt „allein stehend", nicht zusammenveranlagt). Der
    # § 24b-Entlastungsbetrag würde sonst still gewährt = Unter-Besteuerung → fail-closed (dev-2s partner_check).
    # Universell vor der Scheiben-Verzweigung — feuert für jede Scheibe mit fam_alleinstehend + veranlagung (gesamt).
    if PC.alleinerziehend_mit_zusammen(felder):
        return "alleinerziehend_konsistenz_offen"
    # § 34 Abs. 3 >5Mio-Excess (Guard-A, fail-closed, beide Ringe): antrag_ermaessigter_satz ∧ eligible (55+/berufs-
    # unfähig ∧ ¬einmal) ∧ VÄ-Gewinn > 5 Mio → der ermäßigte Satz gilt nur bis 5 Mio (§ 34 Abs. 3 S. 1); der Excess
    # (Stufe-2b) ist unaufgelöst → fail-closed statt still auf Abs.1 fallen (das verweigerte den Abs.3-Benefit auf die
    # ersten 5 Mio = Over-tax). ¬eligible → KEIN Guard (Abs.1-Fünftel auf ganzes ao, kein 5Mio-Cap). _abs3_eligible =
    # bit-identisch zum Chooser. Schwelle auf raw VÄ-Gewinn = äquiv. netto_vg (§16-Abs.4-FB = 0 ab vg>181000 ≪ 5Mio).
    if vz is not None and felder.get("antrag_ermaessigter_satz", {}).get("wert") is True and _abs3_eligible(felder, vz) \
            and int(felder.get("rentner_veraeusserungsgewinn", {}).get("wert") or 0) // 100 > 5_000_000:
        return "abs3_ueber_5mio_offen"
    # § 34 Abs. 3 S. 1 „oder wenn er ... dauernd berufsunfähig ist": Antrag gestellt, die Berufsunfähigkeit
    # unbeantwortet, und NUR sie entscheidet noch über den ermäßigten Satz (unter 55, nicht einmal genutzt).
    # Ohne Antwort lief der Chooser still auf Abs. 1 (gemessen 2026-09-28: vg 500.000 EUR, Jg. 1980 →
    # 155.420 statt 115.221 EUR). Die Frage an _abs3_eligible selbst gestellt, damit Guard und Chooser
    # nicht driften. Ein bestätigtes „nein" ist eine Antwort und sperrt nicht; über 55 sperrt nichts.
    if (vz is not None and felder.get("antrag_ermaessigter_satz", {}).get("wert") is True
            and (felder.get("dauernd_berufsunfaehig") or {}).get("zustand") != "bestaetigt"
            and not _abs3_eligible(felder, vz)
            and _abs3_eligible({**felder, "dauernd_berufsunfaehig": {"wert": True}}, vz)):
        return "berufsunfaehigkeit_offen"
    # an_gesamt Gap-A (K2, Over-tax): Kinder → §31/§32-KiFB-Rechnung NICHT in dieser Scheibe.
    # an_gesamt nutzt catala_est (kein §2-Gesamt-Scope, kein freibetraege_kinder); Kinder-Fälle
    # gehören in Scheibe "gesamt", die den vollen §31-Günstiger-§2-Lauf macht. Der Guard feuert
    # NUR für Scheiben OHNE gesamt_guard (d.h. an_gesamt; gesamt/rentner_gesamt haben gesamt_guard
    # = §31-fähig). Feuert bei bestätigtem fam_anzahl_kinder > 0 — kein stiller §31-loser Bescheid.
    # fam_anzahl_kinder == 0 (bestätigt kinderlose) → normale AN-Berechnung. Pflichtfeld im Kegel
    # (K2-safe: ohne Antwort keine festzusetzende Steuer). UI-Screening vor Scheibe-Wahl = Backlog.
    if cfg and not cfg.get("gesamt_guard") and _positiv("fam_anzahl_kinder"):
        return "kinder_gehoeren_in_gesamt"
    # an_gesamt Gap-B (K2, Over-tax): Verlustvortrag (§10d Abs.2) nicht in catala_est rechenbar
    # (kein §2-Gesamt-Scope, kein sonstige_abzuege_vom_einkommen-Slot). Gleiches Muster wie Gap-A.
    if cfg and not cfg.get("gesamt_guard") and _positiv("verlustvortrag_bestand"):
        return "verlustvortrag_gehoert_in_gesamt"
    # an_gesamt §32b (K2, Over-tax): Lohnersatz (Elterngeld/Krankengeld) ist bei Angestellten
    # HAEUFIG. an_gesamts catala_est hat KEIN extrahierbares zvE → GATE statt Under-tax.
    if cfg and not cfg.get("gesamt_guard") and _positiv("p32b_progressionseinkuenfte"):
        return "progression_gehoert_in_gesamt"
    if cfg and cfg.get("gesamt_guard"):
        # §34c DBA-Anrechnung (Stufe-1, K2): fail-closed bei multi-country,
        # §32d-Kapital. Ohne diese Gates wäre die Anrechnung still 0 (gezahlt=0/ausl=0=absent) —
        # was bei vorhandenem (aber nicht gestütztem) DBA-Sachverhalt legitim = kein silent Over-tax.
        # Die GATES treffen nur die Fälle, wo der Nutzer aktiv DBA-Werte gesetzt hat, die diese
        # Scheibe nicht rechenbar macht → fail-closed (= keine stille 0-Anrechnung).
        dba_methode = (felder.get("dba_methode") or {}).get("wert")
        dba_mehrere = (felder.get("dba_mehrere_staaten") or {}).get("wert")
        if dba_mehrere is True:
            return "dba_multi_country_offen"
        if any(_positiv(k) for k in KAP_TOEPFE) or (felder.get(KAP_ERTRAEGE, {}).get("wert") or 0) > 0:
            if _positiv("dba_auslaendische_einkuenfte"):
                return "dba_kapital_offen"
        # §32b-Koinzidenz (K2, fail-closed): §32b Post-Engine NACH §34/§35/§34c.
        # Co-Präsenz (Lohnersatz + ao-Gewinn/GewSt/DBA) unaufgelöst in Stufe-1 → fail-closed
        # (kein silent-wrong-Basis-Deckel). Stufe-2: korrekte Post-§32b-Höchstbeträge.
        p32b_pe = (felder.get("p32b_progressionseinkuenfte") or {}).get("wert")
        p32b_has_pe = isinstance(p32b_pe, (int, float)) and not isinstance(p32b_pe, bool) and p32b_pe > 0
        if p32b_has_pe:
            # 1. §34 ao-Gewinn / ermäßigter Satz
            ao_gewinn = (felder.get("rentner_veraeusserungsgewinn") or {}).get("wert")
            if (isinstance(ao_gewinn, (int, float)) and not isinstance(ao_gewinn, bool) and ao_gewinn > 0) \
                    or (felder.get("antrag_ermaessigter_satz", {}).get("wert") is True):
                return "p32b_kombi_offen"
            # 1b. §34 ao-Gewinn des Ehegatten: die Fünftelung umfasst beide (p34-fuenftelung-umfasst-beide-ehegatten).
            if felder.get("veranlagung", {}).get("wert") == "zusammen" and _positiv("rentner_veraeusserungsgewinn_partner"):
                return "p32b_kombi_offen"
            # 2. §35 Gewerbesteuer
            if _positiv("gewst_messbetrag"):
                return "p32b_kombi_offen"
            # 3. §34c DBA-Anrechnung
            if _positiv("dba_gezahlte_auslaendische_steuer") or _positiv("dba_auslaendische_einkuenfte"):
                return "p32b_kombi_offen"
        # § 16 Abs. 4 Freibetrag (fail-closed): Veräußerungsgewinn > 0 erfordert, dass alter_55_
        # oder_berufsunfaehig UND freibetrag_erstmalig BEIDE bestätigt sind — egal mit welchem Wert.
        # Naht-Fix (gate-naht-guard-liest-zustand): die Sperre fragt nur nach der ENTSCHEIDUNG, nicht
        # nach ihrem Ausgang. Vorher prüfte sie den Rohwert ("is True"), also liess ein vorläufiges
        # True durch (der Ring filtert auf bestätigt und gewährt den FB dort ohnehin unconditional,
        # s. bescheid_zweige._zweig_festzusetzende_est_gesamt/_rentner) — under-tax. Umgekehrt sperrte
        # ein bestätigtes False (eine gültige, abschliessende Antwort: kein FB) den Bescheid komplett
        # statt eine echte Zahl ohne FB zu liefern — der Ring prüft die Bools jetzt selbst (s. dort).
        if _positiv("rentner_veraeusserungsgewinn"):
            if not (felder.get("rentner_alter_55_oder_berufsunfaehig", {}).get("zustand") == "bestaetigt"
                    and felder.get("rentner_freibetrag_erstmalig", {}).get("zustand") == "bestaetigt"):
                return "p16_4_gate_offen"
        # Dasselbe für den Ehegatten (Stufe 2 der Partnerachse, 2026-08-13). Ohne diesen Spiegel
        # gewährte _gewinn_partner_anteil dem Partner-vg den Freibetrag, OHNE dass die Abs. 4-
        # Bedingungen je geprüft wurden — under-tax. Nur bei Zusammenveranlagung: bei
        # Einzelveranlagung rechnet der Ring den Partner-vg gar nicht, ein dort stehender Wert
        # darf den eigenen Bescheid deshalb auch nicht sperren.
        if felder.get("veranlagung", {}).get("wert") == "zusammen":
            if _positiv("rentner_veraeusserungsgewinn_partner"):
                if not (felder.get("rentner_alter_55_oder_berufsunfaehig_partner", {}).get("zustand") == "bestaetigt"
                        and felder.get("rentner_freibetrag_erstmalig_partner", {}).get("zustand") == "bestaetigt"):
                    return "p16_4_gate_offen"
        # Gesamt-Ring: Flag↔Einkunftsart-Widerspruch (kein_X=true + echtes Feld > 0 bestätigt) surfacen —
        # K2, keine still übergangene Einkunftsart (dev-2s flag_check).
        if FC.flag_widersprueche(felder, bindung):
            return "flag_konsistenz_offen"
        # Kapital-Semantik (Instructor-Q1, fail-closed): E1900701-Aggregat UND Verlust-Töpfe beide gesetzt
        # → additiv-vs-subset ungeklärt (benannter GAP) → kein Rate-Bescheid (die slot_fn nähme sonst still
        # nur die Töpfe und verschluckte das Aggregat).
        if _positiv(KAP_ERTRAEGE) and any(_positiv(t) for t in KAP_TOEPFE):
            return "kapital_semantik_offen"
        # Person B (#4b): dieselbe Single-source-Konsistenz für das Ehegatten-Kapital.
        if (felder.get("veranlagung", {}).get("wert") == "zusammen"
                and _positiv(KAP_ERTRAEGE_PARTNER) and any(_positiv(t) for t in KAP_TOEPFE_PARTNER)):
            return "kapital_semantik_offen"
        # §§ 13-18 Gewinn-Quelle SINGLE-SOURCE (Stufe 2a, fail-closed): direkter einkuenfte_gewinn UND eine
        # EÜR-Komponente (betriebseinnahmen/sonstige_BA/AfA) beide gesetzt → welcher laufende Gewinn gilt? Doppel-
        # quelle → kein Rate-Bescheid (_laufender_gewinn nähme sonst still die EÜR und verschluckte den Direktwert).
        # Spiegel kapital_semantik_offen. Entweder den Betrag DIREKT ODER komponentenweise, nicht beides.
        # GEWINN_QUELLEN_MENGEN, NICHT EUER_KOMPONENTEN (2026-09-26): die Menge muss DIESELBE sein wie die
        # des Umschalters in _laufender_gewinn, und der schaltet auch bei `gwg_summe > 0` um. Mit der
        # engeren Menge hier fiel der Direktwert bei einer bloßen GWG-Zeile still aus der Bemessung
        # (21.063,00 EUR auf 50.000 EUR Gewinn, gemessen — s. api_constants.py und
        # tests/test_gwg_direktwert_quelle.py). Die Konstante ist die gemeinsame Wurzel; zwei Listen
        # für eine Frage waren der Defekt.
        if _positiv("einkuenfte_gewinn") and any(_positiv(k) for k in GEWINN_QUELLEN_MENGEN):
            return "gewinn_quelle_offen"
        # § 13 Land-/Forstwirtschaft ist NICHT EÜR-materialisiert (EuerGewinn-Bedingungen § 15 Abs. 2/§ 18 Abs. 1,
        # nicht § 13): gewinn_betriebsart=land_forst MIT EÜR-Komponente (und OHNE Direktwert) → luf_euer_offen,
        # fail-closed (NIE silent 0 — die EÜR gilt für LuF steuerlich anders, § 13a Durchschnittssätze etc.).
        # land_forst + Direktwert bleibt erlaubt (Stufe-1-Direktwert ist einkunftsart-agnostisch, keine EÜR-Rechnung).
        # GEWINN_QUELLEN_MENGEN wie gewinn_quelle_offen oben: auch eine bloße GWG-Zeile schaltet auf den EÜR-Weg.
        if (felder.get("gewinn_betriebsart", {}).get("wert") == "land_forst"
                and any(_positiv(k) for k in GEWINN_QUELLEN_MENGEN) and not _positiv("einkuenfte_gewinn")):
            return "luf_euer_offen"
        # § 35 GewSt-Anrechnung (S1, fail-closed): der Steuermessbetrag ist da (opt-in), aber der Hebesatz fehlt →
        # die Anrechnung min(4×MB, MB×Hebesatz, …) ist ohne Hebesatz nicht rechenbar. KEIN 4×MB-Default (der
        # über-creditete bei Hebesatz < 400 % = Under-tax) → gewst_hebesatz_offen. Kein gewst_messbetrag = kein § 35
        # (over-tax-safe opt-out, feuert NICHT). Feld-präsenz-getrieben; Scheiben ohne die Felder → _positiv=False.
        if _positiv("gewst_messbetrag") and (felder.get("gewst_hebesatz") or {}).get("zustand") != "bestaetigt":
            return "gewst_hebesatz_offen"
        # Person B (#4): bei Zusammenveranlagung braucht der Ring den vollständig BESTÄTIGTEN Person-B-
        # Kegel (Bruttolohn + IdNr) — sonst kein halber Ehepaar-Bescheid (K2). Bei einzel irrelevant.
        if cfg.get("partner_19") and felder.get("veranlagung", {}).get("wert") == "zusammen":
            if any((felder.get(pf) or {}).get("zustand") != "bestaetigt"
                   for pf in GESAMT_PARTNER_19 + GESAMT_PARTNER_KAP):
                return "partner_kegel_offen"
        # (A.2: der frühere partner_vorsorge_offen-Guard ist ENTFERNT — die Person-B-Vorsorge (VOR + KV/PV + § 24a)
        # ist jetzt im gesamt-slot_fn additiv verdrahtet, ein zusammen-Bescheid rechnet beider Ehegatten-Vorsorge
        # korrekt statt zu sperren. Der alte Schema-Grund wurde nach dem Erzeuger-Sweep entfernt.)
        # Multi-Objekt § 21 (#5): jede WEITERE vv_objekt-Instanz (index ≥ 2) muss VOLLSTÄNDIG bestätigt sein —
        # alle 5 Basis-vv-Felder present UND per-Instanz-meet == bestaetigt (instanzen-Naht). Sonst kein Σ (K2:
        # eine halbe/vorläufige Objekt-Instanz erzeugte sonst ein still zu niedriges §21-Σ). Instanz 1 = der
        # Basis-Kegel, den _feste_zahl separat prüft (input_kegel_nicht_bestaetigt) — hier nur die Zusatzobjekte.
        gruppe = cfg.get("multi_objekt")
        if gruppe and store is not None and bindung is not None:
            pflicht = frozenset(VV_GESAMT_FELDER)   # 6 Pflicht-vv-Felder je Objekt (inkl. § 21-Abs.2-Entgelt-Quote)
            for inst in EM.instanzen(store, bindung, gruppe):
                # Subset-Check (nicht ==): die optionalen §21-Abs.2-Tatbestand-Felder (wohnzwecke/auf_dauer, auch
                # vv_objekt-getaggt) dürfen zusätzlich present sein → ALLE Pflicht-Felder present genügt.
                if inst["index"] >= 2 and (
                        not pflicht <= set(inst["felder"]) or inst["zustand"] != "bestaetigt"):
                    return "vv_instanz_offen"
        # § 22 aa Rentenfreibetrag-Fixierung (K2): ab dem 2. Jahr ist der Freibetrag in EURO fix; fehlt er
        # (aa-Folgejahr, renten_beginn < VZ, kein rentenfreibetrag) → fail-closed, kein %×erhöhte-Rente.
        if cfg.get("rentner"):
            def _fixierung_offen(art, beginn, rf):
                return (art in RENTNER_AA_ARTEN and isinstance(beginn, int) and vz is not None
                        and beginn < vz and not (isinstance(rf, (int, float)) and not isinstance(rf, bool)))
            # Multi-Rente (#6): Fixierung + Vollständigkeit JE Rente-Instanz der Person A (instanzen-Naht). Eine
            # Zusatz-Rente (index≥2) braucht die 4 Kern-Felder present + per-Instanz-meet bestaetigt (rentenfrei-
            # betrag optional = nur aa-Folgejahr) — sonst rente_instanz_offen (kein still zu niedriges §22-Σ, K2).
            # index 1 = Basis-Kegel (prüft _feste_zahl). Ohne store → nur die Basis-Fixierung (Alt-Aufrufer).
            if cfg.get("multi_rente") and store is not None and bindung is not None:
                kern = frozenset(RENTNER_22)
                for inst in EM.instanzen(store, bindung, cfg["multi_rente"]):
                    fi = inst["felder"]
                    if inst["index"] >= 2 and (not kern <= set(fi) or inst["zustand"] != "bestaetigt"):
                        return "rente_instanz_offen"
                    if _fixierung_offen(fi.get("rentner_renten_art", {}).get("wert"),
                                        fi.get("rentner_renten_beginn_jahr", {}).get("wert"),
                                        fi.get("rentner_rentenfreibetrag", {}).get("wert")):
                        return "rentenfreibetrag_fixierung_offen"
            elif _fixierung_offen(felder.get("rentner_renten_art", {}).get("wert"),
                                  felder.get("rentner_renten_beginn_jahr", {}).get("wert"),
                                  felder.get("rentner_rentenfreibetrag", {}).get("wert")):
                return "rentenfreibetrag_fixierung_offen"
            # Person B, Partner-Kegel (K2, BACKLOG rentner-gesamt-partner-kegel-ungeschuetzt, messung_2):
            # rentner_gesamt/zusammen hatte KEINEN Vollständigkeits-Guard für die 28 gewired-ten Partnerfelder
            # (cfg fehlte "partner_19", der einzige zweite Fast-Treffer api.py:2076 liegt nach dem
            # unbedingten `return None` unten und ist toter Code für gesamt_guard-Scheiben). Zwei konkrete
            # Lücken, beide K2 (fail-closed, kein Rate-Bescheid statt stillem Fehl-Ergebnis):
            # (a) Renten-Gruppe: ein einzelnes rentner_renten_art_partner (ohne beginn_jahr_partner) lief
            #     ungefangen in den Fixierungs-Guard unten — _fixierung_offen(beginn=None) liefert False
            #     (kein isinstance(None, int)), der Guard griff NICHT — und crashte im Ring mit HTTP 500
            #     ("RentenfreibetragFixierungOffen"), weil der Ring das fehlende Feld intern als 0 (=
            #     aa-Folgejahr) behandelt und dort einen fixierten Freibetrag verlangt, den niemand gesetzt
            #     hat. Jetzt: entweder ALLE 4 Kernfelder (RENTNER_22_PARTNER) bestätigt (wie Person A) oder
            #     KEINS — sonst rente_instanz_offen (dieselbe Semantik wie die multi_rente-Instanz-
            #     Vollständigkeit oben, nur ohne Instanz-Achse; wandelt den Crash in einen Sperrgrund um).
            # (b) KV/PV-Weiche: versicherungsart_partner ist die Kz-VERZWEIGUNG für basis_kv_partner/
            #     basis_pv_partner (est_mapping.py PARTNER_VERZWEIGUNG: gesetzlich_an/_freiwillig/privat ->
            #     3 verschiedene Kz je Feld) — ohne sie bewegte der Ring Geld (-810/-180 EUR gemessen), ohne
            #     dass klar ist, WELCHES Kz gilt. Individuelle Vorsorge-Einzelfelder (VOR_PARTNER_FELDER,
            #     vorsorge_*_partner) bleiben absichtlich UNGEGATED — das ist der A.2-Präzedenzfall (Kommentar
            #     oben): Sonderausgaben sind je Kategorie eigenständig abzugsfähig, kein Kegel nötig.
            # NICHT hier: person_b_idnr (E0100082) ist auf rentner_gesamt noch gar nicht erreichbar (fehlt in
            # RENTNER_FELDER) — eigener, breiterer Fund, an team-lead gemeldet statt hier mitgezogen (würde
            # ~10 bestehende gruene Tests ohne idnr brechen, die dieser Auftrag nicht anfasst).
            if felder.get("veranlagung", {}).get("wert") == "zusammen":
                _rente_b_kern = frozenset(RENTNER_22_PARTNER)
                _rente_b_da = {f for f in _rente_b_kern
                               if (felder.get(f) or {}).get("zustand") == "bestaetigt"}
                if _rente_b_da and _rente_b_da != _rente_b_kern:
                    return "rente_instanz_offen"
                if ((_positiv("basis_kv_partner") or _positiv("basis_pv_partner"))
                        and (felder.get("versicherungsart_partner") or {}).get("zustand") != "bestaetigt"):
                    return "partner_kegel_offen"
            # Person B (#4b): dieselbe aa-Folgejahr-Fixierungs-Sperre für die Ehegatten-Rente bei zusammen.
            if felder.get("veranlagung", {}).get("wert") == "zusammen" and _fixierung_offen(
                    felder.get("rentner_renten_art_partner", {}).get("wert"),
                    felder.get("rentner_renten_beginn_jahr_partner", {}).get("wert"),
                    felder.get("rentner_rentenfreibetrag_partner", {}).get("wert")):
                return "rentenfreibetrag_fixierung_offen"
        # § 19 Abs. 2 Versorgungsfreibetrag (K2): Versorgungsbezüge vorhanden → beide kritischen Inputs
        # müssen gesetzt sein (Bemessungsgrundlage + Beginnjahr), sonst fail-closed.
        versorgung_jahresrente = felder.get("versorgung_jahresrente", {}).get("wert")
        if isinstance(versorgung_jahresrente, (int, float)) and not isinstance(versorgung_jahresrente, bool) and versorgung_jahresrente > 0:
            _vs_beginn_feld = felder.get("versorgung_beginn_jahr") or {}
            _vs_bmg_feld = felder.get("versorgung_bemessungsgrundlage") or {}
            versorgung_beginn = _vs_beginn_feld.get("wert")
            versorgung_bemessungsgrundlage = _vs_bmg_feld.get("wert")
            # Beide Inputs müssen BESTÄTIGT gesetzt sein; fehlt einer oder ist nur vorläufig → Sperrgrund
            # (Naht-Fix: vorher zählte der Rohwert — der Ring filtert auf bestätigt und liess die 30.000
            # EUR Versorgungsbezüge dann klanglos aus der Summe fallen statt zu sperren).
            if not (isinstance(versorgung_beginn, int) and versorgung_beginn > 0
                    and _vs_beginn_feld.get("zustand") == "bestaetigt"):
                return "versorgungsfreibetrag_offen"
            if not (isinstance(versorgung_bemessungsgrundlage, (int, float)) and not isinstance(versorgung_bemessungsgrundlage, bool)
                    and versorgung_bemessungsgrundlage > 0 and _vs_bmg_feld.get("zustand") == "bestaetigt"):
                return "versorgungsfreibetrag_offen"
        # § 33b Abs. 1 S. 1 Wahlrecht (Stufe 2b, K2): kein over-tax-sicherer Default möglich
        # (Bauanleitung Frage C: kleiner Aufwand -> PB zu hoch, großer Aufwand -> PB zu niedrig) ->
        # der Nutzer MUSS antworten. Nur relevant, wenn (a) ein eigener GdB-PB > 0 vorliegt — bit-
        # identisch zu runner.catala_behinderten_pb (golden/runner.py: hilflos/blind/taubblind ->
        # Höchstbetrag, sonst GdB < 20 -> 0), (b) behinderungsbedingte Aufwendungen > 0 sind, UND
        # (c) Abs. 5 S. 4 (Kind-PB-Übertragung) nicht schon automatisch kürzt — dann hat die Frage
        # keinen Sinn (Reihenfolge: Abs. 5 vor Abs. 1). Sonst NIE fragen/sperren (Gate-Polarität,
        # 519199e: der Normalfall ohne GdB oder ohne Aufwendungen bleibt unberührt).
        def _kind_pb_uebertragen():
            if store is None or bindung is None:
                return False
            for inst in EM.instanzen(store, bindung, "kind"):
                if inst["zustand"] != "bestaetigt":
                    continue
                idnr = inst["felder"].get("kind_idnr", {}).get("wert")
                if not idnr or not isinstance(idnr, str) or len(idnr) < 11:
                    continue
                antrag = inst["felder"].get("kind_behinderten_pb_antrag", {}).get("wert") is True
                nicht_selbst = inst["felder"].get("kind_pb_nicht_selbst_genutzt", {}).get("wert") is True
                if antrag and nicht_selbst:
                    return True
            return False
        _gdb = felder.get("rentner_grad_der_behinderung", {}).get("wert")
        _gdb_num = _gdb if isinstance(_gdb, (int, float)) and not isinstance(_gdb, bool) else 0
        eigener_pb_vorhanden = (_gdb_num >= 20
                                 or felder.get("rentner_hilflos_blind_taubblind", {}).get("wert") is True)
        if eigener_pb_vorhanden and _positiv("behinderungsbedingte_aufwendungen") and not _kind_pb_uebertragen():
            if (felder.get("behinderungsbedingte_aufwendungen_wahlrecht_pb") or {}).get("zustand") != "bestaetigt":
                return "behinderungsbedingte_aufwendungen_wahlrecht_offen"
        # Partner-Spiegel (K2, BACKLOG p33b-partner-pb-doppelabzug): derselbe Sperrgrund für den
        # Partner-Pauschbetrag, der bisher unconditional additiv lief (api.py-Aufbaustellen
        # gesamt/rentner_gesamt) OHNE Wahlrecht-Prüfung — 1.168-1.234 EUR stiller Doppelabzug.
        # Selbständig von Person A's Block (unabhängige Wahlrechte, jede Kombination möglich),
        # daher eigenständiges if, kein elif. Nur zusammen — sonst ist der Normalfall ohne
        # Partner-GdB/-Aufwendungen unberührt (Gate-Polarität, 519199e-Präzedenz). 8-Space-Ebene
        # bewusst NICHT im cfg.get("rentner")-Block oben (partner_19-Analyse api.py:2058-2072) —
        # muss auf "gesamt" UND "rentner_gesamt" gleichermaßen feuern.
        if felder.get("veranlagung", {}).get("wert") == "zusammen":
            _gdb_partner = felder.get("rentner_grad_der_behinderung_partner", {}).get("wert")
            _gdb_partner_num = _gdb_partner if isinstance(_gdb_partner, (int, float)) and not isinstance(_gdb_partner, bool) else 0
            partner_pb_vorhanden = (_gdb_partner_num >= 20
                                     or felder.get("rentner_hilflos_blind_taubblind_partner", {}).get("wert") is True)
            if partner_pb_vorhanden and _positiv("behinderungsbedingte_aufwendungen_partner"):
                if (felder.get("behinderungsbedingte_aufwendungen_wahlrecht_pb_partner") or {}).get("zustand") != "bestaetigt":
                    return "behinderungsbedingte_aufwendungen_wahlrecht_partner_offen"
        # § 35a Einzelaufstellung (Anlass 2026-08-10): hh_dienstleistungen/hh_handwerker_arbeitskosten
        # sind seit dem Fix askable:false — kein Schreiber setzt sie mehr direkt, _positiv(<sum_fid>)
        # sähe hier NIE wieder einen Wert (permanent stumm, gleiche Fehlerklasse wie ein fail-open
        # get(name,0), nur am Guard statt am Ring). Ersatz: INSTANZ-basiert, wie _kind_pb_uebertragen
        # oben — "vorläufig ODER bestätigt" (irgendeine Instanz, jeder zustand), 1:1 zu _positiv's
        # eigener Semantik (Zeile 1731), NUR die Datenquelle wechselt von Skalar auf instanz_gruppe.
        def _hh_instanz_positiv(gruppe, betrag_fid, sum_fid):
            if store is None or bindung is None:
                return _positiv(betrag_fid)
            for inst in EM.instanzen(store, bindung, gruppe):
                v = inst["felder"].get(betrag_fid, {}).get("wert")
                if isinstance(v, (int, float)) and not isinstance(v, bool) and v > 0:
                    return True
            # Bestandsdaten-Fallback (1:1 zu _hh_summe in _shared_steuer_sonder_agb, api.py:363):
            # ohne Instanz auf den alten Flat-Wert zurückfallen. Sonst griffe dieser Guard für
            # einen Bestandsfall NIE — rechnung_unbar_offen/handwerker_foerderung_offen sperrten
            # dann nicht, obwohl der Ring (mit dem Fallback dort) den Flat-Betrag längst mitrechnet.
            return _positiv(sum_fid)
        # § 35a Abs. 5 S. 3 rechnung_unbar = CONDITIONAL-MANDATORY (K2, charge29): NUR wenn Dienstleistung
        # ODER Handwerker (Abs. 2/3) > 0 — Minijob (Abs. 1) verlangt keine unbare Zahlung. Unbeantwortet
        # (nicht bestätigt) → rechnung_unbar_offen (kein Abs2/3-Abzug ohne Beleg-/Überweisungsnachweis);
        # explizit false ist ANTWORT (Ring rechenbar, die slot_fn nullt Abs. 2/3), nur UNSET sperrt.
        # Feld-präsenz-getrieben (gilt für JEDE gesamt_guard-Scheibe, die diese Felder führt — haushalt/agb UND
        # der gefaltete gesamt-Ring, Weg ii). Scheiben ohne die Felder: _positiv/_num liefern absent→False/0.
        if (_hh_instanz_positiv("hh_dienstleistung", "hh_dienstleistung_betrag", "hh_dienstleistungen")
                or _hh_instanz_positiv("hh_handwerker", "hh_handwerker_betrag", "hh_handwerker_arbeitskosten")):
            if (felder.get("hh_rechnung_unbar") or {}).get("zustand") != "bestaetigt":
                return "rechnung_unbar_offen"
        # § 35a Abs. 3 S. 2: öffentlich geförderte Handwerkermaßnahmen (zinsverbilligtes Darlehen
        # oder steuerfreier Zuschuss) → hh_handwerker_keine_foerderung CONDITIONAL-MANDATORY
        # (nur wenn Handwerker > 0). Unbeantwortet → handwerker_foerderung_offen (Abs. 3 unhaltbar).
        if _hh_instanz_positiv("hh_handwerker", "hh_handwerker_betrag", "hh_handwerker_arbeitskosten"):
            if (felder.get("hh_handwerker_keine_foerderung") or {}).get("zustand") != "bestaetigt":
                return "handwerker_foerderung_offen"
        # § 35a Abs. 4 S. 1: der Haushalt muss in der EU oder im EWR liegen. Anders als Abs. 3 S. 2
        # (Zeile darüber) gatet diese Voraussetzung ALLE DREI Toepfe — Abs. 1 (Minijob) genauso wie
        # Abs. 2/3. Deshalb ist die Vorbedingung hier "irgendein § 35a-Aufwand > 0", nicht "Handwerker
        # > 0": ein reiner Minijob-Fall haengt an derselben unbeantworteten Voraussetzung.
        #
        # Gemessen 2026-09-26 (vorher): unbeantwortetes hh_in_eu_ewr -> grund="bestaetigt", 5.000 EUR
        # Handwerker ohne Abzug. Der Fall war bitgleich mit einem, der § 35a nie erwaehnt — der Ring
        # liest `is True` (runner.catala_p35a_haushaltsnahe) und nullt bei unset ALLE Toepfe, ohne
        # Sperre und ohne Hinweis.
        #
        # Was die stille Einbusse IST, gemessen statt erschlossen (die Einheit macht den Satz sonst
        # mehrdeutig — `kette` fuehrt EUR, `zahl_cent` fuehrt Cent): Δ zahl_cent 1067800 -> 967800
        # = 100000 ct = 1.000 EUR = 20 % von 5.000 EUR, also der § 35a-ABZUG SELBST. Er sitzt 1:1
        # auf der FESTZUSETZENDEN Steuer (Δ 10678 -> 9678 EUR), waehrend zvE (49964) und tarifliche
        # ESt (10678) unveraendert bleiben — § 35a ist eine Steuerermaessigung, keine
        # Sonderausgabe. Kalibriert an 3.000 EUR Handwerker: Δ 60000 ct = 600 EUR, dasselbe
        # 20-%-Verhaeltnis. (bau-zweiges Kegel trug zusaetzlich 90 EUR KiSt; diese Kegel fuehren
        # keine KiSt — die 90 EUR gehoeren nicht in dieselbe Messung.)
        #
        # Fail-closed bei WERTEN ([[bedingungsfeld-selbst-versteckt]]): die Anzeige ist der Schaden,
        # nicht die Zahl — "bestaetigt" ueber einer Rechnung, die den Abzug nicht enthaelt.
        # Explizit FALSE ist dagegen eine ANTWORT: Abs. 4 S. 1 SCHLIESST die Ermäßigung dann aus, sie
        # ist rechenbar 0 -> der Ring rechnet, der Guard sperrt nicht. Dieselbe Polaritaet wie beim
        # Geschwisterfeld (keine_foerderung=false nullt Abs. 3 und bleibt bestaetigt), kein zweiter
        # Rechenweg. Nur UNSET/unbestaetigt sperrt.
        if (_hh_instanz_positiv("hh_minijob", "hh_minijob_betrag", "hh_minijob_aufwendungen")
                or _hh_instanz_positiv("hh_dienstleistung", "hh_dienstleistung_betrag", "hh_dienstleistungen")
                or _hh_instanz_positiv("hh_handwerker", "hh_handwerker_betrag", "hh_handwerker_arbeitskosten")):
            if (felder.get("hh_in_eu_ewr") or {}).get("zustand") != "bestaetigt":
                return "haushalt_eu_ewr_offen"
        # § 35c Abs. 3 S. 2 (Zwilling des Guards darüber): die Ermäßigung entfällt GANZ, wenn für
        # dieselben energetischen Maßnahmen § 10f oder § 35a in Anspruch genommen wird oder eine
        # öffentliche Förderung vorliegt. Conditional-mandatory wie bei § 35a — nur wenn § 35c-
        # Aufwendungen erklärt sind. Unbeantwortet sperrt: ohne die Antwort lässt sich nicht
        # sagen, ob die Ermäßigung überhaupt zusteht, und ein stiller Abzug wäre Under-tax
        # (gemessen 2026-08-16: 1.200 EUR zu wenig Steuer bei identischem Betrag in beiden Töpfen).
        if _positiv("p35c_sanierungsaufwendungen") or _positiv("p35c_energieberater_aufwendungen"):
            if (felder.get("p35c_keine_doppelfoerderung") or {}).get("zustand") != "bestaetigt":
                return "p35c_doppelfoerderung_offen"
        # § 6 Abs. 2 GWG-Sofortabzug S. 1-5: die drei Anspruchsvoraussetzungen (selbständig nutzbar S.2/3,
        # netto vorsteuerbereinigt S.1, ab 250 EUR Verzeichnis/Buchführung S.4/5) sind CONDITIONAL-MANDATORY
        # je gwg-Instanz -- nur wenn die Instanz überhaupt einen Betrag > 0 trägt (analog
        # _hh_instanz_positiv oben). Unbeantwortet (nicht bestätigt) sperrt den GANZEN Ring; explizit false
        # ist ANTWORT (Ring rechenbar, _gwg_sofortabzug_summe nullt genau diese Instanz), nur UNSET/vorläufig
        # sperrt. instanzweise wie EM.instanzen(gwg) liefert -- vor Schritt 1 (2026-09-07) waren die drei
        # Bool-Felder unerreichbar, es gibt daher keine Altantworten, die eine neue Sperre stumm auslöst.
        if store is not None and bindung is not None:
            for _inst in EM.instanzen(store, bindung, "gwg"):
                _netto_v = _inst["felder"].get("gwg_anschaffungskosten_netto", {}).get("wert")
                _netto_i = _netto_v if isinstance(_netto_v, (int, float)) and not isinstance(_netto_v, bool) else 0
                if _netto_i <= 0:
                    continue
                # § 6 Abs. 2 S. 1: über 800 EUR netto ist der Sofortabzug strukturell ausgeschlossen
                # (zwingend AfA) -- die drei Tatbestandsfragen sind für DIESES Wirtschaftsgut gegenstandslos,
                # unbeantwortet darf nicht sperren. Spiegelt den > 80000-Cent-Guard in _gwg_sofortabzug_summe
                # (bescheid_einkuenfte.py), sonst fragt die Sperre nach Voraussetzungen, die am Betrag längst
                # gescheitert sind (gemessen 2026-09-07: 1000-EUR-Instanz sperrte die Abgabe grundlos).
                if _netto_i > 80000:
                    continue
                if any((_inst["felder"].get(fb) or {}).get("zustand") != "bestaetigt"
                       for fb in ("gwg_bewegliches_selbstaendig_nutzbar", "gwg_netto_ohne_vorsteuer")):
                    return "gwg_tatbestand_offen"
                # S. 4: Verzeichnispflicht nur > 250 EUR netto -- darunter ist die Frage rechtlich
                # gegenstandslos, unbeantwortet darf hier nicht sperren.
                if (_netto_i > 25000
                        and (_inst["felder"].get("gwg_verzeichnis_ab_250") or {}).get("zustand") != "bestaetigt"):
                    return "gwg_tatbestand_offen"
        # § 10 Abs. 1 Nr. 5 S. 2 + S. 4 Kinderbetreuung: Nachhilfe/Unterricht/Sport (S. 2) und
        # Barzahlung (S. 4) schliessen den Abzug aus. CONDITIONAL-MANDATORY je Kind-Instanz und
        # nur bei Betrag > 0 (analog GWG oben) — ohne Betrag gibt es nichts abzuziehen, dann sind
        # beide Fragen gegenstandslos und duerfen die Abgabe nicht sperren.
        # Anders als bei hh_rechnung_unbar (§ 35a) nullt hier NICHTS eine Teilquote: der Abzug
        # kennt keine Aufteilung, ein gemischter Betrag wird nicht zerlegt (s. ponytail in der
        # Bindungstabelle). Deshalb sperrt auch ein BESTAETIGTES "nein" — der Nutzer muss den
        # Betrag selbst berichtigen. Ein stiller Abzug waere Under-tax (gemessen 2026-09-26:
        # 6.000 EUR Nachhilfe -> 4.800 EUR Abzug), eine stille Null ein Geldverlust ohne Hinweis.
        # Ein Kind, das die Qualifikation aus S. 1 nicht traegt (ueber 14), zaehlt ohnehin nicht
        # mit — seine Antworten duerfen nicht sperren, sonst haengt der Ring an einer Instanz,
        # die _kinderbetreuung_summe selbst schon aussortiert.
        if store is not None and bindung is not None:
            for _inst in EM.instanzen(store, bindung, "kind"):
                _aufw_v = _inst["felder"].get("kinderbetreuungskosten", {}).get("wert")
                _aufw_i = _aufw_v if isinstance(_aufw_v, (int, float)) and not isinstance(_aufw_v, bool) else 0
                if _aufw_i <= 0:
                    continue
                # Nur ein Kind, das die Qualifikation aus S. 1 BESTAETIGT traegt, erreicht den Ring
                # ueberhaupt — _kinderbetreuung_summe zaehlt ausschliesslich `is True`. Unbeantwortet
                # oder false heisst: das Kind faellt aus der Summe, sein Betrag wird nirgends
                # abgezogen, und die beiden Fragen nach S. 2/S. 4 sind fuer DIESES Kind
                # gegenstandslos. Wuerde die Sperre hier feuern, verlangte das Programm Auskuenfte
                # zu einem Abzug, den es selbst nicht gewaehrt (dieselbe Regel wie beim GWG ueber
                # 800 EUR) — und der Nutzer saehe zwei Fragen, die nichts aendern koennen.
                if (_inst["felder"].get("kind_unter_14_haushaltszugehoerig") or {}).get("wert") is not True:
                    continue
                # Zwei getrennte Gruende, nicht eine UND-Kette mit einem Sammelgrund: der Nutzer
                # muss wissen, WAS er korrigieren soll — den Betrag aufteilen (S. 2) oder die
                # Zahlungsart (S. 4). Beide Bedingungen stehen einzeln, damit die zweite nicht
                # wirkungslos wird, wenn die erste schon sperrt.
                _reine = _inst["felder"].get("kind_betreuung_reine_betreuung") or {}
                if _reine.get("zustand") != "bestaetigt" or _reine.get("wert") is not True:
                    return "kinderbetreuung_reine_betreuung_offen"
                _zahlung = _inst["felder"].get("kind_betreuung_rechnung_ueberweisung") or {}
                if _zahlung.get("zustand") != "bestaetigt" or _zahlung.get("wert") is not True:
                    return "kinderbetreuung_zahlung_offen"
        # § 10 Abs. 4b KiSt-Erstattungsüberhang: früher sperrte hier erstattungsueberhang_offen,
        # weil die GdE-Hinzurechnung (S. 3) fehlte und ein stiller Abzug 0 unterbesteuert hätte.
        # Sie ist jetzt gebaut (catala_p10_4b_erstattungsueberhang, im Ring vor den GdE-Verwendungen
        # verdrahtet) — der Fall rechnet. Der alte Schema-Grund wurde nach dem Erzeuger-Sweep entfernt.
        # fremd_arten = Arten, die DIESE Scheibe NICHT rechnet → bestätigt-false (Nutzer HAT die Art) sperrt
        # (Stufe 2). Die von der Scheibe GERECHNETEN Arten stehen NICHT in fremd_arten (kein Fehl-Sperr).
        if any(felder.get(fl, {}).get("wert") is False for fl in cfg.get("fremd_arten", ())):
            return "einkunftsart_nicht_ring_faehig"
        # dHf/Verpflegung sind seit B1 auch im gesamt/rentner-WK-Pfad (catala_werbungskosten_n) verdrahtet →
        # dieselbe fail-closed-Sperre wie an_gesamt (der frühe return None unten würde sie sonst überspringen).
        _dvg = _dhf_vpf_grund()
        if _dvg:
            return _dvg
        return None
    # dHf-Tatbestand + Verpflegungs-Reduktion + Übernachtung + Arbeitsmittel-GWG (§ 9 Abs. 1 Nr. 5/6/7 / Abs. 4a):
    # fail-closed bei Ausland / offener Geltungsbedingung / offener Reduktion / AM > 800 (AfA). Non-gesamt-Pfad
    # (an_gesamt catala_est) — die AM-Sperre (früher GUARD_WERBUNGSKOSTEN) sitzt jetzt in _dhf_vpf_grund (beide Pfade).
    _dvg = _dhf_vpf_grund()
    if _dvg:
        return _dvg
    # Zusammenveranlagung: der Splitting-Ring braucht den vollständigen Kegel BEIDER Personen.
    if felder.get("veranlagung", {}).get("wert") == "zusammen":
        if any((felder.get(pf) or {}).get("zustand") != "bestaetigt" for pf in AN_GESAMT_PARTNER):
            return "partner_kegel_offen"        # Person-B-Pflichtfeld offen → kein halber Bescheid
        if any(_positiv(vf) for vf in VOR_FELDER + VOR_PARTNER_FELDER):
            return "partner_vor_offen"           # MVP-zusammen ohne VOR; VOR-Feld (A/B) gesetzt sperrt
    if any(felder.get(f, {}).get("wert") is False for f in AN_GESAMT_FLAGS):
        return "einkunftsart_nicht_ring_faehig"
    return None
