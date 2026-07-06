"""marlin.klv — MISB ST 0601 (UAS Datalink Local Set) KLV encoder/decoder."""

from .. import _core

KlvError: type[Exception] = _core.KlvError
St0601 = _core.klv.St0601
decode = _core.klv.decode
encode = _core.klv.encode
precision_timestamp = _core.klv.precision_timestamp

__all__ = ["KlvError", "St0601", "decode", "encode", "precision_timestamp"]
