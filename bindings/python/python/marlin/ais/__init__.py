"""Typed AIS decoders + multi-sentence reassembly."""

from typing import Literal, Union

from .. import _core

AisError: type[Exception] = _core.AisError
ReassemblyError: type[Exception] = _core.ReassemblyError

# Data enums
AisVersion = _core.ais.AisVersion
AltitudeSensor = _core.ais.AltitudeSensor
AtonType = _core.ais.AtonType
EpfdType = _core.ais.EpfdType
ManeuverIndicator = _core.ais.ManeuverIndicator
NavStatus = _core.ais.NavStatus
TurnDirection = _core.ais.TurnDirection

# Value types
Dimensions = _core.ais.Dimensions
Eta = _core.ais.Eta

# Power-user primitive
BitReader = _core.ais.BitReader

# Message variants
AidToNavigationReport = _core.ais.AidToNavigationReport
ExtendedPositionReportB = _core.ais.ExtendedPositionReportB
Other = _core.ais.Other
PositionReportA = _core.ais.PositionReportA
PositionReportB = _core.ais.PositionReportB
SarAircraftPositionReport = _core.ais.SarAircraftPositionReport
StaticAndVoyageA = _core.ais.StaticAndVoyageA
StaticDataB24A = _core.ais.StaticDataB24A
StaticDataB24B = _core.ais.StaticDataB24B

# Any decoded message body; `AisMessage.body` is one of these.
AisMessageBody = Union[
    PositionReportA,
    StaticAndVoyageA,
    SarAircraftPositionReport,
    PositionReportB,
    ExtendedPositionReportB,
    AidToNavigationReport,
    StaticDataB24A,
    StaticDataB24B,
    Other,
]

# Outer message wrapper
AisMessage = _core.ais.AisMessage

# Parser
ClockMode = Literal["auto", "manual"]
AisParser = _core.ais.AisParser

__all__ = [
    "AidToNavigationReport",
    "AisError",
    "AisMessage",
    "AisMessageBody",
    "AisParser",
    "AisVersion",
    "AltitudeSensor",
    "AtonType",
    "BitReader",
    "ClockMode",
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
    "SarAircraftPositionReport",
    "StaticAndVoyageA",
    "StaticDataB24A",
    "StaticDataB24B",
    "TurnDirection",
]
