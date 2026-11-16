import binascii
import io
import math
import os
import shutil
import struct
import sys
import uuid
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from excel_press import CompressedVBA
from filetime_compat import patch_filetime

patch_filetime()

from ms_ovba.vbaProject import VbaProject
from ms_ovba.Models.Entities.std_module import StdModule
from ms_ovba.Models.Entities.doc_module import DocModule
from ms_ovba.Models.Entities.module_base import ModuleBase
from ms_ovba.Models.Entities.reference import Reference
from ms_ovba.Models.Entities.reference_registered import ReferenceRegistered
from ms_ovba.Models.Fields.idSizeField import IdSizeField
from ms_ovba.Models.Fields.doubleEncodedString import DoubleEncodedString
from ms_ovba.Models.Fields.packed_data import PackedData
from ms_ovba.Models.Fields.libid_reference import LibidReference
from ms_ovba.Views.dirStream import DirStream
from ms_ovba.Views.project import Project
from ms_ovba.Views.projectWm import ProjectWm
from ms_ovba_crypto import MsOvbaCrypto

PROJECT_NAME = "EqnTide"
MODULE_NAME = "EqnTideMacros"  # Must differ from PROJECT_NAME to avoid VBE namespace collision
_VBA_PROJECT_VERSION = 0x00B5  # Office 2016+ VBA7.1; empty cache forces Word to compile source


# Patch Project.__str__ to emit MS-OVBA compliant PROJECT stream (matching Word/MathType)
# Note: References belong strictly in VBA/dir, NOT in the PROJECT stream.
def patched_project_str(self):
    project = self._project
    project_id = project.project_id
    result = [f'ID="{project_id}"']
    for module in project.modules:
        result += [module.to_project_module_string()]
    result += ['HelpFile=""']
    result += [f'Name="{PROJECT_NAME}"']
    result += ['HelpContextID="' + str(project.help_context_id) + '"']
    result += ['VersionCompatible32="393222000"']
    for name, value in self.attributes.items():
        result += [f'{name}="{value}"']
    cmg = MsOvbaCrypto.encrypt(project_id, project.protection_state)
    dpb = MsOvbaCrypto.encrypt(project_id, project.password)
    gc = MsOvbaCrypto.encrypt(project_id, project.visibility_state)
    result += [f'CMG="{binascii.hexlify(cmg).upper().decode("ascii")}"']
    result += [f'DPB="{binascii.hexlify(dpb).upper().decode("ascii")}"']
    result += [f'GC="{binascii.hexlify(gc).upper().decode("ascii")}"']
    result += ['']
    result += ['[Host Extender Info]']
    result += [self.hostExtenderInfo]
    result += ['']
    result += ['[Workspace]']
    for module in project.modules:
        result += [f'{module.modName.value}=0, 0, 0, 0, C']
    result += ['']
    return '\r\n'.join(result)


Project.__str__ = patched_project_str


# Patch DirStream to use PROJECT_NAME instead of hardcoded "VBAProject"
def patched_dir_load_information(self):
    codepage = 0x04E4
    syskind = IdSizeField(1, 4, 3)  # 64-bit Windows
    compat_version = IdSizeField(74, 4, 3)
    lcid = IdSizeField(2, 4, 0x0409)
    lcid_invoke = IdSizeField(20, 4, 0x0409)
    codepage_record = IdSizeField(3, 2, codepage)
    project_name = IdSizeField(4, len(PROJECT_NAME), PROJECT_NAME)
    docstring = DoubleEncodedString([5, 0x0040], "")
    helpfile = DoubleEncodedString([6, 0x003D], "")
    help_context = IdSizeField(7, 4, self._project.help_context_id)
    lib_flags = IdSizeField(8, 4, 0)
    version = IdSizeField(9, 4, 0x65BE0257)
    minor_version = PackedData("H", 17)
    constants = DoubleEncodedString([12, 0x003C], "")

    information = [syskind]
    if self._include_compat:
        information.append(compat_version)
    information.extend([
        lcid,
        lcid_invoke,
        codepage_record,
        project_name,
        docstring,
        helpfile,
        help_context,
        lib_flags,
        version,
        minor_version,
        constants,
    ])
    return information


DirStream._load_information = patched_dir_load_information


# ---------------------------------------------------------------------------
# Clean MS-CFB (OLE2) builder without ms_cfb's 0x01/0x02 slack or color quirks
# ---------------------------------------------------------------------------
_ENDOFCHAIN = 0xFFFFFFFE
_FREESECT = 0xFFFFFFFF
_FATSECT = 0xFFFFFFFD
_NOSTREAM = 0xFFFFFFFF
_SECTOR_SIZE = 512
_MINI_SECTOR_SIZE = 64
_MINI_CUTOFF = 4096
_FAT_PER_SECTOR = _SECTOR_SIZE // 4


def _cfb_dir_entry(
    name: str,
    etype: int,
    color: int = 1,
    left: int = _NOSTREAM,
    right: int = _NOSTREAM,
    child: int = _NOSTREAM,
    start: int = _ENDOFCHAIN,
    size: int = 0,
) -> bytes:
    e = bytearray(128)
    u16 = name.encode("utf-16-le")
    e[0 : len(u16)] = u16
    struct.pack_into("<H", e, 64, len(u16) + 2)
    e[66] = etype
    e[67] = color
    struct.pack_into("<I", e, 68, left)
    struct.pack_into("<I", e, 72, right)
    struct.pack_into("<I", e, 76, child)
    struct.pack_into("<I", e, 116, start)
    struct.pack_into("<I", e, 120, size)
    return bytes(e)


def _cfb_empty_entry() -> bytes:
    e = bytearray(128)
    struct.pack_into("<I", e, 68, _NOSTREAM)
    struct.pack_into("<I", e, 72, _NOSTREAM)
    struct.pack_into("<I", e, 76, _NOSTREAM)
    struct.pack_into("<I", e, 116, _ENDOFCHAIN)
    return bytes(e)


def build_vba_ole2(streams: dict[str, bytes]) -> bytes:
    """Package VBA streams into a clean MS-CFB v3 container."""
    mod_stream = f"VBA/{MODULE_NAME}"
    stream_order = [
        "VBA/ThisDocument",
        mod_stream,
        "VBA/_VBA_PROJECT",
        "VBA/dir",
        "PROJECTwm",
        "PROJECT",
    ]
    small = [p for p in stream_order if len(streams[p]) < _MINI_CUTOFF]
    large = [p for p in stream_order if len(streams[p]) >= _MINI_CUTOFF]

    info: dict[str, tuple[int, int]] = {}  # path -> (start_sector, size)

    # Mini stream (64-byte mini-sectors, padded to 512-byte sector boundary)
    mini_stream = bytearray()
    mini_fat = bytearray()
    for path in small:
        data = streams[path]
        start_mini = len(mini_stream) // _MINI_SECTOR_SIZE
        info[path] = (start_mini, len(data))
        mini_stream += data
        mini_stream += b"\x00" * ((-len(mini_stream)) % _MINI_SECTOR_SIZE)
        n_mini = max(1, math.ceil(len(data) / _MINI_SECTOR_SIZE))
        for i in range(n_mini - 1):
            mini_fat += struct.pack("<I", start_mini + i + 1)
        mini_fat += struct.pack("<I", _ENDOFCHAIN)

    ms_logical = len(mini_stream)
    mini_stream += b"\x00" * ((-len(mini_stream)) % _SECTOR_SIZE)
    ms_sectors = len(mini_stream) // _SECTOR_SIZE if mini_stream else 0
    mfat_sectors = max(1, math.ceil(len(mini_fat) / _SECTOR_SIZE))
    while len(mini_fat) < mfat_sectors * _SECTOR_SIZE:
        mini_fat += struct.pack("<I", _FREESECT)

    large_blobs = {
        p: streams[p] + b"\x00" * ((-len(streams[p])) % _SECTOR_SIZE)
        for p in large
    }
    large_total = sum(len(b) // _SECTOR_SIZE for b in large_blobs.values())
    dir_sectors = 2

    fat_sectors = 1
    while True:
        total = fat_sectors + dir_sectors + mfat_sectors + ms_sectors + large_total
        if math.ceil(total / _FAT_PER_SECTOR) <= fat_sectors:
            break
        fat_sectors += 1

    dir_start = fat_sectors
    mfat_start = dir_start + dir_sectors
    ms_start = mfat_start + mfat_sectors
    cur = ms_start + ms_sectors
    for path in large:
        info[path] = (cur, len(streams[path]))
        cur += len(large_blobs[path]) // _SECTOR_SIZE

    # Build FAT
    fat = bytearray()
    for _ in range(fat_sectors):
        fat += struct.pack("<I", _FATSECT)
    for i in range(dir_sectors - 1):
        fat += struct.pack("<I", dir_start + i + 1)
    fat += struct.pack("<I", _ENDOFCHAIN)
    for i in range(mfat_sectors - 1):
        fat += struct.pack("<I", mfat_start + i + 1)
    fat += struct.pack("<I", _ENDOFCHAIN)
    if ms_sectors > 0:
        for i in range(ms_sectors - 1):
            fat += struct.pack("<I", ms_start + i + 1)
        fat += struct.pack("<I", _ENDOFCHAIN)
    for path in large:
        start_s, _ = info[path]
        n = len(large_blobs[path]) // _SECTOR_SIZE
        for i in range(n - 1):
            fat += struct.pack("<I", start_s + i + 1)
        fat += struct.pack("<I", _ENDOFCHAIN)
    while len(fat) < fat_sectors * _SECTOR_SIZE:
        fat += struct.pack("<I", _FREESECT)

    # Directory RB-trees sorted by (len(name), name.upper()) per MS-CFB 2.6.4:
    #   Root Entry (0) -> child 7 (PROJECT)
    #     PROJECT (7, len=7, black): left 1 (VBA, len=3, red), right 6 (PROJECTwm, len=9, red)
    #   VBA (1) -> child 2 (ThisDocument)
    #     ThisDocument (2, len=12 'T', black): left 5 (dir, len=3, black), right 4 (_VBA_PROJECT, len=12 '_', black)
    #       _VBA_PROJECT (4): right 3 (EqnTideMacros, len=13, red)
    entries = bytearray()
    entries += _cfb_dir_entry(
        "Root Entry", 5, color=1, child=7,
        start=ms_start if ms_sectors > 0 else _ENDOFCHAIN, size=ms_logical,
    )
    entries += _cfb_dir_entry("VBA", 1, color=0, child=2, start=0, size=0)
    entries += _cfb_dir_entry(
        "ThisDocument", 2, color=1, left=5, right=4,
        start=info["VBA/ThisDocument"][0], size=info["VBA/ThisDocument"][1],
    )
    entries += _cfb_dir_entry(
        MODULE_NAME, 2, color=0,
        start=info[mod_stream][0], size=info[mod_stream][1],
    )
    entries += _cfb_dir_entry(
        "_VBA_PROJECT", 2, color=1, right=3,
        start=info["VBA/_VBA_PROJECT"][0], size=info["VBA/_VBA_PROJECT"][1],
    )
    entries += _cfb_dir_entry(
        "dir", 2, color=1,
        start=info["VBA/dir"][0], size=info["VBA/dir"][1],
    )
    entries += _cfb_dir_entry(
        "PROJECTwm", 2, color=0,
        start=info["PROJECTwm"][0], size=info["PROJECTwm"][1],
    )
    entries += _cfb_dir_entry(
        "PROJECT", 2, color=1, left=1, right=6,
        start=info["PROJECT"][0], size=info["PROJECT"][1],
    )
    while len(entries) < dir_sectors * _SECTOR_SIZE:
        entries += _cfb_empty_entry()

    # Header (512 bytes)
    hdr = bytearray(512)
    hdr[0:8] = b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1"
    struct.pack_into("<H", hdr, 24, 0x003E)
    struct.pack_into("<H", hdr, 26, 0x0003)
    struct.pack_into("<H", hdr, 28, 0xFFFE)
    struct.pack_into("<H", hdr, 30, 9)
    struct.pack_into("<H", hdr, 32, 6)
    struct.pack_into("<I", hdr, 44, fat_sectors)
    struct.pack_into("<I", hdr, 48, dir_start)
    struct.pack_into("<I", hdr, 56, _MINI_CUTOFF)
    struct.pack_into("<I", hdr, 60, mfat_start)
    struct.pack_into("<I", hdr, 64, mfat_sectors)
    struct.pack_into("<I", hdr, 68, _ENDOFCHAIN)
    struct.pack_into("<I", hdr, 72, 0)
    for i in range(min(fat_sectors, 109)):
        struct.pack_into("<I", hdr, 76 + i * 4, i)
    for i in range(fat_sectors, 109):
        struct.pack_into("<I", hdr, 76 + i * 4, _FREESECT)

    out = bytearray(hdr) + fat + entries + mini_fat + mini_stream
    for path in large:
        out += large_blobs[path]
    return bytes(out)


DOTM_PATH = "word-template/EqnTide.dotm"
BAS_PATH = "word-template/EqnTide.bas"
LOGO_PATH = "wps-addin/assets/icon-32.png"

# 1. Build VBA project metadata & streams
proj = VbaProject()
proj.project_id = "{29F9E300-746A-4D92-BF39-382903C5D610}"
proj.include_projectwm()

# External TypeLib references (matches MathType Commands 2016.dotm: stdole 2.0 + Office 2.0)
lib_stdole = LibidReference(
    uuid.UUID("{00020430-0000-0000-C000-000000000046}"),
    "2.0",
    "0",
    r"C:\Windows\System32\stdole2.tlb",
    "OLE Automation",
)
proj.add_reference(Reference(ReferenceRegistered(lib_stdole), "stdole"))

lib_office = LibidReference(
    uuid.UUID("{2DF8D04C-5BFA-101B-BDE5-00AA0044DE52}"),
    "2.0",
    "0",
    r"C:\Program Files\Common Files\Microsoft Shared\OFFICE16\MSO.DLL",
    "Microsoft Office 16.0 Object Library",
)
proj.add_reference(Reference(ReferenceRegistered(lib_office), "Office"))

# ThisDocument document module (must match Word Document coclass attributes)
this_doc = DocModule("ThisDocument")
this_doc_src = (
    b'Attribute VB_Name = "ThisDocument"\r\n'
    b'Attribute VB_Base = "0{00020906-0000-0000-C000-000000000046}"\r\n'
    b"Attribute VB_GlobalNameSpace = False\r\n"
    b"Attribute VB_Creatable = False\r\n"
    b"Attribute VB_PredeclaredId = True\r\n"
    b"Attribute VB_Exposed = True\r\n"
    b"Attribute VB_TemplateDerived = False\r\n"
    b"Attribute VB_Customizable = True\r\n"
)
proj.add_module(this_doc)

# EqnTideMacros standard code module (CRLF line endings required by MS-OVBA)
mod = StdModule(MODULE_NAME)
with open(BAS_PATH, "rb") as bf:
    raw_bas = bf.read()
eqntide_src = raw_bas.replace(b"\r\n", b"\n").replace(b"\n", b"\r\n")
try:
    eqntide_src.decode("ascii")
except UnicodeDecodeError as exc:
    raise ValueError(f"{BAS_PATH} contains non-ASCII byte at offset {exc.start}") from exc
proj.add_module(mod)

raw_dir = DirStream(proj).to_bytes()
project_bytes = str(Project(proj)).encode("ascii")
projectwm_bytes = ProjectWm(proj).to_bytes()
vba_project_hdr = struct.pack("<HHBH", 0x61CC, _VBA_PROJECT_VERSION, 0x00, 0x0000)

mod_stream_name = f"VBA/{MODULE_NAME}"
streams = {
    "VBA/ThisDocument": CompressedVBA(this_doc_src).compress(),
    mod_stream_name: CompressedVBA(eqntide_src).compress(),
    "VBA/_VBA_PROJECT": vba_project_hdr,
    "VBA/dir": CompressedVBA(raw_dir).compress(),
    "PROJECTwm": projectwm_bytes,
    "PROJECT": project_bytes,
}
vba_bytes = build_vba_ole2(streams)
print(f"Generated vbaProject.bin ({len(vba_bytes)} bytes)")

# Verify MS-OVBA & MS-CFB integrity before packaging
import olefile
from oletools.olevba import VBA_Parser, decompress_stream

ole = olefile.OleFileIO(io.BytesIO(vba_bytes))
for required in ("PROJECT", "PROJECTwm", "VBA/_VBA_PROJECT", "VBA/dir", "VBA/ThisDocument", mod_stream_name):
    assert ole.exists(required), f"Missing {required} stream"
    ole.openstream(required).read()

dir_dec = decompress_stream(ole.openstream("VBA/dir").read())
assert dir_dec == raw_dir, f"VBA/dir roundtrip mismatch: {len(dir_dec)} != {len(raw_dir)}"
assert dir_dec[-6:] == struct.pack("<HI", 0x0010, 0), "VBA/dir missing terminator 0x0010"
assert decompress_stream(ole.openstream("VBA/ThisDocument").read()) == this_doc_src
assert decompress_stream(ole.openstream(mod_stream_name).read()) == eqntide_src
ole.close()

# 2. Prepare OOXML components for .dotm
content_types_xml = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="bin" ContentType="application/vnd.ms-office.vbaProject"/>
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.ms-word.template.macroEnabledTemplate.main+xml"/>
  <Override PartName="/word/vbaData.xml" ContentType="application/vnd.ms-word.vbaData+xml"/>
  <Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/customUI/customUI14.xml" ContentType="application/xml"/>
</Types>""".replace(
    "application/openxmlformats-officedocument.wordprocessingml.styles+xml",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
)

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

vba_rels = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.microsoft.com/office/2006/relationships/wordVbaData" Target="vbaData.xml"/>
</Relationships>"""

p_up = PROJECT_NAME.upper()
m_up = MODULE_NAME.upper()
vba_data_xml = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<wne:vbaSuppData xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml">
  <wne:mcds>
    <wne:mcd wne:macroName="{p_up}.{m_up}.AUTOEXEC" wne:name="{PROJECT_NAME}.{MODULE_NAME}.AutoExec" wne:bEncrypt="00" wne:cmg="56"/>
    <wne:mcd wne:macroName="{p_up}.{m_up}.TOGGLELATEXMACRO" wne:name="{PROJECT_NAME}.{MODULE_NAME}.ToggleLatexMacro" wne:bEncrypt="00" wne:cmg="56"/>
    <wne:mcd wne:macroName="{p_up}.{m_up}.INSERTEQUATIONMACRO" wne:name="{PROJECT_NAME}.{MODULE_NAME}.InsertEquationMacro" wne:bEncrypt="00" wne:cmg="56"/>
    <wne:mcd wne:macroName="{p_up}.{m_up}.EDITEQUATIONMACRO" wne:name="{PROJECT_NAME}.{MODULE_NAME}.EditEquationMacro" wne:bEncrypt="00" wne:cmg="56"/>
    <wne:mcd wne:macroName="{p_up}.{m_up}.UPGRADEMATHTYPEMACRO" wne:name="{PROJECT_NAME}.{MODULE_NAME}.UpgradeMathTypeMacro" wne:bEncrypt="00" wne:cmg="56"/>
  </wne:mcds>
</wne:vbaSuppData>"""

customui_rels = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="eqntideLogo" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="images/logo.png"/>
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

customui14_xml = r"""<customUI xmlns="http://schemas.microsoft.com/office/2009/07/customui">
  <ribbon>
    <tabs>
      <tab id="EqnTideTab" label="EqnTide">
        <group id="EqnTideEqGroup" label="Equations">
          <button id="btnToggleLatex" label="Toggle LaTeX" size="large" image="eqntideLogo" onAction="RibbonToggleLatex" screentip="Toggle LaTeX (Alt + \)" supertip="Convert LaTeX to MathType OLE with EMF preview, or an equation back to LaTeX text." />
          <button id="btnInsertEquation" label="Insert Equation" size="large" image="eqntideLogo" onAction="RibbonInsertEquation" screentip="Insert Equation" supertip="Open EqnTide equation editor." />
          <button id="btnEditEquation" label="Edit Equation" size="large" image="eqntideLogo" onAction="RibbonEditEquation" screentip="Edit Equation" supertip="Edit selected equation." />
          <button id="btnUpgradeMathType" label="Upgrade MathType" size="large" image="eqntideLogo" onAction="RibbonUpgradeMathType" screentip="Upgrade MathType" supertip="Upgrade legacy MathType equations." />
        </group>
      </tab>
    </tabs>
  </ribbon>
</customUI>"""

with open(LOGO_PATH, "rb") as lf:
    logo_bytes = lf.read()

# 3. Assemble .dotm ZIP
with zipfile.ZipFile(DOTM_PATH, "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("[Content_Types].xml", content_types_xml)
    z.writestr("_rels/.rels", root_rels)
    z.writestr("word/_rels/document.xml.rels", doc_rels)
    z.writestr("word/_rels/vbaProject.bin.rels", vba_rels)
    z.writestr("word/document.xml", document_xml)
    z.writestr("word/settings.xml", settings_xml)
    z.writestr("word/styles.xml", styles_xml)
    z.writestr("word/vbaData.xml", vba_data_xml)
    z.writestr("customUI/_rels/customUI14.xml.rels", customui_rels)
    z.writestr("customUI/customUI14.xml", customui14_xml)
    z.writestr("customUI/images/logo.png", logo_bytes)
    z.writestr("word/vbaProject.bin", vba_bytes)

# Verify full .dotm via oletools VBA_Parser
vp = VBA_Parser(DOTM_PATH)
extracted = [(name, len(code)) for (_, _, name, code) in vp.extract_macros()]
vp.close()
assert len(extracted) == 2, f"Expected 2 extracted VBA modules, got {extracted}"
print(f"Successfully generated: {DOTM_PATH} ({os.path.getsize(DOTM_PATH)} bytes), modules={extracted}")
