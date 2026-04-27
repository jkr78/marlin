"""Typed NMEA 0183 decoders (GGA, VTG, HDT, PSXN, PRDID)."""

from .. import _core

DecodeError: type[Exception] = _core.DecodeError
DecodeOptions = _core.nmea.DecodeOptions
Gga = _core.nmea.Gga
Vtg = _core.nmea.Vtg
Hdt = _core.nmea.Hdt
Psxn = _core.nmea.Psxn
Prdid = _core.nmea.Prdid
PrdidPitchRollHeading = _core.nmea.PrdidPitchRollHeading
PrdidRollPitchHeading = _core.nmea.PrdidRollPitchHeading
PrdidRaw = _core.nmea.PrdidRaw
Unknown = _core.nmea.Unknown
GgaFixQuality = _core.nmea.GgaFixQuality
VtgMode = _core.nmea.VtgMode
PsxnLayout = _core.nmea.PsxnLayout
PsxnSlot = _core.nmea.PsxnSlot
PrdidDialect = _core.nmea.PrdidDialect
UtcTime = _core.nmea.UtcTime
Nmea0183Parser = _core.nmea.Nmea0183Parser
decode = _core.nmea.decode
decode_with = _core.nmea.decode_with
decode_gga = _core.nmea.decode_gga
decode_vtg = _core.nmea.decode_vtg
decode_hdt = _core.nmea.decode_hdt
decode_psxn = _core.nmea.decode_psxn
decode_prdid = _core.nmea.decode_prdid

__all__ = [
    "DecodeError",
    "DecodeOptions",
    "Gga",
    "GgaFixQuality",
    "Hdt",
    "Nmea0183Parser",
    "Prdid",
    "PrdidDialect",
    "PrdidPitchRollHeading",
    "PrdidRaw",
    "PrdidRollPitchHeading",
    "Psxn",
    "PsxnLayout",
    "PsxnSlot",
    "Unknown",
    "UtcTime",
    "Vtg",
    "VtgMode",
    "decode",
    "decode_gga",
    "decode_hdt",
    "decode_prdid",
    "decode_psxn",
    "decode_vtg",
    "decode_with",
]
