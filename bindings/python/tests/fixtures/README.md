# Test fixtures

Copied verbatim from `crates/marlin-nmea-envelope/tests/fixtures/` at the
commit the binding was built from. If you need to sync changes:

    cp crates/marlin-nmea-envelope/tests/fixtures/*.nmea \
       bindings/python/tests/fixtures/envelope/

These are raw NMEA byte streams reused by three golden-test files
(`tests/golden/test_{envelope,nmea,ais}_fixtures.py`), each verifying a
different layer of the parsing stack against the same input.
