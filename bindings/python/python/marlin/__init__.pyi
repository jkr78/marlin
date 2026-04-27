"""Type stubs for marlin — NMEA 0183 + AIS parser (Rust-backed)."""

__version__: str

class MarlinError(Exception): ...

__all__ = ["__version__", "MarlinError"]
