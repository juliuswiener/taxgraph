"""Ingest frozen legal sources (sources/) into the TaxGraph document store.

For v1 only § 32a and the § 4 excerpt from sources/ are ingested. Each document's
file is checked against the SHA256 recorded in its .meta.yaml before ingest
(freeze integrity). The text is segmented (statute -> Absatz/Nummer/Satz level)
and, for the § 32a 2026 Fassung, demonstrative parameter claims are created whose
citation anchor is verified against the segment text.

Connection via DOCSTORE_DSN (libpq keyword/value or URI). Default targets the
local cluster used in development.

Run: python docstore/ingest.py   (or: make docstore-ingest)
"""

from __future__ import annotations

import glob
import hashlib
import json
import os
import re

import psycopg
import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
DSN = os.environ.get(
    "DOCSTORE_DSN",
    "host=127.0.0.1 port=5432 dbname=taxgraph_docstore user=taxgraph password=taxgraph",
)

_UMLAUT = str.maketrans({"ä": "ae", "ö": "oe", "ü": "ue", "ß": "ss",
                         "Ä": "ae", "Ö": "oe", "Ü": "ue"})


def normalize(text: str) -> str:
    return re.sub(r"\s+", " ", text.translate(_UMLAUT).lower()).strip()


def sha256_of(path: str) -> str:
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


# --- segmentation -----------------------------------------------------------

def segment_p32a(body: str):
    """Segment the § 32a text into (typ, label, text) tuples."""
    segs = []
    abs1 = re.search(r"\(1\)(.*?)\(2\) bis \(4\)", body, re.DOTALL)
    if abs1:
        block = abs1.group(1).strip()
        intro = re.split(r"\n\s*1\.\s", block, maxsplit=1)[0].strip()
        segs.append(("satz", "Abs. 1 Satz 1-2", intro))
        for m in re.finditer(r"^\s*([1-5])\.\s*(.+)$", block, re.MULTILINE):
            segs.append(("nummer", f"Abs. 1 Satz 2 Nr. {m.group(1)}", m.group(2).strip()))
        tail = re.search(r"(3Die Groesse|3Die Größe).*?abzurunden\.", block, re.DOTALL)
        if tail:
            segs.append(("satz", "Abs. 1 Saetze 3-6", tail.group(0).strip()))
    abs5 = re.search(r"\(5\)(.*?)$", body, re.DOTALL)
    if abs5:
        segs.append(("absatz", "Abs. 5", ("(5)" + abs5.group(1)).strip()))
    return segs


def segment_p04(body: str):
    segs = []
    for nr in ("6b", "6c"):
        m = re.search(rf"^\s*{nr}\.\s*(.+?)(?=^\s*6[bc]\.|\Z)", body,
                      re.DOTALL | re.MULTILINE)
        if m:
            segs.append(("nummer", f"Abs. 5 Satz 1 Nr. {nr}", m.group(1).strip()))
    return segs


def segment(kanonische_id: str, body: str):
    if "32a" in kanonische_id:
        return segment_p32a(body)
    if kanonische_id.startswith("estg/4"):
        return segment_p04(body)
    # fallback: paragraph blocks
    return [("absatz", f"Block {i+1}", b.strip())
            for i, b in enumerate(re.split(r"\n\s*\n", body.strip())) if b.strip()]


# --- § 32a 2026 demonstrative parameter claims ------------------------------

P32A_2026_CLAIMS = [
    ("bis 12 348 Euro (Grundfreibetrag): 0", "Abs. 1 Satz 2 Nr. 1",
     {"zone": 1, "grundfreibetrag": 12348, "steuer": 0}),
    ("von 12 349 Euro bis 17 799 Euro: (914,51", "Abs. 1 Satz 2 Nr. 2",
     {"zone": 2, "a": 914.51, "b": 1400}),
    ("von 17 800 Euro bis 69 878 Euro: (173,10", "Abs. 1 Satz 2 Nr. 3",
     {"zone": 3, "a": 173.10, "b": 2397, "c": 1034.87}),
    ("von 69 879 Euro bis 277 825 Euro: 0,42", "Abs. 1 Satz 2 Nr. 4",
     {"zone": 4, "faktor": 0.42, "abzug": 11135.63}),
    ("von 277 826 Euro an: 0,45", "Abs. 1 Satz 2 Nr. 5",
     {"zone": 5, "faktor": 0.45, "abzug": 19470.38}),
]


def ingest_document(cur, meta_path: str) -> tuple[int, int, int]:
    meta = yaml.safe_load(open(meta_path, encoding="utf-8"))["dokument"]
    src_path = os.path.join(os.path.dirname(meta_path), meta["datei"])
    actual = sha256_of(src_path)
    if actual != meta["sha256"]:
        raise SystemExit(f"SHA256 mismatch for {src_path}: freeze integrity violated")

    body = open(src_path, encoding="utf-8").read()
    # drop the leading provenance header (everything up to the first content marker)
    body_content = re.split(r"-{3,}.*?-{3,}\n", body, maxsplit=1)
    body = body_content[1] if len(body_content) > 1 else body

    cur.execute(
        """
        INSERT INTO dokument
          (kanonische_id, typ, authority, redistributable, titel, fassung,
           quelle_url, abrufdatum, sha256, objektpfad)
        VALUES (%s, 'gesetz', %s, %s, %s, %s, %s, %s, %s, %s)
        ON CONFLICT (kanonische_id, fassung) DO UPDATE SET
          authority = EXCLUDED.authority,
          redistributable = EXCLUDED.redistributable,
          quelle_url = EXCLUDED.quelle_url,
          abrufdatum = EXCLUDED.abrufdatum,
          sha256 = EXCLUDED.sha256,
          objektpfad = EXCLUDED.objektpfad
        RETURNING id
        """,
        (meta["norm_uri"], meta["authority"], meta["redistributable"],
         meta.get("norm"), meta.get("fassung"), meta.get("quelle_url"),
         meta.get("abrufdatum"), meta["sha256"],
         os.path.relpath(src_path, ROOT)),
    )
    dok_id = cur.fetchone()[0]

    # idempotent re-ingest: remove dependent claims, then segments
    cur.execute(
        "DELETE FROM claim WHERE segment_id IN "
        "(SELECT id FROM segment WHERE dokument_id = %s)", (dok_id,))
    cur.execute("DELETE FROM segment WHERE dokument_id = %s", (dok_id,))
    segs = segment(meta["norm_uri"], body)
    seg_label_to_id = {}
    for pos, (typ, label, text) in enumerate(segs):
        cur.execute(
            "INSERT INTO segment (dokument_id, typ, label, position, text) "
            "VALUES (%s, %s, %s, %s, %s) RETURNING id",
            (dok_id, typ, label, pos, text),
        )
        seg_label_to_id[label] = cur.fetchone()[0]

    n_claims = 0
    # Authority-Konflikt (Streitstand) als Claim markieren, nicht formalisieren:
    # BMF-Schreiben Rz. 30 (Unfallkosten neben der Pauschale) contra BFH VI R 8/18.
    if meta["norm_uri"].startswith("bmf/"):
        for pos, (typ, label, text) in enumerate(segs):
            if "authority-konflikt" in normalize(text):
                seg_id = seg_label_to_id[label]
                payload = {
                    "thema": "Unfallkosten neben der Entfernungspauschale",
                    "verwaltung": "BMF-Schreiben 18.11.2021 Rz. 30: abziehbar",
                    "bfh": "BFH 19.12.2019 VI R 8/18: nicht abziehbar",
                    "status": "offener authority-Konflikt, nicht formalisiert (out of MVP-scope)",
                }
                cur.execute(
                    """
                    INSERT INTO claim
                      (segment_id, typ, authority, redistributable, zitatanker,
                       anker_verifiziert, payload, status)
                    VALUES (%s, 'streitstand', 'verwaltung', %s, %s, true, %s, 'extracted')
                    """,
                    (seg_id, meta["redistributable"], "Unfallkosten koennen als",
                     json.dumps(payload)),
                )
                n_claims += 1
    if meta["norm_uri"] == "estg/32a":
        seg_norm = {lbl: normalize(t) for (_, lbl, t) in segs}
        for anker, seg_label, payload in P32A_2026_CLAIMS:
            seg_id = seg_label_to_id.get(seg_label)
            if seg_id is None:
                continue
            verified = normalize(anker) in seg_norm.get(seg_label, "")
            cur.execute(
                """
                INSERT INTO claim
                  (segment_id, typ, authority, redistributable, zitatanker,
                   anker_verifiziert, payload, status, veranlagungszeitraum, gueltig_ab)
                VALUES (%s, 'parameter', %s, %s, %s, %s, %s, 'approved', 2026, '2026-01-01')
                """,
                (seg_id, meta["authority"], meta["redistributable"] and verified,
                 anker, verified, json.dumps(payload)),
            )
            n_claims += 1
    return dok_id, len(segs), n_claims


def main():
    metas = sorted(glob.glob(os.path.join(ROOT, "sources", "**", "*.meta.yaml"),
                             recursive=True))
    with psycopg.connect(DSN) as conn:
        with conn.cursor() as cur:
            total_seg = total_claim = 0
            for meta_path in metas:
                dok_id, n_seg, n_claim = ingest_document(cur, meta_path)
                total_seg += n_seg
                total_claim += n_claim
                print(f"ingested {os.path.relpath(meta_path, ROOT)}: "
                      f"dokument={dok_id}, segmente={n_seg}, claims={n_claim}")
        conn.commit()
        with conn.cursor() as cur:
            cur.execute("SELECT count(*) FROM dokument")
            d = cur.fetchone()[0]
            cur.execute("SELECT count(*) FROM claim WHERE anker_verifiziert")
            cv = cur.fetchone()[0]
            cur.execute("SELECT count(*) FROM claim")
            c = cur.fetchone()[0]
    print(f"\nStore: {d} Dokumente, {total_seg} Segmente, {c} Claims "
          f"({cv} mit verifiziertem Zitatanker).")


if __name__ == "__main__":
    main()
