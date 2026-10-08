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
PositioningStatus = _core.ais.PositioningStatus
TurnDirection = _core.ais.TurnDirection

# Value types
Dimensions = _core.ais.Dimensions
Eta = _core.ais.Eta

# Sum types in field position (ADR-0009), one variant class per Rust
# variant: `RateOfTurn.DegPerMin` / `RateOfTurn.NoIndicator`,
# `Timestamp.Second` / `Timestamp.PositioningStatus`,
# `Type24BExtent.Dimensions` / `Type24BExtent.MothershipMmsi`.
RateOfTurn = _core.ais.RateOfTurn
Timestamp = _core.ais.Timestamp
Type24BExtent = _core.ais.Type24BExtent

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
# The `clock` argument of `AisParser`: who drives the reassembly timeout.
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
    "PositioningStatus",
    "RateOfTurn",
    "ReassemblyError",
    "SarAircraftPositionReport",
    "StaticAndVoyageA",
    "StaticDataB24A",
    "StaticDataB24B",
    "Timestamp",
    "TurnDirection",
    "Type24BExtent",
]
