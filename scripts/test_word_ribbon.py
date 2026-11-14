"""Validate packaged VBA event classes with the dependencies in word-template/test-requirements.txt.

Run: python -m unittest discover -s scripts -p test_word_ribbon.py
This verifies the DOTM's XML and VBA streams, not Word runtime behavior.
"""
from pathlib import Path
from io import BytesIO
import unittest
import zipfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent
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
                for name in ('EqnTide', 'EqnTideEvents')
            }

    def test_event_class_is_nonprocedural_and_instantiable_as_a_class(self):
        self.assertIn(b'Class=EqnTideEvents\r\n', self.project)
        self.assertNotIn(b'Document=EqnTideEvents', self.project)
        # MODULETYPE 0x0022 is required for WithEvents class modules.
        event_record = self.directory[self.directory.index(b'EqnTideEvents'):]
        self.assertIn(b'\x22\x00\x00\x00\x00\x00', event_record)
        self.assertNotIn(b'\x21\x00\x00\x00\x00\x00', event_record)

    def test_packaged_sources_match_repository_sources_without_duplication(self):
        for name, suffix in [('EqnTide', 'bas'), ('EqnTideEvents', 'cls')]:
            source = (ROOT / f'word-template/{name}.{suffix}').read_text()
            self.assertEqual(self.modules[name].replace('\r\n', '\n'), source)
            self.assertEqual(self.modules[name].count('Attribute VB_Name ='), 1)

    def test_ribbon_initializes_event_sink_and_activates_the_existing_tab(self):
        self.assertEqual(self.ribbon.attrib['onLoad'], 'RibbonOnLoad')
        ns = {'r': 'http://schemas.microsoft.com/office/2009/07/customui'}
        tab = self.ribbon.find('.//r:tab', ns)
        self.assertIn(f'mRibbon.ActivateTab "{tab.attrib["id"]}"', self.modules['EqnTide'])
        self.assertIn('Public WithEvents App As Word.Application', self.modules['EqnTideEvents'])
        self.assertIn('EqnTide.HandleEquationSelection Sel', self.modules['EqnTideEvents'])
        self.assertIn('Set mWordEvents = New EqnTideEvents', self.modules['EqnTide'])
        self.assertEqual(self.ribbon.findall('.//r:toggleButton', ns), [])

    def test_packaged_word_output_policy_uses_dual_mathtype_package(self):
        source = self.modules['EqnTide']
        self.assertIn('CallBridge(rawLatex, "latex", "flat_opc_mathtype")', source)
        self.assertIn('CallBridge(cleanLatex, "latex", "flat_opc_mathtype")', source)
        self.assertNotIn('"flat_opc_omml"', source)
        self.assertIn('""target_mode"":""mathtype""', source)
        self.assertIn('CallBridge(shapeRange.WordOpenXML, "flat_opc", "latex")', source)


if __name__ == '__main__':
    unittest.main()
