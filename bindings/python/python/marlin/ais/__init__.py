"""Typed AIS decoders + multi-sentence reassembly."""

from .. import _core

AisError: type[Exception] = _core.AisError
ReassemblyError: type[Exception] = _core.ReassemblyError

# Data enums
AisVersion = _core.ais.AisVersion
EpfdType = _core.ais.EpfdType
ManeuverIndicator = _core.ais.ManeuverIndicator
NavStatus = _core.ais.NavStatus

# Value types
Dimensions = _core.ais.Dimensions
Eta = _core.ais.Eta

# Power-user primitive
BitReader = _core.ais.BitReader

# Message variants
ExtendedPositionReportB = _core.ais.ExtendedPositionReportB
Other = _core.ais.Other
PositionReportA = _core.ais.PositionReportA
PositionReportB = _core.ais.PositionReportB
StaticAndVoyageA = _core.ais.StaticAndVoyageA
StaticDataB24A = _core.ais.StaticDataB24A
StaticDataB24B = _core.ais.StaticDataB24B

# Outer message wrapper
AisMessage = _core.ais.AisMessage

# Parser
AisParser = _core.ais.AisParser

__all__ = [
    "AisError",
    "AisMessage",
    "AisParser",
    "AisVersion",
    "BitReader",
    "Dimensions",
    "EpfdType",
    "Eta",
    "ExtendedPositionReportB",
    "ManeuverIndicator",
    "NavStatus",
    "Other",
    "PositionReportA",
    "PositionReportB",
    "ReassemblyError",
    "StaticAndVoyageA",
    "StaticDataB24A",
    "StaticDataB24B",
]
