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
             "produkt/mapping", "produkt/konsistenz", "produkt/import", "produkt/engine", "golden", "elster"):
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
    EUER_KOMPONENTEN,
    GESAMT_PARTNER_19,
    GESAMT_PARTNER_KAP,
    KAP_ERTRAEGE,
    KAP_ERTRAEGE_PARTNER,
    KAP_TOEPFE,
    KAP_TOEPFE_PARTNER,
    MITU_FELDER,
    RENTNER_22,
    RENTNER_22_PARTNER,
    RENTNER_AA_ARTEN,
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
    Abs. 6 S. 4) erklärt wird. Beide Kz sind ZUSAMMEN Pflicht (ohne
    E1901401 bleibt rc=610001002 trotz Antrag, siehe
    bindung_kap_vv_familie.yaml) — deshalb ein gemeinsames Gate.
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
        felder["kap_antrag_guenstigerpruefung"] = {
            "wert": True,
            "zustand": "bestaetigt",
            "herkunft": {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"},
            "schreiber": "engine",
            "signal": {"signal_1": None, "signal_2": None},
        }
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
            pb_genutzt_cent = 0
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
            if felder.get("dhf_im_inland", {}).get("wert") is False:
                return "ausland_dhf_nicht_ring_faehig"
            if any((felder.get(b) or {}).get("zustand") != "bestaetigt" for b in DHF_BEDINGUNGEN):
                return "dhf_tatbestand_offen"
        if sum((felder.get(t, {}).get("wert") or 0) for t in VERPFLEGUNG_TAGE) > 0:
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
        # Übernachtung Auswärtstätigkeit (§ 9 Abs. 1 Nr. 5a): Kosten > 0 → Ring nur fähig bei Inland,
        # allen 3 Tatbestands-Bedingungen bestätigt UND ohne 48-Monats-Schwellenübertritt. Ausland /
        # offener Tatbestand (inkl. UNSET Inland, fail-closed) / überspannender Zeitraum sperren.
        if _positiv(UEBERNACHTUNG_KOSTEN):
            if felder.get("uebernachtung_im_inland", {}).get("wert") is False:
                return "ausland_uebernachtung_nicht_ring_faehig"
            if (felder.get("uebernachtung_im_inland", {}).get("wert") is not True
                    or any((felder.get(b) or {}).get("zustand") != "bestaetigt" for b in UEBERNACHTUNG_BEDINGUNGEN)):
                return "uebernachtung_tatbestand_offen"
            bisher = felder.get("uebernachtung_monate_bisher", {}).get("wert")
            monate = felder.get("uebernachtung_monate", {}).get("wert")
            if (isinstance(bisher, int) and not isinstance(bisher, bool)
                    and isinstance(monate, int) and not isinstance(monate, bool)
                    and bisher < 48 < bisher + monate):
                return "uebernachtung_zeitraum_offen"
        # Arbeitsmittel (§ 9 Abs. 1 Nr. 6/7 i.V.m. § 6 Abs. 2 GWG / § 7 AfA): AK > 0 → Ring nur fähig für den
        # GWG-Sofortabzug (AK ≤ 800 EUR mit ausgeübtem Wahlrecht). AK > 800 → mehrjährige § 7-AfA (A6-L2),
        # die ring-fähig ist sobald die Nutzungsdauer gesetzt ist (arbeitsmittel_nutzungsdauer > 0).
        # Schwelle in CENT (80000): 800,01 EUR floort sonst fälschlich auf 800 (Under-tax).
        _am = felder.get(ARBEITSMITTEL_KOSTEN, {}).get("wert")
        if isinstance(_am, (int, float)) and not isinstance(_am, bool) and _am > 0:
            if _am <= 80000:
                if felder.get("am_gwg_sofortabzug_gewaehlt", {}).get("wert") is not True:
                    return "arbeitsmittel_afa_ueber_gwg_offen"
            else:  # _am > 80000 → § 7 Abs. 1 lineare AfA
                nd = felder.get("arbeitsmittel_nutzungsdauer", {}).get("wert")
                monat = felder.get("am_anschaffung_monat", {}).get("wert")
                ist_aj = felder.get("am_afa_ist_anschaffungsjahr", {}).get("wert")
                # § 7 Abs. 1: Nutzungsdauer MUSS beantwortet sein (fail-closed).
                # Anschaffungsmonat + Zustand-Flag: nur wenn Anschaffungsjahr=true.
                # Flag unbeantwortet → Folgejahr angenommen (voller Jahresbetrag, Monat egal).
                # Grund: S. 4 Zwölftelung gilt NUR im Anschaffungsjahr; Folgejahre voller Betrag.
                if not isinstance(nd, int) or isinstance(nd, bool) or nd <= 0:
                    return "arbeitsmittel_afa_ueber_gwg_offen"
                # Wenn Anschaffungsjahr=true: Monat MUSS beantwortet sein
                if ist_aj is True:
                    if (not isinstance(monat, int) or isinstance(monat, bool) or monat < 1 or monat > 12):
                        return "arbeitsmittel_afa_ueber_gwg_offen"
                # ist_aj == false oder None → Folgejahr/unbeantwortet: voller Jahresbetrag, OK
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
            # 2. §35 Gewerbesteuer
            if _positiv("gewst_messbetrag"):
                return "p32b_kombi_offen"
            # 3. §34c DBA-Anrechnung
            if _positiv("dba_gezahlte_auslaendische_steuer") or _positiv("dba_auslaendische_einkuenfte"):
                return "p32b_kombi_offen"
        # § 16 Abs. 4 Freibetrag (fail-closed): Veräußerungsgewinn > 0 erfordert bestätigt
        # alter_55_oder_berufsunfaehig=True UND freibetrag_erstmalig=True (§ 16 Abs. 4 S. 1+2).
        # Fehlt eine der Bedingungen oder ist explizit false → kein FB (over-tax-safe).
        vg = (felder.get("rentner_veraeusserungsgewinn") or {}).get("wert")
        if isinstance(vg, (int, float)) and not isinstance(vg, bool) and vg > 0:
            if not (felder.get("rentner_alter_55_oder_berufsunfaehig", {}).get("wert") is True
                    and felder.get("rentner_freibetrag_erstmalig", {}).get("wert") is True):
                return "p16_4_gate_offen"
        # Dasselbe für den Ehegatten (Stufe 2 der Partnerachse, 2026-08-13). Ohne diesen Spiegel
        # gewährte _gewinn_partner_anteil dem Partner-vg den Freibetrag, OHNE dass die Abs. 4-
        # Bedingungen je geprüft wurden — under-tax. Nur bei Zusammenveranlagung: bei
        # Einzelveranlagung rechnet der Ring den Partner-vg gar nicht, ein dort stehender Wert
        # darf den eigenen Bescheid deshalb auch nicht sperren.
        if felder.get("veranlagung", {}).get("wert") == "zusammen":
            vg_p = (felder.get("rentner_veraeusserungsgewinn_partner") or {}).get("wert")
            if isinstance(vg_p, (int, float)) and not isinstance(vg_p, bool) and vg_p > 0:
                if not (felder.get("rentner_alter_55_oder_berufsunfaehig_partner", {}).get("wert") is True
                        and felder.get("rentner_freibetrag_erstmalig_partner", {}).get("wert") is True):
                    return "p16_4_gate_offen"
        # Gesamt-Ring: Flag↔Einkunftsart-Widerspruch (kein_X=true + echtes Feld > 0 bestätigt) surfacen —
        # K2, keine still übergangene Einkunftsart (dev-2s flag_check).
        if FC.flag_widersprueche(felder):
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
        if _positiv("einkuenfte_gewinn") and any(_positiv(k) for k in EUER_KOMPONENTEN):
            return "gewinn_quelle_offen"
        # § 13 Land-/Forstwirtschaft ist NICHT EÜR-materialisiert (EuerGewinn-Bedingungen § 15 Abs. 2/§ 18 Abs. 1,
        # nicht § 13): gewinn_betriebsart=land_forst MIT EÜR-Komponente (und OHNE Direktwert) → luf_euer_offen,
        # fail-closed (NIE silent 0 — die EÜR gilt für LuF steuerlich anders, § 13a Durchschnittssätze etc.).
        # land_forst + Direktwert bleibt erlaubt (Stufe-1-Direktwert ist einkunftsart-agnostisch, keine EÜR-Rechnung).
        if (felder.get("gewinn_betriebsart", {}).get("wert") == "land_forst"
                and any(_positiv(k) for k in EUER_KOMPONENTEN) and not _positiv("einkuenfte_gewinn")):
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
        # korrekt statt zu sperren. partner_vorsorge_offen bleibt im Schema-Enum als Alt-Grund erhalten, feuert nicht.)
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
            versorgung_beginn = felder.get("versorgung_beginn_jahr", {}).get("wert")
            versorgung_bemessungsgrundlage = felder.get("versorgung_bemessungsgrundlage", {}).get("wert")
            # Beide Inputs müssen gesetzt sein; fehlt einer → Sperrgrund (Accessor kann nicht rechnen).
            if not (isinstance(versorgung_beginn, int) and versorgung_beginn > 0):
                return "versorgungsfreibetrag_offen"
            if not (isinstance(versorgung_bemessungsgrundlage, (int, float)) and not isinstance(versorgung_bemessungsgrundlage, bool) and versorgung_bemessungsgrundlage > 0):
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
        # § 35c Abs. 3 S. 2 (Zwilling des Guards darüber): die Ermäßigung entfällt GANZ, wenn für
        # dieselben energetischen Maßnahmen § 10f oder § 35a in Anspruch genommen wird oder eine
        # öffentliche Förderung vorliegt. Conditional-mandatory wie bei § 35a — nur wenn § 35c-
        # Aufwendungen erklärt sind. Unbeantwortet sperrt: ohne die Antwort lässt sich nicht
        # sagen, ob die Ermäßigung überhaupt zusteht, und ein stiller Abzug wäre Under-tax
        # (gemessen 2026-08-16: 1.200 EUR zu wenig Steuer bei identischem Betrag in beiden Töpfen).
        if _positiv("p35c_sanierungsaufwendungen") or _positiv("p35c_energieberater_aufwendungen"):
            if (felder.get("p35c_keine_doppelfoerderung") or {}).get("zustand") != "bestaetigt":
                return "p35c_doppelfoerderung_offen"
        # § 10 Abs. 4b KiSt-Erstattungsüberhang: früher sperrte hier erstattungsueberhang_offen,
        # weil die GdE-Hinzurechnung (S. 3) fehlte und ein stiller Abzug 0 unterbesteuert hätte.
        # Sie ist jetzt gebaut (catala_p10_4b_erstattungsueberhang, im Ring vor den GdE-Verwendungen
        # verdrahtet) — der Fall rechnet. erstattungsueberhang_offen bleibt im Schema-Enum als
        # Alt-Grund erhalten, feuert aber nicht mehr.
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
