"""Validate committed pre-rasterized Tabler icon PNGs and manifest sync."""

from pathlib import Path
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import package_native  # noqa: E402
import render_icons  # noqa: E402


class RenderedIconsTests(unittest.TestCase):
    def test_all_desktop_and_ribbon_pngs_exist_and_match_manifest(self):
        self.assertEqual(render_icons.check_all(), 0)

    def test_expected_icon_counts_and_dimensions(self):
        self.assertEqual(len(render_icons.DESKTOP_ICONS), 30)
        self.assertEqual(len(render_icons.RIBBON_ICONS), 5)
        expected_svgs = set(render_icons.DESKTOP_ICONS) | set(render_icons.RIBBON_ICONS)
        actual_svgs = {p.stem for p in render_icons.SVG_DIR.glob("*.svg")}
        self.assertEqual(actual_svgs, expected_svgs)
        for group, name, path, size, _color in render_icons.expected_specs():
            with self.subTest(group=group, name=name):
                self.assertTrue(path.is_file(), f"Missing {path}")
                w, h, color_type = render_icons.png_dimensions_rgba(path)
                self.assertEqual((w, h, color_type), (size, size, 6))

    def test_word_dotm_embeds_distinct_ribbon_icons(self):
        expected = {
            "btnInsertEquation": "images/pencil-plus.png",
            "btnEditEquation": "images/edit.png",
            "btnToggleLatex": "images/tex.png",
            "btnUpgradeMathType": "images/arrow-big-up-lines.png",
            "btnBatchConverter": "images/file-type-docx.png",
        }
        with zipfile.ZipFile(ROOT / "word-template/EqnTide.dotm") as z:
            names = set(z.namelist())
            ui = ET.fromstring(z.read("customUI/customUI14.xml"))
            rels = ET.fromstring(z.read("customUI/_rels/customUI14.xml.rels"))
            rel_map = {r.attrib["Id"]: r.attrib["Target"] for r in rels}
            ns = {"r": "http://schemas.microsoft.com/office/2009/07/customui"}
            buttons = {btn.attrib["id"]: btn.attrib["image"] for btn in ui.findall(".//r:button", ns)}
            self.assertEqual(set(buttons.keys()), set(expected.keys()))
            for btn_id, target in expected.items():
                rel_id = buttons[btn_id]
                self.assertEqual(rel_map.get(rel_id), target)
                self.assertIn(f"customUI/{target}", names)

    def test_package_native_stages_icons_and_license(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            binaries = base / "bin"
            binaries.mkdir()
            for name in ("eqntide-cli", "eqntide-desktop"):
                (binaries / name).write_bytes(b"bin")
            stage = base / "stage"
            stage.mkdir()
            package_native.populate(stage, binaries, windows=False)
            self.assertTrue((stage / "NOTICE").is_file())
            self.assertIn("Tabler Icons", (stage / "NOTICE").read_text(encoding="utf-8"))
            self.assertTrue((stage / "icons/tabler-icons/LICENSE.txt").is_file())
            self.assertTrue((stage / "wps-addin/assets/icons/pencil-plus-32.png").is_file())
            self.assertTrue((stage / "word-template/EqnTide.dotm").is_file())


if __name__ == "__main__":
    unittest.main()
