//! Typisierter Lesezugriff auf `params/<vz>/*.yaml` und `params/kohorten/*.yaml` fuer die
//! Accessor-Schicht (`engine::zugriff`). Ersetzt die Python-Helfer `_az_params`, `_ep_saetze`,
//! `_dhf_params`, `_verpflegung_params`, `_altersentlastung_kohorte` sowie die Direktlesungen in
//! `catala_grundfreibetrag`/`catala_arbeitnehmer_pauschbetrag` (`produkt/engine/runner.py`).
//!
//! Alle Dateien werden EINMAL in [`Params::lade`] gelesen (Python: `lru_cache` je Pfad). Jeder
//! Fehler nennt Datei und Schluessel.
//!
//! Zahlen: ganze Euro-Betraege sind [`Euro`], Saetze mit Nachkommastellen [`Satz`] (nie `f64`).
//! Eine YAML-Gleitkommazahl wird ueber ihre kuerzeste Dezimaldarstellung gelesen -- dieselbe, die
//! Pythons `str(float)` liefert (`Decimal(str(prozent))` in runner.py).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use domain::{Euro, Satz, Vz};
use rust_decimal::Decimal;
use serde_yaml_ng::Value;

use crate::kohorten_datei::{lade_kohorten, KohortenDatei, KohortenFehler};
use crate::params_datei::{lade_params, ParamsDatei, ParamsFehler};

/// Fehler beim Lesen eines Parameterwerts. Jede Variante nennt die Datei.
#[derive(Debug, thiserror::Error)]
pub enum ParamsWertFehler {
    #[error(transparent)]
    Laden(#[from] ParamsFehler),
    #[error(transparent)]
    Kohorten(#[from] KohortenFehler),
    #[error("Parameterdatei {datei} fuer VZ {vz} nicht geladen")]
    DateiFehlt { vz: u16, datei: String },
    #[error("{datei}: Schluessel {schluessel} fehlt")]
    SchluesselFehlt { datei: String, schluessel: String },
    #[error("{datei}: Schluessel {schluessel} ist keine {erwartet}")]
    Typ {
        datei: String,
        schluessel: String,
        erwartet: &'static str,
    },
}

/// Alle Parameterdateien der unterstuetzten Veranlagungszeitraeume plus die Kohortentabellen.
#[derive(Debug, Clone)]
pub struct Params {
    jahre: BTreeMap<(u16, String), ParamsDatei>,
    kohorten: BTreeMap<String, KohortenDatei>,
}

/// § 4 Abs. 5 Nr. 6b/6c `EStG`: Saetze aus `arbeitszimmer_homeoffice.yaml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArbeitszimmerSaetze {
    pub jahrespauschale: Euro,
    pub tagespauschale_pro_tag: Euro,
    pub tagespauschale_hoechstbetrag: Euro,
}

/// § 9 Abs. 1 S. 3 Nr. 4 `EStG`: Saetze aus `entfernungspauschale.yaml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntfernungspauschaleSaetze {
    /// Euro je km (z. B. `0.30`).
    pub satz_bis_20_km: Satz,
    /// Euro je km ab dem 21. km (z. B. `0.38`).
    pub satz_ab_21_km: Satz,
    pub staffelgrenze_km: i64,
    pub hoechstbetrag_ohne_kfz: Euro,
}

/// § 9 Abs. 1 S. 3 Nr. 5 `EStG`: Kappungsgrenzen aus `dhf_p9_1_nr5.yaml`, Euro je Monat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DhfGrenzen {
    pub cap_monat_inland: Euro,
    /// `None` = KEINE Auslandsgrenze (nicht 0). Die 2.000-EUR-Grenze gilt erst ab VZ 2026
    /// (`StÄndG` 2025, `BGBl`. 2025 I Nr. 363); `params/2024` und `params/2025` fuehren sie nicht.
    /// PARITÄT: Python liest `(p.get("cap_monat_ausland") or {}).get("wert")`; fehlender
    /// Schluessel, leerer Block und `wert: null` ergeben dort wie hier `None`.
    pub cap_monat_ausland: Option<Euro>,
}

/// § 9 Abs. 4a `EStG`: Pauschalen (Euro je Tag) und Kuerzungssaetze (Prozent) aus
/// `verpflegung_p9_4a.yaml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerpflegungSaetze {
    pub pauschale_24h: Euro,
    pub pauschale_an_abreise: Euro,
    pub pauschale_ab_8h: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (`p.get(..., {}).get("wert", 0)`, fail-open).
    pub kuerzung_fruehstueck_prozent: i64,
    /// PARITÄT: Python setzt fehlend = 0 (`p.get(..., {}).get("wert", 0)`, fail-open).
    pub kuerzung_mittag_abend_prozent: i64,
}

/// § 24a S. 5 `EStG`: eine Zeile der Kohortenstaffel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AltersentlastungKohorte {
    /// Prozent (z. B. `13.2`), nicht Anteil.
    pub prozentsatz: Satz,
    pub hoechstbetrag: Euro,
}

/// § 33b Abs. 3/4/6 `EStG`: Tabellen aus `behinderten_pauschbetrag_p33b.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct P33bPauschbetraege {
    /// `GdB`-Stufe (20, 30, ... 100) -> Euro.
    pub gdb_staffel: BTreeMap<i64, Euro>,
    pub blind_hilflos_taubblind: Euro,
    pub hinterbliebenen: Euro,
    /// Pflegegrad -> Euro.
    pub pflege_staffel: BTreeMap<i64, Euro>,
    pub pflege_hilflos: Euro,
}

/// § 33 Abs. 2a `EStG`: Pauschalen aus `fahrtkostenpauschale_p33_2a.yaml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FahrtkostenPauschalen {
    pub pauschale_900: Euro,
    pub pauschale_4500: Euro,
}

/// § 10 Abs. 1 Nr. 5/9 `EStG`: `abzugssatz` + `hoechstbetrag_je_kind` (`kinderbetreuung_p10.yaml`,
/// `schulgeld_p10.yaml`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SatzHoechstbetrag {
    /// Anteil (z. B. `0.8`), nicht Prozent.
    pub abzugssatz: Satz,
    pub hoechstbetrag_je_kind: Euro,
}

/// § 34g Satz 2 `EStG`: Satz und Hoechstbetraege der Steuerermaessigung fuer Zuwendungen an politische Parteien, dazu der Deckel
/// des Sonderausgabenabzugs nach § 10b Abs. 2 `EStG` (`parteispenden_p34g.yaml`, Abweichungen Nr. 31 und Nr. 43). Nur Rust;
/// Python kennt die Datei nicht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParteispendenErmaessigung {
    /// Anteil der Ausgaben (z. B. `0.5`), nicht Prozent.
    pub satz: Satz,
    /// Hoechstbetrag der Ermaessigung je Steuerpflichtigen.
    pub hoechstbetrag_einzel: Euro,
    /// Hoechstbetrag der Ermaessigung bei Zusammenveranlagung von Ehegatten.
    pub hoechstbetrag_zusammen: Euro,
    /// § 10b Abs. 2 Satz 1: Deckel der Sonderausgabe je Steuerpflichtigen (der Teil ueber der Basis der Ermaessigung).
    pub sonderausgaben_hoechstbetrag_einzel: Euro,
    /// § 10b Abs. 2 Satz 1: Deckel der Sonderausgabe bei Zusammenveranlagung von Ehegatten.
    pub sonderausgaben_hoechstbetrag_zusammen: Euro,
}

/// § 19 Abs. 2 S. 3 `EStG`: eine Zeile von `versorgungsfreibetrag_p19_2.yaml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersorgungsfreibetragKohorte {
    /// Prozent (z. B. `13.2`), nicht Anteil.
    pub prozentsatz: Satz,
    pub hoechstbetrag: Euro,
    pub zuschlag: Euro,
}

const JAHRE: [Vz; 3] = [Vz::Vz2024, Vz::Vz2025, Vz::Vz2026];

impl Params {
    /// Laedt `params/<vz>/*.yaml` fuer 2024-2026 und `params/kohorten/*.yaml` unter `repo_wurzel`.
    ///
    /// # Errors
    /// [`ParamsWertFehler`], wenn ein Verzeichnis oder eine Datei nicht lesbar ist.
    ///
    /// ```
    /// let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    /// let p = bindung::Params::lade(&wurzel).unwrap();
    /// assert_eq!(p.grundfreibetrag(domain::Vz::Vz2025).unwrap(), domain::Euro::new(12_096));
    /// ```
    pub fn lade(repo_wurzel: &Path) -> Result<Self, ParamsWertFehler> {
        let mut jahre = BTreeMap::new();
        for vz in JAHRE {
            let dir = repo_wurzel.join("params").join(vz.jahr().to_string());
            for pfad in yaml_dateien(&dir)? {
                let name = dateiname(&pfad);
                jahre.insert((vz.jahr(), name), lade_params(&pfad)?);
            }
        }
        let mut kohorten = BTreeMap::new();
        let dir = repo_wurzel.join("params").join("kohorten");
        for pfad in yaml_dateien(&dir)? {
            kohorten.insert(dateiname(&pfad), lade_kohorten(&pfad)?);
        }
        Ok(Self { jahre, kohorten })
    }

    fn datei(&self, vz: Vz, datei: &str) -> Result<&ParamsDatei, ParamsWertFehler> {
        self.jahre
            .get(&(vz.jahr(), datei.to_string()))
            .ok_or_else(|| ParamsWertFehler::DateiFehlt {
                vz: vz.jahr(),
                datei: datei.to_string(),
            })
    }

    /// `p[schluessel]["wert"]` einer Jahresdatei, roh.
    fn wert(&self, vz: Vz, datei: &str, schluessel: &str) -> Result<&Value, ParamsWertFehler> {
        let fehlt = || ParamsWertFehler::SchluesselFehlt {
            datei: format!("{}/{datei}", vz.jahr()),
            schluessel: format!("{schluessel}.wert"),
        };
        self.datei(vz, datei)?
            .werte
            .get(schluessel)
            .and_then(|v| v.get("wert"))
            .ok_or_else(fehlt)
    }

    fn euro(&self, vz: Vz, datei: &str, schluessel: &str) -> Result<Euro, ParamsWertFehler> {
        let v = self.wert(vz, datei, schluessel)?;
        ganzzahl(v)
            .map(Euro::new)
            .ok_or_else(|| typ(vz, datei, schluessel, "ganze Zahl"))
    }

    fn ganz(&self, vz: Vz, datei: &str, schluessel: &str) -> Result<i64, ParamsWertFehler> {
        let v = self.wert(vz, datei, schluessel)?;
        ganzzahl(v).ok_or_else(|| typ(vz, datei, schluessel, "ganze Zahl"))
    }

    fn dezimal(&self, vz: Vz, datei: &str, schluessel: &str) -> Result<Decimal, ParamsWertFehler> {
        let v = self.wert(vz, datei, schluessel)?;
        dezimalzahl(v).ok_or_else(|| typ(vz, datei, schluessel, "Dezimalzahl"))
    }

    /// § 32a Abs. 1 S. 2 Nr. 1 `EStG` Grundfreibetrag (`einkommensteuertarif_p32a.yaml`).
    ///
    /// # Errors
    /// [`ParamsWertFehler`], wenn Datei oder Schluessel fehlen oder keine ganze Zahl tragen.
    ///
    /// ```
    /// let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    /// let p = bindung::Params::lade(&wurzel).unwrap();
    /// assert_eq!(p.grundfreibetrag(domain::Vz::Vz2024).unwrap(), domain::Euro::new(11_784));
    /// ```
    pub fn grundfreibetrag(&self, vz: Vz) -> Result<Euro, ParamsWertFehler> {
        self.euro(vz, "einkommensteuertarif_p32a.yaml", "grundfreibetrag")
    }

    /// § 9a S. 1 Nr. 1a `EStG` Arbeitnehmer-Pauschbetrag (`arbeitnehmerpauschbetrag.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    /// let p = bindung::Params::lade(&wurzel).unwrap();
    /// assert_eq!(p.arbeitnehmer_pauschbetrag(domain::Vz::Vz2026).unwrap(), domain::Euro::new(1230));
    /// ```
    pub fn arbeitnehmer_pauschbetrag(&self, vz: Vz) -> Result<Euro, ParamsWertFehler> {
        self.euro(vz, "arbeitnehmerpauschbetrag.yaml", "wert")
    }

    /// § 4 Abs. 5 Nr. 6b/6c `EStG` (`arbeitszimmer_homeoffice.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    /// let p = bindung::Params::lade(&wurzel).unwrap();
    /// assert_eq!(p.arbeitszimmer(domain::Vz::Vz2025).unwrap().jahrespauschale, domain::Euro::new(1260));
    /// ```
    pub fn arbeitszimmer(&self, vz: Vz) -> Result<ArbeitszimmerSaetze, ParamsWertFehler> {
        let d = "arbeitszimmer_homeoffice.yaml";
        Ok(ArbeitszimmerSaetze {
            jahrespauschale: self.euro(vz, d, "jahrespauschale")?,
            tagespauschale_pro_tag: self.euro(vz, d, "tagespauschale_pro_tag")?,
            tagespauschale_hoechstbetrag: self.euro(vz, d, "tagespauschale_hoechstbetrag")?,
        })
    }

    /// § 9 Abs. 1 S. 3 Nr. 4 `EStG` (`entfernungspauschale.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// use rust_decimal::Decimal;
    /// let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    /// let p = bindung::Params::lade(&wurzel).unwrap();
    /// assert_eq!(p.entfernungspauschale(domain::Vz::Vz2024).unwrap().satz_bis_20_km.get(), Decimal::new(30, 2));
    /// ```
    pub fn entfernungspauschale(
        &self,
        vz: Vz,
    ) -> Result<EntfernungspauschaleSaetze, ParamsWertFehler> {
        let d = "entfernungspauschale.yaml";
        Ok(EntfernungspauschaleSaetze {
            satz_bis_20_km: Satz::new(self.dezimal(vz, d, "satz_bis_20_km")?),
            satz_ab_21_km: Satz::new(self.dezimal(vz, d, "satz_ab_21_km")?),
            staffelgrenze_km: self.ganz(vz, d, "staffelgrenze_km")?,
            hoechstbetrag_ohne_kfz: self.euro(vz, d, "hoechstbetrag_ohne_kfz")?,
        })
    }

    /// § 9 Abs. 1 S. 3 Nr. 5 `EStG` (`dhf_p9_1_nr5.yaml`).
    ///
    /// `cap_monat_ausland` ist `None`, wenn die Datei keine Auslandsgrenze fuehrt: VZ 2024 und 2025.
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`]; zusaetzlich bei einem `cap_monat_ausland`, das kein Block
    /// ist oder dessen `wert` keine ganze Zahl ist.
    ///
    /// ```
    /// let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    /// let p = bindung::Params::lade(&wurzel).unwrap();
    /// assert_eq!(p.dhf(domain::Vz::Vz2026).unwrap().cap_monat_inland, domain::Euro::new(1000));
    /// assert_eq!(p.dhf(domain::Vz::Vz2026).unwrap().cap_monat_ausland, Some(domain::Euro::new(2000)));
    /// assert_eq!(p.dhf(domain::Vz::Vz2025).unwrap().cap_monat_ausland, None);
    /// ```
    pub fn dhf(&self, vz: Vz) -> Result<DhfGrenzen, ParamsWertFehler> {
        let d = "dhf_p9_1_nr5.yaml";
        // Die Inlandsgrenze fehlt nie still (Python: KeyError); nur die Auslandsgrenze darf fehlen.
        let cap_monat_ausland = match self.datei(vz, d)?.werte.get("cap_monat_ausland") {
            None | Some(Value::Null) => None,
            Some(block) if block.is_mapping() => match block.get("wert") {
                None | Some(Value::Null) => None,
                Some(_) => Some(self.euro(vz, d, "cap_monat_ausland")?),
            },
            Some(_) => return Err(typ(vz, d, "cap_monat_ausland", "Block")),
        };
        Ok(DhfGrenzen {
            cap_monat_inland: self.euro(vz, d, "cap_monat_inland")?,
            cap_monat_ausland,
        })
    }

    /// § 9 Abs. 4a `EStG` (`verpflegung_p9_4a.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    /// let p = bindung::Params::lade(&wurzel).unwrap();
    /// assert_eq!(p.verpflegung(domain::Vz::Vz2025).unwrap().pauschale_24h, domain::Euro::new(28));
    /// ```
    pub fn verpflegung(&self, vz: Vz) -> Result<VerpflegungSaetze, ParamsWertFehler> {
        let d = "verpflegung_p9_4a.yaml";
        let optional = |k: &str| -> Result<i64, ParamsWertFehler> {
            match self.wert(vz, d, k) {
                Ok(_) => self.ganz(vz, d, k),
                Err(ParamsWertFehler::SchluesselFehlt { .. }) => Ok(0),
                Err(e) => Err(e),
            }
        };
        Ok(VerpflegungSaetze {
            pauschale_24h: self.euro(vz, d, "pauschale_24h")?,
            pauschale_an_abreise: self.euro(vz, d, "pauschale_an_abreise")?,
            pauschale_ab_8h: self.euro(vz, d, "pauschale_ab_8h")?,
            kuerzung_fruehstueck_prozent: optional("kuerzung_fruehstueck_prozent")?,
            kuerzung_mittag_abend_prozent: optional("kuerzung_mittag_abend_prozent")?,
        })
    }

    /// § 24a S. 5 `EStG` Kohortenzeile fuer das massgebende Folgejahr. Ausserhalb der Tabelle
    /// geklemmt (vor 2005 -> Hoechststaffel, ab 2058 -> letzte Zeile), wie
    /// `_altersentlastung_kohorte`.
    ///
    /// # Errors
    /// [`ParamsWertFehler`], wenn die Tabelle fehlt, leer ist oder eine Zeile unlesbar ist.
    ///
    /// ```
    /// use rust_decimal::Decimal;
    /// let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    /// let p = bindung::Params::lade(&wurzel).unwrap();
    /// let k = p.altersentlastung_kohorte(2025).unwrap();
    /// assert_eq!((k.prozentsatz.get(), k.hoechstbetrag), (Decimal::new(132, 1), domain::Euro::new(627)));
    /// assert_eq!(p.altersentlastung_kohorte(1990).unwrap().hoechstbetrag, domain::Euro::new(1900));
    /// ```
    pub fn altersentlastung_kohorte(
        &self,
        folgejahr: i64,
    ) -> Result<AltersentlastungKohorte, ParamsWertFehler> {
        let d = "altersentlastungsbetrag_p24a.yaml";
        let fehlt = |k: &str| ParamsWertFehler::SchluesselFehlt {
            datei: format!("kohorten/{d}"),
            schluessel: k.to_string(),
        };
        let tabelle = self
            .kohorten
            .get(d)
            .and_then(|k| k.koerper.get("kohorten"))
            .and_then(Value::as_mapping)
            .ok_or_else(|| fehlt("kohorten"))?;
        let jahre: Vec<i64> = tabelle.keys().filter_map(Value::as_i64).collect();
        let (Some(min), Some(max)) = (jahre.iter().min(), jahre.iter().max()) else {
            return Err(fehlt("kohorten.<jahr>"));
        };
        let j = folgejahr.clamp(*min, *max);
        let zeile = tabelle
            .get(Value::from(j))
            .ok_or_else(|| fehlt(&format!("kohorten.{j}")))?;
        let feld = |k: &str| {
            zeile
                .get(k)
                .ok_or_else(|| fehlt(&format!("kohorten.{j}.{k}")))
        };
        let typfehler = |k: &str, erwartet| ParamsWertFehler::Typ {
            datei: format!("kohorten/{d}"),
            schluessel: format!("kohorten.{j}.{k}"),
            erwartet,
        };
        Ok(AltersentlastungKohorte {
            prozentsatz: dezimalzahl(feld("prozentsatz")?)
                .map(Satz::new)
                .ok_or_else(|| typfehler("prozentsatz", "Dezimalzahl"))?,
            hoechstbetrag: ganzzahl(feld("hoechstbetrag")?)
                .map(Euro::new)
                .ok_or_else(|| typfehler("hoechstbetrag", "ganze Zahl"))?,
        })
    }
}

/// Zugriffe der Accessoren ab `catala_sparer_pb` (`engine::zugriff::teil2`).
impl Params {
    /// `p[schluessel]` einer Jahresdatei ohne `wert`-Huelle (z. B. `pauschale_900: 900`).
    fn oben(&self, vz: Vz, datei: &str, schluessel: &str) -> Result<&Value, ParamsWertFehler> {
        self.datei(vz, datei)?.werte.get(schluessel).ok_or_else(|| {
            ParamsWertFehler::SchluesselFehlt {
                datei: format!("{}/{datei}", vz.jahr()),
                schluessel: schluessel.to_string(),
            }
        })
    }

    fn oben_euro(&self, vz: Vz, datei: &str, schluessel: &str) -> Result<Euro, ParamsWertFehler> {
        ganzzahl(self.oben(vz, datei, schluessel)?)
            .map(Euro::new)
            .ok_or_else(|| ParamsWertFehler::Typ {
                datei: format!("{}/{datei}", vz.jahr()),
                schluessel: schluessel.to_string(),
                erwartet: "ganze Zahl",
            })
    }

    /// Staffel `{int: int}` ohne `wert`-Huelle.
    fn oben_staffel(
        &self,
        vz: Vz,
        datei: &str,
        schluessel: &str,
    ) -> Result<BTreeMap<i64, Euro>, ParamsWertFehler> {
        let typfehler = || ParamsWertFehler::Typ {
            datei: format!("{}/{datei}", vz.jahr()),
            schluessel: schluessel.to_string(),
            erwartet: "Staffel {ganze Zahl: ganze Zahl}",
        };
        let map = self
            .oben(vz, datei, schluessel)?
            .as_mapping()
            .ok_or_else(typfehler)?;
        map.iter()
            .map(|(k, v)| {
                Ok((
                    k.as_i64().ok_or_else(typfehler)?,
                    Euro::new(ganzzahl(v).ok_or_else(typfehler)?),
                ))
            })
            .collect()
    }

    /// Tabelle `kohorten` einer Kohortendatei.
    fn kohorten_tabelle(&self, datei: &str) -> Result<&serde_yaml_ng::Mapping, ParamsWertFehler> {
        self.kohorten
            .get(datei)
            .and_then(|k| k.koerper.get("kohorten"))
            .and_then(Value::as_mapping)
            .ok_or_else(|| ParamsWertFehler::SchluesselFehlt {
                datei: format!("kohorten/{datei}"),
                schluessel: "kohorten".to_string(),
            })
    }

    /// `kohorten[schluessel][feld]` als Dezimalzahl; `Ok(None)`, wenn die Zeile fehlt (Python
    /// `KeyError` -- der Aufrufer entscheidet).
    fn kohorten_dezimal(
        &self,
        datei: &str,
        schluessel: i64,
        feld: &str,
    ) -> Result<Option<Decimal>, ParamsWertFehler> {
        let Some(zeile) = self.kohorten_tabelle(datei)?.get(Value::from(schluessel)) else {
            return Ok(None);
        };
        zeile
            .get(feld)
            .and_then(dezimalzahl)
            .map(Some)
            .ok_or_else(|| ParamsWertFehler::Typ {
                datei: format!("kohorten/{datei}"),
                schluessel: format!("kohorten.{schluessel}.{feld}"),
                erwartet: "Dezimalzahl",
            })
    }

    /// § 20 Abs. 9 S. 1 `EStG` Sparer-Pauschbetrag je Person (`sparer_pauschbetrag_p20_9.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.sparer_pauschbetrag(domain::Vz::Vz2025).unwrap(), domain::Euro::new(1000));
    /// ```
    pub fn sparer_pauschbetrag(&self, vz: Vz) -> Result<Euro, ParamsWertFehler> {
        self.euro(vz, "sparer_pauschbetrag_p20_9.yaml", "wert")
    }

    /// § 32d Abs. 1 S. 1 `EStG` Abgeltungsteuersatz in Prozent (`abgeltungssatz_p32d.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.abgeltungssatz_prozent(domain::Vz::Vz2025).unwrap(), 25);
    /// ```
    pub fn abgeltungssatz_prozent(&self, vz: Vz) -> Result<i64, ParamsWertFehler> {
        self.ganz(vz, "abgeltungssatz_p32d.yaml", "wert")
    }

    /// § 9a S. 1 Nr. 3 `EStG` Werbungskosten-Pauschbetrag Renten
    /// (`renten_werbungskostenpauschbetrag_p9a.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.renten_wk_pauschbetrag(domain::Vz::Vz2025).unwrap(), domain::Euro::new(102));
    /// ```
    pub fn renten_wk_pauschbetrag(&self, vz: Vz) -> Result<Euro, ParamsWertFehler> {
        self.euro(vz, "renten_werbungskostenpauschbetrag_p9a.yaml", "wert")
    }

    /// § 33b `EStG` Pauschbetrag-Tabellen (`behinderten_pauschbetrag_p33b.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// let t = p.p33b_pauschbetraege(domain::Vz::Vz2025).unwrap();
    /// assert_eq!(t.gdb_staffel.get(&50), Some(&domain::Euro::new(1140)));
    /// ```
    pub fn p33b_pauschbetraege(&self, vz: Vz) -> Result<P33bPauschbetraege, ParamsWertFehler> {
        let d = "behinderten_pauschbetrag_p33b.yaml";
        Ok(P33bPauschbetraege {
            gdb_staffel: self.oben_staffel(vz, d, "gdb_staffel")?,
            blind_hilflos_taubblind: self.oben_euro(vz, d, "blind_hilflos_taubblind")?,
            hinterbliebenen: self.oben_euro(vz, d, "hinterbliebenen")?,
            pflege_staffel: self.oben_staffel(vz, d, "pflege_staffel")?,
            pflege_hilflos: self.oben_euro(vz, d, "pflege_hilflos")?,
        })
    }

    /// § 33 Abs. 2a `EStG` Fahrtkostenpauschalen (`fahrtkostenpauschale_p33_2a.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.fahrtkostenpauschale_p33_2a(domain::Vz::Vz2025).unwrap().pauschale_4500, domain::Euro::new(4500));
    /// ```
    pub fn fahrtkostenpauschale_p33_2a(
        &self,
        vz: Vz,
    ) -> Result<FahrtkostenPauschalen, ParamsWertFehler> {
        let d = "fahrtkostenpauschale_p33_2a.yaml";
        Ok(FahrtkostenPauschalen {
            pauschale_900: self.oben_euro(vz, d, "pauschale_900")?,
            pauschale_4500: self.oben_euro(vz, d, "pauschale_4500")?,
        })
    }

    fn satz_hoechstbetrag(&self, vz: Vz, d: &str) -> Result<SatzHoechstbetrag, ParamsWertFehler> {
        Ok(SatzHoechstbetrag {
            abzugssatz: Satz::new(self.dezimal(vz, d, "abzugssatz")?),
            hoechstbetrag_je_kind: self.euro(vz, d, "hoechstbetrag_je_kind")?,
        })
    }

    /// § 10 Abs. 1 Nr. 5 `EStG` Kinderbetreuung (`kinderbetreuung_p10.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.kinderbetreuung(domain::Vz::Vz2025).unwrap().abzugssatz.get(), rust_decimal::Decimal::new(8, 1));
    /// ```
    pub fn kinderbetreuung(&self, vz: Vz) -> Result<SatzHoechstbetrag, ParamsWertFehler> {
        self.satz_hoechstbetrag(vz, "kinderbetreuung_p10.yaml")
    }

    /// § 10 Abs. 1 Nr. 9 `EStG` Schulgeld (`schulgeld_p10.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.schulgeld(domain::Vz::Vz2025).unwrap().hoechstbetrag_je_kind, domain::Euro::new(2500));
    /// ```
    pub fn schulgeld(&self, vz: Vz) -> Result<SatzHoechstbetrag, ParamsWertFehler> {
        self.satz_hoechstbetrag(vz, "schulgeld_p10.yaml")
    }

    /// § 34g Satz 2 `EStG` Parteispenden (`parteispenden_p34g.yaml`): Satz und Hoechstbetraege der Ermaessigung, dazu die
    /// Deckel des Sonderausgabenabzugs nach § 10b Abs. 2. Ein Jahr ohne diese Datei ist ein Fehler
    /// ([`ParamsWertFehler::DateiFehlt`]), kein Standardwert: ohne belegte Fassung rechnet der Aufrufer nicht (Vault:
    /// `decisions/parteispenden-deckel-kommt-je-jahr-aus-der-eingefrorenen-fassung`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.parteispenden_p34g(domain::Vz::Vz2025).unwrap().hoechstbetrag_einzel, domain::Euro::new(825));
    /// assert_eq!(p.parteispenden_p34g(domain::Vz::Vz2026).unwrap().hoechstbetrag_zusammen, domain::Euro::new(3300));
    /// assert_eq!(p.parteispenden_p34g(domain::Vz::Vz2025).unwrap().sonderausgaben_hoechstbetrag_einzel, domain::Euro::new(1650));
    /// assert_eq!(p.parteispenden_p34g(domain::Vz::Vz2026).unwrap().sonderausgaben_hoechstbetrag_zusammen, domain::Euro::new(6600));
    /// ```
    pub fn parteispenden_p34g(
        &self,
        vz: Vz,
    ) -> Result<ParteispendenErmaessigung, ParamsWertFehler> {
        let d = "parteispenden_p34g.yaml";
        Ok(ParteispendenErmaessigung {
            satz: Satz::new(self.dezimal(vz, d, "ermaessigungssatz")?),
            hoechstbetrag_einzel: self.euro(vz, d, "hoechstbetrag_einzel")?,
            hoechstbetrag_zusammen: self.euro(vz, d, "hoechstbetrag_zusammen")?,
            sonderausgaben_hoechstbetrag_einzel: self.euro(vz, d, "sonderausgaben_hoechstbetrag_einzel")?,
            sonderausgaben_hoechstbetrag_zusammen: self.euro(vz, d, "sonderausgaben_hoechstbetrag_zusammen")?,
        })
    }

    /// § 66 `EStG` Kindergeld je Kind und Monat (`kindergeld_p66.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.kindergeld_monatlich_je_kind(domain::Vz::Vz2025).unwrap(), domain::Euro::new(255));
    /// ```
    pub fn kindergeld_monatlich_je_kind(&self, vz: Vz) -> Result<Euro, ParamsWertFehler> {
        self.euro(vz, "kindergeld_p66.yaml", "kindergeld_monatlich_je_kind")
    }

    /// § 32 Abs. 6 `EStG` Kinderfreibetrag JE ELTERNTEIL und Kind: saechliches Existenzminimum +
    /// BEA-Freibetrag (`kinderfreibetrag_p32.yaml`), Python `runner._kinderfreibetrag` vor der
    /// Verdopplung bei Zusammenveranlagung.
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`]; dazu ein `i64`-Ueberlauf der Summe (als `Typ`-Fehler).
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.kinderfreibetrag_je_elternteil(domain::Vz::Vz2025).unwrap(), domain::Euro::new(3336 + 1464));
    /// ```
    pub fn kinderfreibetrag_je_elternteil(&self, vz: Vz) -> Result<Euro, ParamsWertFehler> {
        let d = "kinderfreibetrag_p32.yaml";
        let a = self.euro(vz, d, "kinderfreibetrag_je_elternteil")?;
        let b = self.euro(vz, d, "bea_freibetrag_je_elternteil")?;
        a.get()
            .checked_add(b.get())
            .map(Euro::new)
            .ok_or_else(|| typ(vz, d, "kinderfreibetrag_je_elternteil", "Summe in i64"))
    }

    /// § 10 Abs. 3 `EStG` Vorsorge-Hoechstbeitrag (`vorsorge_hoechstbetrag_p10.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.vorsorge_hoechstbeitrag(domain::Vz::Vz2025).unwrap(), domain::Euro::new(29_344));
    /// ```
    pub fn vorsorge_hoechstbeitrag(&self, vz: Vz) -> Result<Euro, ParamsWertFehler> {
        self.euro(vz, "vorsorge_hoechstbetrag_p10.yaml", "hoechstbeitrag")
    }

    /// § 10c S. 1 `EStG` Sonderausgaben-Pauschbetrag je Person (`sonderausgabenpauschbetrag.yaml`).
    ///
    /// # Errors
    /// Wie [`Params::grundfreibetrag`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.sonderausgaben_pauschbetrag(domain::Vz::Vz2025).unwrap(), domain::Euro::new(36));
    /// ```
    pub fn sonderausgaben_pauschbetrag(&self, vz: Vz) -> Result<Euro, ParamsWertFehler> {
        self.euro(vz, "sonderausgabenpauschbetrag.yaml", "wert")
    }

    /// § 22 Nr. 1 S. 3 a aa `EStG` Besteuerungsanteil in Prozent je Rentenbeginn-Jahr
    /// (`rente_besteuerungsanteil_p22.yaml`); `None` ausserhalb der Tabelle.
    ///
    /// # Errors
    /// [`ParamsWertFehler`], wenn die Tabelle fehlt oder eine Zeile unlesbar ist.
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.rente_besteuerungsanteil(2025).unwrap(), Some(domain::Satz::new(rust_decimal::Decimal::new(835, 1))));
    /// assert_eq!(p.rente_besteuerungsanteil(2004).unwrap(), None);
    /// ```
    pub fn rente_besteuerungsanteil(&self, jahr: i64) -> Result<Option<Satz>, ParamsWertFehler> {
        self.kohorten_dezimal(
            "rente_besteuerungsanteil_p22.yaml",
            jahr,
            "besteuerungsanteil_prozent",
        )
        .map(|s| s.map(Satz::new))
    }

    /// § 22 Nr. 1 S. 3 a bb `EStG` Ertragsanteil in Prozent je Alter bei Rentenbeginn
    /// (`rente_ertragsanteil_p22.yaml`); `None` ausserhalb der Tabelle.
    ///
    /// # Errors
    /// Wie [`Params::rente_besteuerungsanteil`].
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.rente_ertragsanteil(0).unwrap(), Some(domain::Satz::new(rust_decimal::Decimal::new(59, 0))));
    /// ```
    pub fn rente_ertragsanteil(&self, alter: i64) -> Result<Option<Satz>, ParamsWertFehler> {
        self.kohorten_dezimal(
            "rente_ertragsanteil_p22.yaml",
            alter,
            "ertragsanteil_prozent",
        )
        .map(|s| s.map(Satz::new))
    }

    /// § 19 Abs. 2 S. 3 `EStG` Kohortenzeile je Versorgungsbeginn, ausserhalb der Tabelle
    /// geklemmt wie `_versorgungsfreibetrag_kohorte`.
    ///
    /// # Errors
    /// [`ParamsWertFehler`], wenn die Tabelle fehlt, leer ist oder eine Zeile unlesbar ist.
    ///
    /// ```
    /// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    /// assert_eq!(p.versorgungsfreibetrag_kohorte(2025).unwrap().zuschlag, domain::Euro::new(297));
    /// assert_eq!(p.versorgungsfreibetrag_kohorte(1990).unwrap().hoechstbetrag, domain::Euro::new(3000));
    /// ```
    pub fn versorgungsfreibetrag_kohorte(
        &self,
        beginn: i64,
    ) -> Result<VersorgungsfreibetragKohorte, ParamsWertFehler> {
        let d = "versorgungsfreibetrag_p19_2.yaml";
        let tabelle = self.kohorten_tabelle(d)?;
        let jahre: Vec<i64> = tabelle.keys().filter_map(Value::as_i64).collect();
        let fehlt = |k: String| ParamsWertFehler::SchluesselFehlt {
            datei: format!("kohorten/{d}"),
            schluessel: k,
        };
        let (Some(min), Some(max)) = (jahre.iter().min(), jahre.iter().max()) else {
            return Err(fehlt("kohorten.<jahr>".to_string()));
        };
        let j = beginn.clamp(*min, *max);
        let zeile = tabelle
            .get(Value::from(j))
            .ok_or_else(|| fehlt(format!("kohorten.{j}")))?;
        let typfehler = |k: &str, erwartet| ParamsWertFehler::Typ {
            datei: format!("kohorten/{d}"),
            schluessel: format!("kohorten.{j}.{k}"),
            erwartet,
        };
        let euro = |k: &str| {
            zeile
                .get(k)
                .and_then(ganzzahl)
                .map(Euro::new)
                .ok_or_else(|| typfehler(k, "ganze Zahl"))
        };
        Ok(VersorgungsfreibetragKohorte {
            prozentsatz: zeile
                .get("prozentsatz")
                .and_then(dezimalzahl)
                .map(Satz::new)
                .ok_or_else(|| typfehler("prozentsatz", "Dezimalzahl"))?,
            hoechstbetrag: euro("hoechstbetrag")?,
            zuschlag: euro("zuschlag")?,
        })
    }
}

fn typ(vz: Vz, datei: &str, schluessel: &str, erwartet: &'static str) -> ParamsWertFehler {
    ParamsWertFehler::Typ {
        datei: format!("{}/{datei}", vz.jahr()),
        schluessel: format!("{schluessel}.wert"),
        erwartet,
    }
}

/// Ganze Zahl; eine Gleitkommazahl nur ohne Nachkommaanteil. Python schneidet an den
/// `int(...)`-Stellen einen Bruchteil still ab -- hier ist er ein Fehler (fail-closed). Heute
/// traegt kein gelesener Schluessel einen Bruchteil.
fn ganzzahl(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| {
        let d = dezimalzahl(v)?;
        if d.fract().is_zero() {
            i64::try_from(d).ok()
        } else {
            None
        }
    })
}

/// Zahl als `Decimal` ueber die kuerzeste Dezimaldarstellung (Pythons `str(float)`).
fn dezimalzahl(v: &Value) -> Option<Decimal> {
    if let Some(i) = v.as_i64() {
        return Some(Decimal::from(i));
    }
    let f = v.as_f64()?;
    Decimal::from_str(&f.to_string()).ok()
}

fn yaml_dateien(dir: &Path) -> Result<Vec<PathBuf>, ParamsWertFehler> {
    let io = |e: std::io::Error| ParamsFehler::Io {
        pfad: dir.to_path_buf(),
        nachricht: e.to_string(),
    };
    let mut out = Vec::new();
    for eintrag in std::fs::read_dir(dir).map_err(io)? {
        let pfad = eintrag.map_err(io)?.path();
        if pfad.extension().is_some_and(|e| e == "yaml") {
            out.push(pfad);
        }
    }
    out.sort();
    Ok(out)
}

fn dateiname(pfad: &Path) -> String {
    pfad.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::Params;
    use domain::{Euro, Vz};
    use rust_decimal::Decimal;

    fn params() -> Params {
        Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
    }

    #[test]
    fn saetze_2026_entfernung() {
        let s = params().entfernungspauschale(Vz::Vz2026).unwrap();
        assert_eq!(s.satz_bis_20_km.get(), Decimal::new(38, 2));
        assert_eq!(s.staffelgrenze_km, 20);
        assert_eq!(s.hoechstbetrag_ohne_kfz, Euro::new(4500));
    }

    #[test]
    fn kohorte_ueber_tabellenende_geklemmt() {
        let p = params();
        assert_eq!(
            p.altersentlastung_kohorte(3000).unwrap(),
            p.altersentlastung_kohorte(2058).unwrap()
        );
    }

    #[test]
    fn verpflegung_kuerzungssaetze() {
        let v = params().verpflegung(Vz::Vz2024).unwrap();
        assert_eq!(
            (
                v.kuerzung_fruehstueck_prozent,
                v.kuerzung_mittag_abend_prozent
            ),
            (20, 40)
        );
    }

    /// Die 2.000-EUR-Auslandsgrenze gilt erst ab VZ 2026 (`StÄndG` 2025, `BGBl`. 2025 I Nr. 363);
    /// `params/2024` und `params/2025` fuehren sie nicht. Die Inlandsgrenze ist in allen drei 1.000.
    /// Gegenstueck: Python `runner._dhf_params` (`tests/test_werbungskosten_n.py`).
    #[test]
    fn dhf_auslandsgrenze_gilt_erst_ab_vz2026() {
        let p = params();
        for (vz, ausland) in [
            (Vz::Vz2024, None),
            (Vz::Vz2025, None),
            (Vz::Vz2026, Some(Euro::new(2000))),
        ] {
            let g = p.dhf(vz).unwrap();
            assert_eq!(g.cap_monat_inland, Euro::new(1000));
            assert_eq!(g.cap_monat_ausland, ausland);
        }
    }

    /// `params` mit einer ersetzten `dhf_p9_1_nr5.yaml` fuer VZ 2025: Kopf wie die echte Datei,
    /// der Rest aus `rumpf`.
    fn mit_dhf(rumpf: &str) -> Params {
        let kopf = "parameter: dhf_p9_1_nr5\nveranlagungszeitraum: 2025\nauthority: gesetz\n\
                    redistributable: true\ngueltig_ab: \"2025-01-01\"\n";
        let datei = serde_yaml_ng::from_str(&format!("{kopf}{rumpf}")).unwrap();
        let mut p = params();
        p.jahre
            .insert((2025, "dhf_p9_1_nr5.yaml".to_string()), datei);
        p
    }

    const INLAND: &str = "cap_monat_inland:\n  wert: 1000\n";

    /// Fehlender Schluessel, leerer Block, leerer Wert: alle drei heissen "keine Grenze" (`None`),
    /// weder Fehler noch 0. Python `(p.get("cap_monat_ausland") or {}).get("wert")` liest sie gleich.
    #[test]
    fn dhf_fehlende_auslandsgrenze_ist_keine_grenze_kein_fehler() {
        for ausland in [
            "",
            "cap_monat_ausland:\n",
            "cap_monat_ausland: {}\n",
            "cap_monat_ausland:\n  wert:\n",
            "cap_monat_ausland:\n  einheit: euro_je_monat\n",
        ] {
            let g = mit_dhf(&format!("{INLAND}{ausland}"))
                .dhf(Vz::Vz2025)
                .unwrap();
            assert_eq!(g.cap_monat_inland, Euro::new(1000), "{ausland:?}");
            assert_eq!(g.cap_monat_ausland, None, "{ausland:?}");
        }
    }

    /// Eine vorhandene Auslandsgrenze wird gelesen -- 2026 traegt sie, und ein spaeteres Wiederauftauchen
    /// in einer anderen Datei darf nicht still verschluckt werden.
    #[test]
    fn dhf_vorhandene_auslandsgrenze_wird_gelesen() {
        let g = mit_dhf(&format!("{INLAND}cap_monat_ausland:\n  wert: 2000\n"))
            .dhf(Vz::Vz2025)
            .unwrap();
        assert_eq!(g.cap_monat_ausland, Some(Euro::new(2000)));
    }

    /// Kaputte Daten sind Fehler, keine "keine Grenze": ein Auslandsschluessel, der kein Block ist,
    /// oder ein `wert` ohne ganze Zahl. Und die Inlandsgrenze darf nie still fehlen.
    #[test]
    fn dhf_kaputte_daten_sind_fehler_nicht_keine_grenze() {
        for ausland in [
            "cap_monat_ausland: 2000\n",
            "cap_monat_ausland:\n  wert: zweitausend\n",
        ] {
            assert!(
                mit_dhf(&format!("{INLAND}{ausland}"))
                    .dhf(Vz::Vz2025)
                    .is_err(),
                "{ausland:?}"
            );
        }
        assert!(mit_dhf("").dhf(Vz::Vz2025).is_err());
    }
}
