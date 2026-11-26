"""Check installed/portable paths without installing into the user's profile."""
import importlib.util
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS_DIR = ROOT / "scripts"
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

spec = importlib.util.spec_from_file_location("package_native", SCRIPTS_DIR / "package_native.py")
pack = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pack)


class NativeIntegration(unittest.TestCase):
    def stage(self, root, windows):
        binaries = root / "bin"
        binaries.mkdir()
        for name in ("eqntide-cli", "eqntide-desktop"):
            file = binaries / (name + (".exe" if windows else ""))
            file.write_bytes(b"test executable")
            file.chmod(0o755)
        stage = root / "portable folder"
        stage.mkdir()
        pack.populate(stage, binaries, windows)
        return stage

    def test_windows_installer_inputs_and_uri_target_exist(self):
        with tempfile.TemporaryDirectory() as tmp:
            stage = self.stage(Path(tmp), True)
            self.assertIn("eqntide-desktop.exe", (stage / "register-uri.ps1").read_text())
            self.assertIn("EQNTIDE_DESKTOP_BIN=", (stage / "start-server.bat").read_text())
            installer = (pack.ROOT / "scripts/installer.nsi").read_text()
            self.assertIn(r'OutFile "${PACKAGE_DIR}\eqntide-${APP_VERSION}-windows-${APP_ARCH}-setup.exe"', installer)
            for path in re.findall(r'^\s*File(?: /r)? "\$\{PACKAGE_DIR\}\\windows-stage\\([^"]+)"', installer, re.M):
                path = path.replace("\\", "/")
                self.assertTrue(list(stage.glob(path)), path)
            self.assertNotIn("src-tauri", installer)
            self.assertFalse((stage / "word-addin").exists())
            self.assertIn(r'RMDir /r "$LOCALAPPDATA\EqnTide"', installer)
            self.assertIn(r'RMDir /r "$LOCALAPPDATA\eqntide\EqnTide"', installer)
            self.assertIn("addin_uninstall_retry:", installer)
            template = stage / "word-template/EqnTide.dotm"
            import zipfile
            with zipfile.ZipFile(template) as archive:
                self.assertIn("word/vbaProject.bin", archive.namelist())
                self.assertIn("customUI/customUI14.xml", archive.namelist())
            ps = (stage / "scripts/install-addin.ps1").read_text()
            self.assertIn("word-template\\EqnTide.dotm", ps)
            self.assertNotIn("Set-ItemProperty", ps)
            self.assertNotIn("CheckNetIsolation.exe", ps)
            bat_uninstall = (stage / "scripts/uninstall-addin.bat").read_text()
            self.assertIn(r"HKCU\Software\Classes\eqntide", bat_uninstall)

    def test_staged_fonts_exclude_pdfs_and_keep_licenses(self):
        with tempfile.TemporaryDirectory() as tmp:
            stage = self.stage(Path(tmp), False)
            fonts_dir = stage / "fonts"
            self.assertEqual(list(fonts_dir.rglob("*.pdf")), [])
            self.assertTrue((fonts_dir / "stixfonts-2.13b171/STIXTwoMath-Regular.ttf").is_file())
            self.assertTrue((fonts_dir / "stixfonts-2.13b171/OFL.txt").is_file())
            self.assertTrue((fonts_dir / "Nunito/Nunito-Regular.ttf").is_file())
            self.assertTrue((fonts_dir / "Nunito/OFL.txt").is_file())
            self.assertTrue((fonts_dir / "JetBrainsMono-2.304/JetBrainsMono-Regular.ttf").is_file())
            self.assertTrue((fonts_dir / "JetBrainsMono-2.304/OFL.txt").is_file())

    def test_windows_repack_stages_installer_only_and_removes_stale_web_payload(self):
        from unittest.mock import patch
        import shutil
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            stage = self.stage(base, True)
            output = base / 'out'
            version_dir = output / 'windows' / pack.VERSION
            stale = version_dir / 'windows-stage/word-addin'
            stale.mkdir(parents=True)
            (stale / 'manifest.xml').write_text('obsolete')
            def populate(target, binary_dir, windows):
                shutil.copytree(stage, target, dirs_exist_ok=True)
            with patch.object(pack, 'resolve_release_version', return_value=pack.VERSION), patch.object(pack, 'populate', side_effect=populate), patch.object(sys, 'argv', ['package_native.py', '--target', 'x86_64-pc-windows-msvc', '--output', str(output)]):
                pack.main()
            self.assertFalse(stale.exists())
            config = (version_dir / 'installer-config.nsh').read_text()
            self.assertIn(f'!define APP_VERSION "{pack.VERSION}"', config)
            self.assertIn('!define APP_ARCH "x64"', config)
            self.assertFalse((version_dir / 'README-WINDOWS.txt').exists())
            self.assertEqual(list(version_dir.glob('*.zip')), [])
            self.assertTrue((version_dir / 'windows-stage/word-template/EqnTide.dotm').exists())
            self.assertTrue((version_dir / 'windows-stage/wps-addin/main.js').exists())

    def test_outputs_are_isolated_by_platform_and_version(self):
        from unittest.mock import patch
        import shutil
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            fixture = self.stage(base, False)
            output = base / 'custom output'
            def populate(target, binary_dir, windows):
                shutil.copytree(fixture, target, dirs_exist_ok=True)
            linux_archives = []
            for version in ('1.2.3', '1.3.0', '1.3.0-beta.1'):
                with patch.object(pack, 'VERSION', '1.3.0' if 'beta' in version else version), patch.object(pack, 'resolve_release_version', return_value=version), patch.object(pack, 'populate', side_effect=populate), patch.object(sys, 'argv', ['package_native.py', '--target', 'x86_64-unknown-linux-gnu', '--output', str(output)]):
                    pack.main()
                folder = output / 'linux' / version
                archive = folder / f'eqntide-{version}-linux-x64.tar.gz'
                self.assertTrue(archive.is_file(), archive)
                linux_archives.append((archive, archive.read_bytes()))

                with patch.object(pack, 'VERSION', '1.3.0' if 'beta' in version else version), patch.object(pack, 'resolve_release_version', return_value=version), patch.object(pack, 'populate', side_effect=populate), patch.object(sys, 'argv', ['package_native.py', '--target', 'x86_64-pc-windows-msvc', '--output', str(output)]):
                    pack.main()
                win_folder = output / 'windows' / version
                self.assertIn(f'!define APP_VERSION "{version}"', (win_folder / 'installer-config.nsh').read_text())
                self.assertEqual(list(win_folder.glob('*.zip')), [])
            for archive, content in linux_archives:
                self.assertEqual(archive.read_bytes(), content)
            self.assertFalse((output / 'windows-stage').exists())

    def test_resolve_release_version_handles_tags_and_fallbacks(self):
        self.assertEqual(pack.resolve_release_version('1.2.3', 'v1.2.3-beta.1'), '1.2.3-beta.1')
        self.assertEqual(pack.resolve_release_version('1.2.3', 'V1.2.3-rc.2'), '1.2.3-rc.2')
        self.assertEqual(pack.resolve_release_version('1.2.3', 'v1.2.3'), '1.2.3')
        self.assertEqual(pack.resolve_release_version('1.2.3', 'v1.2.4-beta.1'), '1.2.3')
        self.assertEqual(pack.resolve_release_version('1.2.3', 'main'), '1.2.3')
        self.assertEqual(pack.resolve_release_version('1.2.3', 'v1.2.3-'), '1.2.3')
        self.assertEqual(pack.resolve_release_version('1.2.3', 'v1.2.3-beta+meta'), '1.2.3')
        self.assertEqual(pack.deb_package_version('1.2.3', '1.2.3-beta.1'), '1.2.3~beta.1')
        self.assertEqual(pack.rpm_version_release('1.2.3', '1.2.3-beta.1'), ('1.2.3', '0.beta.1'))
        self.assertEqual(pack.rpm_version_release('1.2.3', '1.2.3'), ('1.2.3', '1'))

    def test_validate_versions_rejects_prerelease_and_manifest_drift(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_root = Path(tmp)
            for rel in pack.WORKSPACE_MANIFESTS:
                manifest = fake_root / rel
                manifest.parent.mkdir(parents=True, exist_ok=True)
                manifest.write_text('[package]\nname = "demo"\nversion = "0.12.0"\n', encoding='utf-8')
            pack.validate_versions('0.12.0', root=fake_root)

            with self.assertRaises(RuntimeError):
                pack.validate_versions('0.12.0-beta.1', root=fake_root)

            (fake_root / 'desktop/Cargo.toml').write_text('[package]\nname = "eqntide-desktop"\nversion = "0.11.0"\n', encoding='utf-8')
            with self.assertRaises(RuntimeError):
                pack.validate_versions('0.12.0', root=fake_root)

    def test_repo_skills_follow_agent_skills_spec(self):
        for skill_name in ('eqntide-release', 'eqntide-precommit'):
            skill_path = ROOT / f'.agents/skills/{skill_name}/SKILL.md'
            text = skill_path.read_text(encoding='utf-8')
            self.assertTrue(text.startswith(f'---\nname: {skill_name}\ndescription: '), skill_path)

    @unittest.skipIf(sys.platform == 'win32', 'Linux systemd installer')
    def test_portable_service_resolves_bundled_executables(self):
        if sys.platform != 'linux':
            self.skipTest('Linux systemd installer')
        with tempfile.TemporaryDirectory() as tmp:
            stage = self.stage(Path(tmp), False)
            result = subprocess.run(["bash", str(stage / "scripts/install-bridge-service.sh"), "--print-unit"], capture_output=True, text=True, check=True)
            self.assertIn(f'ExecStart="{stage}/eqntide-cli" --server', result.stdout)
            self.assertIn(f'EQNTIDE_DESKTOP_BIN={stage}/eqntide-desktop', result.stdout)
            self.assertNotIn("target/release", result.stdout)
            self.assertTrue((stage / "scripts/install-addin.sh").is_file())


class WpsConfig(unittest.TestCase):
    def test_install_uninstall_preserve_other_addins(self):
        import configure_wps
        import json
        import xml.etree.ElementTree as ET
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'publish.xml').write_text('<jsplugins><jsplugin name="Other" url="other/"/></jsplugins>')
            (root / 'authaddin.json').write_text(json.dumps({'wps': {'namelist': 'Other', 'Other': {'mode': 2}}}))
            configure_wps.configure(root)
            configure_wps.configure(root)
            nodes = list(ET.parse(root / 'publish.xml').getroot())
            self.assertEqual(len([n for n in nodes if n.get('name') == 'Other']), 1)
            self.assertEqual(len([n for n in nodes if n.get('name') == 'EqnTide']), 1)
            configure_wps.configure(root, uninstall=True)
            config = json.loads((root / 'authaddin.json').read_text())
            self.assertEqual(config['wps']['Other'], {'mode': 2})
            self.assertEqual(config['wps']['namelist'], 'Other')
            self.assertNotIn('EqnTide', config['wps'])

    def test_uninstall_removes_empty_wps_files_and_dirs(self):
        import configure_wps
        with tempfile.TemporaryDirectory() as tmp:
            kingsoft = Path(tmp) / 'Kingsoft'
            jsaddons = kingsoft / 'wps/jsaddons'
            configure_wps.configure(jsaddons)
            self.assertTrue((jsaddons / 'publish.xml').exists())
            self.assertTrue((jsaddons / 'authaddin.json').exists())
            configure_wps.configure(jsaddons, uninstall=True)
            self.assertFalse(kingsoft.exists())

    def test_uninstall_on_unconfigured_dir_creates_no_files(self):
        import configure_wps
        with tempfile.TemporaryDirectory() as tmp:
            jsaddons = Path(tmp) / 'Kingsoft/wps/jsaddons'
            jsaddons.mkdir(parents=True)
            configure_wps.configure(jsaddons, uninstall=True)
            self.assertFalse((jsaddons / 'publish.xml').exists())
            self.assertFalse((jsaddons / 'authaddin.json').exists())
            self.assertFalse((Path(tmp) / 'Kingsoft').exists())


@unittest.skipUnless(sys.platform == 'linux', 'Linux desktop package layout')
class LinuxMigration(unittest.TestCase):
    def test_previous_tauri_desktop_id_launches_native_binary(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            source = base / 'stage'
            source.mkdir()
            for name in ('eqntide-cli', 'eqntide-desktop'):
                (source / name).write_bytes(b'test')
            (source / 'icons').mkdir()
            (source / 'icons/icon.png').write_bytes(b'icon')
            tree = base / 'root'
            pack.linux_tree(source, tree)
            alias = (tree / 'usr/share/applications/EqnTide.desktop').read_text()
            self.assertIn('Exec=/usr/bin/eqntide-desktop %u', alias)
            self.assertIn('Name=EqnTide\n', alias)
            self.assertIn('NoDisplay=true', alias)
            self.assertTrue((tree / 'usr/bin/eqntide-cli').exists())


@unittest.skipUnless(sys.platform == 'linux', 'Linux package/session setup')
class SessionAddins(unittest.TestCase):
    def test_session_install_retries_and_updates_only_when_needed(self):
        from unittest.mock import patch
        spec = importlib.util.spec_from_file_location('session_addins', SCRIPTS_DIR / 'install-session-addins.py')
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            home = root / 'home'
            home.mkdir()
            stage = NativeIntegration().stage(root, False)
            import os
            with patch.dict(os.environ, {'HOME': str(home), 'XDG_STATE_HOME': str(home / '.local/state')}), patch.object(helper.os, 'getuid', return_value=1000):
                helper.install_for_session(stage)
                marker = home / '.local/state/eqntide/addins.sha256'
                old_digest = marker.read_text()
                with patch.object(helper.subprocess, 'run') as run:
                    helper.install_for_session(stage)
                    run.assert_not_called()
                (stage / 'wps-addin/main.js').write_text('// new revision')
                helper.install_for_session(stage)
                self.assertNotEqual(marker.read_text(), old_digest)
                target = home / '.local/share/Kingsoft/wps/jsaddons/EqnTide/main.js'
                self.assertEqual(target.read_text(), '// new revision')
                target.unlink()
                helper.install_for_session(stage)
                self.assertTrue(target.exists())

    def test_package_setup_drops_privileges_and_ignores_root(self):
        from unittest.mock import patch
        from types import SimpleNamespace
        import os
        spec = importlib.util.spec_from_file_location('session_addins', SCRIPTS_DIR / 'install-session-addins.py')
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        with tempfile.TemporaryDirectory() as tmp:
            account = SimpleNamespace(pw_name='desktop', pw_dir=tmp, pw_gid=1001)
            with patch.dict(os.environ, {'SUDO_UID': '1001', 'PKEXEC_UID': '0'}, clear=True), patch.object(helper.pwd, 'getpwuid', return_value=account), patch.object(helper.os, 'getgrouplist', return_value=[1001]), patch.object(helper.subprocess, 'run', return_value=SimpleNamespace(stdout='')) as run:
                helper.package_install()
                self.assertEqual(run.call_count, 2)
                kwargs = run.call_args.kwargs
                self.assertEqual(kwargs['user'], 1001)
                self.assertEqual(kwargs['group'], 1001)
                self.assertEqual(kwargs['env']['HOME'], tmp)

    def test_deb_and_rpm_naming_prerelease_and_migration_metadata(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'out'
            stage = NativeIntegration().stage(Path(tmp), False)
            import shutil
            def populate(target, binary_dir, windows):
                shutil.copytree(stage, target, dirs_exist_ok=True)
            def inspect_deb(args, **kwargs):
                self.assertEqual(args[0], 'dpkg-deb')
                tree = Path(args[-2])
                out_deb = Path(args[-1])
                self.assertEqual(out_deb.name, f'eqntide-{pack.VERSION}-beta.1-linux-x64.deb')
                control_text = (tree / 'DEBIAN/control').read_text()
                self.assertIn('Package: eqntide\n', control_text)
                self.assertIn(f'Version: {pack.VERSION}~beta.1\n', control_text)
                self.assertIn('Conflicts: eqn-tide, eqntide-native-preview\n', control_text)
                self.assertIn('Replaces: eqn-tide, eqntide-native-preview\n', control_text)
                self.assertIn('Provides: eqn-tide, eqntide-native-preview\n', control_text)
                hook = tree / 'DEBIAN/postinst'
                self.assertTrue(hook.stat().st_mode & 0o111)
                self.assertIn('"$1" = configure', hook.read_text())
                self.assertIn('--package-install', hook.read_text())
                removal = tree / 'DEBIAN/prerm'
                self.assertIn('--package-remove', removal.read_text())
                self.assertIn('remove|deconfigure', removal.read_text())
                subprocess.run_original(['sh', '-n', str(removal)], check=True)
                subprocess.run_original(['sh', '-n', str(hook)], check=True)
                self.assertTrue((tree / 'etc/xdg/autostart/eqntide-addins.desktop').exists())
                self.assertTrue((tree / 'usr/lib/eqntide/scripts/install-session-addins.py').exists())
            original = subprocess.run
            with patch.object(pack, 'resolve_release_version', return_value=f'{pack.VERSION}-beta.1'), patch.object(pack, 'populate', side_effect=populate), patch.object(subprocess, 'run_original', original, create=True), patch.object(pack.subprocess, 'run', side_effect=inspect_deb), patch.object(sys, 'argv', ['package_native.py', '--target', 'x86_64-unknown-linux-gnu', '--format', 'deb', '--output', str(output)]):
                pack.main()

            def inspect_rpm(args, **kwargs):
                self.assertEqual(args[0], 'rpmbuild')
                spec_path = Path(args[-1])
                spec_text = spec_path.read_text()
                self.assertIn('Name: eqntide\n', spec_text)
                self.assertIn(f'Version: {pack.VERSION}\n', spec_text)
                self.assertIn('Release: 0.beta.1\n', spec_text)
                self.assertIn('Provides: eqntide-native-preview = %{version}-%{release}\n', spec_text)
                self.assertIn('Obsoletes: eqntide-native-preview\n', spec_text)
                top_dir = spec_path.parents[1]
                built = top_dir / 'RPMS/x86_64/eqntide-1.0.0-0.beta.1.x86_64.rpm'
                built.parent.mkdir(parents=True, exist_ok=True)
                built.write_bytes(b'fake-rpm')
            with patch.object(pack, 'resolve_release_version', return_value=f'{pack.VERSION}-beta.1'), patch.object(pack, 'populate', side_effect=populate), patch.object(pack.subprocess, 'run', side_effect=inspect_rpm), patch.object(sys, 'argv', ['package_native.py', '--target', 'x86_64-unknown-linux-gnu', '--format', 'rpm', '--output', str(output)]):
                pack.main()
            self.assertTrue((output / f'linux/{pack.VERSION}-beta.1/eqntide-{pack.VERSION}-beta.1-linux-x64.rpm').is_file())

    def test_service_enable_reuse_and_cleanup_in_isolated_home(self):
        import os
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            stage = NativeIntegration().stage(root, False)
            home = root / 'home'
            home.mkdir()
            fake = root / 'tools'
            fake.mkdir()
            log = root / 'systemctl.log'
            command = fake / 'systemctl'
            command.write_text('#!/bin/sh\nprintf "%s\\n" "$*" >> "$TEST_SYSTEMCTL_LOG"\n')
            command.chmod(0o755)
            env = dict(os.environ, HOME=str(home), PATH=str(fake) + ':' + os.environ['PATH'], TEST_SYSTEMCTL_LOG=str(log))
            script = str(stage / 'scripts/install-bridge-service.sh')
            subprocess.run(['bash', script], env=env, check=True, capture_output=True)
            subprocess.run(['bash', script, '--ensure'], env=env, check=True, capture_output=True)
            unit = home / '.config/systemd/user/eqntide.service'
            self.assertTrue(unit.exists())
            self.assertIn(f'WorkingDirectory={stage}', unit.read_text())
            import shutil
            if shutil.which('systemd-analyze'):
                subprocess.run(['systemd-analyze', '--user', 'verify', str(unit)], check=True, capture_output=True)
            calls = log.read_text()
            self.assertEqual(calls.count('restart eqntide.service'), 1)
            self.assertIn('enable --now eqntide.service', calls)
            subprocess.run(['bash', script, '--uninstall'], env=env, check=True, capture_output=True)
            self.assertFalse(unit.exists())
            self.assertIn('disable --now eqntide.service', log.read_text())

    def test_server_failure_does_not_skip_addin_setup(self):
        from unittest.mock import patch
        import os
        spec = importlib.util.spec_from_file_location('session_addins', SCRIPTS_DIR / 'install-session-addins.py')
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        with tempfile.TemporaryDirectory() as tmp:
            with patch.dict(os.environ, {'HOME': tmp}), patch.object(helper.os, 'getuid', return_value=1001), patch.object(helper.subprocess, 'run', side_effect=subprocess.CalledProcessError(1, 'systemctl')), patch.object(helper, 'install_for_session') as install:
                with self.assertRaises(RuntimeError):
                    helper.configure_session(Path(tmp))
                install.assert_called_once()

    def test_service_removal_requires_matching_installation_marker(self):
        from unittest.mock import patch
        import os
        spec = importlib.util.spec_from_file_location('session_addins', SCRIPTS_DIR / 'install-session-addins.py')
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp) / 'installed'
            state = Path(tmp) / '.local/state/eqntide'
            state.mkdir(parents=True)
            marker = state / 'server-managed'
            with patch.dict(os.environ, {'HOME': tmp}), patch.object(helper.os, 'getuid', return_value=1001), patch.object(helper.subprocess, 'run') as run:
                marker.write_text('/different/installation')
                helper.remove_server(base)
                run.assert_not_called()
                marker.write_text(str(base.resolve()))
                helper.remove_server(base)
                self.assertEqual(run.call_args.args[0][-1], '--uninstall')
                self.assertFalse(marker.exists())

    def test_session_removal_cleans_addins_markers_and_data_dir(self):
        from unittest.mock import patch
        import os
        spec = importlib.util.spec_from_file_location('session_addins', SCRIPTS_DIR / 'install-session-addins.py')
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            stage = NativeIntegration().stage(root, False)
            home = root / 'home'
            home.mkdir()
            fake = root / 'tools'
            fake.mkdir()
            command = fake / 'systemctl'
            command.write_text('#!/bin/sh\nexit 0\n')
            command.chmod(0o755)
            state_home = home / '.local/state'
            data_home = home / '.local/share'
            env = {
                'HOME': str(home),
                'XDG_STATE_HOME': str(state_home),
                'XDG_DATA_HOME': str(data_home),
                'PATH': str(fake) + ':' + os.environ['PATH'],
            }
            with patch.dict(os.environ, env), patch.object(helper.os, 'getuid', return_value=1001):
                helper.configure_session(stage, refresh=True)
                data_dir = data_home / 'eqntide'
                data_dir.mkdir(parents=True, exist_ok=True)
                (data_dir / 'desktop.lock').write_text('')
                self.assertTrue((home / '.local/share/Kingsoft/wps/jsaddons/EqnTide/main.js').is_file())
                self.assertTrue((home / '.config/systemd/user/eqntide.service').is_file())

                # Different installation marker must leave session intact.
                (state_home / 'eqntide/server-managed').write_text('/other/install')
                helper.remove_session(stage)
                self.assertTrue((home / '.local/share/Kingsoft/wps/jsaddons/EqnTide/main.js').is_file())
                self.assertTrue(data_dir.exists())

                # Matching installation removes service, WPS add-ins, markers, and data dir.
                (state_home / 'eqntide/server-managed').write_text(str(stage.resolve()))
                helper.remove_session(stage)
                self.assertFalse((home / '.config/systemd/user/eqntide.service').exists())
                self.assertFalse((home / '.local/share/Kingsoft').exists())
                self.assertFalse((state_home / 'eqntide').exists())
                self.assertFalse(data_dir.exists())


class SupportedPlatforms(unittest.TestCase):
    def test_macos_packaging_is_rejected_before_staging(self):
        from unittest.mock import patch
        import contextlib
        import io
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'unsupported-output'
            for target in ('aarch64-apple-darwin', 'x86_64-apple-darwin'):
                stderr = io.StringIO()
                with self.subTest(target=target), patch.object(sys, 'argv', ['package_native.py', '--target', target, '--output', str(output)]), patch.object(pack, 'populate') as populate, contextlib.redirect_stderr(stderr):
                    with self.assertRaises(SystemExit) as error:
                        pack.main()
                    self.assertEqual(error.exception.code, 2)
                    self.assertIn('macOS support is temporarily disabled', stderr.getvalue())
                    populate.assert_not_called()
                    self.assertFalse(output.exists())
