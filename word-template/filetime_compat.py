"""Work around ms_dtyp's pre-Unix FILETIME conversion on Windows."""
from datetime import datetime, timedelta


def from_msfiletime(cls, ticks):
    # FILETIME is 100 ns ticks since 1601. Avoid timestamp()/fromtimestamp(),
    # whose supported range depends on the OS C runtime and local timezone.
    value = datetime(1601, 1, 1) + timedelta(microseconds=ticks // 10)
    return cls(value.year, value.month, value.day, value.hour, value.minute,
               value.second, value.microsecond)


def patch_filetime():
    from ms_dtyp.filetime import Filetime
    Filetime.from_msfiletime = classmethod(from_msfiletime)
