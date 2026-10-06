//! Die Lader von `PyWert` (`serde_json`, `serde_yaml_ng`) gegen die festgehaltenen Abweichungen von Python, ohne
//! `PARITY=1`, ohne Python.
//!
//! Weg B voll, Stufe 2, S2.6. Die Listen standen nur in `rust/parity/tests/wert_paritaet.rs` (`lader_json_gegen_cpython`,
//! `lader_yaml_gegen_cpython`, `py_eq_json_gegen_cpython`), und diese Suite braucht `CPython`. Faellt Python weg, faellt sie
//! mit. Hier stehen dieselben Eintraege (Text, Rust, `CPython`) und der Test prueft die RUST-Seite: ladet der Lader den Text
//! noch so, wie die Liste sagt (`repr` der ersten 80 Zeichen, oder `FEHLER`)? Wird ein Lader besser (oder schlechter), wird er
//! rot, und wer ihn aendert, aendert die Liste und die Abweichungsliste in `rust/fixtures/README.md`. Die Spalte `CPython` ist
//! Herkunft und wird nicht geprueft: dafuer braucht es Python (die Parity-Suite, solange sie lebt).
//!
//! Nicht hier: dass KEINE weitere Abweichung dazukommt. Das misst nur der Vergleich mit `CPython`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use domain::PyWert;

/// Ein festgehaltener Abweichungsfall: Text, Rust, `CPython`.
type Pin = (&'static str, &'static str, &'static str);

#[rustfmt::skip]
const JSON_SKALARE_ABW: &[Pin] = &[
    ("NaN", "FEHLER", "nan"),
    ("Infinity", "FEHLER", "inf"),
    ("-Infinity", "FEHLER", "-inf"),
    ("-0", "-0.0", "0"),
    ("-9223372036854775809", "-9.223372036854776e+18", "-9223372036854775809"),
    ("18446744073709551616", "1.8446744073709552e+19", "18446744073709551616"),
    ("18446744073709551617", "1.8446744073709552e+19", "18446744073709551617"),
    ("340282366920938463463374607431768211455", "3.402823669209385e+38", "340282366920938463463374607431768211455"),
    ("340282366920938463463374607431768211456", "3.402823669209385e+38", "340282366920938463463374607431768211456"),
    ("-340282366920938463463374607431768211456", "-3.402823669209385e+38", "-340282366920938463463374607431768211456"),
    ("1.7976931348623159e308", "FEHLER", "inf"),
    ("1e309", "FEHLER", "inf"),
    ("-1e309", "FEHLER", "-inf"),
    ("1e999999999", "FEHLER", "inf"),
    ("-1e999999999", "FEHLER", "-inf"),
    ("123456789012345678901234567890", "1.2345678901234568e+29", "123456789012345678901234567890"),
    ("1E400", "FEHLER", "inf"),
    ("1e+400", "FEHLER", "inf"),
    ("\"\\ud800\"", "FEHLER", "'\\ud800'"),
    ("\"\\udc00\"", "FEHLER", "'\\udc00'"),
    ("\"\\ud800\\u0041\"", "FEHLER", "'\\ud800A'"),
    ("\"\\ud83d\"", "FEHLER", "'\\ud83d'"),
    ("\"\\ude00\\ud83d\"", "FEHLER", "'\\ude00\\ud83d'"),
];

#[rustfmt::skip]
const JSON_STRUKTUREN_ABW: &[Pin] = &[
    ("{\"a\":NaN}", "FEHLER", "{'a': nan}"),
    ("[NaN]", "FEHLER", "[nan]"),
    ("[-Infinity, Infinity]", "FEHLER", "[-inf, inf]"),
    ("[1e999]", "FEHLER", "[inf]"),
    ("{\"\\ud800\": 1}", "FEHLER", "{'\\ud800': 1}"),
];

#[rustfmt::skip]
const YAML_SKALARE_ABW: &[Pin] = &[
    ("yes", "'yes'", "True"),
    ("Yes", "'Yes'", "True"),
    ("YES", "'YES'", "True"),
    ("no", "'no'", "False"),
    ("No", "'No'", "False"),
    ("NO", "'NO'", "False"),
    ("on", "'on'", "True"),
    ("On", "'On'", "True"),
    ("ON", "'ON'", "True"),
    ("off", "'off'", "False"),
    ("Off", "'Off'", "False"),
    ("OFF", "'OFF'", "False"),
    ("007", "'007'", "7"),
    ("010", "'010'", "8"),
    ("017", "'017'", "15"),
    ("-017", "'-017'", "-15"),
    ("+017", "'+017'", "15"),
    ("0o17", "15", "'0o17'"),
    ("-0o17", "-15", "'-0o17'"),
    ("0x_1", "'0x_1'", "1"),
    ("0b1_1", "'0b1_1'", "3"),
    ("1_000", "'1_000'", "1000"),
    ("1__0", "'1__0'", "10"),
    ("1_", "'1_'", "1"),
    ("0_1", "'0_1'", "1"),
    ("0_0", "'0_0'", "0"),
    ("1_0_0", "'1_0_0'", "100"),
    ("-1_0", "'-1_0'", "-10"),
    ("+1_0", "'+1_0'", "10"),
    ("0b_1", "'0b_1'", "1"),
    ("0x1_F", "'0x1_F'", "31"),
    ("0_", "'0_'", "0"),
    ("-9223372036854775809", "-9.223372036854776e+18", "-9223372036854775809"),
    ("18446744073709551616", "1.8446744073709552e+19", "18446744073709551616"),
    ("18446744073709551617", "1.8446744073709552e+19", "18446744073709551617"),
    ("340282366920938463463374607431768211455", "3.402823669209385e+38", "340282366920938463463374607431768211455"),
    ("340282366920938463463374607431768211456", "3.402823669209385e+38", "340282366920938463463374607431768211456"),
    ("-170141183460469231731687303715884105728", "-1.7014118346046923e+38", "-170141183460469231731687303715884105728"),
    ("-170141183460469231731687303715884105729", "-1.7014118346046923e+38", "-170141183460469231731687303715884105729"),
    ("0x10000000000000000", "1.8446744073709552e+19", "18446744073709551616"),
    ("0b10000000000000000000000000000000000000000000000000000000000000000", "1.8446744073709552e+19", "18446744073709551616"),
    ("01777777777777777777777", "'01777777777777777777777'", "18446744073709551615"),
    ("02000000000000000000000", "'02000000000000000000000'", "18446744073709551616"),
    ("1:30", "'1:30'", "90"),
    ("190:20:30", "'190:20:30'", "685230"),
    ("-1:30", "'-1:30'", "-90"),
    ("+1:30", "'+1:30'", "90"),
    ("1:59", "'1:59'", "119"),
    ("1:2:3:4", "'1:2:3:4'", "223384"),
    ("1:30.5", "'1:30.5'", "90.5"),
    ("190:20:30.15", "'190:20:30.15'", "685230.15"),
    ("1_0:30", "'1_0:30'", "630"),
    ("0:30.5", "'0:30.5'", "30.5"),
    ("1:3", "'1:3'", "63"),
    ("1:03", "'1:03'", "63"),
    ("1:", "{'1': None}", "{1: None}"),
    ("12:30:45", "'12:30:45'", "45045"),
    ("-12:30:45", "'-12:30:45'", "-45045"),
    ("1:30:00", "'1:30:00'", "5400"),
    ("1:00", "'1:00'", "60"),
    ("10:10:10:10:10", "'10:10:10:10:10'", "131796610"),
    ("1:30:00.5", "'1:30:00.5'", "5400.5"),
    ("-190:20:30.15", "'-190:20:30.15'", "-685230.15"),
    ("+190:20:30.15", "'+190:20:30.15'", "685230.15"),
    ("-.5", "-0.5", "'-.5'"),
    ("+.5", "0.5", "'+.5'"),
    ("1.5e3", "1500.0", "'1.5e3'"),
    ("1.5E3", "1500.0", "'1.5E3'"),
    ("1e3", "1000.0", "'1e3'"),
    ("1E3", "1000.0", "'1E3'"),
    ("1e+3", "1000.0", "'1e+3'"),
    ("1e-3", "0.001", "'1e-3'"),
    ("1.e3", "1000.0", "'1.e3'"),
    (".1e3", "100.0", "'.1e3'"),
    ("1_0.5", "'1_0.5'", "10.5"),
    ("1.5_5", "'1.5_5'", "1.55"),
    ("1.0_", "'1.0_'", "1.0"),
    ("1_.0", "'1_.0'", "1.0"),
    ("1.0e+400", "'1.0e+400'", "inf"),
    ("-1.0e+400", "'-1.0e+400'", "-inf"),
    ("1e-400", "0.0", "'1e-400'"),
    ("1.7976931348623159e+308", "'1.7976931348623159e+308'", "inf"),
    ("5e-324", "5e-324", "'5e-324'"),
    ("1e22", "1e+22", "'1e22'"),
    ("2001-12-14", "'2001-12-14'", "datetime.date(2001, 12, 14)"),
    ("2001-12-14t21:59:43.10-05:00", "'2001-12-14t21:59:43.10-05:00'", "datetime.datetime(2001, 12, 14, 21, 59, 43, 100000, tzinfo=datetime.timezone(dat"),
    ("2001-12-14 21:59:43.10 -5", "'2001-12-14 21:59:43.10 -5'", "datetime.datetime(2001, 12, 14, 21, 59, 43, 100000, tzinfo=datetime.timezone(dat"),
    ("2001-12-14 21:59:43.10", "'2001-12-14 21:59:43.10'", "datetime.datetime(2001, 12, 14, 21, 59, 43, 100000)"),
    ("2001-12-14T21:59:43Z", "'2001-12-14T21:59:43Z'", "datetime.datetime(2001, 12, 14, 21, 59, 43, tzinfo=datetime.timezone.utc)"),
    ("2001-12-14 21:59:43", "'2001-12-14 21:59:43'", "datetime.datetime(2001, 12, 14, 21, 59, 43)"),
    ("2001-13-14", "'2001-13-14'", "FEHLER ValueError"),
    ("2001-02-30", "'2001-02-30'", "FEHLER ValueError"),
    ("2001-12-14T21:59:43+05:30", "'2001-12-14T21:59:43+05:30'", "datetime.datetime(2001, 12, 14, 21, 59, 43, tzinfo=datetime.timezone(datetime.ti"),
    ("2001-12-14T21:59:43.123456789Z", "'2001-12-14T21:59:43.123456789Z'", "datetime.datetime(2001, 12, 14, 21, 59, 43, 123456, tzinfo=datetime.timezone.utc"),
    ("0001-01-01", "'0001-01-01'", "datetime.date(1, 1, 1)"),
    ("0000-01-01", "'0000-01-01'", "FEHLER ValueError"),
    ("9999-12-31", "'9999-12-31'", "datetime.date(9999, 12, 31)"),
    ("=", "'='", "FEHLER ConstructorError"),
    ("<<", "'<<'", "FEHLER ConstructorError"),
    ("!!bool yes", "FEHLER", "True"),
    ("!!null ''", "FEHLER", "None"),
    ("!!null x", "FEHLER", "None"),
    ("!!binary aGVsbG8=", "'aGVsbG8='", "b'hello'"),
    ("!!binary '!'", "'!'", "b''"),
    ("!!binary ''", "''", "b''"),
    ("!!timestamp 2001-12-14", "'2001-12-14'", "datetime.date(2001, 12, 14)"),
    ("!!timestamp x", "'x'", "FEHLER AttributeError"),
    ("!!custom x", "'x'", "FEHLER ConstructorError"),
    ("! x", "FEHLER", "'x'"),
    ("!!python/object:os.system x", "'x'", "FEHLER ConstructorError"),
    ("!!python/name:os.system ''", "''", "FEHLER ConstructorError"),
    ("!!python/tuple [1]", "[1]", "FEHLER ConstructorError"),
    ("!!int 1_000", "FEHLER", "1000"),
    ("!!float 1_0.5", "FEHLER", "10.5"),
];

#[rustfmt::skip]
const YAML_STRUKTUREN_ABW: &[Pin] = &[
    ("%TAG !e! tag:example.com,2000:\n---\n!e!x a", "'a'", "FEHLER ConstructorError"),
    ("%FOO bar\n---\na", "FEHLER", "'a'"),
    ("a: 1\n\u{feff}", "{'a': 1}", "FEHLER ScannerError"),
    ("a:\tb", "{'a': 'b'}", "FEHLER ScannerError"),
    ("1: a\n'1': b", "{'1': 'b'}", "{1: 'a', '1': 'b'}"),
    ("'1': a\n1: b", "{'1': 'b'}", "{'1': 'a', 1: 'b'}"),
    ("true: a\n'true': b", "{'true': 'b'}", "{True: 'a', 'true': 'b'}"),
    ("null: a\n~: b", "{'null': 'a', '~': 'b'}", "{None: 'b'}"),
    ("~: a\nnull: b", "{'~': 'a', 'null': 'b'}", "{None: 'b'}"),
    ("1: a\n1.0: b", "{'1': 'a', '1.0': 'b'}", "{1: 'b'}"),
    ("0x10: a\n16: b", "{'0x10': 'a', '16': 'b'}", "{16: 'b'}"),
    ("1: a\n01: b", "{'1': 'a', '01': 'b'}", "{1: 'b'}"),
    ("1: a\n+1: b", "{'1': 'a', '+1': 'b'}", "{1: 'b'}"),
    ("yes: a\n'yes': b", "{'yes': 'b'}", "{True: 'a', 'yes': 'b'}"),
    ("yes: a\ntrue: b", "{'yes': 'a', 'true': 'b'}", "{True: 'b'}"),
    ("1: a\n0b1: b", "{'1': 'a', '0b1': 'b'}", "{1: 'b'}"),
    ("1.5: a\n'1.5': b", "{'1.5': 'b'}", "{1.5: 'a', '1.5': 'b'}"),
    (".5: a\n0.5: b", "{'.5': 'a', '0.5': 'b'}", "{0.5: 'b'}"),
    ("1: a\n1_0: b", "{'1': 'a', '1_0': 'b'}", "{1: 'a', 10: 'b'}"),
    ("{1: a}", "{'1': 'a'}", "{1: 'a'}"),
    ("{1.5: a}", "{'1.5': 'a'}", "{1.5: 'a'}"),
    ("{true: a}", "{'true': 'a'}", "{True: 'a'}"),
    ("{yes: a}", "{'yes': 'a'}", "{True: 'a'}"),
    ("{~: a}", "{'~': 'a'}", "{None: 'a'}"),
    ("{null: a}", "{'null': 'a'}", "{None: 'a'}"),
    ("{.nan: a}", "{'.nan': 'a'}", "{nan: 'a'}"),
    ("{.inf: a}", "{'.inf': 'a'}", "{inf: 'a'}"),
    ("{0x10: a}", "{'0x10': 'a'}", "{16: 'a'}"),
    ("{2001-12-14: a}", "{'2001-12-14': 'a'}", "{datetime.date(2001, 12, 14): 'a'}"),
    ("{1:30: a}", "{'1:30': 'a'}", "{90: 'a'}"),
    ("{!!binary YQ==: a}", "{'YQ==': 'a'}", "{b'a': 'a'}"),
    ("{!!int '1': a}", "{'1': 'a'}", "{1: 'a'}"),
    ("? \n: a", "{'': 'a'}", "{None: 'a'}"),
    ("&a [*a]", "FEHLER", "[[...]]"),
    ("&a {k: *a}", "FEHLER", "{'k': {...}}"),
    ("a: &x 1\nb: &x 2\nc: *x", "{'a': 1, 'b': 2, 'c': 2}", "FEHLER ComposerError"),
    ("&a\n- 1\n- *a", "FEHLER", "[1, [...]]"),
    ("a: &a yes\nb: *a", "{'a': 'yes', 'b': 'yes'}", "{'a': True, 'b': True}"),
    ("- &a x\n- &a y\n- *a", "['x', 'y', 'y']", "FEHLER ComposerError"),
    ("base: &b {a: 1, b: 2}\nd:\n  <<: *b\n  b: 3", "{'base': {'a': 1, 'b': 2}, 'd': {'<<': {'a': 1, 'b': 2}, 'b': 3}}", "{'base': {'a': 1, 'b': 2}, 'd': {'a': 1, 'b': 3}}"),
    ("base: &b {a: 1, b: 2}\nd:\n  b: 3\n  <<: *b", "{'base': {'a': 1, 'b': 2}, 'd': {'b': 3, '<<': {'a': 1, 'b': 2}}}", "{'base': {'a': 1, 'b': 2}, 'd': {'a': 1, 'b': 3}}"),
    ("x: &a {p: 1}\ny: &b {p: 2, q: 3}\nd:\n  <<: [*a, *b]", "{'x': {'p': 1}, 'y': {'p': 2, 'q': 3}, 'd': {'<<': [{'p': 1}, {'p': 2, 'q': 3}]}", "{'x': {'p': 1}, 'y': {'p': 2, 'q': 3}, 'd': {'p': 1, 'q': 3}}"),
    ("d:\n  <<: [{a: 1}, {a: 2, b: 3}]", "{'d': {'<<': [{'a': 1}, {'a': 2, 'b': 3}]}}", "{'d': {'a': 1, 'b': 3}}"),
    ("d: {<<: {a: 1}}", "{'d': {'<<': {'a': 1}}}", "{'d': {'a': 1}}"),
    ("<<: {a: 1}\nb: 2", "{'<<': {'a': 1}, 'b': 2}", "{'a': 1, 'b': 2}"),
    ("d:\n  <<: 1", "{'d': {'<<': 1}}", "FEHLER ConstructorError"),
    ("d:\n  <<: [1]", "{'d': {'<<': [1]}}", "FEHLER ConstructorError"),
    ("d:\n  <<: ~", "{'d': {'<<': None}}", "FEHLER ConstructorError"),
    ("{<<: {a: 1}, a: 2}", "{'<<': {'a': 1}, 'a': 2}", "{'a': 2}"),
    ("a: &a {x: 1}\nb: &b\n  <<: *a\n  y: 2\nc:\n  <<: *b", "{'a': {'x': 1}, 'b': {'<<': {'x': 1}, 'y': 2}, 'c': {'<<': {'<<': {'x': 1}, 'y':", "{'a': {'x': 1}, 'b': {'x': 1, 'y': 2}, 'c': {'x': 1, 'y': 2}}"),
    ("d:\n  <<: {a: 1}\n  <<: {b: 2}", "{'d': {'<<': {'b': 2}}}", "{'d': {'a': 1, 'b': 2}}"),
    ("d:\n  <<: []", "{'d': {'<<': []}}", "{'d': {}}"),
    ("d:\n  <<: {}", "{'d': {'<<': {}}}", "{'d': {}}"),
    ("d:\n  <<: [[1]]", "{'d': {'<<': [[1]]}}", "FEHLER ConstructorError"),
    ("d:\n  <<: \"x\"", "{'d': {'<<': 'x'}}", "FEHLER ConstructorError"),
    ("<<: 1", "{'<<': 1}", "FEHLER ConstructorError"),
    ("- <<: {a: 1}\n  b: 2", "[{'<<': {'a': 1}, 'b': 2}]", "[{'a': 1, 'b': 2}]"),
    ("a: {x: 1}\nd:\n  <<: {y: 2}\n  x: 3", "{'a': {'x': 1}, 'd': {'<<': {'y': 2}, 'x': 3}}", "{'a': {'x': 1}, 'd': {'y': 2, 'x': 3}}"),
    ("d:\n  ? <<\n  : {a: 1}", "{'d': {'<<': {'a': 1}}}", "{'d': {'a': 1}}"),
    ("d:\n  !!merge <<: {a: 1}", "{'d': {'<<': {'a': 1}}}", "{'d': {'a': 1}}"),
    ("!!set {a, b}", "{'a': None, 'b': None}", "{'a', 'b'}"),
    ("!!set [a]", "['a']", "FEHLER ConstructorError"),
    ("!!set {a: 1}", "{'a': 1}", "{'a'}"),
    ("!!omap [a: 1, b: 2]", "[{'a': 1}, {'b': 2}]", "[('a', 1), ('b', 2)]"),
    ("!!omap [a, b]", "['a', 'b']", "FEHLER ConstructorError"),
    ("!!omap [a: 1, a: 2]", "[{'a': 1}, {'a': 2}]", "[('a', 1), ('a', 2)]"),
    ("!!pairs [a: 1, a: 2]", "[{'a': 1}, {'a': 2}]", "[('a', 1), ('a', 2)]"),
    ("!!str [1]", "[1]", "FEHLER ConstructorError"),
    ("!!seq 1", "'1'", "FEHLER ConstructorError"),
    ("!!map []", "[]", "FEHLER ConstructorError"),
    ("!!map 1", "'1'", "FEHLER ConstructorError"),
    ("!!python/tuple [1]", "[1]", "FEHLER ConstructorError"),
    ("!!binary |\n  aGVs\n  bG8=", "'aGVs\\nbG8='", "b'hello'"),
    ("!!binary YQ", "'YQ'", "FEHLER ConstructorError"),
    ("!!set\n? a\n? b", "{'a': None, 'b': None}", "{'a', 'b'}"),
    ("!!str\n- a", "['a']", "FEHLER ConstructorError"),
    ("!!float [1]", "[1]", "FEHLER ConstructorError"),
    ("!!int {a: 1}", "{'a': 1}", "FEHLER ConstructorError"),
    ("!<!> x", "FEHLER", "'x'"),
    ("!!!x y", "'y'", "FEHLER ConstructorError"),
    ("!!python/object/apply:os.system [x]", "['x']", "FEHLER ConstructorError"),
    ("!!python/object/new:str [x]", "['x']", "FEHLER ConstructorError"),
    ("!!python/unicode x", "'x'", "FEHLER ConstructorError"),
    ("!!python/bytes x", "'x'", "FEHLER ConstructorError"),
    ("!!python/complex 1+2j", "'1+2j'", "FEHLER ConstructorError"),
    ("!!python/module:os x", "'x'", "FEHLER ConstructorError"),
    ("!!python/long 1", "'1'", "FEHLER ConstructorError"),
    ("!!python/float 1.5", "'1.5'", "FEHLER ConstructorError"),
    ("!!python/dict {a: 1}", "{'a': 1}", "FEHLER ConstructorError"),
    ("!!python/list [1]", "[1]", "FEHLER ConstructorError"),
    ("!!python/str x", "'x'", "FEHLER ConstructorError"),
    ("!!python/bool true", "'true'", "FEHLER ConstructorError"),
    ("!!python/none ''", "''", "FEHLER ConstructorError"),
    ("!!js/function x", "'x'", "FEHLER ConstructorError"),
    ("!!java.util.Date x", "'x'", "FEHLER ConstructorError"),
    ("!!merge x", "'x'", "FEHLER ConstructorError"),
    ("!!value x", "'x'", "FEHLER ConstructorError"),
    ("!!yaml x", "'x'", "FEHLER ConstructorError"),
    ("!!null", "FEHLER", "None"),
    ("!!null 1", "FEHLER", "None"),
    ("!!timestamp", "''", "FEHLER AttributeError"),
    ("\"\\ud800\"", "FEHLER", "'\\ud800'"),
    ("\"\\udc00\"", "FEHLER", "'\\udc00'"),
    ("\"\\ud83d\\ude00\"", "FEHLER", "'\\ud83d\\ude00'"),
    ("\"\\U0000D800\"", "FEHLER", "'\\ud800'"),
    ("a\tb", "'a\\tb'", "FEHLER ScannerError"),
    ("a\t", "'a'", "FEHLER ScannerError"),
    ("[\t1]", "[1]", "FEHLER ScannerError"),
    ("{\ta: 1}", "{'a': 1}", "FEHLER ScannerError"),
    ("?", "{'': None}", "{None: None}"),
    ("? ", "{'': None}", "{None: None}"),
    ("!", "FEHLER", "None"),
    ("a: !", "FEHLER", "{'a': None}"),
];

const D1_ABWEICHUNGEN: [(&str, &str); 4] = [
    ("18446744073709551616", "18446744073709551617"),
    ("18446744073709551617", "1.8446744073709552e19"),
    ("-9223372036854775809", "-9223372036854775808"),
    ("-9223372036854775809", "-9223372036854775808.0"),
];

fn lade_json(text: &str) -> Result<PyWert, String> {
    serde_json::from_str::<PyWert>(text).map_err(|e| e.to_string())
}

fn lade_yaml(text: &str) -> Result<PyWert, String> {
    serde_yaml_ng::from_str::<PyWert>(text).map_err(|e| e.to_string())
}

/// Die ersten 80 Zeichen: lange `repr` bleiben lesbar in der Liste (wie in der Parity-Suite).
fn kurz(s: &str) -> String {
    s.chars().take(80).collect()
}

fn rust_seite(r: Result<PyWert, String>) -> String {
    match r {
        Ok(w) => kurz(&w.repr()),
        Err(_) => "FEHLER".to_owned(),
    }
}

fn pruefe(name: &str, lade: fn(&str) -> Result<PyWert, String>, pins: &[Pin], anzahl: usize) {
    assert_eq!(
        pins.len(),
        anzahl,
        "{name}: Zahl der festgehaltenen Eintraege (ein gestrichener Eintrag faellt sonst nicht auf)"
    );
    let falsch: Vec<String> = pins
        .iter()
        .filter_map(|(text, rust, _python)| {
            let ist = rust_seite(lade(text));
            (ist != *rust).then(|| format!("{text:?}: ist {ist:?}, festgehalten {rust:?}"))
        })
        .collect();
    assert!(
        falsch.is_empty(),
        "{name}: {} von {} Eintraegen laden anders als festgehalten:\n{}",
        falsch.len(),
        pins.len(),
        falsch.join("\n")
    );
}

#[test]
fn json_lader_haelt_die_abweichungen_von_json_loads() {
    pruefe("json Skalare", lade_json, JSON_SKALARE_ABW, 23);
    pruefe("json Strukturen", lade_json, JSON_STRUKTUREN_ABW, 5);
}

#[test]
fn yaml_lader_haelt_die_abweichungen_von_safe_load() {
    pruefe("yaml Skalare", lade_yaml, YAML_SKALARE_ABW, 114);
    pruefe("yaml Strukturen", lade_yaml, YAML_STRUKTUREN_ABW, 113);
}

/// D1: Eine Ganzzahl ausserhalb `i64::MIN..=u64::MAX` wird `Gleit`; `py_eq` gegen die Nachbarzahl ist dann wahr, wo
/// `CPython` (exakte Ganzzahlen) falsch sagt. Die Paare stehen in `D1_ABWEICHUNGEN`; die Rust-Seite ist "gleich".
#[test]
fn d1_ganzzahl_ausserhalb_des_bereichs_ist_gleich_der_nachbarzahl() {
    assert_eq!(D1_ABWEICHUNGEN.len(), 4);
    for (a, b) in D1_ABWEICHUNGEN {
        let (x, y) = (lade_json(a).unwrap(), lade_json(b).unwrap());
        assert!(
            x.py_eq(&y),
            "{a} == {b}: Rust sagt ungleich, die Liste verlangt gleich"
        );
        assert!(y.py_eq(&x), "{b} == {a}: nicht symmetrisch");
    }
}
