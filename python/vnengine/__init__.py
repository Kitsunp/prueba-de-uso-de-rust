"""Stable Python interface for the Visual Novel Engine."""

from .app import EngineApp
from .builder import ScriptBuilder
from .engine import Engine
from .types import (
    CharacterPatch,
    CharacterPlacement,
    Choice,
    ChoiceOption,
    CondFlag,
    CondVarCmp,
    Dialogue,
    Event,
    Jump,
    JumpIf,
    Patch,
    Scene,
    Script,
    SCRIPT_SCHEMA_VERSION,
    SetFlag,
    SetVar,
)

__all__ = [
    "CharacterPatch",
    "CharacterPlacement",
    "Choice",
    "ChoiceOption",
    "CondFlag",
    "CondVarCmp",
    "Dialogue",
    "Engine",
    "EngineApp",
    "Event",
    "Jump",
    "JumpIf",
    "Patch",
    "Scene",
    "Script",
    "SCRIPT_SCHEMA_VERSION",
    "ScriptBuilder",
    "SetFlag",
    "SetVar",
]
