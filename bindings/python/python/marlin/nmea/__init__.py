"""Typed NMEA 0183 decoders (GGA, GLL, HDT, RMC, VTG, PSXN, PRDID)."""

from .. import _core

DecodeError: type[Exception] = _core.DecodeError
DecodeOptions = _core.nmea.DecodeOptions
Gga = _core.nmea.Gga
Gll = _core.nmea.Gll
Vtg = _core.nmea.Vtg
Hdt = _core.nmea.Hdt
Hdg = _core.nmea.Hdg
Ttm = _core.nmea.Ttm
Tll = _core.nmea.Tll
Rmc = _core.nmea.Rmc
Psxn = _core.nmea.Psxn
Prdid = _core.nmea.Prdid
PrdidPitchRollHeading = _core.nmea.PrdidPitchRollHeading
PrdidRollPitchHeading = _core.nmea.PrdidRollPitchHeading
PrdidRaw = _core.nmea.PrdidRaw
Unknown = _core.nmea.Unknown
GgaFixQuality = _core.nmea.GgaFixQuality
VtgMode = _core.nmea.VtgMode
DataStatus = _core.nmea.DataStatus
RmcNavStatus = _core.nmea.RmcNavStatus
PsxnLayout = _core.nmea.PsxnLayout
PsxnSlot = _core.nmea.PsxnSlot
PrdidDialect = _core.nmea.PrdidDialect
TargetStatus = _core.nmea.TargetStatus
AngleReference = _core.nmea.AngleReference
DistanceUnits = _core.nmea.DistanceUnits
AcquisitionType = _core.nmea.AcquisitionType
UtcTime = _core.nmea.UtcTime
UtcDate = _core.nmea.UtcDate
Nmea0183Parser = _core.nmea.Nmea0183Parser
decode = _core.nmea.decode
decode_with = _core.nmea.decode_with
decode_gga = _core.nmea.decode_gga
decode_gll = _core.nmea.decode_gll
decode_vtg = _core.nmea.decode_vtg
decode_hdt = _core.nmea.decode_hdt
decode_hdg = _core.nmea.decode_hdg
decode_ttm = _core.nmea.decode_ttm
decode_tll = _core.nmea.decode_tll
decode_rmc = _core.nmea.decode_rmc
decode_psxn = _core.nmea.decode_psxn
decode_prdid = _core.nmea.decode_prdid

__all__ = [
    "AcquisitionType",
    "AngleReference",
    "DataStatus",
    "DecodeError",
    "DecodeOptions",
    "DistanceUnits",
    "Gga",
    "GgaFixQuality",
    "Gll",
    "Hdg",
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
    "Rmc",
    "RmcNavStatus",
    "TargetStatus",
    "Tll",
    "Ttm",
    "Unknown",
    "UtcDate",
    "UtcTime",
    "Vtg",
    "VtgMode",
    "decode",
    "decode_gga",
    "decode_gll",
    "decode_hdg",
    "decode_hdt",
    "decode_prdid",
    "decode_psxn",
    "decode_rmc",
    "decode_tll",
    "decode_ttm",
    "decode_vtg",
    "decode_with",
]
