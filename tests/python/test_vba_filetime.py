"""Exercise the VBA builder's FILETIME compatibility without build dependencies."""
from datetime import datetime
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('filetime_compat', ROOT / 'word-template/filetime_compat.py')
compat = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compat)


class PlatformLimitedFiletime(datetime):
    def timestamp(self):
        raise OSError('Simulated Windows pre-Unix timestamp limitation')

    @classmethod
    def fromtimestamp(cls, *args, **kwargs):
        raise OSError('Simulated Windows pre-Unix timestamp limitation')


class VbaFiletime(unittest.TestCase):
    def test_zero_filetime_does_not_use_platform_timestamp(self):
        value = compat.from_msfiletime(PlatformLimitedFiletime, 0)
        self.assertIsInstance(value, PlatformLimitedFiletime)
        self.assertEqual(value, datetime(1601, 1, 1))

    def test_unix_epoch_and_subsecond_precision(self):
        ticks = 116444736000000000
        self.assertEqual(compat.from_msfiletime(PlatformLimitedFiletime, ticks), datetime(1970, 1, 1))
        self.assertEqual(compat.from_msfiletime(PlatformLimitedFiletime, ticks + 1234567), datetime(1970, 1, 1, microsecond=123456))
