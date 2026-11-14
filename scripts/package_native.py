#!/usr/bin/env python3
"""Package already-built native binaries; never builds or uploads a release."""
import argparse
from pathlib import Path
import plistlib
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent
VERSION = "0.3.0"


def desktop_entry(executable):
    return f"""[Desktop Entry]
Type=Application
Name=EqnTide Native Preview
Comment=Equation editor and DOCX converter
Exec={executable} %u
Icon=eqntide
Terminal=false
Categories=Office;Education;Math;
MimeType=x-scheme-handler/eqntide;
StartupNotify=true
"""


def populate(stage, binary_dir, windows):
    suffix = ".exe" if windows else ""
    for name in ("eqntide-cli", "eqntide-desktop"):
        shutil.copy2(binary_dir / (name + suffix), stage / (name + suffix))
    for name in ("word-addin", "wps-addin"):
        shutil.copytree(ROOT / name, stage / name)
    (stage / "word-template").mkdir()
    shutil.copy2(ROOT / "word-template/EqnTide.dotm", stage / "word-template/EqnTide.dotm")
    for name in ("README.md", "LICENSE", "NOTICE"):
        shutil.copy2(ROOT / name, stage / name)
    shutil.copytree(ROOT / "assets/icons", stage / "icons")
    shutil.copytree(ROOT / "assets/fonts", stage / "fonts")
    if windows:
        (stage / "start-server.bat").write_text('@echo off\ncd /d "%~dp0"\nset "EQNTIDE_DESKTOP_BIN=%~dp0eqntide-desktop.exe"\neqntide-cli.exe --server\n')
    else:
        (stage / "eqntide.desktop").write_text(desktop_entry("eqntide-desktop"))
        (stage / "install-user.sh").write_text('''#!/bin/sh
set -eu
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$HOME/.local/share/applications"
python3 - "$app_dir" "$HOME/.local/share/applications/eqntide.desktop" <<'PY'
import pathlib, sys
root = pathlib.Path(sys.argv[1]).resolve()
exe = str(root / 'eqntide-desktop')
for c in ('\\\\', '"', '`', '$'):
    exe = exe.replace(c, '\\\\' + c)
text = (root / 'eqntide.desktop').read_text().replace('Exec=eqntide-desktop %u', 'Exec="' + exe + '" %u')
pathlib.Path(sys.argv[2]).write_text(text)
PY
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
xdg-mime default eqntide.desktop x-scheme-handler/eqntide
''')
        (stage / "install-user.sh").chmod(0o755)


def linux_tree(stage, tree):
    lib = tree / "usr/lib/eqntide"
    lib.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(stage, lib)
    bins = tree / "usr/bin"
    bins.mkdir(parents=True)
    for name in ("eqntide-cli", "eqntide-desktop"):
        (bins / name).symlink_to("../lib/eqntide/" + name)
    desktop = tree / "usr/share/applications"
    desktop.mkdir(parents=True)
    (desktop / "eqntide.desktop").write_text(desktop_entry("/usr/bin/eqntide-desktop"))
    icons = tree / "usr/share/icons/hicolor/128x128/apps"
    icons.mkdir(parents=True)
    shutil.copy2(stage / "icons/icon.png", icons / "eqntide.png")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--profile", default="release")
    parser.add_argument("--format", choices=("portable", "deb", "rpm", "all"), default="portable")
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    windows = "windows" in args.target
    binary_dir = ROOT / "target" / args.target / args.profile
    if not binary_dir.exists():
        binary_dir = ROOT / "target" / args.profile
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="eqntide-package-") as temp:
        temp = Path(temp)
        label = "eqntide-native-preview-" + args.target
        stage = temp / label
        stage.mkdir()
        populate(stage, binary_dir, windows)
        if "apple" in args.target:
            macos = stage / "EqnTide.app/Contents/MacOS"
            macos.mkdir(parents=True)
            for name in ("eqntide-desktop", "eqntide-cli", "word-addin", "wps-addin", "word-template"):
                shutil.move(str(stage / name), macos / name)
            with (macos.parent / "Info.plist").open("wb") as file:
                plistlib.dump({"CFBundleIdentifier": "com.eqntide.desktop", "CFBundleName": "EqnTide", "CFBundleExecutable": "eqntide-desktop", "CFBundleVersion": VERSION, "CFBundleShortVersionString": VERSION, "CFBundlePackageType": "APPL", "CFBundleURLTypes": [{"CFBundleURLName": "EqnTide", "CFBundleURLSchemes": ["eqntide"]}]}, file)
        if args.format in ("portable", "all"):
            if windows:
                archive = args.output / (label + ".zip")
                with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as file:
                    for path in stage.rglob("*"):
                        if path.is_file():
                            file.write(path, path.relative_to(temp))
                shutil.copytree(stage, args.output / "windows-stage", dirs_exist_ok=True)
            else:
                archive = args.output / (label + ".tar.gz")
                with tarfile.open(archive, "w:gz") as file:
                    file.add(stage, arcname=label)
            print(archive)
        if args.format in ("deb", "all") and "linux" in args.target:
            tree = temp / "deb"
            linux_tree(stage, tree)
            control = tree / "DEBIAN"
            control.mkdir()
            arch = "amd64" if args.target.startswith("x86_64") else "arm64"
            (control / "control").write_text(f"Package: eqntide-native-preview\nVersion: {VERSION}\nArchitecture: {arch}\nMaintainer: EqnTide contributors\nDepends: libc6, libgcc-s1, libgl1, libx11-6, libxcb1, libxkbcommon0, libwayland-client0, libdbus-1-3, xdg-desktop-portal, zenity\nDescription: EqnTide native Rust equation editor (preview)\n")
            subprocess.run(["dpkg-deb", "--build", "--root-owner-group", str(tree), str(args.output / f"eqntide-native-preview_{VERSION}_{arch}.deb")], check=True)
        if args.format in ("rpm", "all") and "linux" in args.target:
            tree = temp / "rpm-tree"
            linux_tree(stage, tree)
            top = temp / "rpmbuild"
            for name in ("BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"):
                (top / name).mkdir(parents=True)
            spec = top / "SPECS/eqntide.spec"
            spec.write_text(f"""Name: eqntide-native-preview
Version: {VERSION}
Release: 1
Summary: Native Rust equation editor (preview)
License: MIT
Requires: libX11, libxcb, libxkbcommon, wayland-libs, mesa-libGL, dbus-libs, xdg-desktop-portal, zenity
AutoReqProv: no
%description
EqnTide equation editor and DOCX conversion tools.
%install
mkdir -p %{{buildroot}}
cp -a {tree}/usr %{{buildroot}}/
%files
/usr/bin/eqntide-cli
/usr/bin/eqntide-desktop
/usr/lib/eqntide
/usr/share/applications/eqntide.desktop
/usr/share/icons/hicolor/128x128/apps/eqntide.png
""")
            subprocess.run(["rpmbuild", "-bb", "--define", f"_topdir {top}", "--define", "_build_id_links none", str(spec)], check=True)
            for path in (top / "RPMS").rglob("*.rpm"):
                shutil.copy2(path, args.output / path.name)


if __name__ == "__main__":
    main()
