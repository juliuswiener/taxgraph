# Nicht-Saettigungs-Steuer 2026-07-11 — EIN Review

Nach Daempfung 2a (unabhaengige Norm-Teile -> Formalisierungs-Backlog, mechanisch). 
Detektor schlaegt vor, die Ratsche schreibt erst nach deiner Zeile. Vage Passung = Reject 
(-> zurueck auf offen). Backlog-Items brauchen keinen Entscheid.


**Summe: 82 Bedingung-, 19 Konvention-Vorschlaege · 49 offen · 23 Backlog (auto).**


## p24b_entlastungsbetrag
_konv 3 · bedingung 14 · offen 12 · backlog 2 (auto)_

### Det→Konvention (bestaetige alle / streiche Zeile)
| # | Anker | konv-ID | Zitat |
|--|--|--|--|
| 1 | `ergebnis / rundung` | `konv:keine_zusaetzliche_rundung` | Die Berechnung verwendet Gleitkommadivision ohne Rundungsregel, was zu Bruchteilen von Cen |
| 2 | `monate_ohne_voraussetzung / sonstige` | `konv:ganzzahl_monate` | The input monate_ohne_voraussetzung is assumed to be between 0 and 12 inclusive, as the no |
| 3 | `ergebnis / rundung` | `konv:keine_zusaetzliche_rundung` | Die Formalisierung rundet das Ergebnis nicht, obwohl die Norm keine Rundungsregel enthält  |

### Det→Bedingung (bestaetige zeilenweise; nicht bestaetigt -> offen)

**`alleinstehend_im_sinne_des_absatzes_3`** — _Allein stehend im Sinne des Absatzes 1 sind Steuerpflichtige_
| # | Anker | Zitat |
|--|--|--|
| 1 | `alleinstehend / interpretation` | Der boolesche Wert wird als Erfüllung der Definition des Alleinstehenden nach § 24b Abs. 3 |
| 2 | `alleinstehend / interpretation` | Der boolesche Wert 'alleinstehend' wird als korrekte Umsetzung der komplexen Definition in |
| 3 | `alleinstehend / interpretation` | Die Formalisierung setzt voraus, dass alleinstehend bereits gemäß §24b Abs. 3 EStG bestimm |
| 4 | `alleinstehend / interpretation` | The boolean input alleinstehend is assumed to correctly reflect the legal definition of 'a |
| 5 | `§ 24b abs. 3` | Allein stehend im Sinne des Absatzes 1 sind Steuerpflichtige, die nicht die Voraussetzunge |

**`kinder_mit_freibetrag_oder_kindergeld_im_haushalt`** — _Freibetrag nach § 32 Absatz 6 oder Kindergeld zusteht_
| # | Anker | Zitat |
|--|--|--|
| 1 | `anzahl_kinder / interpretation` | Die Anzahl der Kinder wird als Anzahl der Kinder interpretiert, die die Voraussetzungen de |
| 2 | `anzahl_kinder / interpretation` | Die Formalisierung nimmt an, dass anzahl_kinder ausschließlich Kinder zählt, für die ein F |
| 3 | `anzahl_kinder / interpretation` | The integer anzahl_kinder is assumed to count only children who meet the criteria of § 24b |
| 4 | `§ 24b abs. 1 satz 1` | wenn zu ihrem Haushalt mindestens ein Kind gehört, für das ihnen ein Freibetrag nach § 32  |
| 5 | `§ 24b abs. 1 s. 1` | wenn zu ihrem Haushalt mindestens ein Kind gehört, für das ihnen ein Freibetrag nach § 32  |

**`monate_ohne_voraussetzung_sind_volle_kalendermonate`** — _Für jeden vollen Kalendermonat, in dem die Voraussetzungen des Absatzes 1 nicht vorgelegen haben_
| # | Anker | Zitat |
|--|--|--|
| 1 | `monate_ohne_voraussetzung / zeitbezug` | Die Anzahl wird als Anzahl der vollen Kalendermonate ohne Voraussetzungen des Abs. 1 geles |
| 2 | `monate_ohne_voraussetzung / interpretation` | Die Anzahl der Monate ohne Voraussetzungen wird als korrekte Zählung voller Kalendermonate |
| 3 | `monate_ohne_voraussetzung / interpretation` | Die Formalisierung geht davon aus, dass monate_ohne_voraussetzung die Anzahl der vollen Ka |
| 4 | `monate_ohne_voraussetzung / interpretation` | The integer monate_ohne_voraussetzung is assumed to correctly count the full calendar mont |

### Offen — Triage noetig
| # | Art | Anker | Zitat |
|--|--|--|--|
| 1 | abweichung | `?` | Bei mehr als 12 Monaten ohne Voraussetzungen wird der Entlastungsbetrag negativ, obwohl di |
| 2 | abweichung | `?` | Die Berechnung kann zu einem negativen Entlastungsbetrag führen, wenn monate_ohne_vorausse |
| 3 | abweichung | `?` | Die Formalisierung kappt monate_ohne_voraussetzung nicht auf 12, was bei Werten über 12 zu |
| 4 | abweichung | `?` | Die Formalisierung lässt Werte über 12 zu und erzeugt dann einen negativen Entlastungsbetr |
| 5 | annahme | `ergebnis / rundung` | Das Ergebnis der Berechnung wird kaufmännisch auf Cent gerundet, obwohl die Norm keine Run |
| 6 | annahme | `ergebnis / rundung` | Die Formalisierung verwendet Dezimalarithmetik ohne explizite Rundungsregel, was implizit  |
| 7 | annahme | `ergebnis / rundung` | The computation uses decimal arithmetic without explicit rounding, assuming that the resul |
| 8 | annahme | `anzahl_kinder / sonstige` | Negative values for anzahl_kinder are treated as zero, which is not specified in the norm. |
| 9 | norm_teil | `§ 24b abs. 1 satz 2` | Die Zugehörigkeit zum Haushalt ist anzunehmen, wenn das Kind in der Wohnung des allein ste |
| 10 | norm_teil | `§ 24b abs. 1 s. 2` | Die Zugehörigkeit zum Haushalt ist anzunehmen, wenn das Kind in der Wohnung des allein ste |
| 11 | norm_teil | `§ 24b abs. 4` | Für jeden vollen Kalendermonat, in dem die Voraussetzungen des Absatzes 1 nicht vorgelegen |
| 12 | norm_teil | `§ 24b abs. 2` | beträgt der Entlastungsbetrag im Kalenderjahr 4 260 Euro |

## p10_1_7_berufsausbildung
_konv 5 · bedingung 1 · offen 0 · backlog 6 (auto)_

### Det→Konvention (bestaetige alle / streiche Zeile)
| # | Anker | konv-ID | Zitat |
|--|--|--|--|
| 1 | `aufwendungen / interpretation` | `konv:input_nur_etikettiertes` | Die Eingabe 'aufwendungen' entspricht den Aufwendungen im Sinne des § 10 Abs. 1 Nr. 7 EStG |
| 2 | `aufwendungen / zeitbezug` | `konv:vz_bezug_der_regel` | Die Eingabe bezieht sich auf ein Kalenderjahr. |
| 3 | `aufwendungen / interpretation` | `konv:input_nur_etikettiertes` | Die Eingabe wird als Aufwendungen für die eigene Berufsausbildung verstanden, ohne dass di |
| 4 | `aufwendungen / interpretation` | `konv:input_nur_etikettiertes` | Die Eingabe wird als bereits nach § 4 Abs. 5 und § 9 EStG ermittelte Aufwendungen behandel |
| 5 | `aufwendungen / zeitbezug` | `konv:vz_bezug_der_regel` | Die Eingabe wird als die Aufwendungen eines Kalenderjahres behandelt, ohne dass ein Zeitra |

### Det→Bedingung (bestaetige zeilenweise; nicht bestaetigt -> offen)

**`hoechstbetrag_gilt_je_person`** — _Bei Ehegatten, die die Voraussetzungen des § 26 Absatz 1 Satz 1 erfüllen, gilt Satz 1 für jeden Eheg_
| # | Anker | Zitat |
|--|--|--|
| 1 | `aufwendungen / geltungsvoraussetzung` | Es wird vorausgesetzt, dass die Ehegattenregelung des Satzes 2 nicht anwendbar ist oder be |

## p9_6_erstausbildung_abgrenzung
_konv 5 · bedingung 11 · offen 1 · backlog 1 (auto)_

### Det→Konvention (bestaetige alle / streiche Zeile)
| # | Anker | konv-ID | Zitat |
|--|--|--|--|
| 1 | `im_dienstverhaeltnis / interpretation` | `konv:input_nur_etikettiertes` | Die Eingabe 'im_dienstverhaeltnis' wird als zutreffende Feststellung interpretiert, dass d |
| 2 | `aufwendungen / interpretation` | `konv:input_nur_etikettiertes` | Die Eingabe 'aufwendungen' wird als Aufwendungen für die eigene Berufsausbildung oder das  |
| 3 | `im_dienstverhaeltnis / interpretation` | `konv:input_nur_etikettiertes` | Die Eingabe 'im_dienstverhaeltnis' wird als zutreffende Prüfung des Dienstverhältnisses vo |
| 4 | `im_dienstverhaeltnis / interpretation` | `konv:input_nur_etikettiertes` | The input 'im_dienstverhaeltnis' is assumed to correctly indicate that the education occur |
| 5 | `aufwendungen / interpretation` | `konv:input_nur_etikettiertes` | The input 'aufwendungen' is assumed to be the taxpayer's expenses for their own vocational |

### Det→Bedingung (bestaetige zeilenweise; nicht bestaetigt -> offen)

**`erstausbildung_nach_legaldefinition`** — _geordnete Ausbildung mit einer Mindestdauer_
| # | Anker | Zitat |
|--|--|--|
| 1 | `erstausbildung_abgeschlossen / interpretation` | Die Eingabe 'erstausbildung_abgeschlossen' wird als zutreffende rechtliche Feststellung ei |
| 2 | `erstausbildung_abgeschlossen / interpretation` | Die Eingabe 'erstausbildung_abgeschlossen' wird als bereits gemäß § 9 Abs. 6 Sätze 2-5 ESt |
| 3 | `erstausbildung_abgeschlossen / interpretation` | The boolean input 'erstausbildung_abgeschlossen' is assumed to already reflect the legal d |
| 4 | `erstausbildung_abgeschlossen / interpretation` | The input 'erstausbildung_abgeschlossen' is assumed to correctly indicate a prior complete |
| 5 | `§ 9 abs. 6 satz 2` | Eine Berufsausbildung als Erstausbildung nach Satz 1 liegt vor, wenn eine geordnete Ausbil |
| 6 | `§ 9 abs. 6 satz 3` | Eine geordnete Ausbildung liegt vor, wenn sie auf der Grundlage von Rechts- oder Verwaltun |
| 7 | `§ 9 abs. 6 satz 4` | Ist eine Abschlussprüfung nach dem Ausbildungsplan nicht vorgesehen, gilt die Ausbildung m |
| 8 | `§ 9 abs. 6 satz 5` | Eine Berufsausbildung als Erstausbildung hat auch abgeschlossen, wer die Abschlussprüfung  |
| 9 | `satz 2` | geordnete Ausbildung mit einer Mindestdauer von 12 Monaten |
| 10 | `satz 4` | Ist eine Abschlussprüfung nach dem Ausbildungsplan nicht vorgesehen |
| 11 | `satz 5` | wer die Abschlussprüfung einer durch Rechts- oder Verwaltungsvorschriften geregelten Beruf |

### Offen — Triage noetig
| # | Art | Anker | Zitat |
|--|--|--|--|
| 1 | norm_teil | `§ 9 abs. 6 satz 1` | Aufwendungen des Steuerpflichtigen für seine Berufsausbildung oder für sein Studium |

## p9_1_3_nr5_doppelte_haushaltsfuehrung
_konv 1 · bedingung 4 · offen 6 · backlog 6 (auto)_

### Det→Konvention (bestaetige alle / streiche Zeile)
| # | Anker | konv-ID | Zitat |
|--|--|--|--|
| 1 | `monate / zeitbezug` | `konv:ganzzahl_monate` | Die Formalisierung setzt voraus, dass die Unterkunft für eine ganzzahlige Anzahl von Monat |

### Det→Bedingung (bestaetige zeilenweise; nicht bestaetigt -> offen)

**`keine_verpflichtende_dienst_oder_werkswohnung`** — _die Grenze von 2 000 Euro bei einer Unterkunft im Ausland gilt nicht, wenn eine Dienst- oder Werkswo_
| # | Anker | Zitat |
|--|--|--|
| 1 | `ergebnis / geltungsvoraussetzung` | Die Formalisation setzt voraus, dass die Höchstgrenze von 2.000 Euro im Ausland stets gilt |
| 2 | `im_inland / geltungsvoraussetzung` | Die Formalisierung setzt voraus, dass die Voraussetzungen für die Ausnahme von der 2.000-E |
| 3 | `satz 4` | die Grenze von 2 000 Euro bei einer Unterkunft im Ausland gilt nicht, wenn eine Dienst- od |
| 4 | `satz 4, 2. halbsatz` | die Grenze von 2 000 Euro bei einer Unterkunft im Ausland gilt nicht, wenn eine Dienst- od |

### Offen — Triage noetig
| # | Art | Anker | Zitat |
|--|--|--|--|
| 1 | annahme | `monate / interpretation` | Die Formalisation legt die monatliche Höchstgrenze so aus, dass der Gesamtbetrag durch Mul |
| 2 | annahme | `monate / interpretation` | Die Formalisierung interpretiert die monatliche Höchstgrenze so, dass sie für jeden Monat  |
| 3 | annahme | `unterkunftskosten_monat / interpretation` | Die Formalisierung unterstellt, dass der Steuerpflichtige stets die tatsächlichen Aufwendu |
| 4 | annahme | `unterkunftskosten_monat / interpretation` | Die Formalisierung nimmt an, dass die monatlichen Unterkunftskosten über alle Monate konst |
| 5 | annahme | `monate / zeitbezug` | Die Norm nennt nur eine monatliche Höchstgrenze; die Formalisierung berechnet den Gesamtab |
| 6 | norm_teil | `satz 1` | notwendige Mehraufwendungen, die einem Arbeitnehmer wegen einer beruflich veranlassten dop |

## p9_4a_verpflegungsmehraufwand
_konv 3 · bedingung 15 · offen 16 · backlog 4 (auto)_

### Det→Konvention (bestaetige alle / streiche Zeile)
| # | Anker | konv-ID | Zitat |
|--|--|--|--|
| 1 | `abwesenheit_stunden / interpretation` | `konv:input_nur_etikettiertes` | Die Formalisierung setzt voraus, dass die Stundenanzahl die Abwesenheit von Wohnung und er |
| 2 | `monate_am_ort / geltungsvoraussetzung` | `konv:input_nur_etikettiertes` | Die Formalisierung setzt voraus, dass eine längerfristige Tätigkeit an derselben Tätigkeit |
| 3 | `monate_am_ort / interpretation` | `konv:input_nur_etikettiertes` | Die Formalisierung interpretiert 'monate_am_ort' als die Anzahl der Monate einer längerfri |

### Det→Bedingung (bestaetige zeilenweise; nicht bestaetigt -> offen)

**`auswaertige_berufliche_taetigkeit`** — _Wird der Arbeitnehmer außerhalb seiner Wohnung und ersten Tätigkeitsstätte beruflich tätig_
| # | Anker | Zitat |
|--|--|--|
| 1 | `ergebnis / geltungsvoraussetzung` | Die Formalisierung setzt voraus, dass eine auswärtige berufliche Tätigkeit vorliegt, ohne  |
| 2 | `satz 2` | außerhalb seiner Wohnung und ersten Tätigkeitsstätte beruflich tätig (auswärtige beruflich |

**`keine_mahlzeitengestellung`** — _['eine Mahlzeit zur Verfügung gestellt', 'Hat der Arbeitnehmer für die Mahlzeit ein Entgelt gezahlt']_
| # | Anker | Zitat |
|--|--|--|
| 1 | `satz 8` | Wird dem Arbeitnehmer anlässlich oder während einer Tätigkeit außerhalb seiner ersten Täti |
| 2 | `satz 8` | für Frühstück um 20 Prozent, für Mittag- und Abendessen um jeweils 40 Prozent |

**`keine_steuerfreien_verpflegungserstattungen`** — _Erhält der Arbeitnehmer steuerfreie Erstattungen für Verpflegung_
| # | Anker | Zitat |
|--|--|--|
| 1 | `satz 11` | Erhält der Arbeitnehmer steuerfreie Erstattungen für Verpflegung, ist ein Werbungskostenab |

**`keine_unterbrechung_mit_neubeginn`** — _Eine Unterbrechung der beruflichen Tätigkeit an derselben Tätigkeitsstätte führt zu einem Neubeginn_
| # | Anker | Zitat |
|--|--|--|
| 1 | `monate_am_ort / interpretation` | Die Formalisierung setzt voraus, dass die Monatsangabe die Unterbrechungsregel von vier Wo |
| 2 | `monate_am_ort / interpretation` | Die Formalisierung nimmt an, dass 'monate_am_ort' die Dauer ohne Unterbrechungen nach Satz |
| 3 | `monate_am_ort / interpretation` | Die Monatsangabe wird als ununterbrochene Dauer am selben Ort ohne Berücksichtigung von Un |
| 4 | `monate_am_ort / interpretation` | Die Eingabe monate_am_ort wird als bereits um Unterbrechungen von mindestens vier Wochen b |
| 5 | `satz 7` | Eine Unterbrechung der beruflichen Tätigkeit an derselben Tätigkeitsstätte führt zu einem  |

**`taetigkeit_im_inland`** — _Bei einer Tätigkeit im Ausland treten an die Stelle der Pauschbeträge_
| # | Anker | Zitat |
|--|--|--|
| 1 | `satz 5` | Bei einer Tätigkeit im Ausland treten an die Stelle der Pauschbeträge nach Satz 3 länderwe |

**`uebernachtung_oder_eintaegig_ueber_acht_stunden`** — _['beginnt die auswärtige berufliche Tätigkeit an einem Kalendertag und endet am nachfolgenden Kalendertag ohne Übernachtung', 'wenn der Arbeitnehmer an diesem, einem anschließenden oder vorhergehenden Tag außerhalb seiner Wohnung übernachtet']_
| # | Anker | Zitat |
|--|--|--|
| 1 | `abwesenheit_stunden / interpretation` | Die Formalisierung nimmt an, dass bei tagesübergreifender Tätigkeit ohne Übernachtung die  |
| 2 | `abwesenheit_stunden / interpretation` | Die Eingabe abwesenheit_stunden wird als die für den Tag maßgebliche Stundenzahl nach der  |
| 3 | `satz 3 nr. 3 halbsatz 2` | beginnt die auswärtige berufliche Tätigkeit an einem Kalendertag und endet am nachfolgende |
| 4 | `satz 3 nr. 3 satz 2` | beginnt die auswärtige berufliche Tätigkeit an einem Kalendertag und endet am nachfolgende |

### Offen — Triage noetig
| # | Art | Anker | Zitat |
|--|--|--|--|
| 1 | abweichung | `?` | Die Formalisierung verlangt eine Übernachtung am An- oder Abreisetag selbst, die Norm läss |
| 2 | abweichung | `?` | Die Formalisierung gewährt 28 Euro bei 24 oder mehr Stunden Abwesenheit, die Norm verlangt |
| 3 | abweichung | `?` | Die Formalisierung setzt für den An- und Abreisetag eine Übernachtung am selben Tag voraus |
| 4 | abweichung | `?` | Die 14-Euro-Regel für An-/Abreisetag erfordert fälschlich eine Übernachtung am selben Tag, |
| 5 | abweichung | `?` | Die Ausgabe der Verpflegungspauschale erfolgt in Dollar statt in Euro. |
| 6 | abweichung | `?` | Die Bedingung für den An- oder Abreisetag verlangt eine Übernachtung am selben Tag, die No |
| 7 | abweichung | `?` | Die Formalisierung gewährt 14 Euro für jeden Tag mit mehr als 8 Stunden Abwesenheit ohne Ü |
| 8 | annahme | `mit_uebernachtung / interpretation` | Die Formalisierung interpretiert die Übernachtungsangabe als auf den jeweiligen Tag bezoge |
| 9 | annahme | `ergebnis / geltungsvoraussetzung` | Die Formalisierung geht davon aus, dass der Arbeitnehmer eine erste Tätigkeitsstätte hat. |
| 10 | annahme | `abwesenheit_stunden / interpretation` | Die Formalisierung nimmt an, dass die eingegebene Abwesenheit in Stunden 24 nicht überstei |
| 11 | annahme | `mit_uebernachtung / interpretation` | Die Formalisierung nimmt an, dass 'mit_uebernachtung' nur Übernachtung am selben Tag bedeu |
| 12 | annahme | `abwesenheit_stunden / interpretation` | Die Stundenangabe wird als exakte Abwesenheitsdauer an einem Kalendertag interpretiert. |
| 13 | annahme | `mit_uebernachtung / interpretation` | Die Eingabe mit_uebernachtung wird so interpretiert, dass sie auch Übernachtungen an angre |
| 14 | annahme | `mit_uebernachtung / interpretation` | Die Formalisierung interpretiert 'mit_uebernachtung' so, dass eine Übernachtung am An-/Abr |
| 15 | norm_teil | `§ 9 abs. 4a` | Verpflegungsmehraufwand, Dreimonatsfrist |
| 16 | norm_teil | `satz 6` | Der Abzug der Verpflegungspauschalen ist auf die ersten drei Monate einer längerfristigen  |

## p35a_2_3_haushaltsnahe
_konv 2 · bedingung 37 · offen 14 · backlog 4 (auto)_

### Det→Konvention (bestaetige alle / streiche Zeile)
| # | Anker | konv-ID | Zitat |
|--|--|--|--|
| 1 | `haushaltsnahe_dienstleistungen / interpretation` | `konv:input_nur_etikettiertes` | Die haushaltsnahen Dienstleistungen sind keine Handwerkerleistungen und werden im Haushalt |
| 2 | `ergebnis / interpretation` | `konv:input_nur_etikettiertes` | Es wird angenommen, dass sich die Aufwendungen der drei Kategorien nicht überschneiden. |

### Det→Bedingung (bestaetige zeilenweise; nicht bestaetigt -> offen)

**`antrag_gestellt`** — _auf Antrag_
| # | Anker | Zitat |
|--|--|--|
| 1 | `minijob_aufwendungen / geltungsvoraussetzung` | Der Steuerpflichtige hat einen Antrag auf Steuerermäßigung gestellt. |
| 2 | `ergebnis / geltungsvoraussetzung` | Es wird ein Antrag auf Steuerermäßigung gestellt. |
| 3 | `§ 35a abs. 1` | auf Antrag |
| 4 | `§ 35a abs. 2 satz 1` | ermäßigt sich die tarifliche Einkommensteuer, vermindert um die sonstigen Steuerermäßigung |

**`dienstleistungen_sind_keine_handwerkerleistungen`** — _die nicht Dienstleistungen nach Absatz 3 sind_
| # | Anker | Zitat |
|--|--|--|
| 1 | `haushaltsnahe_dienstleistungen / interpretation` | Die Eingabe betrifft nur haushaltsnahe Dienstleistungen oder Beschäftigungsverhältnisse, d |

**`dienstleistungsbetrag_enthaelt_nur_arbeitskosten`** — _nach den Absätzen 2 und 3 gilt nur für Arbeitskosten_
| # | Anker | Zitat |
|--|--|--|
| 1 | `haushaltsnahe_dienstleistungen / interpretation` | Die haushaltsnahen Dienstleistungen umfassen ausschließlich Arbeitskosten. |
| 2 | `haushaltsnahe_dienstleistungen / interpretation` | Die haushaltsnahe_dienstleistungen werden als Arbeitskosten für haushaltsnahe Dienstleistu |

**`handwerker_keine_oeffentliche_foerderung`** — _nicht für öffentlich geförderte Maßnahmen_
| # | Anker | Zitat |
|--|--|--|
| 1 | `handwerker_arbeitskosten / geltungsvoraussetzung` | Die Handwerkerleistungen sind nicht öffentlich gefördert im Sinne des § 35a Abs. 3 Satz 2  |
| 2 | `handwerker_arbeitskosten / interpretation` | Die Eingabe wird als Aufwendungen für nicht öffentlich geförderte Handwerkerleistungen int |
| 3 | `§ 35a abs. 3 satz 2` | Dies gilt nicht für öffentlich geförderte Maßnahmen, für die zinsverbilligte Darlehen oder |

**`handwerker_nur_renovierung_erhaltung_modernisierung`** — _für Renovierungs-, Erhaltungs- und Modernisierungsmaßnahmen_
| # | Anker | Zitat |
|--|--|--|
| 1 | `§ 35a abs. 3 satz 1` | Handwerkerleistungen für Renovierungs-, Erhaltungs- und Modernisierungsmaßnahmen |

**`handwerkerbetrag_enthaelt_nur_arbeitskosten`** — _gilt nur für Arbeitskosten_
| # | Anker | Zitat |
|--|--|--|
| 1 | `handwerker_arbeitskosten / interpretation` | Die handwerker_arbeitskosten umfassen ausschließlich Arbeitskosten. |
| 2 | `handwerker_arbeitskosten / interpretation` | Die Eingabe umfasst ausschließlich Arbeitskosten für Handwerkerleistungen im Sinne des § 3 |

**`haushalt_in_eu_ewr`** — _Europäischen Union oder dem Europäischen Wirtschaftsraum liegenden Haushalt_
| # | Anker | Zitat |
|--|--|--|
| 1 | `haushaltsnahe_dienstleistungen / geltungsvoraussetzung` | Die Dienstleistung wird in einem Haushalt in der EU oder dem EWR erbracht. |
| 2 | `ergebnis / geltungsvoraussetzung` | Die Leistungen werden im Haushalt des Steuerpflichtigen in der EU oder dem EWR erbracht. |
| 3 | `minijob_aufwendungen / geltungsvoraussetzung` | Es wird angenommen, dass das Beschäftigungsverhältnis im Haushalt in der EU oder dem EWR a |
| 4 | `handwerker_arbeitskosten / geltungsvoraussetzung` | Es wird angenommen, dass die Handwerkerleistung im Haushalt in der EU oder dem EWR erbrach |
| 5 | `§ 35a abs. 4 satz 1` | wenn das Beschäftigungsverhältnis, die Dienstleistung oder die Handwerkerleistung in einem |
| 6 | `§ 35a abs. 4` | wenn das Beschäftigungsverhältnis, die Dienstleistung oder die Handwerkerleistung in einem |

**`kein_gemeinsamer_haushalt_zweier_alleinstehender`** — _Höchstbeträge nach den Absätzen 1 bis 3 insgesamt jeweils nur einmal_
| # | Anker | Zitat |
|--|--|--|
| 1 | `ergebnis / geltungsvoraussetzung` | Der Steuerpflichtige lebt nicht mit einem anderen Alleinstehenden in einem Haushalt zusamm |
| 2 | `ergebnis / geltungsvoraussetzung` | Die Höchstbeträge werden nicht mit einer weiteren alleinstehenden Person im selben Haushal |
| 3 | `ergebnis / geltungsvoraussetzung` | Es wird angenommen, dass keine zwei Alleinstehenden im Haushalt zusammenleben und die Höch |
| 4 | `§ 35a abs. 5 satz 4` | Leben zwei Alleinstehende in einem Haushalt zusammen, können sie die Höchstbeträge nach de |

**`keine_beruecksichtigung_als_wk_sa_agb`** — _nicht Betriebsausgaben oder Werbungskosten darstellen_
| # | Anker | Zitat |
|--|--|--|
| 1 | `minijob_aufwendungen / geltungsvoraussetzung` | Die Aufwendungen sind keine Betriebsausgaben oder Werbungskosten und nicht als Sonderausga |
| 2 | `haushaltsnahe_dienstleistungen / geltungsvoraussetzung` | Es wird angenommen, dass die Aufwendungen keine Betriebsausgaben oder Werbungskosten sind  |
| 3 | `handwerker_arbeitskosten / geltungsvoraussetzung` | Es wird angenommen, dass die Aufwendungen keine Betriebsausgaben oder Werbungskosten sind  |
| 4 | `§ 35a abs. 5 satz 1` | soweit die Aufwendungen nicht Betriebsausgaben oder Werbungskosten darstellen und soweit s |

**`minijob_ist_geringfuegige_beschaeftigung_nach_8a_sgb4`** — _bei denen es sich um eine geringfügige Beschäftigung im Sinne des § 8a des Vierten Buches Sozialgese_
| # | Anker | Zitat |
|--|--|--|
| 1 | `minijob_aufwendungen / interpretation` | Die minijob_aufwendungen stammen aus einem haushaltsnahen geringfügigen Beschäftigungsverh |
| 2 | `minijob_aufwendungen / interpretation` | Die minijob_aufwendungen werden als Aufwendungen für eine geringfügige Beschäftigung im Si |
| 3 | `minijob_aufwendungen / geltungsvoraussetzung` | Die Aufwendungen stammen aus einer geringfügigen Beschäftigung im Sinne des § 8a SGB IV. |
| 4 | `minijob_aufwendungen / interpretation` | Die Eingabe wird als Aufwendungen für ein haushaltsnahes Minijob nach §8a SGB IV interpret |
| 5 | `§ 35a abs. 1` | geringfügige Beschäftigung im Sinne des § 8a des Vierten Buches Sozialgesetzbuch |

**`rechnung_und_unbare_zahlung`** — _eine Rechnung erhalten hat und die Zahlung auf das Konto des Erbringers der Leistung erfolgt ist_
| # | Anker | Zitat |
|--|--|--|
| 1 | `haushaltsnahe_dienstleistungen / geltungsvoraussetzung` | Der Steuerpflichtige hat eine Rechnung erhalten und die Zahlung auf das Konto des Leistung |
| 2 | `handwerker_arbeitskosten / geltungsvoraussetzung` | Der Steuerpflichtige hat eine Rechnung erhalten und die Zahlung auf das Konto des Leistung |
| 3 | `haushaltsnahe_dienstleistungen / geltungsvoraussetzung` | Es wird angenommen, dass eine Rechnung vorliegt und die Zahlung auf das Konto des Leistung |
| 4 | `handwerker_arbeitskosten / geltungsvoraussetzung` | Es wird angenommen, dass eine Rechnung vorliegt und die Zahlung auf das Konto des Leistung |
| 5 | `§ 35a abs. 5 satz 3` | Voraussetzung ... ist, dass der Steuerpflichtige für die Aufwendungen eine Rechnung erhalt |

### Offen — Triage noetig
| # | Art | Anker | Zitat |
|--|--|--|--|
| 1 | abweichung | `?` | Die Formaliserung wendet 20% auf den gesamten Betrag an, obwohl nach § 35a Abs. 5 Satz 2 n |
| 2 | annahme | `handwerker_arbeitskosten / interpretation` | Die Handwerkerleistungen sind für Renovierungs-, Erhaltungs- und Modernisierungsmaßnahmen  |
| 3 | annahme | `minijob_aufwendungen / geltungsvoraussetzung` | Die minijob_aufwendungen erfüllen die Voraussetzungen der Absätze 4 und 5 (keine Betriebsa |
| 4 | annahme | `haushaltsnahe_dienstleistungen / geltungsvoraussetzung` | Die haushaltsnahe_dienstleistungen erfüllen die Voraussetzungen der Absätze 4 und 5 (keine |
| 5 | annahme | `handwerker_arbeitskosten / geltungsvoraussetzung` | Die handwerker_arbeitskosten sind nicht für öffentlich geförderte Maßnahmen und erfüllen d |
| 6 | annahme | `haushaltsnahe_dienstleistungen / interpretation` | Die Eingabe umfasst ausschließlich Arbeitskosten im Sinne des § 35a Abs. 5 Satz 2 EStG. |
| 7 | annahme | `ergebnis / geltungsvoraussetzung` | Die Aufwendungen sind nicht als Betriebsausgaben, Werbungskosten oder Sonderausgaben berüc |
| 8 | annahme | `haushaltsnahe_dienstleistungen / interpretation` | Die Eingabe umfasst nur die begünstigten Kosten für Pflege- und Betreuungsleistungen sowie |
| 9 | annahme | `haushaltsnahe_dienstleistungen / interpretation` | Die Eingabe wird als ausschließlich Arbeitskosten interpretiert, obwohl die Norm dies nich |
| 10 | annahme | `ergebnis / geltungsvoraussetzung` | Es wird angenommen, dass die tarifliche Einkommensteuer nach anderen Ermäßigungen die Summ |
| 11 | annahme | `ergebnis / rundung` | Die 20%-Berechnung der Aufwendungen erfolgt ohne gesetzliche Rundungsvorschrift, es wird s |
| 12 | annahme | `minijob_aufwendungen / zeitbezug` | Es wird angenommen, dass die Aufwendungen im jeweiligen Veranlagungsjahr angefallen sind. |
| 13 | norm_teil | `§ 35a abs. 5 satz 2` | Der Abzug von der tariflichen Einkommensteuer nach den Absätzen 2 und 3 gilt nur für Arbei |
| 14 | norm_teil | `§ 35a abs. 3 satz 1` | ermäßigt sich die tarifliche Einkommensteuer, vermindert um die sonstigen Steuerermäßigung |