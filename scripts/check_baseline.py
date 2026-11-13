#!/usr/bin/env python3
import sys
import os
import subprocess
import zipfile
import xml.dom.minidom
from PIL import Image

def analyze_docx_xml(docx_path):
    print(f"\n--- [XML Structure] {docx_path} ---")
    with zipfile.ZipFile(docx_path) as z:
        xml_bytes = z.read('word/document.xml')
    dom = xml.dom.minidom.parseString(xml_bytes)
    paragraphs = dom.getElementsByTagName('w:p')
    print(f"Total paragraphs: {len(paragraphs)}")
    for p_idx, p in enumerate(paragraphs):
        p_text_parts = []
        runs = p.getElementsByTagName('w:r')
        has_content = False
        for r in runs:
            pos_elem = r.getElementsByTagName('w:position')
            pos_val = pos_elem[0].getAttribute('w:val') if pos_elem else "0"
            
            t_elems = r.getElementsByTagName('w:t')
            t_text = "".join(t.childNodes[0].nodeValue for t in t_elems if t.childNodes)
            
            objs = r.getElementsByTagName('w:object')
            if objs:
                p_text_parts.append(f"[OLE Equation pos={pos_val}]")
                has_content = True
            elif t_text:
                disp = repr(t_text) if len(t_text) < 30 else repr(t_text[:27] + "...")
                p_text_parts.append(f"[Text: {disp} pos={pos_val}]")
                has_content = True
        if has_content:
            print(f"  Paragraph #{p_idx + 1}: {' '.join(p_text_parts)}")

def analyze_docx_visual(docx_path, out_png=None):
    print(f"\n--- [Visual Rendering] {docx_path} ---")
    base = os.path.splitext(os.path.basename(docx_path))[0]
    outdir = "/tmp/eqntide_check"
    os.makedirs(outdir, exist_ok=True)
    pdf_path = os.path.join(outdir, f"{base}.pdf")
    png_prefix = os.path.join(outdir, f"{base}_page")
    page1_png = os.path.join(outdir, f"{base}_page-1.png")

    for f in [pdf_path, page1_png]:
        if os.path.exists(f):
            os.remove(f)

    # 1. LibreOffice headless docx -> pdf
    res = subprocess.run(['libreoffice', '--headless', '--convert-to', 'pdf', docx_path, '--outdir', outdir],
                         capture_output=True, text=True)
    if res.returncode != 0:
        print(f"Error: LibreOffice conversion failed: {res.stderr}")
        return False

    # 2. pdftoppm pdf -> png
    res2 = subprocess.run(['pdftoppm', '-png', '-r', '150', pdf_path, png_prefix],
                          capture_output=True, text=True)
    if not os.path.exists(page1_png):
        print(f"Error: pdftoppm failed to produce {page1_png}")
        return False

    im = Image.open(page1_png).convert('L')
    w, h = im.size
    pixels = im.load()
    dark_pixels = []
    for y in range(h):
        for x in range(w):
            if pixels[x, y] < 200:
                dark_pixels.append((x, y))

    if not dark_pixels:
        print("Notice: Document appears blank (no dark pixels).")
        return True

    min_x = min(p[0] for p in dark_pixels)
    max_x = max(p[0] for p in dark_pixels)
    min_y = min(p[1] for p in dark_pixels)
    max_y = max(p[1] for p in dark_pixels)
    line_height = max_y - min_y

    print(f"Rendered line bounding box: ({min_x}, {min_y}) to ({max_x}, {max_y}), height = {line_height}px")
    
    # Save crop
    target_crop = out_png or os.path.join(outdir, f"{base}_crop.png")
    crop = im.crop((max(0, min_x - 10), max(0, min_y - 10), min(w, max_x + 10), min(h, max_y + 10)))
    crop.save(target_crop)
    print(f"Cropped visualization saved to: {target_crop}")

    # Interpretation:
    # A single line with normal formula has height around 45 - 60px.
    # When baseline drops significantly, height exceeds 70px.
    if line_height > 70:
        print(f"FAIL: Line height ({line_height}px) indicates dropped baseline (tụt baseline)!")
        return False
    else:
        print(f"PASS: Line height ({line_height}px) is within normal range (thẳng hàng).")
        return True

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python3 check_baseline.py <path-to-docx>")
        sys.exit(1)
    docx = sys.argv[1]
    analyze_docx_xml(docx)
    analyze_docx_visual(docx)
