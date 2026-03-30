"""Engine wrapper with stable signatures."""

from __future__ import annotations

import json
from typing import Any, Dict, Mapping, Optional, Union

from .types import Script


class Engine:
    """Python wrapper around the native VN engine.

    Args:
        script_json: Stable JSON representation of the script.
    """

    def __init__(self, script_json: str) -> None:
        self._engine = _load_native_engine()(script_json)
        self._last_audio: Any = []

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

        result = self._engine.step()
        if hasattr(result, "event"):
            self._last_audio = getattr(result, "audio", [])
            return result.event
        self._last_audio = []
        return result

    def choose(self, option_index: int) -> Dict[str, Any]:
        """Apply a choice selection and return the choice event."""

        return self._engine.choose(option_index)

    def register_handler(self, callback: Any) -> None:
        """Register a native ext-call callback, if exposed by the binding."""

        if not hasattr(self._engine, "register_handler"):
            raise RuntimeError(
                "Native engine module does not provide callback bindings"
            )
        self._engine.register_handler(callback)

    def allow_ext_call_command(self, command: str) -> None:
        """Allow a single ext-call command for callback dispatch."""

        if not hasattr(self._engine, "allow_ext_call_command"):
            raise RuntimeError(
                "Native engine module does not provide ext-call capability bindings"
            )
        self._engine.allow_ext_call_command(command)

    def clear_ext_call_capabilities(self) -> None:
        """Clear the ext-call capability allowlist."""

        if not hasattr(self._engine, "clear_ext_call_capabilities"):
            raise RuntimeError(
                "Native engine module does not provide ext-call capability bindings"
            )
        self._engine.clear_ext_call_capabilities()

    def last_ext_call_error(self) -> Optional[str]:
        """Return the last ext-call dispatch error, if any."""

        if not hasattr(self._engine, "last_ext_call_error"):
            raise RuntimeError("Native engine module does not provide error tracking")
        return self._engine.last_ext_call_error()

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

    def is_current_dialogue_read(self) -> bool:
        """Return whether the current dialogue event was already shown in this session."""

        if not hasattr(self._engine, "is_current_dialogue_read"):
            raise RuntimeError(
                "Native engine module does not provide read-tracking bindings"
            )
        return bool(self._engine.is_current_dialogue_read())

    def choice_history(self) -> Any:
        """Return recorded choice decisions for the current engine session."""

        if not hasattr(self._engine, "choice_history"):
            raise RuntimeError(
                "Native engine module does not provide choice-history bindings"
            )
        return self._engine.choice_history()

    def supported_event_types(self) -> Any:
        """Return event types supported by the native runtime binding."""

        if hasattr(self._engine, "supported_event_types"):
            return self._engine.supported_event_types()
        # Conservative fallback for very old native modules.
        return ["dialogue", "choice", "scene", "jump", "set_flag"]

    def set_prefetch_depth(self, depth: int) -> None:
        """Configure lookahead depth used by native prefetch hints."""

        if not hasattr(self._engine, "set_prefetch_depth"):
            raise RuntimeError("Native engine module does not provide prefetch API")
        self._engine.set_prefetch_depth(depth)

    def prefetch_assets_hint(self) -> Any:
        """Return upcoming asset paths suggested for prefetching."""

        if hasattr(self._engine, "prefetch_assets_hint"):
            return self._engine.prefetch_assets_hint()
        return []

    def last_audio_commands(self) -> Any:
        """Return the audio commands emitted by the last `step()` call."""

        return list(self._last_audio)

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
