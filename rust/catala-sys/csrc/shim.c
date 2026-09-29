/* rust/catala-sys/csrc/shim.c — flat C ABI over the Catala-generated Einkommensteuertarif
 * scopes (grundtarif, splittingtarif, festzusetzende_est_einzel — the tariff-level scopes
 * needed for the Schritt-1 parity demo, REWRITE_PLAN.md §7).
 *
 * Every entry point runs its scope under catala_do(); a Catala assertion failure longjmps
 * back into catala_do and never crosses into a Rust frame (Audit C Teil 2.4). Inputs travel
 * via thread-local `_in` structs because catala_do() takes a nullary function.
 *
 * Defensive bounds check on vz_code even though the Rust side makes an invalid code
 * unrepresentable (a typed `Vz` enum): a generated `switch` on an out-of-range enum code
 * calls abort() (Audit C Teil 2.3 Schritt F), so this is belt-and-suspenders against a future
 * caller that skips the typed wrapper.
 */
#include <catala_runtime.h>
#include "Einkommensteuertarif.h"
#include "SpendenAbzug.h"
#include "ZumutbareBelastung.h"
#include "AgbAbzug.h"
#include "Kirchensteuerabzug.h"
#include "Altersentlastungsbetrag.h"
#include "Entlastungsbetrag.h"
#include "Familienleistungsausgleich.h"
#include "VerbilligteVermietungWk.h"
#include "KrankenPflegeVorsorge.h"
#include "Berufsausbildungsaufwendungen.h"
#include "BetriebsFreibetrag.h"
#include "EuerGewinn.h"
#include "MitunternehmerEinkuenfte.h"
#include "GwgSofortabzug.h"
#include "Verlustvortrag.h"
#include "ErmaessigterDurchschnittssatz.h"
#include "Entfernungspauschale.h"
#include "Arbeitszimmer_homeoffice.h"

#define TG_OK 0
#define TG_ERR_CATALA 1
#define TG_ERR_VZ 2

static int tg_vz(int code, Einkommensteuertarif__Veranlagungszeitraum *out) {
  if (code < 0 || code > 2) {
    return TG_ERR_VZ;
  }
  out->code = (enum Einkommensteuertarif__Veranlagungszeitraum__code)code;
  return TG_OK;
}

static __thread Einkommensteuertarif__Grundtarif_in tg_grundtarif_in;

static void *tg_run_grundtarif(void) {
  return (void *)Einkommensteuertarif__grundtarif(&tg_grundtarif_in);
}

int tg_grundtarif(long zve_cents, int vz_code, long *out_cents) {
  Einkommensteuertarif__Veranlagungszeitraum vz;
  int rc = tg_vz(vz_code, &vz);
  if (rc != TG_OK) {
    return rc;
  }
  catala_init();
  tg_grundtarif_in.Einkommensteuertarif__zu_versteuerndes_einkommen_in =
      catala_new_money(zve_cents);
  tg_grundtarif_in.Einkommensteuertarif__veranlagungszeitraum_in = &vz;
  const Einkommensteuertarif__Grundtarif *r = catala_do(tg_run_grundtarif);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->Einkommensteuertarif__tarifliche_steuer);
  catala_free_all();
  return TG_OK;
}

static __thread Einkommensteuertarif__Splittingtarif_in tg_splittingtarif_in;

static void *tg_run_splittingtarif(void) {
  return (void *)Einkommensteuertarif__splittingtarif(&tg_splittingtarif_in);
}

int tg_splittingtarif(long zve_gemeinsam_cents, int vz_code, long *out_cents) {
  Einkommensteuertarif__Veranlagungszeitraum vz;
  int rc = tg_vz(vz_code, &vz);
  if (rc != TG_OK) {
    return rc;
  }
  catala_init();
  tg_splittingtarif_in.Einkommensteuertarif__zu_versteuerndes_einkommen_gemeinsam_in =
      catala_new_money(zve_gemeinsam_cents);
  tg_splittingtarif_in.Einkommensteuertarif__veranlagungszeitraum_in = &vz;
  const Einkommensteuertarif__Splittingtarif *r = catala_do(tg_run_splittingtarif);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->Einkommensteuertarif__tarifliche_steuer);
  catala_free_all();
  return TG_OK;
}

static __thread Einkommensteuertarif__FestzusetzendeEstEinzel_in tg_fee_in;

static void *tg_run_fee(void) {
  return (void *)Einkommensteuertarif__festzusetzende_est_einzel(&tg_fee_in);
}

int tg_festzusetzende_est_einzel(long bruttoarbeitslohn_cents, long werbungskosten_cents,
                                  long sonderausgaben_cents, int vz_code, long *out_cents) {
  Einkommensteuertarif__Veranlagungszeitraum vz;
  int rc = tg_vz(vz_code, &vz);
  if (rc != TG_OK) {
    return rc;
  }
  catala_init();
  tg_fee_in.Einkommensteuertarif__bruttoarbeitslohn_in = catala_new_money(bruttoarbeitslohn_cents);
  tg_fee_in.Einkommensteuertarif__werbungskosten_in = catala_new_money(werbungskosten_cents);
  tg_fee_in.Einkommensteuertarif__sonderausgaben_in = catala_new_money(sonderausgaben_cents);
  tg_fee_in.Einkommensteuertarif__veranlagungszeitraum_in = &vz;
  const Einkommensteuertarif__FestzusetzendeEstEinzel *r = catala_do(tg_run_fee);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->Einkommensteuertarif__festzusetzende_est);
  catala_free_all();
  return TG_OK;
}

/* -- Schritt 4a Teil A: die uebrigen Catala-rufenden Scopes aus runner.py -----------------
 * Jedes Modul unten folgt demselben Muster wie oben: thread-lokaler `_in`, nullary Run-
 * Funktion, oeffentlicher `tg_*`-Wrapper, der `catala_do()` einmal aufruft und NULL (Catala-
 * Assertion) von einem gueltigen Ergebnis unterscheidet. Kein `tg_vz`-Bounds-Check hier, weil
 * diese Scopes keinen `Veranlagungszeitraum` entgegennehmen (anders als die Tarif-Familie).
 */

static __thread SpendenAbzug__SpendenAbzug_in tg_spenden_in;

static void *tg_run_spenden(void) {
  return (void *)SpendenAbzug__spenden_abzug(&tg_spenden_in);
}

int tg_spenden_abzug(long zuwendungen_cents, long gde_cents, long *out_cents) {
  catala_init();
  tg_spenden_in.SpendenAbzug__zuwendungen_in = catala_new_money(zuwendungen_cents);
  tg_spenden_in.SpendenAbzug__gesamtbetrag_der_einkuenfte_in = catala_new_money(gde_cents);
  const SpendenAbzug__SpendenAbzug *r = catala_do(tg_run_spenden);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->SpendenAbzug__spenden_abzug);
  catala_free_all();
  return TG_OK;
}

static __thread ZumutbareBelastung__ZumutbareBelastung_in tg_zumutbar_in;

static void *tg_run_zumutbar(void) {
  return (void *)ZumutbareBelastung__zumutbare_belastung(&tg_zumutbar_in);
}

int tg_zumutbare_belastung(long gde_cents, long anzahl_kinder, int splitting, long *out_cents) {
  catala_init();
  tg_zumutbar_in.ZumutbareBelastung__gesamtbetrag_der_einkuenfte_in = catala_new_money(gde_cents);
  tg_zumutbar_in.ZumutbareBelastung__anzahl_kinder_in = catala_new_int(anzahl_kinder);
  tg_zumutbar_in.ZumutbareBelastung__splitting_in = catala_new_bool(splitting);
  const ZumutbareBelastung__ZumutbareBelastung *r = catala_do(tg_run_zumutbar);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->ZumutbareBelastung__zumutbare_belastung);
  catala_free_all();
  return TG_OK;
}

static __thread AgbAbzug__AgbAbzug_in tg_agb_in;

static void *tg_run_agb(void) {
  return (void *)AgbAbzug__agb_abzug(&tg_agb_in);
}

int tg_agb_abzug(long agb_cents, long zumutbare_belastung_cents, long *out_cents) {
  catala_init();
  tg_agb_in.AgbAbzug__aussergewoehnliche_belastungen_in = catala_new_money(agb_cents);
  tg_agb_in.AgbAbzug__zumutbare_belastung_in = catala_new_money(zumutbare_belastung_cents);
  const AgbAbzug__AgbAbzug *r = catala_do(tg_run_agb);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->AgbAbzug__abzug_agb);
  catala_free_all();
  return TG_OK;
}

static __thread Kirchensteuerabzug__Kirchensteuerabzug_in tg_kist_in;

static void *tg_run_kist(void) {
  return (void *)Kirchensteuerabzug__kirchensteuerabzug(&tg_kist_in);
}

int tg_kirchensteuerabzug(long gezahlt_cents, long erstattet_cents, long *out_cents) {
  catala_init();
  tg_kist_in.Kirchensteuerabzug__gezahlte_kirchensteuer_in = catala_new_money(gezahlt_cents);
  tg_kist_in.Kirchensteuerabzug__erstattete_kirchensteuer_in = catala_new_money(erstattet_cents);
  const Kirchensteuerabzug__Kirchensteuerabzug *r = catala_do(tg_run_kist);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->Kirchensteuerabzug__abziehbare_kirchensteuer);
  catala_free_all();
  return TG_OK;
}

static __thread Altersentlastungsbetrag__Altersentlastungsbetrag_in tg_altersentlastung_in;

static void *tg_run_altersentlastung(void) {
  return (void *)Altersentlastungsbetrag__altersentlastungsbetrag(&tg_altersentlastung_in);
}

int tg_altersentlastungsbetrag(long arbeitslohn_cents, long positive_andere_cents,
                                long prozentsatz_num, unsigned long prozentsatz_den,
                                long hoechstbetrag_cents, long *out_cents) {
  catala_init();
  tg_altersentlastung_in.Altersentlastungsbetrag__arbeitslohn_in = catala_new_money(arbeitslohn_cents);
  tg_altersentlastung_in.Altersentlastungsbetrag__positive_andere_einkuenfte_in =
      catala_new_money(positive_andere_cents);
  tg_altersentlastung_in.Altersentlastungsbetrag__prozentsatz_in =
      catala_new_frac(prozentsatz_num, prozentsatz_den);
  tg_altersentlastung_in.Altersentlastungsbetrag__hoechstbetrag_in = catala_new_money(hoechstbetrag_cents);
  const Altersentlastungsbetrag__Altersentlastungsbetrag *r = catala_do(tg_run_altersentlastung);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->Altersentlastungsbetrag__altersentlastungsbetrag);
  catala_free_all();
  return TG_OK;
}

static __thread Entlastungsbetrag__Entlastungsbetrag_in tg_entlastung_in;

static void *tg_run_entlastung(void) {
  return (void *)Entlastungsbetrag__entlastungsbetrag(&tg_entlastung_in);
}

int tg_entlastungsbetrag(int alleinstehend, long anzahl_kinder, long monate_ohne_voraussetzung,
                          long *out_cents) {
  catala_init();
  tg_entlastung_in.Entlastungsbetrag__alleinstehend_in = catala_new_bool(alleinstehend);
  tg_entlastung_in.Entlastungsbetrag__anzahl_kinder_in = catala_new_int(anzahl_kinder);
  tg_entlastung_in.Entlastungsbetrag__monate_ohne_voraussetzung_in =
      catala_new_int(monate_ohne_voraussetzung);
  const Entlastungsbetrag__Entlastungsbetrag *r = catala_do(tg_run_entlastung);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->Entlastungsbetrag__entlastungsbetrag);
  catala_free_all();
  return TG_OK;
}

static __thread Familienleistungsausgleich__Familienleistungsausgleich_in tg_familienleistung_in;

static void *tg_run_familienleistung(void) {
  return (void *)Familienleistungsausgleich__familienleistungsausgleich(&tg_familienleistung_in);
}

int tg_familienleistungsausgleich(long est_ohne_cents, long est_mit_cents, long kindergeld_cents,
                                   long *out_cents) {
  catala_init();
  tg_familienleistung_in.Familienleistungsausgleich__est_ohne_freibetraege_in =
      catala_new_money(est_ohne_cents);
  tg_familienleistung_in.Familienleistungsausgleich__est_mit_freibetraegen_in =
      catala_new_money(est_mit_cents);
  tg_familienleistung_in.Familienleistungsausgleich__kindergeld_in = catala_new_money(kindergeld_cents);
  const Familienleistungsausgleich__Familienleistungsausgleich *r =
      catala_do(tg_run_familienleistung);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->Familienleistungsausgleich__est_nach_familienausgleich);
  catala_free_all();
  return TG_OK;
}

static __thread VerbilligteVermietungWk__VerbilligteVermietungWk_in tg_verbilligt_in;

static void *tg_run_verbilligt(void) {
  return (void *)VerbilligteVermietungWk__verbilligte_vermietung_wk(&tg_verbilligt_in);
}

int tg_verbilligte_vermietung_wk(long werbungskosten_cents, long entgelt_quote_num,
                                  unsigned long entgelt_quote_den, long *out_cents) {
  catala_init();
  tg_verbilligt_in.VerbilligteVermietungWk__werbungskosten_in = catala_new_money(werbungskosten_cents);
  tg_verbilligt_in.VerbilligteVermietungWk__entgelt_quote_prozent_in =
      catala_new_frac(entgelt_quote_num, entgelt_quote_den);
  const VerbilligteVermietungWk__VerbilligteVermietungWk *r = catala_do(tg_run_verbilligt);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->VerbilligteVermietungWk__abziehbare_werbungskosten);
  catala_free_all();
  return TG_OK;
}

static __thread KrankenPflegeVorsorge__KrankenPflegeVorsorge_in tg_kvpv_in;

static void *tg_run_kvpv(void) {
  return (void *)KrankenPflegeVorsorge__kranken_pflege_vorsorge(&tg_kvpv_in);
}

int tg_kranken_pflege_vorsorge(long basis_cents, long weitere_cents, int mit_zuschuss,
                                long *out_cents) {
  catala_init();
  tg_kvpv_in.KrankenPflegeVorsorge__basis_kv_pv_in = catala_new_money(basis_cents);
  tg_kvpv_in.KrankenPflegeVorsorge__weitere_vorsorgeaufwendungen_in = catala_new_money(weitere_cents);
  tg_kvpv_in.KrankenPflegeVorsorge__mit_anspruch_auf_zuschuss_in = catala_new_bool(mit_zuschuss);
  const KrankenPflegeVorsorge__KrankenPflegeVorsorge *r = catala_do(tg_run_kvpv);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->KrankenPflegeVorsorge__abziehbare_kv_pv_vorsorge);
  catala_free_all();
  return TG_OK;
}

static __thread Berufsausbildungsaufwendungen__Berufsausbildung_in tg_berufsausbildung_in;

static void *tg_run_berufsausbildung(void) {
  return (void *)Berufsausbildungsaufwendungen__berufsausbildung(&tg_berufsausbildung_in);
}

int tg_berufsausbildung(long aufwendungen_cents, long *out_cents) {
  catala_init();
  tg_berufsausbildung_in.Berufsausbildungsaufwendungen__aufwendungen_in =
      catala_new_money(aufwendungen_cents);
  const Berufsausbildungsaufwendungen__Berufsausbildung *r = catala_do(tg_run_berufsausbildung);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->Berufsausbildungsaufwendungen__abziehbare_sonderausgaben);
  catala_free_all();
  return TG_OK;
}

static __thread BetriebsFreibetrag__BetriebsFreibetrag_in tg_betriebsfreibetrag_in;

static void *tg_run_betriebsfreibetrag(void) {
  return (void *)BetriebsFreibetrag__betriebs_freibetrag(&tg_betriebsfreibetrag_in);
}

int tg_betriebs_freibetrag(long veraeusserungsgewinn_cents, long *out_cents) {
  catala_init();
  tg_betriebsfreibetrag_in.BetriebsFreibetrag__veraeusserungsgewinn_in =
      catala_new_money(veraeusserungsgewinn_cents);
  const BetriebsFreibetrag__BetriebsFreibetrag *r = catala_do(tg_run_betriebsfreibetrag);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->BetriebsFreibetrag__freibetrag);
  catala_free_all();
  return TG_OK;
}

static __thread EuerGewinn__EuerGewinn_in tg_euer_in;

static void *tg_run_euer(void) {
  return (void *)EuerGewinn__euer_gewinn(&tg_euer_in);
}

int tg_euer_gewinn(long betriebseinnahmen_cents, long betriebsausgaben_cents, long *out_cents) {
  catala_init();
  tg_euer_in.EuerGewinn__betriebseinnahmen_in = catala_new_money(betriebseinnahmen_cents);
  tg_euer_in.EuerGewinn__betriebsausgaben_in = catala_new_money(betriebsausgaben_cents);
  const EuerGewinn__EuerGewinn *r = catala_do(tg_run_euer);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->EuerGewinn__gewinn);
  catala_free_all();
  return TG_OK;
}

static __thread MitunternehmerEinkuenfte__MitunternehmerEinkuenfte_in tg_mitunternehmer_in;

static void *tg_run_mitunternehmer(void) {
  return (void *)MitunternehmerEinkuenfte__mitunternehmer_einkuenfte(&tg_mitunternehmer_in);
}

int tg_mitunternehmer_einkuenfte(long gewinnanteil_cents, long verguetung_taetigkeit_cents,
                                  long verguetung_darlehen_cents, long verguetung_ueberlassung_cents,
                                  long *out_cents) {
  catala_init();
  tg_mitunternehmer_in.MitunternehmerEinkuenfte__gewinnanteil_in = catala_new_money(gewinnanteil_cents);
  tg_mitunternehmer_in.MitunternehmerEinkuenfte__verguetung_taetigkeit_in =
      catala_new_money(verguetung_taetigkeit_cents);
  tg_mitunternehmer_in.MitunternehmerEinkuenfte__verguetung_darlehen_in =
      catala_new_money(verguetung_darlehen_cents);
  tg_mitunternehmer_in.MitunternehmerEinkuenfte__verguetung_ueberlassung_in =
      catala_new_money(verguetung_ueberlassung_cents);
  const MitunternehmerEinkuenfte__MitunternehmerEinkuenfte *r = catala_do(tg_run_mitunternehmer);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->MitunternehmerEinkuenfte__einkuenfte_mitunternehmer);
  catala_free_all();
  return TG_OK;
}

static __thread GwgSofortabzug__GwgSofortabzug_in tg_gwg_in;

static void *tg_run_gwg(void) {
  return (void *)GwgSofortabzug__gwg_sofortabzug(&tg_gwg_in);
}

int tg_gwg_sofortabzug(long anschaffungskosten_netto_cents, long *out_cents) {
  catala_init();
  tg_gwg_in.GwgSofortabzug__anschaffungskosten_netto_in = catala_new_money(anschaffungskosten_netto_cents);
  const GwgSofortabzug__GwgSofortabzug *r = catala_do(tg_run_gwg);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->GwgSofortabzug__sofortabzug);
  catala_free_all();
  return TG_OK;
}

static __thread Verlustvortrag__VerlustvortragAbzug_in tg_verlustvortrag_in;

static void *tg_run_verlustvortrag(void) {
  return (void *)Verlustvortrag__verlustvortrag_abzug(&tg_verlustvortrag_in);
}

int tg_verlustvortrag_abzug(long gde_cents, long bestand_cents, int zusammenveranlagung,
                             long *out_cents) {
  catala_init();
  tg_verlustvortrag_in.Verlustvortrag__gesamtbetrag_einkuenfte_in = catala_new_money(gde_cents);
  tg_verlustvortrag_in.Verlustvortrag__verlustvortrag_bestand_in = catala_new_money(bestand_cents);
  tg_verlustvortrag_in.Verlustvortrag__zusammenveranlagung_in = catala_new_bool(zusammenveranlagung);
  const Verlustvortrag__VerlustvortragAbzug *r = catala_do(tg_run_verlustvortrag);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->Verlustvortrag__verlustabzug);
  catala_free_all();
  return TG_OK;
}

static __thread ErmaessigterDurchschnittssatz__ErmaessigterDurchschnittssatz_in tg_ermaessigt_in;

static void *tg_run_ermaessigt(void) {
  return (void *)ErmaessigterDurchschnittssatz__ermaessigter_durchschnittssatz(&tg_ermaessigt_in);
}

int tg_ermaessigter_durchschnittssatz(long ao_cents, long est_gesamt_zzgl_progression_cents,
                                       long bemessungsgrundlage_durchschnitt_cents,
                                       long *out_cents) {
  catala_init();
  tg_ermaessigt_in.ErmaessigterDurchschnittssatz__ao_einkuenfte_in = catala_new_money(ao_cents);
  tg_ermaessigt_in.ErmaessigterDurchschnittssatz__est_gesamt_zzgl_progression_in =
      catala_new_money(est_gesamt_zzgl_progression_cents);
  tg_ermaessigt_in.ErmaessigterDurchschnittssatz__bemessungsgrundlage_durchschnitt_in =
      catala_new_money(bemessungsgrundlage_durchschnitt_cents);
  const ErmaessigterDurchschnittssatz__ErmaessigterDurchschnittssatz *r = catala_do(tg_run_ermaessigt);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->ErmaessigterDurchschnittssatz__est_ao);
  catala_free_all();
  return TG_OK;
}

/* Entfernungspauschale::Berechnung liefert ZWEI Money-Felder (entfernungspauschale VOR
 * Hoechstbetragsdeckel, abziehbarer_betrag NACH Deckel); Python (catala_entfernungspauschale)
 * liest nur abziehbarer_betrag, catala_ep_ab_21km rechnet mit demselben abziehbarer_betrag
 * weiter -- ein Feld genuegt fuer Teil A, aber beide Felder rauszugeben ist genauso billig
 * und macht den Wrapper vollstaendig fuer kuenftige Aufrufer (Part B).
 */
typedef struct TgEntfernungspauschaleIn {
  long entfernung_km_roh_num;
  unsigned long entfernung_km_roh_den;
  long arbeitstage;
  int eigenes_oder_ueberlassenes_kfz;
  long oepnv_kosten_jahr_cents;
  long satz_bis_20_km_cents;
  long satz_ab_21_km_cents;
  long staffelgrenze_km;
  long hoechstbetrag_cents;
} TgEntfernungspauschaleIn;

typedef struct TgEntfernungspauschaleOut {
  long entfernungspauschale_cents;
  long abziehbarer_betrag_cents;
} TgEntfernungspauschaleOut;

static __thread Entfernungspauschale__Berechnung_in tg_ep_in;

static void *tg_run_ep(void) {
  return (void *)Entfernungspauschale__berechnung(&tg_ep_in);
}

int tg_entfernungspauschale(const TgEntfernungspauschaleIn *in, TgEntfernungspauschaleOut *out) {
  catala_init();
  tg_ep_in.Entfernungspauschale__entfernung_km_roh_in =
      catala_new_frac(in->entfernung_km_roh_num, in->entfernung_km_roh_den);
  tg_ep_in.Entfernungspauschale__arbeitstage_in = catala_new_int(in->arbeitstage);
  tg_ep_in.Entfernungspauschale__eigenes_oder_ueberlassenes_kfz_in =
      catala_new_bool(in->eigenes_oder_ueberlassenes_kfz);
  tg_ep_in.Entfernungspauschale__oepnv_kosten_jahr_in = catala_new_money(in->oepnv_kosten_jahr_cents);
  tg_ep_in.Entfernungspauschale__satz_bis_20_km_in = catala_new_money(in->satz_bis_20_km_cents);
  tg_ep_in.Entfernungspauschale__satz_ab_21_km_in = catala_new_money(in->satz_ab_21_km_cents);
  tg_ep_in.Entfernungspauschale__staffelgrenze_km_in = catala_new_int(in->staffelgrenze_km);
  tg_ep_in.Entfernungspauschale__hoechstbetrag_in = catala_new_money(in->hoechstbetrag_cents);
  const Entfernungspauschale__Berechnung *r = catala_do(tg_run_ep);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  out->entfernungspauschale_cents = mpz_get_si(r->Entfernungspauschale__entfernungspauschale);
  out->abziehbarer_betrag_cents = mpz_get_si(r->Entfernungspauschale__abziehbarer_betrag);
  catala_free_all();
  return TG_OK;
}

typedef struct TgRaumkostenabzugIn {
  int arbeitszimmer_vorhanden;
  int ist_mittelpunkt;
  long tatsaechliche_aufwendungen_cents;
  int jahrespauschale_gewaehlt;
  long monate_ohne_mittelpunkt;
  long homeoffice_tage;
  long jahrespauschale_cents;
  long tagespauschale_pro_tag_cents;
  long tagespauschale_hoechstbetrag_cents;
} TgRaumkostenabzugIn;

typedef struct TgRaumkostenabzugOut {
  long abzug_arbeitszimmer_cents;
  long abzug_homeoffice_cents;
  long abzug_gesamt_cents;
} TgRaumkostenabzugOut;

static __thread ArbeitszimmerHomeoffice__Raumkostenabzug_in tg_raumkosten_in;

static void *tg_run_raumkosten(void) {
  return (void *)ArbeitszimmerHomeoffice__raumkostenabzug(&tg_raumkosten_in);
}

int tg_raumkostenabzug(const TgRaumkostenabzugIn *in, TgRaumkostenabzugOut *out) {
  catala_init();
  tg_raumkosten_in.ArbeitszimmerHomeoffice__arbeitszimmer_vorhanden_in =
      catala_new_bool(in->arbeitszimmer_vorhanden);
  tg_raumkosten_in.ArbeitszimmerHomeoffice__ist_mittelpunkt_in = catala_new_bool(in->ist_mittelpunkt);
  tg_raumkosten_in.ArbeitszimmerHomeoffice__tatsaechliche_aufwendungen_in =
      catala_new_money(in->tatsaechliche_aufwendungen_cents);
  tg_raumkosten_in.ArbeitszimmerHomeoffice__jahrespauschale_gewaehlt_in =
      catala_new_bool(in->jahrespauschale_gewaehlt);
  tg_raumkosten_in.ArbeitszimmerHomeoffice__monate_ohne_mittelpunkt_in =
      catala_new_int(in->monate_ohne_mittelpunkt);
  tg_raumkosten_in.ArbeitszimmerHomeoffice__homeoffice_tage_in = catala_new_int(in->homeoffice_tage);
  tg_raumkosten_in.ArbeitszimmerHomeoffice__jahrespauschale_in = catala_new_money(in->jahrespauschale_cents);
  tg_raumkosten_in.ArbeitszimmerHomeoffice__tagespauschale_pro_tag_in =
      catala_new_money(in->tagespauschale_pro_tag_cents);
  tg_raumkosten_in.ArbeitszimmerHomeoffice__tagespauschale_hoechstbetrag_in =
      catala_new_money(in->tagespauschale_hoechstbetrag_cents);
  const ArbeitszimmerHomeoffice__Raumkostenabzug *r = catala_do(tg_run_raumkosten);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  out->abzug_arbeitszimmer_cents = mpz_get_si(r->ArbeitszimmerHomeoffice__abzug_arbeitszimmer);
  out->abzug_homeoffice_cents = mpz_get_si(r->ArbeitszimmerHomeoffice__abzug_homeoffice);
  out->abzug_gesamt_cents = mpz_get_si(r->ArbeitszimmerHomeoffice__abzug_gesamt);
  catala_free_all();
  return TG_OK;
}

/* -- Einkommensteuertarif: die restlichen drei Scopes (zusammen/gesamt/gesamt_zusammen) ---
 * `TgEstOut` fasst alle 6 Money-Ausgabefelder zusammen, die `FestzusetzendeEstEinzel`,
 * `FestzusetzendeEstGesamt` und `FestzusetzendeEstGesamtZusammen` TEILEN (gleiche Feldnamen,
 * bei `FestzusetzendeEstGesamt(Zusammen)` gleiche Bedeutung wie bei Einzel). EIN Struct statt
 * dreier fast identischer spart Duplikation, ohne dass ein Aufrufer, der nur EIN Feld braucht,
 * mehr tun muesste als `out.festzusetzende_est_cents` zu lesen.
 */
typedef struct TgEstOut {
  long summe_der_einkuenfte_cents;
  long gesamtbetrag_der_einkuenfte_cents;
  long einkommen_cents;
  long zu_versteuerndes_einkommen_cents;
  long tarifliche_est_cents;
  long festzusetzende_est_cents;
} TgEstOut;

static __thread Einkommensteuertarif__FestzusetzendeEstEinzel_in tg_fee_voll_in;

static void *tg_run_fee_voll(void) {
  return (void *)Einkommensteuertarif__festzusetzende_est_einzel(&tg_fee_voll_in);
}

int tg_festzusetzende_est_einzel_voll(long bruttoarbeitslohn_cents, long werbungskosten_cents,
                                       long sonderausgaben_cents, int vz_code, TgEstOut *out) {
  Einkommensteuertarif__Veranlagungszeitraum vz;
  int rc = tg_vz(vz_code, &vz);
  if (rc != TG_OK) {
    return rc;
  }
  catala_init();
  tg_fee_voll_in.Einkommensteuertarif__bruttoarbeitslohn_in = catala_new_money(bruttoarbeitslohn_cents);
  tg_fee_voll_in.Einkommensteuertarif__werbungskosten_in = catala_new_money(werbungskosten_cents);
  tg_fee_voll_in.Einkommensteuertarif__sonderausgaben_in = catala_new_money(sonderausgaben_cents);
  tg_fee_voll_in.Einkommensteuertarif__veranlagungszeitraum_in = &vz;
  const Einkommensteuertarif__FestzusetzendeEstEinzel *r = catala_do(tg_run_fee_voll);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  out->summe_der_einkuenfte_cents = mpz_get_si(r->Einkommensteuertarif__summe_der_einkuenfte);
  out->gesamtbetrag_der_einkuenfte_cents = mpz_get_si(r->Einkommensteuertarif__gesamtbetrag_der_einkuenfte);
  out->einkommen_cents = mpz_get_si(r->Einkommensteuertarif__einkommen);
  out->zu_versteuerndes_einkommen_cents = mpz_get_si(r->Einkommensteuertarif__zu_versteuerndes_einkommen);
  out->tarifliche_est_cents = mpz_get_si(r->Einkommensteuertarif__tarifliche_est);
  out->festzusetzende_est_cents = mpz_get_si(r->Einkommensteuertarif__festzusetzende_est);
  catala_free_all();
  return TG_OK;
}

static __thread Einkommensteuertarif__FestzusetzendeEstZusammen_in tg_fez_in;

static void *tg_run_fez(void) {
  return (void *)Einkommensteuertarif__festzusetzende_est_zusammen(&tg_fez_in);
}

int tg_festzusetzende_est_zusammen(long bruttoarbeitslohn_a_cents, long werbungskosten_a_cents,
                                    long bruttoarbeitslohn_b_cents, long werbungskosten_b_cents,
                                    long sonderausgaben_gemeinsam_cents, int vz_code,
                                    long *out_cents) {
  Einkommensteuertarif__Veranlagungszeitraum vz;
  int rc = tg_vz(vz_code, &vz);
  if (rc != TG_OK) {
    return rc;
  }
  catala_init();
  tg_fez_in.Einkommensteuertarif__bruttoarbeitslohn_a_in = catala_new_money(bruttoarbeitslohn_a_cents);
  tg_fez_in.Einkommensteuertarif__werbungskosten_a_in = catala_new_money(werbungskosten_a_cents);
  tg_fez_in.Einkommensteuertarif__bruttoarbeitslohn_b_in = catala_new_money(bruttoarbeitslohn_b_cents);
  tg_fez_in.Einkommensteuertarif__werbungskosten_b_in = catala_new_money(werbungskosten_b_cents);
  tg_fez_in.Einkommensteuertarif__sonderausgaben_gemeinsam_in =
      catala_new_money(sonderausgaben_gemeinsam_cents);
  tg_fez_in.Einkommensteuertarif__veranlagungszeitraum_in = &vz;
  const Einkommensteuertarif__FestzusetzendeEstZusammen *r = catala_do(tg_run_fez);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  *out_cents = mpz_get_si(r->Einkommensteuertarif__festzusetzende_est);
  catala_free_all();
  return TG_OK;
}

/* `TgGesamtIn` bildet 1:1 die 16 Money/Bool-Felder ab, die `FestzusetzendeEstGesamt_in` UND
 * `FestzusetzendeEstGesamtZusammen_in` TEILEN (identische Feldliste, siehe
 * Einkommensteuertarif.h Zeile 157ff/181ff) -- welcher der beiden Catala-Scopes laeuft,
 * entscheidet einzig `zusammen` im Aufrufer (`tg_festzusetzende_est_gesamt` vs.
 * `tg_festzusetzende_est_gesamt_zusammen`), nicht dieses Struct.
 */
typedef struct TgGesamtIn {
  long einkuenfte_nichtselbststaendig_cents;
  long einkuenfte_kapitalvermoegen_cents;
  long einkuenfte_vermietung_cents;
  long einkuenfte_sonstige_cents;
  long einkuenfte_gewinn_cents;
  long altersentlastungsbetrag_cents;
  long entlastungsbetrag_alleinerziehende_cents;
  long sonderausgaben_cents;
  long aussergewoehnliche_belastungen_cents;
  long freibetraege_kinder_cents;
  long sonstige_abzuege_vom_einkommen_cents;
  long anzurechnende_auslaendische_steuern_cents;
  long steuerermaessigungen_cents;
  long steuer_kapital_gesondert_cents;
  long hinzurechnung_kindergeld_cents;
  long hinzurechnung_zulage_cents;
  int tarif_modifiziert;
  long tarifliche_est_modifiziert_cents;
} TgGesamtIn;

static __thread Einkommensteuertarif__FestzusetzendeEstGesamt_in tg_gesamt_in;

static void *tg_run_gesamt(void) {
  return (void *)Einkommensteuertarif__festzusetzende_est_gesamt(&tg_gesamt_in);
}

int tg_festzusetzende_est_gesamt(const TgGesamtIn *in, int vz_code, TgEstOut *out) {
  Einkommensteuertarif__Veranlagungszeitraum vz;
  int rc = tg_vz(vz_code, &vz);
  if (rc != TG_OK) {
    return rc;
  }
  catala_init();
  tg_gesamt_in.Einkommensteuertarif__einkuenfte_nichtselbststaendig_in =
      catala_new_money(in->einkuenfte_nichtselbststaendig_cents);
  tg_gesamt_in.Einkommensteuertarif__einkuenfte_kapitalvermoegen_in =
      catala_new_money(in->einkuenfte_kapitalvermoegen_cents);
  tg_gesamt_in.Einkommensteuertarif__einkuenfte_vermietung_in =
      catala_new_money(in->einkuenfte_vermietung_cents);
  tg_gesamt_in.Einkommensteuertarif__einkuenfte_sonstige_in =
      catala_new_money(in->einkuenfte_sonstige_cents);
  tg_gesamt_in.Einkommensteuertarif__einkuenfte_gewinn_in = catala_new_money(in->einkuenfte_gewinn_cents);
  tg_gesamt_in.Einkommensteuertarif__altersentlastungsbetrag_in =
      catala_new_money(in->altersentlastungsbetrag_cents);
  tg_gesamt_in.Einkommensteuertarif__entlastungsbetrag_alleinerziehende_in =
      catala_new_money(in->entlastungsbetrag_alleinerziehende_cents);
  tg_gesamt_in.Einkommensteuertarif__sonderausgaben_in = catala_new_money(in->sonderausgaben_cents);
  tg_gesamt_in.Einkommensteuertarif__aussergewoehnliche_belastungen_in =
      catala_new_money(in->aussergewoehnliche_belastungen_cents);
  tg_gesamt_in.Einkommensteuertarif__freibetraege_kinder_in =
      catala_new_money(in->freibetraege_kinder_cents);
  tg_gesamt_in.Einkommensteuertarif__sonstige_abzuege_vom_einkommen_in =
      catala_new_money(in->sonstige_abzuege_vom_einkommen_cents);
  tg_gesamt_in.Einkommensteuertarif__anzurechnende_auslaendische_steuern_in =
      catala_new_money(in->anzurechnende_auslaendische_steuern_cents);
  tg_gesamt_in.Einkommensteuertarif__steuerermaessigungen_in =
      catala_new_money(in->steuerermaessigungen_cents);
  tg_gesamt_in.Einkommensteuertarif__steuer_kapital_gesondert_in =
      catala_new_money(in->steuer_kapital_gesondert_cents);
  tg_gesamt_in.Einkommensteuertarif__hinzurechnung_kindergeld_in =
      catala_new_money(in->hinzurechnung_kindergeld_cents);
  tg_gesamt_in.Einkommensteuertarif__hinzurechnung_zulage_in =
      catala_new_money(in->hinzurechnung_zulage_cents);
  tg_gesamt_in.Einkommensteuertarif__tarif_modifiziert_in = catala_new_bool(in->tarif_modifiziert);
  tg_gesamt_in.Einkommensteuertarif__tarifliche_est_modifiziert_in =
      catala_new_money(in->tarifliche_est_modifiziert_cents);
  tg_gesamt_in.Einkommensteuertarif__veranlagungszeitraum_in = &vz;
  const Einkommensteuertarif__FestzusetzendeEstGesamt *r = catala_do(tg_run_gesamt);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  out->summe_der_einkuenfte_cents = mpz_get_si(r->Einkommensteuertarif__summe_der_einkuenfte);
  out->gesamtbetrag_der_einkuenfte_cents = mpz_get_si(r->Einkommensteuertarif__gesamtbetrag_der_einkuenfte);
  out->einkommen_cents = mpz_get_si(r->Einkommensteuertarif__einkommen);
  out->zu_versteuerndes_einkommen_cents = mpz_get_si(r->Einkommensteuertarif__zu_versteuerndes_einkommen);
  out->tarifliche_est_cents = mpz_get_si(r->Einkommensteuertarif__tarifliche_est);
  out->festzusetzende_est_cents = mpz_get_si(r->Einkommensteuertarif__festzusetzende_est);
  catala_free_all();
  return TG_OK;
}

static __thread Einkommensteuertarif__FestzusetzendeEstGesamtZusammen_in tg_gesamt_zus_in;

static void *tg_run_gesamt_zusammen(void) {
  return (void *)Einkommensteuertarif__festzusetzende_est_gesamt_zusammen(&tg_gesamt_zus_in);
}

int tg_festzusetzende_est_gesamt_zusammen(const TgGesamtIn *in, int vz_code, TgEstOut *out) {
  Einkommensteuertarif__Veranlagungszeitraum vz;
  int rc = tg_vz(vz_code, &vz);
  if (rc != TG_OK) {
    return rc;
  }
  catala_init();
  tg_gesamt_zus_in.Einkommensteuertarif__einkuenfte_nichtselbststaendig_in =
      catala_new_money(in->einkuenfte_nichtselbststaendig_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__einkuenfte_kapitalvermoegen_in =
      catala_new_money(in->einkuenfte_kapitalvermoegen_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__einkuenfte_vermietung_in =
      catala_new_money(in->einkuenfte_vermietung_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__einkuenfte_sonstige_in =
      catala_new_money(in->einkuenfte_sonstige_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__einkuenfte_gewinn_in =
      catala_new_money(in->einkuenfte_gewinn_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__altersentlastungsbetrag_in =
      catala_new_money(in->altersentlastungsbetrag_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__entlastungsbetrag_alleinerziehende_in =
      catala_new_money(in->entlastungsbetrag_alleinerziehende_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__sonderausgaben_in = catala_new_money(in->sonderausgaben_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__aussergewoehnliche_belastungen_in =
      catala_new_money(in->aussergewoehnliche_belastungen_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__freibetraege_kinder_in =
      catala_new_money(in->freibetraege_kinder_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__sonstige_abzuege_vom_einkommen_in =
      catala_new_money(in->sonstige_abzuege_vom_einkommen_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__anzurechnende_auslaendische_steuern_in =
      catala_new_money(in->anzurechnende_auslaendische_steuern_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__steuerermaessigungen_in =
      catala_new_money(in->steuerermaessigungen_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__steuer_kapital_gesondert_in =
      catala_new_money(in->steuer_kapital_gesondert_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__hinzurechnung_kindergeld_in =
      catala_new_money(in->hinzurechnung_kindergeld_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__hinzurechnung_zulage_in =
      catala_new_money(in->hinzurechnung_zulage_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__tarif_modifiziert_in = catala_new_bool(in->tarif_modifiziert);
  tg_gesamt_zus_in.Einkommensteuertarif__tarifliche_est_modifiziert_in =
      catala_new_money(in->tarifliche_est_modifiziert_cents);
  tg_gesamt_zus_in.Einkommensteuertarif__veranlagungszeitraum_in = &vz;
  const Einkommensteuertarif__FestzusetzendeEstGesamtZusammen *r = catala_do(tg_run_gesamt_zusammen);
  if (!r) {
    catala_free_all();
    return TG_ERR_CATALA;
  }
  out->summe_der_einkuenfte_cents = mpz_get_si(r->Einkommensteuertarif__summe_der_einkuenfte);
  out->gesamtbetrag_der_einkuenfte_cents = mpz_get_si(r->Einkommensteuertarif__gesamtbetrag_der_einkuenfte);
  out->einkommen_cents = mpz_get_si(r->Einkommensteuertarif__einkommen);
  out->zu_versteuerndes_einkommen_cents = mpz_get_si(r->Einkommensteuertarif__zu_versteuerndes_einkommen);
  out->tarifliche_est_cents = mpz_get_si(r->Einkommensteuertarif__tarifliche_est);
  out->festzusetzende_est_cents = mpz_get_si(r->Einkommensteuertarif__festzusetzende_est);
  catala_free_all();
  return TG_OK;
}
