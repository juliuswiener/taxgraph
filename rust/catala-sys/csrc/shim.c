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
