import os
import sys
import zipfile
import shutil

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from excel_press import CompressedVBA
from filetime_compat import patch_filetime

patch_filetime()

from ms_ovba.vbaProject import VbaProject
from ms_ovba.Models.Entities.std_module import StdModule
from ms_ovba.Models.Entities.module_base import ModuleBase
from ms_ovba.Views.project_ole_file import ProjectOleFile

# Patch ModuleBase.write_file to use working CompressedVBA
def patched_write_file(self):
    with open(self._file_path + ".bin", "wb") as bin_f:
        bin_f.write(self._cache)
        with open(self._file_path + ".new", mode="rb") as new_f:
            contents = new_f.read()
        compressed = CompressedVBA(contents).compress()
        bin_f.write(compressed)

ModuleBase.write_file = patched_write_file

DOTM_PATH = "word-template/EqnTide.dotm"
BAS_PATH = "word-template/EqnTide.bas"
TMP_DIR = "word-template/tmp_build"

if os.path.exists(TMP_DIR):
    shutil.rmtree(TMP_DIR)
os.makedirs(TMP_DIR, exist_ok=True)

# 1. Generate vbaProject.bin using ms_ovba with patched compressor
proj = VbaProject()
proj.project_id = "{29F9E300-746A-4D92-BF39-382903C5D610}"
class EventClassModule(StdModule):
    """VBA class: non-procedural MODULETYPE (0x22), PROJECT Class= entry.

    ms-ovba 0.0.1 only exposes StdModule/DocModule; reuse its non-procedural
    record writer without declaring this event sink as a host document module.
    """
    def __init__(self, name):
        super().__init__(name)
        self.type = "Document"  # ModuleBase.pack emits MODULETYPE 0x22.

    def to_project_module_string(self):
        return "Class=" + self.name


for module, source in [
    (StdModule("EqnTide"), BAS_PATH),
    (EventClassModule("EqnTideEvents"), "word-template/EqnTideEvents.cls"),
]:
    # normalize_file appends to .new; always use fresh temporary source files.
    module_source = os.path.abspath(os.path.join(TMP_DIR, os.path.basename(source)))
    shutil.copyfile(source, module_source)
    module.add_file(module_source)
    module.normalize_file()
    proj.add_module(module)

vba_bin_path = os.path.join(TMP_DIR, "vbaProject.bin")
cur_cwd = os.getcwd()
os.chdir(TMP_DIR)
ProjectOleFile.write_file(proj)
os.chdir(cur_cwd)

assert os.path.exists(vba_bin_path), "vbaProject.bin not generated!"
print(f"Generated vbaProject.bin ({os.path.getsize(vba_bin_path)} bytes)")

# 2. Prepare XML components for .dotm
content_types_xml = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="bin" ContentType="application/vnd.ms-office.vbaProject"/>
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.ms-word.template.macroEnabledTemplate.main+xml"/>
  <Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/customUI/customUI14.xml" ContentType="application/xml"/>
</Types>"""

root_rels = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
  <Relationship Id="rId2" Type="http://schemas.microsoft.com/office/2007/relationships/ui/extensibility" Target="customUI/customUI14.xml"/>
</Relationships>"""

doc_rels = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId3" Type="http://schemas.microsoft.com/office/2006/relationships/vbaProject" Target="vbaProject.bin"/>
</Relationships>"""

document_xml = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p/>
  </w:body>
</w:document>"""

settings_xml = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>"""

styles_xml = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>"""

customui14_xml = r"""<customUI xmlns="http://schemas.microsoft.com/office/2009/07/customui" onLoad="RibbonOnLoad">
  <ribbon>
    <tabs>
      <tab id="EqnTideTab" label="EqnTide">
        <group id="EqnTideEqGroup" label="Equations">
          <button id="btnToggleLatex" label="Toggle LaTeX" size="large" imageMso="EquationProfessional" onAction="RibbonToggleLatex" screentip="Toggle LaTeX (Alt + \)" supertip="Convert LaTeX to OMML or OMML equation back to LaTeX text." />
          <button id="btnInsertEquation" label="Insert Equation" size="large" imageMso="EquationInsertGallery" onAction="RibbonInsertEquation" screentip="Insert Equation" supertip="Open EqnTide equation editor." />
          <button id="btnEditEquation" label="Edit Equation" size="large" imageMso="EquationNormalText" onAction="RibbonEditEquation" screentip="Edit Equation" supertip="Edit selected equation." />
          <button id="btnUpgradeMathType" label="Upgrade MathType" size="large" imageMso="EquationLinearFormat" onAction="RibbonUpgradeMathType" screentip="Upgrade MathType" supertip="Upgrade legacy MathType equations." />
        </group>
      </tab>
    </tabs>
  </ribbon>
</customUI>"""

# 3. Assemble .dotm ZIP
with zipfile.ZipFile(DOTM_PATH, "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("[Content_Types].xml", content_types_xml)
    z.writestr("_rels/.rels", root_rels)
    z.writestr("word/_rels/document.xml.rels", doc_rels)
    z.writestr("word/document.xml", document_xml)
    z.writestr("word/settings.xml", settings_xml)
    z.writestr("word/styles.xml", styles_xml)
    z.writestr("customUI/customUI14.xml", customui14_xml)
    with open(vba_bin_path, "rb") as vf:
        z.writestr("word/vbaProject.bin", vf.read())

shutil.rmtree(TMP_DIR, ignore_errors=True)
print(f"Successfully generated: {DOTM_PATH} ({os.path.getsize(DOTM_PATH)} bytes)")
