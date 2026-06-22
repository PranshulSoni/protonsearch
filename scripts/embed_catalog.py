"""
Build-time embedding pipeline. Run once to produce catalog.bin.
Usage: python scripts/embed_catalog.py
"""

import csv
import json
import struct
import numpy as np
from pathlib import Path
from sentence_transformers import SentenceTransformer

CATALOG_PATH = Path(__file__).parent.parent / "settings_catalog.csv"
OUTPUT_PATH  = Path(__file__).parent.parent / "assets" / "catalog.bin"
MODEL_NAME   = "BAAI/bge-base-en-v1.5"

KEEP_FIELDS = {"id", "control_name", "breadcrumb_path", "launch_command", "description", "source", "synonyms"}


def load_catalog():
    rows = []
    with open(CATALOG_PATH, encoding="utf-8") as f:
        for row in csv.DictReader(f):
            synonyms = row["synonyms"].replace("|", " ")
            text = f"{row['control_name']} {synonyms} {row['description']} {row['breadcrumb_path']}"
            meta = {k: row[k] for k in KEEP_FIELDS}
            rows.append((text, meta))
    return rows


def main():
    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)

    print(f"Loading catalog from {CATALOG_PATH}...")
    rows = load_catalog()
    print(f"  {len(rows)} entries")

    print(f"\nLoading model: {MODEL_NAME}")
    model = SentenceTransformer(MODEL_NAME)

    texts = [text for text, _ in rows]
    print(f"\nEmbedding {len(texts)} entries...")
    vectors = model.encode(
        texts,
        batch_size=64,
        normalize_embeddings=True,
        show_progress_bar=True,
    )
    dim = vectors.shape[1]
    print(f"  Done. Shape: {vectors.shape}")

    print(f"\nWriting {OUTPUT_PATH}...")
    with open(OUTPUT_PATH, "wb") as f:
        f.write(struct.pack("<II", len(rows), dim))
        for i, ((_, meta), vec) in enumerate(zip(rows, vectors)):
            f.write(vec.astype(np.float32).tobytes())
            meta_bytes = json.dumps(meta, ensure_ascii=False).encode("utf-8")
            f.write(struct.pack("<H", len(meta_bytes)))
            f.write(meta_bytes)
            if (i + 1) % 100 == 0:
                print(f"  {i + 1}/{len(rows)}")

    size_kb = OUTPUT_PATH.stat().st_size / 1024
    print(f"\nDone. File size: {size_kb:.1f} KB")
    print(f"\nFirst entry verification:")
    with open(OUTPUT_PATH, "rb") as f:
        row_count, dim_out = struct.unpack("<II", f.read(8))
        vec0 = np.frombuffer(f.read(dim_out * 4), dtype=np.float32)
        meta_len = struct.unpack("<H", f.read(2))[0]
        meta0 = json.loads(f.read(meta_len))
    print(f"  rows={row_count}, dim={dim_out}")
    print(f"  first entry: {meta0['id']} — {meta0['control_name']}")
    print(f"  vector norm: {np.linalg.norm(vec0):.6f} (should be ~1.0)")


if __name__ == "__main__":
    main()
