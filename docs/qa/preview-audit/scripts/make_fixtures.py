#!/usr/bin/env python
"""Generate deterministic preview-audit fixtures (Gate 0 evidence)."""
import os, random, sys

out = sys.argv[1] if len(sys.argv) > 1 else "docs/qa/preview-audit/fixtures"
os.makedirs(out, exist_ok=True)

# 1. Plain text
open(os.path.join(out, "notes.txt"), "w", encoding="utf-8").write(
    "Quarterly preview audit notes\n\n"
    "This plain-text file exists to test the file/text preview pipeline.\n"
    "Unique token: ZEBRA-TXT-2026.\n"
    "Second paragraph with the same token repeated: ZEBRA-TXT-2026.\n"
)

# 2. Markdown (indented code block; no fence backticks)
open(os.path.join(out, "readme.md"), "w", encoding="utf-8").write(
    "# Preview Audit Readme\n\n"
    "## Markdown document\n\n"
    "This markdown file carries the token UNICORN-MD-2026 so content search can find it.\n\n"
    "- bullet one\n- bullet two\n\n"
    "    fn demo() { println!(\"hello preview\"); }\n"
)

# 3. Code
open(os.path.join(out, "sample.rs"), "w", encoding="utf-8").write(
    "//! Preview audit code fixture.\n"
    "// Unique token: PANDA-CODE-2026\n\n"
    "pub fn preview_probe(input: &str) -> usize {\n"
    "    let mut total = 0usize;\n"
    "    for ch in input.chars() {\n"
    "        if ch.is_alphabetic() { total += 1; }\n"
    "    }\n"
    "    total\n"
    "}\n"
)

# 4. Text PDF (hand-built, valid xref so pdf-extract can parse it)
def build_pdf(path, text):
    objects = []
    objects.append(b"<< /Type /Catalog /Pages 2 0 R >>")
    objects.append(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
    objects.append(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
    )
    stream = ("BT /F1 16 Tf 72 720 Td (" + text + ") Tj ET").encode("latin-1")
    objects.append(b"<< /Length " + str(len(stream)).encode() + b" >>\nstream\n" + stream + b"\nendstream")
    objects.append(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
    data = bytearray(b"%PDF-1.4\n")
    offsets = []
    for i, body in enumerate(objects, start=1):
        offsets.append(len(data))
        data += str(i).encode() + b" 0 obj\n" + body + b"\nendobj\n"
    xref_at = len(data)
    data += b"xref\n0 " + str(len(objects) + 1).encode() + b"\n"
    data += b"0000000000 65535 f \n"
    for off in offsets:
        data += ("%010d 00000 n \n" % off).encode()
    data += (
        b"trailer\n<< /Size " + str(len(objects) + 1).encode() + b" /Root 1 0 R >>\n"
        b"startxref\n" + str(xref_at).encode() + b"\n%%EOF\n"
    )
    open(path, "wb").write(bytes(data))

build_pdf(os.path.join(out, "report.pdf"),
          "WALRUS-PDF-2026 preview audit sample document text. Second line: WALRUS-PDF-2026.")

# 5. DOCX
try:
    import docx
    d = docx.Document()
    d.add_heading("Preview Audit DOCX", level=1)
    d.add_paragraph("This Word document carries the token KOALA-DOCX-2026 for content search.")
    d.add_paragraph("Second paragraph: KOALA-DOCX-2026 again.")
    d.save(os.path.join(out, "report.docx"))
except Exception as e:
    print("docx generation failed:", e, file=sys.stderr)

# 6. XLSX
try:
    from openpyxl import Workbook
    wb = Workbook()
    ws = wb.active
    ws.title = "Audit"
    ws["A1"] = "Token"
    ws["A2"] = "TIGER-XLSX-2026"
    ws["B1"] = "Meaning"
    ws["B2"] = "spreadsheet preview audit"
    wb.save(os.path.join(out, "sheet.xlsx"))
except Exception as e:
    print("xlsx generation failed:", e, file=sys.stderr)

# 7. PNG with rendered text (OCR fixture) and image-only (scanned) PDF
try:
    from PIL import Image, ImageDraw, ImageFont
    img = Image.new("RGB", (900, 260), (250, 250, 250))
    draw = ImageDraw.Draw(img)
    try:
        font = ImageFont.truetype("C:/Windows/Fonts/segoeui.ttf", 44)
    except Exception:
        font = ImageFont.load_default()
    draw.text((30, 40), "IMAGE-OCR-PROBE-2026", fill=(10, 10, 10), font=font)
    draw.text((30, 130), "screenshot preview audit", fill=(40, 40, 40), font=font)
    img.save(os.path.join(out, "diagram.png"))
    img.save(os.path.join(out, "scanned.pdf"), "PDF", resolution=96.0)
except Exception as e:
    print("pillow generation failed:", e, file=sys.stderr)

# 8. Binary blob
rnd = random.Random(20261110)
open(os.path.join(out, "blob.bin"), "wb").write(bytes(rnd.getrandbits(8) for _ in range(4096)))

# 9. Unsupported extension with text content
open(os.path.join(out, "unknown.xyz"), "w", encoding="utf-8").write("UNSUPPORTED-XYZ-2026 text payload\n")

# 10. Large text to exercise truncation (~1 MB)
open(os.path.join(out, "large.txt"), "w", encoding="utf-8").write(
    ("LARGE-TXT-2026 line\n" * 60000)
)

print("fixtures written to", os.path.abspath(out))
for name in sorted(os.listdir(out)):
    print(" ", name, os.path.getsize(os.path.join(out, name)))
