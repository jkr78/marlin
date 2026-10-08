"""Unit tests for AIS data enums, value types, sum types and messages."""

from __future__ import annotations

from unittest.mock import patch

import pytest

from marlin.ais import (
    AidToNavigationReport,
    AisMessage,
    AisParser,
    AisVersion,
    AltitudeSensor,
    AtonType,
    BitReader,
    Dimensions,
    EpfdType,
    Eta,
    ExtendedPositionReportB,
    ManeuverIndicator,
    NavStatus,
    Other,
    PositioningStatus,
    PositionReportA,
    PositionReportB,
    RateOfTurn,
    ReassemblyError,
    SarAircraftPositionReport,
    StaticAndVoyageA,
    StaticDataB24A,
    StaticDataB24B,
    Timestamp,
    TurnDirection,
    Type24BExtent,
)
from marlin.field import FieldState

from .ais_vectors import (
    AIVDM_TYPE1,
    AIVDM_TYPE1_INVALID_AND_OVER_RANGE,
    AIVDM_TYPE1_ROT_MINUS_127,
    AIVDM_TYPE1_ROT_PLUS_127,
    AIVDM_TYPE5_FRAG1,
    AIVDM_TYPE5_FRAG2,
    AIVDM_TYPE9_GPSD_T9_2,
    AIVDM_TYPE21_GPSD_T21_1_FRAG1,
    AIVDM_TYPE21_GPSD_T21_1_FRAG2,
    AIVDM_TYPE24B_AUXILIARY_CRAFT,
    aivdm,
)

# ---------- int-backed enums ----------


def test_nav_status_values() -> None:
    # Every member is pinned to its wire code. Code 15 (not defined) is the
    # not-available field state and 9..=13 are the invalid field state on
    # the message, so the members jump from 8 to 14.
    assert int(NavStatus.UNDERWAY_USING_ENGINE) == 0
    assert int(NavStatus.AT_ANCHOR) == 1
    assert int(NavStatus.NOT_UNDER_COMMAND) == 2
    assert int(NavStatus.RESTRICTED_MANEUVERABILITY) == 3
    assert int(NavStatus.CONSTRAINED_BY_DRAFT) == 4
    assert int(NavStatus.MOORED) == 5
    assert int(NavStatus.AGROUND) == 6
    assert int(NavStatus.ENGAGED_IN_FISHING) == 7
    assert int(NavStatus.UNDERWAY_SAILING) == 8
    assert int(NavStatus.AIS_SART_ACTIVE) == 14
    assert not hasattr(NavStatus, "NOT_DEFINED")


def test_maneuver_indicator_values() -> None:
    # Code 0 is not available and code 3 is invalid on the message.
    assert int(ManeuverIndicator.NO_SPECIAL) == 1
    assert int(ManeuverIndicator.SPECIAL) == 2
    assert not hasattr(ManeuverIndicator, "NOT_AVAILABLE")
    assert not hasattr(ManeuverIndicator, "RESERVED")


def test_turn_direction_values() -> None:
    # Enum discriminants, not wire codes: the wire carries raw ROT ±127.
    assert int(TurnDirection.RIGHT) == 0
    assert int(TurnDirection.LEFT) == 1
    assert TurnDirection.RIGHT != TurnDirection.LEFT


def test_positioning_status_values_are_the_timestamp_wire_codes() -> None:
    assert int(PositioningStatus.MANUAL_INPUT) == 61
    assert int(PositioningStatus.DEAD_RECKONING) == 62
    assert int(PositioningStatus.INOPERATIVE) == 63


def test_altitude_sensor_values() -> None:
    # Wire codes of the one-bit Table 59 altitude-sensor field.
    assert int(AltitudeSensor.GNSS) == 0
    assert int(AltitudeSensor.BAROMETRIC) == 1
    assert AltitudeSensor.GNSS != AltitudeSensor.BAROMETRIC


def test_aton_type_values() -> None:
    # Wire codes of the 5-bit Table 74 field: 0..=31, all 32 named.
    assert int(AtonType.NOT_SPECIFIED) == 0
    assert int(AtonType.LIGHT_WITHOUT_SECTORS) == 5
    assert int(AtonType.BEACON_SPECIAL_MARK) == 19
    assert int(AtonType.CARDINAL_MARK_NORTH) == 20
    assert int(AtonType.LIGHT_VESSEL) == 31
    members = [getattr(AtonType, n) for n in dir(AtonType) if n.isupper()]
    assert sorted(int(m) for m in members if isinstance(m, AtonType)) == list(
        range(32)
    )


def test_ais_enums_are_hashable() -> None:
    # The stubs declare __hash__ on every int-backed enum; pin it at
    # runtime so the enums work as set members and dict keys.
    members = {
        NavStatus.MOORED,
        ManeuverIndicator.SPECIAL,
        TurnDirection.LEFT,
        PositioningStatus.DEAD_RECKONING,
        AltitudeSensor.BAROMETRIC,
        AtonType.LIGHT_VESSEL,
        EpfdType.GALILEO,
        AisVersion.ITU1371V5,
    }
    assert len(members) == 8
    assert NavStatus.MOORED in members
    assert hash(NavStatus.MOORED) == hash(NavStatus.MOORED)


def test_epfd_type_values() -> None:
    # Code 0 (undefined) is not available and 9..=14 are invalid on the
    # message, so the members jump from 8 to 15.
    assert int(EpfdType.GPS) == 1
    assert int(EpfdType.GLONASS) == 2
    assert int(EpfdType.COMBINED_GPS_GLONASS) == 3
    assert int(EpfdType.LORAN_C) == 4
    assert int(EpfdType.CHAYKA) == 5
    assert int(EpfdType.INTEGRATED_NAVIGATION) == 6
    assert int(EpfdType.SURVEYED) == 7
    assert int(EpfdType.GALILEO) == 8
    assert int(EpfdType.INTERNAL_GNSS) == 15
    assert not hasattr(EpfdType, "UNDEFINED")


def test_ais_version_values() -> None:
    assert int(AisVersion.ITU1371V1) == 0
    assert int(AisVersion.ITU1371V3) == 1
    assert int(AisVersion.ITU1371V5) == 2
    assert int(AisVersion.FUTURE) == 3


# ---------- Dimensions and Eta ----------


def test_dimensions_fields_are_field_states() -> None:
    d = Dimensions(to_bow_m=10, to_stern_m=20, to_port_m=3, to_starboard_m=4)
    assert d.to_bow_m == FieldState.Value(10)
    assert d.to_stern_m == FieldState.Value(20)
    assert d.to_port_m == FieldState.Value(3)
    assert d.to_starboard_m == FieldState.Value(4)


def test_dimensions_default_to_not_available() -> None:
    d = Dimensions()
    assert d.to_bow_m == FieldState.NotAvailable()
    assert d.to_stern_m == FieldState.NotAvailable()
    assert d.to_port_m == FieldState.NotAvailable()
    assert d.to_starboard_m == FieldState.NotAvailable()


def test_dimensions_accept_a_field_state_and_none() -> None:
    d = Dimensions(to_bow_m=FieldState.AtLeast(511), to_stern_m=None)
    assert d.to_bow_m == FieldState.AtLeast(511)
    assert d.to_stern_m == FieldState.NotAvailable()
    assert repr(d) == (
        "Dimensions(to_bow_m=FieldState.AtLeast(511), "
        "to_stern_m=FieldState.NotAvailable(), to_port_m=FieldState.NotAvailable(), "
        "to_starboard_m=FieldState.NotAvailable())"
    )


def test_dimensions_frozen() -> None:
    d = Dimensions(to_bow_m=10)
    with pytest.raises((AttributeError, TypeError)):
        d.to_bow_m = 20  # type: ignore[misc, assignment]


def test_dimensions_eq_hash() -> None:
    a = Dimensions(to_bow_m=10, to_stern_m=20, to_port_m=3, to_starboard_m=4)
    b = Dimensions(to_bow_m=10, to_stern_m=20, to_port_m=3, to_starboard_m=4)
    assert a == b
    assert hash(a) == hash(b)
    assert a != Dimensions(to_bow_m=FieldState.AtLeast(10), to_stern_m=20, to_port_m=3, to_starboard_m=4)


def test_eta_fields_are_field_states() -> None:
    e = Eta(month=3, day=15, hour=12, minute=30)
    assert e.month == FieldState.Value(3)
    assert e.day == FieldState.Value(15)
    assert e.hour == FieldState.Value(12)
    assert e.minute == FieldState.Value(30)


def test_eta_default_to_not_available() -> None:
    e = Eta()
    assert e.month == FieldState.NotAvailable()
    assert e.day == FieldState.NotAvailable()
    assert e.hour == FieldState.NotAvailable()
    assert e.minute == FieldState.NotAvailable()


def test_eta_eq_hash() -> None:
    a = Eta(month=3, day=15, hour=12, minute=30)
    b = Eta(month=3, day=15, hour=12, minute=30)
    assert a == b
    assert hash(a) == hash(b)
    assert Eta(month=FieldState.Invalid(13)) != Eta()


# ---------- the three sum types ----------


def test_rate_of_turn_variant_classes() -> None:
    measured = RateOfTurn.DegPerMin(19.7)
    status = RateOfTurn.NoIndicator(TurnDirection.RIGHT)
    assert isinstance(measured, RateOfTurn)
    assert isinstance(measured, RateOfTurn.DegPerMin)
    assert not isinstance(measured, RateOfTurn.NoIndicator)
    assert isinstance(status, RateOfTurn.NoIndicator)
    assert measured.deg_per_min == pytest.approx(19.7)
    assert status.direction == TurnDirection.RIGHT
    assert RateOfTurn.DegPerMin.__match_args__ == ("deg_per_min",)
    assert RateOfTurn.NoIndicator.__match_args__ == ("direction",)
    assert type(measured).__name__ == "DegPerMin"
    assert type(measured).__qualname__ == "RateOfTurn.DegPerMin"
    assert RateOfTurn.DegPerMin.__module__ == "marlin.ais"


def test_rate_of_turn_eq_hash_repr() -> None:
    assert RateOfTurn.DegPerMin(1.5) == RateOfTurn.DegPerMin(1.5)
    assert RateOfTurn.DegPerMin(1.5) != RateOfTurn.DegPerMin(1.25)
    assert RateOfTurn.NoIndicator(TurnDirection.LEFT) == RateOfTurn.NoIndicator(TurnDirection.LEFT)
    assert RateOfTurn.NoIndicator(TurnDirection.LEFT) != RateOfTurn.NoIndicator(TurnDirection.RIGHT)
    assert RateOfTurn.DegPerMin(0.0) != RateOfTurn.NoIndicator(TurnDirection.LEFT)
    assert hash(RateOfTurn.DegPerMin(1.5)) == hash(RateOfTurn.DegPerMin(1.5))
    assert len({RateOfTurn.DegPerMin(1.5), RateOfTurn.DegPerMin(1.5), RateOfTurn.NoIndicator(TurnDirection.LEFT)}) == 2
    assert repr(RateOfTurn.DegPerMin(1.5)) == "RateOfTurn.DegPerMin(1.5)"
    assert repr(RateOfTurn.NoIndicator(TurnDirection.LEFT)) == "RateOfTurn.NoIndicator(TurnDirection.LEFT)"


def test_rate_of_turn_base_is_not_instantiable() -> None:
    with pytest.raises(TypeError, match="cannot create 'marlin.ais.RateOfTurn' instances"):
        RateOfTurn()


def test_timestamp_variant_classes() -> None:
    second = Timestamp.Second(42)
    status = Timestamp.PositioningStatus(PositioningStatus.DEAD_RECKONING)
    assert isinstance(second, Timestamp)
    assert isinstance(second, Timestamp.Second)
    assert isinstance(status, Timestamp.PositioningStatus)
    assert second.second == 42
    assert status.status == PositioningStatus.DEAD_RECKONING
    assert Timestamp.Second.__match_args__ == ("second",)
    assert Timestamp.PositioningStatus.__match_args__ == ("status",)
    # The variant class and the enum share a name but not a qualified name.
    assert type(status).__name__ == "PositioningStatus"
    assert type(status).__qualname__ == "Timestamp.PositioningStatus"
    assert type(status) is not PositioningStatus  # type: ignore[comparison-overlap]


def test_timestamp_eq_hash_repr() -> None:
    assert Timestamp.Second(42) == Timestamp.Second(42)
    assert Timestamp.Second(42) != Timestamp.Second(41)
    assert Timestamp.PositioningStatus(PositioningStatus.INOPERATIVE) == Timestamp.PositioningStatus(PositioningStatus.INOPERATIVE)
    assert Timestamp.Second(61) != Timestamp.PositioningStatus(PositioningStatus.MANUAL_INPUT)
    assert hash(Timestamp.Second(42)) == hash(Timestamp.Second(42))
    assert repr(Timestamp.Second(42)) == "Timestamp.Second(42)"
    assert repr(Timestamp.PositioningStatus(PositioningStatus.MANUAL_INPUT)) == "Timestamp.PositioningStatus(PositioningStatus.MANUAL_INPUT)"


def test_timestamp_base_is_not_instantiable() -> None:
    with pytest.raises(TypeError, match="cannot create 'marlin.ais.Timestamp' instances"):
        Timestamp()


def test_type24b_extent_variant_classes() -> None:
    dims = Dimensions(to_bow_m=30, to_stern_m=10, to_port_m=5, to_starboard_m=3)
    extent = Type24BExtent.Dimensions(dims)
    mother = Type24BExtent.MothershipMmsi(211000123)
    assert isinstance(extent, Type24BExtent)
    assert isinstance(extent, Type24BExtent.Dimensions)
    assert isinstance(mother, Type24BExtent.MothershipMmsi)
    assert extent.dimensions == dims
    assert mother.mmsi == 211000123
    assert Type24BExtent.Dimensions.__match_args__ == ("dimensions",)
    assert Type24BExtent.MothershipMmsi.__match_args__ == ("mmsi",)
    # The variant class and the value type share a name but not a qualified name.
    assert type(extent).__name__ == "Dimensions"
    assert type(extent).__qualname__ == "Type24BExtent.Dimensions"
    assert type(extent) is not Dimensions  # type: ignore[comparison-overlap]


def test_type24b_extent_eq_hash_repr() -> None:
    assert Type24BExtent.Dimensions(Dimensions(to_bow_m=1)) == Type24BExtent.Dimensions(Dimensions(to_bow_m=1))
    assert Type24BExtent.Dimensions(Dimensions(to_bow_m=1)) != Type24BExtent.Dimensions(Dimensions(to_bow_m=2))
    assert Type24BExtent.MothershipMmsi(1) == Type24BExtent.MothershipMmsi(1)
    assert Type24BExtent.MothershipMmsi(1) != Type24BExtent.Dimensions(Dimensions())
    assert hash(Type24BExtent.MothershipMmsi(1)) == hash(Type24BExtent.MothershipMmsi(1))
    assert hash(Type24BExtent.Dimensions(Dimensions(to_bow_m=1))) == hash(Type24BExtent.Dimensions(Dimensions(to_bow_m=1)))
    assert repr(Type24BExtent.MothershipMmsi(211000123)) == "Type24BExtent.MothershipMmsi(211000123)"
    assert repr(Type24BExtent.Dimensions(Dimensions())) == (
        "Type24BExtent.Dimensions(Dimensions(to_bow_m=FieldState.NotAvailable(), "
        "to_stern_m=FieldState.NotAvailable(), to_port_m=FieldState.NotAvailable(), "
        "to_starboard_m=FieldState.NotAvailable()))"
    )


def test_type24b_extent_base_is_not_instantiable() -> None:
    with pytest.raises(TypeError, match="cannot create 'marlin.ais.Type24BExtent' instances"):
        Type24BExtent()


# ---------- message constructors ----------


def test_position_report_a_shape() -> None:
    p = PositionReportA(
        mmsi=123456789,
        navigation_status=NavStatus.UNDERWAY_USING_ENGINE,
        latitude_deg=48.5,
        longitude_deg=11.5,
        speed_over_ground=12.4,
        true_heading=90,
        timestamp=Timestamp.Second(7),
    )
    assert p.mmsi == 123456789
    assert p.navigation_status == FieldState.Value(NavStatus.UNDERWAY_USING_ENGINE)
    assert p.latitude_deg.value == pytest.approx(48.5)
    assert p.longitude_deg.value == pytest.approx(11.5)
    assert p.speed_over_ground.value == pytest.approx(12.4)
    assert p.true_heading == FieldState.Value(90)
    assert p.timestamp == FieldState.Value(Timestamp.Second(7))
    # Field-state keywords default to NotAvailable(), plain ones to false / 0:
    assert p.rate_of_turn == FieldState.NotAvailable()
    assert p.course_over_ground == FieldState.NotAvailable()
    assert p.special_maneuver == FieldState.NotAvailable()
    assert p.position_accuracy is False
    assert p.raim is False
    assert p.radio_status == 0
    assert not hasattr(p, "turn_direction")


def test_position_report_a_all_defaults() -> None:
    p = PositionReportA()
    assert p.mmsi == 0
    assert p.navigation_status == FieldState.NotAvailable()
    assert p.rate_of_turn == FieldState.NotAvailable()
    assert p.timestamp == FieldState.NotAvailable()
    assert p.special_maneuver == FieldState.NotAvailable()


def test_position_report_a_field_state_coercion() -> None:
    # A FieldState passes through, a bare value is Value, None is NotAvailable().
    # 102.5 is exact in the f32 the field is stored as; 102.2 is not.
    p = PositionReportA(
        speed_over_ground=FieldState.AtLeast(102.5),
        rate_of_turn=RateOfTurn.NoIndicator(TurnDirection.LEFT),
        navigation_status=FieldState.Invalid(9),
        latitude_deg=None,
    )
    assert p.speed_over_ground == FieldState.AtLeast(102.5)
    assert p.rate_of_turn == FieldState.Value(RateOfTurn.NoIndicator(TurnDirection.LEFT))
    assert p.navigation_status == FieldState.Invalid(9)
    assert p.latitude_deg == FieldState.NotAvailable()


def test_position_report_a_rejects_a_wrong_typed_payload() -> None:
    with pytest.raises(TypeError, match="'str' object cannot be cast as 'NavStatus'"):
        PositionReportA(navigation_status="moored")  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="'int' object cannot be cast as 'Timestamp'"):
        PositionReportA(timestamp=40)  # type: ignore[arg-type]


def test_position_report_a_repr_prints_the_variant_form() -> None:
    p = PositionReportA(mmsi=1, latitude_deg=48.5, speed_over_ground=FieldState.AtLeast(102.5))
    assert repr(p) == (
        "PositionReportA(mmsi=1, lat=FieldState.Value(48.5), "
        "lon=FieldState.NotAvailable(), sog=FieldState.AtLeast(102.5))"
    )


def test_static_and_voyage_a_shape() -> None:
    s = StaticAndVoyageA(
        mmsi=123456789,
        ais_version=AisVersion.ITU1371V5,
        imo_number=9074729,
        call_sign="ABCD",
        vessel_name="MY VESSEL",
        ship_type=70,  # cargo
        dimensions=Dimensions(to_bow_m=100, to_stern_m=20, to_port_m=10, to_starboard_m=10),
        epfd=EpfdType.GPS,
        eta=Eta(month=6, day=15, hour=14, minute=30),
        draught_m=FieldState.AtLeast(25.5),
        destination="HAMBURG",
        dte=False,
    )
    assert s.mmsi == 123456789
    assert s.ais_version == AisVersion.ITU1371V5
    assert s.imo_number == FieldState.Value(9074729)
    assert s.call_sign == FieldState.Value("ABCD")
    assert s.vessel_name == FieldState.Value("MY VESSEL")
    assert s.ship_type == FieldState.Value(70)
    assert s.dimensions.to_bow_m == FieldState.Value(100)
    assert s.epfd == FieldState.Value(EpfdType.GPS)
    assert s.eta.month == FieldState.Value(6)
    assert s.draught_m == FieldState.AtLeast(25.5)
    assert s.destination == FieldState.Value("HAMBURG")
    assert s.dte == FieldState.Value(False)


def test_static_and_voyage_a_all_defaults() -> None:
    s = StaticAndVoyageA()
    assert s.ais_version == AisVersion.FUTURE
    assert s.imo_number == FieldState.NotAvailable()
    assert s.vessel_name == FieldState.NotAvailable()
    assert s.ship_type == FieldState.NotAvailable()
    assert s.dimensions == Dimensions()
    assert s.epfd == FieldState.NotAvailable()
    assert s.eta == Eta()
    assert s.dte == FieldState.NotAvailable()


def test_sar_aircraft_position_report_shape() -> None:
    p = SarAircraftPositionReport(
        mmsi=111222333,
        altitude_m=FieldState.AtLeast(4094),
        speed_over_ground=120,
        latitude_deg=58.1,
        longitude_deg=-6.2,
        altitude_sensor=AltitudeSensor.BAROMETRIC,
        dte=True,
    )
    assert p.mmsi == 111222333
    assert p.altitude_m == FieldState.AtLeast(4094)
    assert p.speed_over_ground == FieldState.Value(120)
    assert p.latitude_deg.value == pytest.approx(58.1)
    assert p.longitude_deg.value == pytest.approx(-6.2)
    assert p.altitude_sensor == AltitudeSensor.BAROMETRIC
    assert p.dte is True
    # Defaults fire for unset fields:
    assert p.position_accuracy is False
    assert p.course_over_ground == FieldState.NotAvailable()
    assert p.timestamp == FieldState.NotAvailable()
    assert p.assigned_flag is False
    assert p.raim is False
    assert p.radio_status == 0
    # No heading, rate of turn or navigational status exists on Type 9.
    assert not hasattr(p, "true_heading")
    assert not hasattr(p, "rate_of_turn")
    assert not hasattr(p, "navigation_status")


def test_position_report_b_shape() -> None:
    p = PositionReportB(
        mmsi=222333444,
        latitude_deg=48.5,
        speed_over_ground=5.0,
        class_b_cs_flag=True,
        class_b_message22_flag=True,
    )
    assert p.mmsi == 222333444
    assert p.latitude_deg.value == pytest.approx(48.5)
    assert p.speed_over_ground.value == pytest.approx(5.0)
    assert p.class_b_cs_flag is True
    assert p.class_b_message22_flag is True
    # Defaults:
    assert p.timestamp == FieldState.NotAvailable()
    assert p.class_b_display_flag is False
    assert p.class_b_dsc_flag is False
    assert p.class_b_band_flag is False
    assert p.radio_status == 0


def test_extended_position_report_b_shape() -> None:
    p = ExtendedPositionReportB(
        mmsi=222333444,
        vessel_name="CLASS B",
        ship_type=37,
        dimensions=Dimensions(to_bow_m=15),
        epfd=EpfdType.GLONASS,
    )
    assert p.mmsi == 222333444
    assert p.vessel_name == FieldState.Value("CLASS B")
    assert p.ship_type == FieldState.Value(37)
    assert p.dimensions.to_bow_m == FieldState.Value(15)
    assert p.epfd == FieldState.Value(EpfdType.GLONASS)
    assert p.timestamp == FieldState.NotAvailable()
    assert p.dte is False
    assert p.assigned_flag is False


def test_aid_to_navigation_report_shape() -> None:
    p = AidToNavigationReport(
        mmsi=992471234,
        aton_type=AtonType.PORT_HAND_MARK,
        name="RED BUOY 7",
        latitude_deg=-4.8,
        longitude_deg=11.0,
        dimensions=Dimensions(to_bow_m=3, to_stern_m=4, to_port_m=1, to_starboard_m=2),
        epfd=EpfdType.GPS,
        timestamp=Timestamp.Second(42),
        off_position=True,
        aton_status=0xA5,
        virtual_aton=True,
    )
    assert p.mmsi == 992471234
    assert p.aton_type == AtonType.PORT_HAND_MARK
    assert p.name == FieldState.Value("RED BUOY 7")
    assert p.latitude_deg.value == pytest.approx(-4.8)
    assert p.longitude_deg.value == pytest.approx(11.0)
    assert p.dimensions.to_bow_m == FieldState.Value(3)
    assert p.dimensions.to_starboard_m == FieldState.Value(2)
    assert p.epfd == FieldState.Value(EpfdType.GPS)
    assert p.timestamp == FieldState.Value(Timestamp.Second(42))
    assert p.off_position is True
    assert p.aton_status == 0xA5
    assert p.virtual_aton is True
    # Defaults fire for unset fields:
    assert p.position_accuracy is False
    assert p.raim is False
    assert p.assigned_flag is False
    # No speed, course or heading exists on Type 21.
    assert not hasattr(p, "speed_over_ground")
    assert not hasattr(p, "course_over_ground")
    assert not hasattr(p, "true_heading")


def test_aid_to_navigation_report_all_defaults() -> None:
    p = AidToNavigationReport()
    assert p.mmsi == 0
    assert p.aton_type == AtonType.NOT_SPECIFIED
    assert p.name == FieldState.NotAvailable()
    assert p.dimensions == Dimensions()
    assert p.epfd == FieldState.NotAvailable()
    assert p.timestamp == FieldState.NotAvailable()
    assert p.aton_status == 0


def test_static_data_b24a_shape() -> None:
    s = StaticDataB24A(mmsi=222333444, vessel_name="NAMED")
    assert s.mmsi == 222333444
    assert s.vessel_name == FieldState.Value("NAMED")
    assert StaticDataB24A().vessel_name == FieldState.NotAvailable()


def test_static_data_b24b_shape() -> None:
    s = StaticDataB24B(
        mmsi=222333444,
        ship_type=37,
        vendor_id="VND1",
        call_sign="CS1",
        extent=Type24BExtent.Dimensions(Dimensions(to_bow_m=12)),
        epfd=EpfdType.GALILEO,
    )
    assert s.mmsi == 222333444
    assert s.ship_type == FieldState.Value(37)
    assert s.vendor_id == FieldState.Value("VND1")
    assert s.call_sign == FieldState.Value("CS1")
    assert isinstance(s.extent, Type24BExtent.Dimensions)
    assert s.extent.dimensions.to_bow_m == FieldState.Value(12)
    assert s.epfd == FieldState.Value(EpfdType.GALILEO)
    assert not hasattr(s, "dimensions")
    assert not hasattr(s, "mothership_mmsi")


def test_static_data_b24b_defaults() -> None:
    # The extent defaults to dimensions with every member not available,
    # as a Rust message always carries one of the two variants.
    s = StaticDataB24B()
    assert s.extent == Type24BExtent.Dimensions(Dimensions())
    assert s.ship_type == FieldState.NotAvailable()
    assert s.epfd == FieldState.NotAvailable()


def test_other_shape() -> None:
    # Type 8 (binary broadcast) is not decoded, so it is a realistic Other.
    o = Other(msg_type=8, raw_payload=b"\x01\x02\x03", total_bits=24)
    assert o.msg_type == 8
    assert o.raw_payload == b"\x01\x02\x03"
    assert o.total_bits == 24


def test_ais_message_class_exists() -> None:
    assert AisMessage.__name__ == "AisMessage"


def test_ais_message_construct_type1() -> None:
    body = PositionReportA(mmsi=123456789, latitude_deg=48.5)
    msg = AisMessage(is_own_ship=False, type_tag="type1", body=body)
    assert msg.is_own_ship is False
    assert msg.type_tag == "type1"
    # body getter returns a reference to the same body instance (Py<PyAny>
    # clone_ref bumps refcount, does not deep-copy).
    assert msg.body is body


def test_ais_message_construct_own_ship() -> None:
    body = StaticDataB24A(mmsi=222333444, vessel_name="OWN")
    msg = AisMessage(is_own_ship=True, type_tag="type24a", body=body)
    assert msg.is_own_ship is True
    assert msg.type_tag == "type24a"
    assert isinstance(msg.body, StaticDataB24A)
    assert msg.body.vessel_name == FieldState.Value("OWN")


def test_ais_message_body_with_other_variant() -> None:
    body = Other(msg_type=8, raw_payload=b"\x01\x02", total_bits=16)
    msg = AisMessage(is_own_ship=False, type_tag="other", body=body)
    assert msg.type_tag == "other"
    assert isinstance(msg.body, Other)
    assert msg.body.msg_type == 8


def test_ais_message_repr() -> None:
    msg = AisMessage(
        is_own_ship=True,
        type_tag="type1",
        body=PositionReportA(mmsi=1),
    )
    # Pin the exact format: `{:?}` on String gives quoted, `{}` on bool gives
    # lowercase `true`/`false`. A substring check would silently accept a
    # regression that drops either the label or the quotes.
    assert repr(msg) == 'AisMessage(type_tag="type1", is_own_ship=true)'


# ---------- BitReader ----------


def test_bit_reader_basic() -> None:
    # 0b10101010 0b11110000 → first bit 1, next 7 bits 0101010, then 8 remain.
    data = bytes([0b10101010, 0b11110000])
    reader = BitReader(data, total_bits=16)
    assert reader.u(1) == 1
    assert reader.u(7) == 0b0101010
    assert reader.remaining() == 8


def test_bit_reader_signed() -> None:
    # Upper 6 bits of 0b11111100 = 0b111111 = -1 in 6-bit two's complement.
    data = bytes([0b11111100])
    reader = BitReader(data, total_bits=6)
    assert reader.i(6) == -1


def test_bit_reader_past_end() -> None:
    # Total bits 4, buffer has 8; read past the declared end saturates to 0.
    reader = BitReader(bytes([0xFF]), total_bits=4)
    assert reader.u(4) == 0b1111
    assert reader.u(4) == 0  # past-end → 0
    assert reader.remaining() == 0


def test_bit_reader_bool() -> None:
    # 0b10000000: first bit True, next bit False.
    reader = BitReader(bytes([0b10000000]), total_bits=8)
    assert reader.b() is True
    assert reader.b() is False


def test_bit_reader_string_6bit() -> None:
    # "AB" as two 6-bit AIS chars = 000001 000010, packed into bytes.
    # 000001 000010 00xxxx xxxxxxxx (padding) = 0b00000100 0b00100000
    reader = BitReader(bytes([0b00000100, 0b00100000]), total_bits=12)
    assert reader.string(2) == "AB"


def test_bit_reader_string_preserves_at_padding() -> None:
    # Upstream marlin_ais::BitReader::string does NOT trim trailing '@'
    # (which is AIS 6-bit code 0). The typed message decoders (e.g.
    # StaticAndVoyageA.vessel_name) are the layer that trims. This test
    # pins the primitive's raw-preservation behavior.
    # "A@@@" = 000001 000000 000000 000000 = 24 bits packed big-endian.
    reader = BitReader(bytes([0b00000100, 0b00000000, 0b00000000]), total_bits=24)
    assert reader.string(4) == "A@@@"


def test_bit_reader_read_across_byte_boundary() -> None:
    # Two bytes 0xAB 0xCD = 1010 1011 1100 1101. Read 4 bits, then 12
    # bits (spans the byte boundary: last 4 bits of byte 0 + all of byte 1).
    reader = BitReader(bytes([0xAB, 0xCD]), total_bits=16)
    assert reader.u(4) == 0xA       # 1010
    assert reader.u(12) == 0xBCD    # 1011 1100 1101
    assert reader.remaining() == 0


# ---------- decoding through the parser ----------


def test_ais_streaming_single_fragment() -> None:
    p = AisParser.streaming()
    p.feed(AIVDM_TYPE1)
    messages = list(p)
    assert len(messages) == 1
    assert isinstance(messages[0], AisMessage)
    assert isinstance(messages[0].body, PositionReportA)
    assert messages[0].type_tag == "type1"


def test_ais_classic_type1_decodes_field_states() -> None:
    # The classic Annex 5 fixture: ROT -128 and heading 511 are not
    # available, the timestamp is second 40, the manoeuvre code 0 is not
    # available, navigation status 0 is a value.
    p = AisParser.streaming()
    p.feed(AIVDM_TYPE1)
    body = list(p)[0].body
    assert isinstance(body, PositionReportA)
    assert body.rate_of_turn == FieldState.NotAvailable()
    assert body.true_heading == FieldState.NotAvailable()
    assert body.timestamp == FieldState.Value(Timestamp.Second(40))
    assert body.special_maneuver == FieldState.NotAvailable()
    assert body.navigation_status == FieldState.Value(NavStatus.UNDERWAY_USING_ENGINE)
    assert body.speed_over_ground == FieldState.Value(0.0)
    assert body.latitude_deg.value == pytest.approx(51.229637, abs=1e-6)


@pytest.mark.parametrize(
    ("sentence", "expected"),
    [
        (AIVDM_TYPE1_ROT_PLUS_127, TurnDirection.RIGHT),
        (AIVDM_TYPE1_ROT_MINUS_127, TurnDirection.LEFT),
    ],
    ids=["plus_127_right", "minus_127_left"],
)
def test_position_report_a_rate_of_turn_no_indicator(
    sentence: bytes, expected: TurnDirection
) -> None:
    # Raw ROT ±127 is the "turning faster than 5°/30 s, no turn indicator"
    # status: a value of the field's own sum type, not a field state and
    # not a fabricated ±720 °/min.
    p = AisParser.streaming()
    p.feed(sentence)
    msgs = list(p)
    assert len(msgs) == 1
    body = msgs[0].body
    assert isinstance(body, PositionReportA)
    assert body.mmsi == 123456789
    assert body.rate_of_turn == FieldState.Value(RateOfTurn.NoIndicator(expected))
    match body.rate_of_turn:
        case FieldState.Value(RateOfTurn.NoIndicator(direction)):
            assert direction == expected
        case other:
            pytest.fail(f"unexpected rate of turn {other!r}")


def test_position_report_a_invalid_and_over_range_states() -> None:
    # Navigation status 9 and manoeuvre 3 are reserved, longitude +181°
    # + 1 and latitude +91° + 1 lie beyond the defined range by a code
    # other than not available, COG 3601 and heading 360 are undefined:
    # each is FieldState.Invalid with the wire integer. SOG 1022 is the
    # over-range bound and timestamp 61 the manual-input status.
    p = AisParser.streaming()
    p.feed(AIVDM_TYPE1_INVALID_AND_OVER_RANGE)
    body = list(p)[0].body
    assert isinstance(body, PositionReportA)
    assert body.navigation_status == FieldState.Invalid(9)
    assert isinstance(body.speed_over_ground, FieldState.AtLeast)
    assert body.speed_over_ground.bound == pytest.approx(102.2)
    assert body.longitude_deg == FieldState.Invalid(181 * 600_000 + 1)
    assert body.latitude_deg == FieldState.Invalid(91 * 600_000 + 1)
    assert body.course_over_ground == FieldState.Invalid(3601)
    assert body.true_heading == FieldState.Invalid(360)
    assert body.timestamp == FieldState.Value(
        Timestamp.PositioningStatus(PositioningStatus.MANUAL_INPUT)
    )
    assert body.special_maneuver == FieldState.Invalid(3)
    assert body.speed_over_ground.value is None
    assert body.speed_over_ground.value_or_bound == pytest.approx(102.2)


def test_static_and_voyage_a_gpsd_vector() -> None:
    p = AisParser.streaming()
    p.feed(AIVDM_TYPE5_FRAG1 + AIVDM_TYPE5_FRAG2)
    msgs = list(p)
    assert len(msgs) == 1
    body = msgs[0].body
    assert isinstance(body, StaticAndVoyageA)
    assert body.mmsi == 369190000
    assert body.vessel_name == FieldState.Value("MT.MITCHELL")
    assert body.destination == FieldState.Value("SEATTLENF]ISA")
    assert isinstance(body.ship_type, FieldState.Value)
    # The 424-bit payload carries the DTE bit.
    assert body.dte == FieldState.Value(True)


def test_static_data_b24b_mothership_mmsi() -> None:
    # An auxiliary-craft MMSI (98MIDxxxx) carries the mother ship's MMSI
    # in the 30 bits that otherwise hold dimensions (ADR-0002); the extent
    # is the MothershipMmsi variant class (ADR-0009).
    p = AisParser.streaming()
    p.feed(AIVDM_TYPE24B_AUXILIARY_CRAFT)
    msgs = list(p)
    assert len(msgs) == 1
    body = msgs[0].body
    assert isinstance(body, StaticDataB24B)
    assert body.mmsi == 987654321
    assert body.ship_type == FieldState.Value(37)
    assert body.vendor_id == FieldState.Value("VND1234")
    assert body.call_sign == FieldState.Value("CS001")
    assert body.extent == Type24BExtent.MothershipMmsi(211000123)
    assert body.epfd == FieldState.Value(EpfdType.GPS)


def test_sar_aircraft_position_report_gpsd_t9_2() -> None:
    # A Type 9 payload decodes to SarAircraftPositionReport with
    # type_tag "type9" instead of landing in Other. Expected values are
    # the gpsd .chk entries for T9-2; lon/lat converted to degrees.
    p = AisParser.streaming()
    p.feed(AIVDM_TYPE9_GPSD_T9_2)
    msgs = list(p)
    assert len(msgs) == 1
    assert msgs[0].type_tag == "type9"
    body = msgs[0].body
    assert isinstance(body, SarAircraftPositionReport)
    assert body.mmsi == 111232511
    assert body.altitude_m == FieldState.Value(303)
    assert body.speed_over_ground == FieldState.Value(42)
    assert body.position_accuracy is False
    assert body.longitude_deg.value == pytest.approx(-6.278843, abs=1e-6)
    assert body.latitude_deg.value == pytest.approx(58.144, abs=1e-6)
    assert body.course_over_ground.value == pytest.approx(154.5, abs=1e-4)
    assert body.timestamp == FieldState.Value(Timestamp.Second(15))
    assert body.altitude_sensor == AltitudeSensor.GNSS
    assert body.dte is True
    assert body.assigned_flag is False
    assert body.raim is False
    assert body.radio_status == 0x8270


def test_aid_to_navigation_report_gpsd_t21_1() -> None:
    # A two-fragment Type 21 payload (346 bits, misaligned fill) decodes to
    # AidToNavigationReport with type_tag "type21". Expected values are the
    # gpsd .chk entries for T21-1; the 12-character name extension ends in
    # a trailing @ that the trim removes.
    p = AisParser.streaming()
    p.feed(AIVDM_TYPE21_GPSD_T21_1_FRAG1 + AIVDM_TYPE21_GPSD_T21_1_FRAG2)
    msgs = list(p)
    assert len(msgs) == 1
    assert msgs[0].type_tag == "type21"
    body = msgs[0].body
    assert isinstance(body, AidToNavigationReport)
    assert body.mmsi == 123456789
    assert body.aton_type == AtonType.CARDINAL_MARK_NORTH
    assert body.name == FieldState.Value("CHINA ROSE MURPHY EXPRESS ALERT")
    assert body.position_accuracy is False
    assert body.longitude_deg.value == pytest.approx(-122.698592, abs=1e-6)
    assert body.latitude_deg.value == pytest.approx(47.920618, abs=1e-6)
    assert body.dimensions == Dimensions(
        to_bow_m=5, to_stern_m=5, to_port_m=5, to_starboard_m=5
    )
    assert body.epfd == FieldState.Value(EpfdType.GPS)
    assert body.timestamp == FieldState.Value(Timestamp.Second(50))
    assert body.off_position is False
    assert body.aton_status == 165
    assert body.raim is False
    assert body.virtual_aton is False
    assert body.assigned_flag is False


# ---------- reassembly clock ----------


def test_ais_auto_clock_reads_time() -> None:
    # timeout_ms triggers the "auto" clock path. Patch time.monotonic_ns
    # and assert the parser calls it.
    with patch("time.monotonic_ns", return_value=0) as mock_now:
        p = AisParser.streaming(timeout_ms=60_000)  # clock="auto" default
        p.feed(AIVDM_TYPE1)
        list(p)
        assert mock_now.called


def test_ais_manual_clock_never_reads_time() -> None:
    # CRITICAL: the "manual" clock must never touch time.monotonic_ns.
    # This is the testability contract that lets users replay historical
    # AIS data deterministically without wall-clock interference.
    with patch("time.monotonic_ns") as mock_now:
        p = AisParser.streaming(timeout_ms=60_000, clock="manual")
        p.tick(now_ms=1_000_000)
        p.feed(AIVDM_TYPE1)
        list(p)
        mock_now.assert_not_called()


def test_ais_tick_on_auto_raises() -> None:
    p = AisParser.streaming(timeout_ms=60_000, clock="auto")
    with pytest.raises(ValueError):
        p.tick(now_ms=1000)


def test_ais_manual_tick_past_timeout_raises_on_next_message() -> None:
    # Fragment 1 of a 2-fragment message at t=0; the clock then jumps past
    # the timeout. tick() evicts the partial and the eviction surfaces
    # as ReassemblyError from the next next_message().
    p = AisParser.streaming(timeout_ms=1_000, clock="manual")
    p.tick(now_ms=0)
    p.feed(aivdm(2, 1, 1, "A", b"XXXXXXX", 0))
    assert p.next_message() is None
    p.tick(now_ms=5_000)
    with pytest.raises(ReassemblyError):
        p.next_message()
    assert p.next_message() is None


def test_ais_manual_clock_starts_at_zero() -> None:
    # A fragment fed before the first tick() is stamped 0, so the first
    # tick past the timeout evicts it: the same result as before 0.2.0.
    p = AisParser.streaming(timeout_ms=1_000, clock="manual")
    p.feed(aivdm(2, 1, 1, "A", b"XXXXXXX", 0))
    assert p.next_message() is None
    p.tick(now_ms=5_000)
    with pytest.raises(ReassemblyError):
        p.next_message()


def test_ais_manual_tick_within_timeout_keeps_partial() -> None:
    p = AisParser.streaming(timeout_ms=10_000, clock="manual")
    p.tick(now_ms=0)
    p.feed(aivdm(2, 1, 1, "A", b"XXXXXXX", 0))
    assert p.next_message() is None
    p.tick(now_ms=5_000)
    assert p.next_message() is None


def test_ais_reassembly_out_of_order_raises() -> None:
    # Feed part 2 of a 2-fragment message without part 1 — the
    # reassembler emits ReassemblyError (subclass of AisError). Strict
    # iteration surfaces it; lenient would swallow.
    frag2 = aivdm(2, 2, 1, "A", b"XXXXXXX", 0)
    p = AisParser.streaming()
    p.feed(frag2)
    with pytest.raises(ReassemblyError):
        list(p.iter(strict=True))
