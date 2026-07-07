"""marlin.klv — MISB ST 0601 (UAS Datalink Local Set) KLV encoder/decoder."""

from .. import _core

KlvError: type[Exception] = _core.KlvError
St0601 = _core.klv.St0601
TagInfo = _core.klv.TagInfo
UAS_LS_KEY: bytes = _core.klv.UAS_LS_KEY
decode = _core.klv.decode
encode = _core.klv.encode
precision_timestamp = _core.klv.precision_timestamp
tags = _core.klv.tags
tag_number = _core.klv.tag_number
tag_name = _core.klv.tag_name

__all__ = [
    "UAS_LS_KEY",
    "KlvError",
    "St0601",
    "TagInfo",
    "decode",
    "encode",
    "precision_timestamp",
    "tag_name",
    "tag_number",
    "tags",
]
