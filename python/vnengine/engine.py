"""Engine wrapper with stable signatures."""

from __future__ import annotations

import json
from typing import Any, Dict, Mapping, Union

from .types import Script


class Engine:
    """Python wrapper around the native VN engine.

    Args:
        script_json: Stable JSON representation of the script.
    """

    def __init__(self, script_json: str) -> None:
        self._engine = _load_native_engine()(script_json)

    @classmethod
    def from_script(cls, script: Union[Script, Mapping[str, Any], str]) -> "Engine":
        """Create an engine from a Script object, dict, or JSON string."""

        if isinstance(script, Script):
            return cls(script.to_json())
        if isinstance(script, str):
            return cls(script)
        return cls(json.dumps(script, separators=(",", ":"), sort_keys=True))

    def current_event(self) -> Dict[str, Any]:
        """Return the current event as a Python dict."""

        return self._engine.current_event()

    def step(self) -> Dict[str, Any]:
        """Advance the engine and return the event that was processed."""

        return self._engine.step()

    def choose(self, option_index: int) -> Dict[str, Any]:
        """Apply a choice selection and return the choice event."""

        return self._engine.choose(option_index)

    def current_event_json(self) -> str:
        """Return the current event in stable JSON form."""

        return self._engine.current_event_json()

    def visual_state(self) -> Dict[str, Any]:
        """Return the current visual state as a Python dict."""

        return self._engine.visual_state()

    def ui_state(self) -> Dict[str, Any]:
        """Return the current UI state as a Python dict."""

        if not hasattr(self._engine, "ui_state"):
            raise RuntimeError("Native engine module does not provide ui_state")
        return self._engine.ui_state()

    @property
    def raw(self) -> Any:
        """Return the underlying native engine instance."""

        return self._engine


def _load_native_engine() -> Any:
    try:
        import visual_novel_engine as native
    except ImportError as exc:  # pragma: no cover - optional dependency
        raise RuntimeError("Native engine module not available") from exc

    if hasattr(native, "Engine"):
        return native.Engine
    if hasattr(native, "PyEngine"):
        return native.PyEngine
    raise RuntimeError("Native engine module does not provide Engine bindings")
