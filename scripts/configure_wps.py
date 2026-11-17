#!/usr/bin/env python3
"""Merge EqnTide registration without discarding other WPS add-ins."""
import argparse
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET


def _prune_empty_wps_dirs(base):
    for candidate in (base, base.parent, base.parent.parent):
        if candidate.name not in ('jsaddons', 'wps', 'wps_intl', 'Kingsoft'):
            break
        try:
            candidate.rmdir()
        except OSError:
            break


def configure(base, uninstall=False):
    base = Path(base)
    publish = base / 'publish.xml'
    auth_path = base / 'authaddin.json'
    if uninstall:
        if publish.exists():
            root = ET.parse(publish).getroot()
            for node in list(root):
                if node.get('name') in ('EqnTide', 'MathEditor'):
                    root.remove(node)
            if len(root) == 0:
                publish.unlink()
            else:
                ET.ElementTree(root).write(publish, encoding='utf-8', xml_declaration=True)
        if auth_path.exists():
            auth = json.loads(auth_path.read_text(encoding='utf-8-sig'))
            wps = auth.get('wps')
            if isinstance(wps, dict):
                names = wps.get('namelist', '')
                names = names if isinstance(names, list) else re.split(r'[,;\s]+', names)
                names = [name for name in names if name and name not in ('EqnTide', 'MathEditor')]
                wps.pop('EqnTide', None)
                wps.pop('MathEditor', None)
                remaining_keys = [k for k in wps.keys() if k != 'namelist']
                if not names and not remaining_keys:
                    auth.pop('wps', None)
                else:
                    wps['namelist'] = ','.join(names)
            if not auth:
                auth_path.unlink()
            else:
                auth_path.write_text(json.dumps(auth, ensure_ascii=False, indent=2), encoding='utf-8')
        _prune_empty_wps_dirs(base)
        return

    root = ET.parse(publish).getroot() if publish.exists() else ET.Element('jsplugins')
    auth = json.loads(auth_path.read_text(encoding='utf-8-sig')) if auth_path.exists() else {}
    wps = auth.setdefault('wps', {})
    for node in list(root):
        if node.get('name') in ('EqnTide', 'MathEditor'):
            root.remove(node)
    names = wps.get('namelist', '')
    names = names if isinstance(names, list) else re.split(r'[,;\s]+', names)
    names = [name for name in names if name and name not in ('EqnTide', 'MathEditor')]
    wps.pop('EqnTide', None)
    wps.pop('MathEditor', None)
    ET.SubElement(root, 'jsplugin', name='EqnTide', type='wps', version='1.0.0', enable='enable_dev', url='EqnTide_1.0.0/')
    names.append('EqnTide')
    wps['EqnTide'] = dict(name='EqnTide', enable=True, isload=True, path='EqnTide_1.0.0/', md5='', mode=2)
    wps['namelist'] = ','.join(names)
    base.mkdir(parents=True, exist_ok=True)
    ET.ElementTree(root).write(publish, encoding='utf-8', xml_declaration=True)
    auth_path.write_text(json.dumps(auth, ensure_ascii=False, indent=2), encoding='utf-8')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--uninstall', action='store_true')
    args = parser.parse_args()
    configure(args.directory, args.uninstall)
