"""Type stubs for the `marlin._core` extension module.

Private. The public stubs live beside each package `__init__.py`; the
submodules are `Any` here so those re-export files type-check.
"""

from typing import Any

__version__: str

class MarlinError(Exception): ...
class EnvelopeError(MarlinError):
    variant: str

class DecodeError(MarlinError): ...
class AisError(MarlinError): ...
class ReassemblyError(AisError): ...
class KlvError(MarlinError):
    variant: str

class KlvEncodeError(KlvError):
    tag: int
    kind: str | None

envelope: Any
field: Any
nmea: Any
ais: Any
klv: Any
