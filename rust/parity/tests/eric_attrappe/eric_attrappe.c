/*
 * Attrappe fuer libericapi.so (ERiC 44, Singlethread-API) — NUR fuer den Differenz-Harness
 * `rust/parity/tests/eric_attrappe/mod.rs`. Sie prueft nichts: sie antwortet nach einem Skript und
 * schreibt auf, was sie bekam. Python (ctypes) und Rust (libloading) laden dieselbe Datei und
 * muessen dasselbe sehen.
 *
 * Gebaut wird sie im Test mit `cc -shared -fPIC`; die Binaerdatei kommt nie ins Repo. Sie sendet
 * nichts, braucht keine Hersteller-ID und kein Zertifikat.
 *
 * Steuerdateien in $ERIC_ATTRAPPE_DIR:
 *   init_rc          Rueckgabe von EricInitialisiere (Ganzzahl); fehlt die Datei: 0
 *   einstellung_rc   Rueckgabe von EricEinstellungSetzen; fehlt die Datei: 0
 *   skript           ein Block je Aufruf von EricBearbeiteVorgang, getrennt durch eine Zeile "=====":
 *                      Zeile 1: der Rueckgabecode
 *                      danach der Text fuer den Rueckgabepuffer (leer = leerer Puffer)
 *                      eine Zeile "--LOG--", danach Zeilen, die an <logPfad>/eric.log angehaengt werden
 *                    Gibt es weniger Bloecke als Aufrufe, gilt der letzte; ohne Datei: rc 99999.
 *   zaehler          Zahl der Aufrufe seit dem Loeschen der Datei (schreibt die Attrappe)
 *   gesehen/<n>.xml  das XML des n-ten Aufrufs, Byte fuer Byte (bis zum ersten NUL, wie C es sieht)
 *   gesehen/<n>.meta datenart, flags, ob ein Druck-, Crypto- oder Serverantwort-Parameter kam, und thread=
 *
 * Nur fuer `rust/versand` (der Vergleichslauf sieht davon nichts; die Dateien oben bleiben byte-gleich):
 *   skript           ein Block darf "--SERVER--" tragen: danach der Text fuer den Serverantwort-Puffer
 *   init_zaehler     Zahl der EricInitialisiere-Aufrufe; beende_zaehler: der EricBeende-Aufrufe
 *   zertifikat_rc    Rueckgabe von EricGetHandleToCertificate (fehlt die Datei: 0, Handle 77)
 *   zertifikat_pfad, zertifikat_zaehler, zertifikat_geschlossen  was die Zertifikatsfunktionen sahen
 *   gesehen/<n>.crypto  Version, Handle und PIN-LAENGE des Crypto-Parameters (nie die PIN selbst)
 */
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

typedef struct {
    char *text;
} Puffer;

static char g_log_pfad[4096];

static const char *steuer_dir(void) {
    const char *d = getenv("ERIC_ATTRAPPE_DIR");
    return d ? d : ".";
}

static void pfad_in(char *aus, size_t n, const char *name) {
    snprintf(aus, n, "%s/%s", steuer_dir(), name);
}

static char *lies_datei(const char *pfad, size_t *laenge) {
    FILE *f = fopen(pfad, "rb");
    if (!f) {
        return NULL;
    }
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *p = malloc((size_t)n + 1);
    size_t gelesen = fread(p, 1, (size_t)n, f);
    p[gelesen] = '\0';
    fclose(f);
    if (laenge) {
        *laenge = gelesen;
    }
    return p;
}

static int lies_zahl(const char *name, int vorgabe) {
    char pfad[4200];
    pfad_in(pfad, sizeof pfad, name);
    char *t = lies_datei(pfad, NULL);
    if (!t) {
        return vorgabe;
    }
    int z = atoi(t);
    free(t);
    return z;
}

/* Haengt eins an die Zaehldatei `name` an (Ganzzahl); ohne Steuerverzeichnis wirkungslos. */
static void zaehle(const char *name) {
    char pfad[4200];
    pfad_in(pfad, sizeof pfad, name);
    int n = lies_zahl(name, 0) + 1;
    FILE *f = fopen(pfad, "wb");
    if (f) {
        fprintf(f, "%d", n);
        fclose(f);
    }
}

int EricInitialisiere(const char *plugin_pfad, const char *log_pfad) {
    (void)plugin_pfad;
    zaehle("init_zaehler");
    snprintf(g_log_pfad, sizeof g_log_pfad, "%s", log_pfad ? log_pfad : "");
    int rc = lies_zahl("init_rc", 0);
    if (rc == 0 && g_log_pfad[0]) {
        /* Wie ERiC: eric.log entsteht bei der Initialisierung. */
        char p[4200];
        snprintf(p, sizeof p, "%s/eric.log", g_log_pfad);
        FILE *f = fopen(p, "ab");
        if (f) {
            fclose(f);
        }
    }
    return rc;
}

int EricEinstellungSetzen(const char *name, const char *wert) {
    (void)name;
    (void)wert;
    return lies_zahl("einstellung_rc", 0);
}

void *EricRueckgabepufferErzeugen(void) {
    Puffer *p = malloc(sizeof *p);
    p->text = strdup("");
    return p;
}

const char *EricRueckgabepufferInhalt(void *h) {
    return h ? ((Puffer *)h)->text : NULL;
}

int EricRueckgabepufferFreigeben(void *h) {
    if (h) {
        free(((Puffer *)h)->text);
        free(h);
    }
    return 0;
}

int EricBeende(void) {
    zaehle("beende_zaehler");
    return 0;
}

/* Zertifikat (nur fuer `rust/versand`): die Attrappe oeffnet nichts, sie antwortet nach `zertifikat_rc`
 * (fehlt die Datei: 0), gibt das Handle 77 zurueck und schreibt auf, was sie sah:
 *   zertifikat_pfad       der Pfad, wie ihn der Aufrufer uebergab (ein Test-Platzhalter, nie ein echtes Zertifikat)
 *   zertifikat_geschlossen das Handle, das EricCloseHandleToCertificate bekam (Datei fehlt: nie geschlossen) */
int EricGetHandleToCertificate(uint32_t *handle, uint32_t *info, const char *pfad) {
    char p[4200];
    pfad_in(p, sizeof p, "zertifikat_pfad");
    FILE *f = fopen(p, "wb");
    if (f) {
        fputs(pfad ? pfad : "", f);
        fclose(f);
    }
    zaehle("zertifikat_zaehler");
    int rc = lies_zahl("zertifikat_rc", 0);
    if (rc == 0) {
        if (handle) {
            *handle = 77;
        }
        if (info) {
            *info = 0;
        }
    }
    return rc;
}

int EricCloseHandleToCertificate(uint32_t handle) {
    char p[4200];
    pfad_in(p, sizeof p, "zertifikat_geschlossen");
    FILE *f = fopen(p, "ab");
    if (f) {
        fprintf(f, "%u\n", (unsigned)handle);
        fclose(f);
    }
    return 0;
}

/* Der Block `n` (ab 1) des Skripts; der letzte, wenn es weniger gibt. */
static char *skript_block(int n) {
    char pfad[4200];
    pfad_in(pfad, sizeof pfad, "skript");
    char *alles = lies_datei(pfad, NULL);
    if (!alles) {
        return NULL;
    }
    char *start = alles;
    char *block = NULL;
    for (int i = 1;; i++) {
        char *ende = strstr(start, "\n=====\n");
        size_t len = ende ? (size_t)(ende - start) : strlen(start);
        free(block);
        block = malloc(len + 1);
        memcpy(block, start, len);
        block[len] = '\0';
        if (i >= n || !ende) {
            break;
        }
        start = ende + strlen("\n=====\n");
    }
    free(alles);
    return block;
}

int EricBearbeiteVorgang(const char *xml, const char *datenart, uint32_t flags, const void *druck,
                         const void *crypto, void *rueckgabe, void *serverantwort) {
    char p[4200];
    pfad_in(p, sizeof p, "zaehler");
    int n = lies_zahl("zaehler", 0) + 1;
    FILE *z = fopen(p, "wb");
    if (z) {
        fprintf(z, "%d", n);
        fclose(z);
    }

    snprintf(p, sizeof p, "%s/gesehen", steuer_dir());
    mkdir(p, 0777); /* existiert er schon, ist das in Ordnung */
    char name[4300];
    snprintf(name, sizeof name, "%s/%d.xml", p, n);
    FILE *fx = fopen(name, "wb");
    if (fx) {
        fwrite(xml, 1, strlen(xml), fx);
        fclose(fx);
    }
    snprintf(name, sizeof name, "%s/%d.meta", p, n);
    FILE *fm = fopen(name, "wb");
    if (fm) {
        fprintf(fm, "datenart=%s\nflags=%u\ndruck=%s\ncrypto=%s\nserverantwort=%s\n", datenart,
                (unsigned)flags, druck ? "gesetzt" : "NULL", crypto ? "gesetzt" : "NULL",
                serverantwort ? "gesetzt" : "NULL");
        /* Der Thread, auf dem der Aufruf lief: ERiC (Singlethread-API) sieht ihn immer gleich. */
        fprintf(fm, "thread=%lu\n", (unsigned long)pthread_self());
        fclose(fm);
    }
    if (crypto) {
        /* Nur fuer `rust/versand`: eric_verschluesselungs_parameter_t (eric_types.h), von der Attrappe
         * gelesen wie von ERiC. Die PIN selbst wird NIE aufgeschrieben, nur ihre Laenge. */
        const struct {
            uint32_t version;
            uint32_t zertifikat_handle;
            const char *pin;
        } *c = crypto;
        snprintf(name, sizeof name, "%s/%d.crypto", p, n);
        FILE *fc = fopen(name, "wb");
        if (fc) {
            fprintf(fc, "version=%u\nhandle=%u\npin_laenge=%zu\n", (unsigned)c->version,
                    (unsigned)c->zertifikat_handle, c->pin ? strlen(c->pin) : (size_t)0);
            fclose(fc);
        }
    }

    char *block = skript_block(n);
    if (!block) {
        return 99999;
    }
    int rc = atoi(block);
    char *nl = strchr(block, '\n');
    char *text = nl ? nl + 1 : block + strlen(block);
    char *log = strstr(text, "\n--LOG--\n");
    if (log) {
        *log = '\0';
        log += strlen("\n--LOG--\n");
    } else if (strncmp(text, "--LOG--\n", 8) == 0) {
        log = text + 8;
        *text = '\0';
    }
    /* Nur fuer `rust/versand`: der Text der Serverantwort, getrennt durch eine Zeile "--SERVER--". */
    char *server_text = NULL;
    char *srv = strstr(text, "\n--SERVER--\n");
    if (srv) {
        *srv = '\0';
        server_text = srv + strlen("\n--SERVER--\n");
    } else if (strncmp(text, "--SERVER--\n", 11) == 0) {
        server_text = text + 11;
        *text = '\0';
    }
    Puffer *pf = rueckgabe;
    if (pf) {
        free(pf->text);
        pf->text = strdup(text);
    }
    Puffer *ps = serverantwort;
    if (ps) {
        free(ps->text);
        ps->text = strdup(server_text ? server_text : "");
    }
    if (log && g_log_pfad[0]) {
        snprintf(name, sizeof name, "%s/eric.log", g_log_pfad);
        FILE *fl = fopen(name, "ab");
        if (fl) {
            fputs(log, fl);
            if (*log && log[strlen(log) - 1] != '\n') {
                fputc('\n', fl);
            }
            fclose(fl);
        }
    }
    free(block);
    return rc;
}
