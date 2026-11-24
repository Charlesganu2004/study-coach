"""Check generated OOXML and native import using synthetic, local fixtures only."""

import json
import pathlib
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile


ROOT = pathlib.Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "target" / "document-verification"
EVIDENCE.mkdir(parents=True, exist_ok=True)
HELPER = ROOT / "scripts" / "document-import.ps1"
POWERSHELL = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"


def archive(name, entries):
    path = EVIDENCE / name
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as package:
        for entry_name, text in entries.items():
            package.writestr(entry_name, text)
    return path


def converted(path, expected=None, error=False):
    output = EVIDENCE / (path.stem + "-import.json")
    completed = subprocess.run(
        [POWERSHELL, "-NoLogo", "-NoProfile", "-NonInteractive", "-File", str(HELPER),
         "-InputFile", str(path), "-OutputFile", str(output), "-Format", path.suffix[1:]],
        capture_output=True, timeout=95, check=False,
    )
    result = json.loads(output.read_text(encoding="utf-8-sig"))
    if error:
        assert completed.returncode != 0 and result.get("error"), (path.name, result)
        return result
    assert completed.returncode == 0, (path.name, result)
    assert expected in result["text"], (path.name, result)
    return result


def pdf_fixture():
    content = b"BT /F1 24 Tf 50 730 Td (A unit test checks one function in isolation.) Tj ET"
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Length " + str(len(content)).encode() + b" >>\nstream\n" + content + b"\nendstream",
    ]
    output = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for number, body in enumerate(objects, 1):
        offsets.append(len(output))
        output.extend(f"{number} 0 obj\n".encode() + body + b"\nendobj\n")
    xref = len(output)
    output.extend(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode())
    for offset in offsets[1:]:
        output.extend(f"{offset:010d} 00000 n \n".encode())
    output.extend(f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    path = EVIDENCE / "notes.pdf"
    path.write_bytes(output)
    return path


def main():
    exported = EVIDENCE / "study-notes.docx"
    assert exported.is_file(), "Run Rust document tests with STUDY_COACH_DOCUMENT_EVIDENCE first."
    with zipfile.ZipFile(exported) as package:
        assert package.testzip() is None
        for name in package.namelist():
            if name.endswith((".xml", ".rels", ".svg")):
                ET.fromstring(package.read(name))
        document = ET.fromstring(package.read("word/document.xml"))
        ns = {"w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main"}
        paragraphs = ["".join(p.itertext()) for p in document.findall(".//w:p", ns)]
        assert any("Tests & evidence" in paragraph for paragraph in paragraphs)
        assert any("Unicode: 中文." in paragraph for paragraph in paragraphs)
        assert any("Unit test connects to Integration test." in paragraph for paragraph in paragraphs)
        assert "word/media/diagram.svg" in package.namelist()
    results = {"docx_package": "ZIP checksums and all XML/SVG parse", "fixtures_only": True}
    results["docx_import"] = converted(exported, "A unit test checks one function in isolation.")
    slide_namespace = "http://schemas.openxmlformats.org/drawingml/2006/main"
    pptx = archive("notes.pptx", {
        "ppt/slides/slide2.xml": f'<a:p xmlns:a="{slide_namespace}"><a:r><a:t>Second slide</a:t></a:r></a:p>',
        "ppt/slides/slide1.xml": f'<a:p xmlns:a="{slide_namespace}"><a:r><a:t>First slide about tests.</a:t></a:r></a:p>',
    })
    results["pptx_import"] = converted(pptx, "First slide about tests.")
    assert results["pptx_import"]["text"].index("First slide") < results["pptx_import"]["text"].index("Second slide")
    sheet_namespace = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
    xlsx = archive("notes.xlsx", {
        "xl/sharedStrings.xml": f'<sst xmlns="{sheet_namespace}"><si><t>Unit tests</t></si></sst>',
        "xl/worksheets/sheet1.xml": f'<worksheet xmlns="{sheet_namespace}"><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1"><v>42</v></c><c r="C1" t="inlineStr"><is><t>Evidence</t></is></c></row></sheetData></worksheet>',
    })
    results["xlsx_import"] = converted(xlsx, "A1: Unit tests\tB1: 42\tC1: Evidence")
    malicious = archive("macro.docx", {"word/document.xml": "<document/>", "word/vbaProject.bin": "not a macro"})
    results["macro_rejection"] = converted(malicious, error=True)
    xxe = archive("external-entity.docx", {"word/document.xml": '<!DOCTYPE x [<!ENTITY ex SYSTEM "file:///C:/Windows/win.ini">]><x>&ex;</x>'})
    results["external_entity_rejection"] = converted(xxe, error=True)
    bomb = archive("too-expanded.docx", {"word/document.xml": " " * (33 * 1024 * 1024)})
    results["zip_bomb_rejection"] = converted(bomb, error=True)
    image = EVIDENCE / "ocr-input.png"
    drawing = (
        "Add-Type -AssemblyName System.Drawing; "
        "$image = New-Object System.Drawing.Bitmap(1400,500); "
        "$graphics = [System.Drawing.Graphics]::FromImage($image); "
        "$graphics.Clear([System.Drawing.Color]::White); "
        "$font = New-Object System.Drawing.Font('Arial',34); "
        "$graphics.DrawString('A unit test checks one function in isolation.', $font, [System.Drawing.Brushes]::Black, 45,120); "
        f"$image.Save('{str(image).replace(chr(39), chr(39) * 2)}', [System.Drawing.Imaging.ImageFormat]::Png); "
        "$graphics.Dispose(); $font.Dispose(); $image.Dispose()"
    )
    subprocess.run([POWERSHELL, "-NoProfile", "-NonInteractive", "-Command", drawing], check=True, timeout=20)
    results["screenshot_ocr"] = converted(image, "A unit test checks one function in isolation.")
    results["pdf_ocr"] = converted(pdf_fixture(), "A unit test checks one function in isolation.")
    (EVIDENCE / "verification.json").write_text(json.dumps(results, ensure_ascii=False, indent=2), encoding="utf-8")
    print("PASS: editable DOCX with vector diagram; DOCX/PPTX/XLSX/PDF/screenshot import; macro/XXE/ZIP-bomb rejection")


if __name__ == "__main__":
    main()
