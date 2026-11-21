"""Validate packaged VBA event classes with the dependencies in word-template/test-requirements.txt.

Run: python -m unittest discover -s tests/python -p test_word_ribbon.py
This verifies the DOTM's XML and VBA streams, not Word runtime behavior.
"""
from pathlib import Path
from io import BytesIO
import unittest
import zipfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
import olefile
from oletools.olevba import decompress_stream


class WordRibbon(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        with zipfile.ZipFile(ROOT / 'word-template/EqnTide.dotm') as package:
            cls.ribbon = ET.fromstring(package.read('customUI/customUI14.xml'))
            binary = package.read('word/vbaProject.bin')
        with olefile.OleFileIO(BytesIO(binary)) as ole:
            cls.project = ole.openstream('PROJECT').read()
            cls.directory = decompress_stream(ole.openstream('VBA/dir').read())
            cls.modules = {
                name: decompress_stream(ole.openstream('VBA/' + name).read()).decode('cp1252')
                for name in ('EqnTideMacros', 'ThisDocument')
            }

    def test_vba_project_modules_and_names_avoid_vbe_collision(self):
        self.assertIn(b'Name="EqnTide"\r\n', self.project)
        self.assertIn(b'Document=ThisDocument/&H00000000\r\n', self.project)
        self.assertIn(b'Module=EqnTideMacros\r\n', self.project)
        # MODULETYPE 0x0022 is required for ThisDocument, 0x0021 for standard EqnTideMacros.
        macros_idx = self.directory.index(b'EqnTideMacros')
        doc_record = self.directory[self.directory.index(b'ThisDocument'):macros_idx]
        macros_record = self.directory[macros_idx:]
        self.assertIn(b'\x22\x00\x00\x00\x00\x00', doc_record)
        self.assertIn(b'\x21\x00\x00\x00\x00\x00', macros_record)

    def test_packaged_sources_match_repository_sources_without_duplication(self):
        source = (ROOT / 'word-template/EqnTide.bas').read_text()
        self.assertEqual(self.modules['EqnTideMacros'].replace('\r\n', '\n'), source)
        self.assertEqual(self.modules['EqnTideMacros'].count('Attribute VB_Name ='), 1)
        self.assertIn('Attribute VB_Name = "ThisDocument"', self.modules['ThisDocument'])

    def test_ribbon_buttons_map_to_packaged_vba_callbacks(self):
        ns = {'r': 'http://schemas.microsoft.com/office/2009/07/customui'}
        tab = self.ribbon.find('.//r:tab', ns)
        self.assertIsNotNone(tab)
        self.assertEqual(tab.attrib['id'], 'EqnTideTab')
        buttons = self.ribbon.findall('.//r:button', ns)
        self.assertGreaterEqual(len(buttons), 5)
        for button in buttons:
            callback = button.attrib['onAction']
            self.assertIn(f'Public Sub {callback}(control As IRibbonControl)', self.modules['EqnTideMacros'])
        self.assertEqual(self.ribbon.findall('.//r:toggleButton', ns), [])

    def test_packaged_word_output_policy_uses_dual_mathtype_package(self):
        source = self.modules['EqnTideMacros']
        self.assertIn('CallBridgeEx(rawLatex, "latex", "flat_opc_mathtype"', source)
        self.assertIn('CallBridge(cleanLatex, "latex", "flat_opc_mathtype")', source)
        self.assertNotIn('"flat_opc_omml"', source)
        self.assertIn('""target_mode"":""mathtype""', source)
        self.assertIn('CallBridgeEx(CompactFlatOpc(shapeXml), "flat_opc", "latex"', source)

    def test_upgrade_releases_word_file_before_bridge_and_reopens_on_error(self):
        macro = self.modules['EqnTideMacros'].split('Public Sub UpgradeMathTypeMacro()', 1)[1].split('End Sub', 1)[0]
        close = macro.index('doc.Close 0')
        send = macro.index('http.send payload')
        self.assertLess(macro.index('If Not doc.Saved Then'), close)
        self.assertLess(close, send)
        self.assertGreater(macro.index('Documents.Open docPath'), send)
        self.assertIn('Documents.Open docPath', macro.split('UpgradeError:', 1)[1])

    def test_toggle_latex_declares_word_constants_for_option_explicit(self):
        source = self.modules['EqnTideMacros']
        self.assertIn('Option Explicit', source)
        self.assertIn('Private Const wdInlineShapeEmbeddedOLEObject As Long = 1', source)
        self.assertIn('Private Const wdCollapseEnd As Long = 0', source)
        self.assertIn('Private Function ResolveMatchRange(', source)
        self.assertIn('Private Function IsEquationShape(', source)
        self.assertIn('Private Function CompactFlatOpc(', source)
        toggle = source.split('Public Sub ToggleLatexMacro()', 1)[1].split('End Sub', 1)[0]
        self.assertIn('om.Remove', toggle)
        self.assertIn('ResolveMatchRange(', toggle)
        self.assertIn('IsEquationShape(shape, shapeXml)', toggle)
        self.assertIn('For Each shape In shapesCol', toggle)
        self.assertIn('For Each om In omathsCol', toggle)
        self.assertNotIn('shapesCol(i)', toggle)
        self.assertNotIn('omathsCol(i)', toggle)
        self.assertEqual(toggle.count('shapeRange.WordOpenXML'), 1)
        self.assertIn('Application.Options.Pagination = False', toggle)
        self.assertIn('Application.UndoRecord.StartCustomRecord', toggle)

    def test_json_helpers_avoid_quadratic_char_concatenation(self):
        source = self.modules['EqnTideMacros']
        escape_fn = source.split('Private Function JsonEscape(', 1)[1].split('End Function', 1)[0]
        extract_fn = source.split('Private Function ExtractJsonField(', 1)[1].split('End Function', 1)[0]
        self.assertIn('Replace$(', escape_fn)
        self.assertNotIn('out = out & ch', escape_fn)
        self.assertIn('Join(chunks, "")', extract_fn)
        self.assertNotIn('value = value & ch', extract_fn)

    def test_all_flat_opc_insertions_use_inline_break_cleanup(self):
        source = self.modules['EqnTideMacros']
        self.assertIn('Private Sub InsertFlatOpcInline(', source)
        self.assertEqual(source.count('.InsertXML'), 1)
        self.assertIn('target.InsertXML flatOpc', source)
        self.assertIn('InsertFlatOpcInline subRng, flatOpc, False', source)
        self.assertIn('InsertFlatOpcInline sel.Range.Duplicate, opc, True', source)
        self.assertIn('InsertFlatOpcInline target, flatOpc, True', source)
        self.assertIn('If actualSuffix = suffix Then', source)
        self.assertIn('tail.SetRange insertedEnd - 1, insertedEnd', source)
        self.assertIn('tail.Delete', source)


if __name__ == '__main__':
    unittest.main()
