#!/usr/bin/env python3
"""Install bundled Linux add-ins in a user session, once per payload revision."""
import hashlib
import os
from pathlib import Path
import pwd
import subprocess
import sys

BASE = Path(__file__).resolve().parent.parent


def payload_digest(base):
    digest = hashlib.sha256()
    inputs = [base / 'scripts/install-addin.sh', base / 'scripts/configure_wps.py']
    inputs += sorted((base / 'wps-addin').rglob('*'))
    for path in inputs:
        if path.is_file():
            digest.update(str(path.relative_to(base)).encode())
            digest.update(b'\0')
            digest.update(path.read_bytes())
    return digest.hexdigest()


def install_for_session(base=BASE):
    if os.getuid() == 0:
        raise RuntimeError('Run session setup as a desktop user, not root')
    home = Path.home()
    state = Path(os.environ.get('XDG_STATE_HOME', home / '.local/state')) / 'eqntide'
    marker = state / 'addins.sha256'
    digest = payload_digest(base)
    targets = [home / '.local/share/Kingsoft' / name / 'jsaddons/EqnTide/main.js'
               for name in ('wps', 'wps_intl')]
    if marker.exists() and marker.read_text().strip() == digest and all(p.is_file() for p in targets):
        return
    subprocess.run(['bash', str(base / 'scripts/install-addin.sh')], check=True)
    state.mkdir(parents=True, exist_ok=True)
    temporary = marker.with_suffix('.tmp')
    temporary.write_text(digest + '\n')
    temporary.replace(marker)


def configure_session(base=BASE, refresh=False):
    if os.getuid() == 0:
        raise RuntimeError('Session setup must not run as root')
    # Track server ownership even if no user bus is available yet.
    state = Path.home() / '.local/state/eqntide'
    state.mkdir(parents=True, exist_ok=True)
    (state / 'server-managed').write_text(str(base.resolve()))
    errors = []
    try:
        args = ['bash', str(base / 'scripts/install-bridge-service.sh')]
        if not refresh:
            args.append('--ensure')
        subprocess.run(args, check=True, timeout=30)
    except (OSError, subprocess.SubprocessError) as error:
        errors.append(str(error))
    try:
        install_for_session(base)
    except (OSError, subprocess.SubprocessError) as error:
        errors.append(str(error))
    if errors:
        raise RuntimeError('; '.join(errors))


def remove_server(base=BASE):
    if os.getuid() == 0:
        raise RuntimeError('Session cleanup must not run as root')
    marker = Path.home() / '.local/state/eqntide/server-managed'
    if marker.exists() and marker.read_text() == str(base.resolve()):
        subprocess.run(['bash', str(base / 'scripts/install-bridge-service.sh'), '--uninstall'], check=True, timeout=30)
        marker.unlink()


def package_install(remove=False):
    # sudo/DNF/APT, graphical package managers, then logged-in desktop users.
    uids = set()
    for key in ('SUDO_UID', 'PKEXEC_UID'):
        value = os.environ.get(key, '')
        if value.isdecimal():
            uids.add(int(value))
    try:
        result = subprocess.run(['loginctl', 'list-users', '--no-legend', '--no-pager'],
                                capture_output=True, text=True, timeout=5)
        for line in result.stdout.splitlines():
            fields = line.split()
            if fields and fields[0].isdecimal():
                uids.add(int(fields[0]))
    except (OSError, subprocess.TimeoutExpired):
        pass
    if remove:
        # Only profiles marked by our package setup need offline cleanup.
        for account in pwd.getpwall():
            marker = Path(account.pw_dir) / '.local/state/eqntide/server-managed'
            if account.pw_uid >= 1000 and marker.is_file():
                uids.add(account.pw_uid)
    for uid in sorted(uids):
        try:
            account = pwd.getpwuid(uid)
            if uid < 1000 or not Path(account.pw_dir).is_dir():
                continue
            # Do not inherit root's HOME/XDG/Python environment into user setup.
            env = {'HOME': account.pw_dir, 'USER': account.pw_name,
                   'LOGNAME': account.pw_name, 'PATH': '/usr/bin:/bin',
                   'XDG_RUNTIME_DIR': f'/run/user/{uid}',
                   'DBUS_SESSION_BUS_ADDRESS': f'unix:path=/run/user/{uid}/bus'}
            subprocess.run(['/usr/bin/python3', str(Path(__file__).resolve()),
                            '--remove-server' if remove else '--refresh-service'],
                           env=env, user=uid, group=account.pw_gid,
                           extra_groups=os.getgrouplist(account.pw_name, account.pw_gid),
                           check=True, timeout=60)
        except (KeyError, OSError, subprocess.SubprocessError) as error:
            print(f'EqnTide: session setup for UID {uid} deferred to login: {error}', file=sys.stderr)


if __name__ == '__main__':
    try:
        if '--package-remove' in sys.argv and os.getuid() == 0:
            package_install(remove=True)
        elif '--package-install' in sys.argv and os.getuid() == 0:
            package_install()
        elif '--remove-server' in sys.argv:
            remove_server()
        else:
            configure_session(refresh='--refresh-service' in sys.argv)
    except Exception as error:
        print(f'EqnTide session setup failed (will retry at next login): {error}', file=sys.stderr)
        sys.exit(1)
