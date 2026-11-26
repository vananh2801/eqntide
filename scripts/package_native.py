#!/usr/bin/env python3
"""Package already-built native binaries; never builds or uploads a release."""
import argparse
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent.parent
VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]

# Both package managers run this as root; the helper drops privileges first.
SESSION_POSTINSTALL = """/usr/bin/python3 /usr/lib/eqntide/scripts/install-session-addins.py --package-install || true
"""


def output_directory(root, target, version=VERSION):
    platform = "windows" if "windows" in target else "linux"
    return root / platform / version


def windows_installer_config(output):
    (output / "installer-config.nsh").write_text(f'!define APP_VERSION "{VERSION}"\n')


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
    import importlib.util
    render_spec = importlib.util.spec_from_file_location("render_icons", ROOT / "scripts/render_icons.py")
    render_mod = importlib.util.module_from_spec(render_spec)
    render_spec.loader.exec_module(render_mod)
    if render_mod.check_all() != 0:
        raise RuntimeError("Rendered Tabler PNG icons are out of sync; run python3 scripts/render_icons.py")

    suffix = ".exe" if windows else ""
    for name in ("eqntide-cli", "eqntide-desktop"):
        shutil.copy2(binary_dir / (name + suffix), stage / (name + suffix))
    shutil.copytree(ROOT / "wps-addin", stage / "wps-addin")
    (stage / "word-template").mkdir()
    shutil.copy2(ROOT / "word-template/EqnTide.dotm", stage / "word-template/EqnTide.dotm")
    for name in ("README.md", "LICENSE", "NOTICE"):
        shutil.copy2(ROOT / name, stage / name)
    (stage / "icons").mkdir()
    shutil.copy2(ROOT / "assets/brand/icon.png", stage / "icons/icon.png")
    shutil.copy2(ROOT / "assets/brand/icon.ico", stage / "icons/icon.ico")
    shutil.copy2(ROOT / "assets/icons/sources/tabler/LICENSE.txt", stage / "icons/tabler-LICENSE.txt")
    shutil.copytree(ROOT / "assets/fonts", stage / "fonts", ignore=shutil.ignore_patterns("*.pdf"))
    (stage / "scripts").mkdir()
    for script in ("install-addin.sh", "install-addin.ps1", "install-addin.bat", "uninstall-addin.bat", "install-bridge-service.sh", "configure_wps.py", "install-session-addins.py", "install-bridge-startup.ps1"):
        shutil.copy2(ROOT / "scripts" / script, stage / "scripts" / script)
        if script.endswith('.sh'):
            (stage / "scripts" / script).chmod(0o755)
    if windows:
        (stage / "install-addins.bat").write_text('@echo off\ncall "%~dp0scripts\\install-addin.bat"\nexit /b %errorlevel%\n')
        (stage / "register-uri.ps1").write_text(r'''$exe = Join-Path $PSScriptRoot 'eqntide-desktop.exe'
$key = 'HKCU:\Software\Classes\eqntide'
New-Item $key -Force | Out-Null
New-ItemProperty $key -Name 'URL Protocol' -Value '' -Force | Out-Null
New-Item ($key + '\shell\open\command') -Force | Out-Null
Set-Item ($key + '\shell\open\command') ('"' + $exe + '" "%1"')
''')
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
    # Preserve the desktop-file ID used by existing Tauri URI associations.
    (desktop / "EqnTide.desktop").write_text(desktop_entry("/usr/bin/eqntide-desktop") + "NoDisplay=true\n")
    autostart = tree / "etc/xdg/autostart"
    autostart.mkdir(parents=True)
    (autostart / "eqntide-addins.desktop").write_text("""[Desktop Entry]
Type=Application
Name=EqnTide Background Setup
Exec=/usr/bin/python3 /usr/lib/eqntide/scripts/install-session-addins.py
TryExec=/usr/bin/eqntide-desktop
Terminal=false
NoDisplay=true
X-GNOME-Autostart-enabled=true
""")
    icons = tree / "usr/share/icons/hicolor/128x128/apps"
    icons.mkdir(parents=True)
    shutil.copy2(stage / "icons/icon.png", icons / "eqntide.png")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target")
    parser.add_argument("--print-version", action="store_true", help="Print the package version from Cargo.toml and exit")
    parser.add_argument("--profile", default="release")
    parser.add_argument("--format", choices=("portable", "deb", "rpm", "all"), default="portable")
    parser.add_argument("--output", type=Path, default=ROOT / "dist", help="Output root; packages go under <root>/<platform>/<version>/")
    args = parser.parse_args()
    if args.print_version:
        print(VERSION)
        return
    if not args.target:
        parser.error("--target is required when packaging")
    if not any(platform in args.target for platform in ("linux", "windows")):
        parser.error("Only Linux and Windows packaging is supported; macOS support is temporarily disabled (no test hardware).")
    windows = "windows" in args.target
    binary_dir = ROOT / "target" / args.target / args.profile
    if not binary_dir.exists():
        binary_dir = ROOT / "target" / args.profile
    args.output = output_directory(args.output, args.target, VERSION)
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="eqntide-package-") as temp:
        temp = Path(temp)
        label = f"eqntide-native-preview-{VERSION}-{args.target}"
        stage = temp / label
        stage.mkdir()
        populate(stage, binary_dir, windows)
        if args.format in ("portable", "all"):
            if windows:
                archive = args.output / (label + ".zip")
                with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as file:
                    for path in stage.rglob("*"):
                        if path.is_file():
                            file.write(path, path.relative_to(temp))
                windows_stage = args.output / "windows-stage"
                if windows_stage.exists():
                    shutil.rmtree(windows_stage)
                shutil.copytree(stage, windows_stage)
                windows_installer_config(args.output)
                shutil.copy2(ROOT / "docs/WINDOWS-INSTALLATION.txt", args.output / "README-WINDOWS.txt")
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
            (control / "control").write_text(f"Package: eqntide-native-preview\nVersion: {VERSION}\nArchitecture: {arch}\nMaintainer: EqnTide contributors\nConflicts: eqn-tide\nReplaces: eqn-tide\nProvides: eqn-tide\nDepends: libc6, libgcc-s1, libgl1, libx11-6, libxcb1, libxkbcommon0, libwayland-client0, libdbus-1-3, xdg-desktop-portal, zenity, python3\nDescription: EqnTide native Rust equation editor (preview)\n")
            (control / "postinst").write_text('#!/bin/sh\nset -e\nif [ "$1" = configure ]; then\n' + SESSION_POSTINSTALL + 'fi\nexit 0\n')
            (control / "postinst").chmod(0o755)
            (control / "prerm").write_text('#!/bin/sh\ncase "$1" in remove|deconfigure)\n/usr/bin/python3 /usr/lib/eqntide/scripts/install-session-addins.py --package-remove || true\n;;\nesac\nexit 0\n')
            (control / "prerm").chmod(0o755)
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
Provides: eqn-tide = %{{version}}-%{{release}}
Obsoletes: eqn-tide <= %{{version}}-%{{release}}
Requires: libX11.so.6()(64bit), libxcb.so.1()(64bit), libxkbcommon.so.0()(64bit), libwayland-client.so.0()(64bit), libwayland-cursor.so.0()(64bit), libwayland-egl.so.1()(64bit), libGL.so.1()(64bit), libdbus-1.so.3()(64bit), xdg-desktop-portal, zenity, python3
# Keep ELF dependency generation enabled for libc/libgcc ABI requirements.
# SONAME capabilities also cover dynamically loaded UI libraries regardless
# of distribution-specific package names (e.g. Fedora 45 split Wayland).
%description
EqnTide equation editor and DOCX conversion tools.
%install
mkdir -p %{{buildroot}}
cp -a {tree}/usr {tree}/etc %{{buildroot}}/
%post
{SESSION_POSTINSTALL}
%preun
if [ "$1" -eq 0 ]; then
/usr/bin/python3 /usr/lib/eqntide/scripts/install-session-addins.py --package-remove || true
fi
%files
/etc/xdg/autostart/eqntide-addins.desktop
/usr/bin/eqntide-cli
/usr/bin/eqntide-desktop
/usr/lib/eqntide
/usr/share/applications/eqntide.desktop
/usr/share/applications/EqnTide.desktop
/usr/share/icons/hicolor/128x128/apps/eqntide.png
""")
            subprocess.run(["rpmbuild", "-bb", "--define", f"_topdir {top}", "--define", "_build_id_links none", str(spec)], check=True)
            for path in (top / "RPMS").rglob("*.rpm"):
                shutil.copy2(path, args.output / path.name)


if __name__ == "__main__":
    main()
