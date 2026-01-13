"""Script builder with stable, documented signatures."""

from __future__ import annotations

from typing import Dict, Iterable, List, Optional, Tuple, Union

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
    SetFlag,
    SetVar,
    normalize_character_patches,
    normalize_characters,
    normalize_choice_options,
)

ChoiceOptionInput = Union[ChoiceOption, Tuple[str, str]]
CharacterInput = Union[Tuple[str, Optional[str], Optional[str]], CharacterPlacement]
CharacterPatchInput = Union[Tuple[str, Optional[str], Optional[str]], CharacterPatch]


class ScriptBuilder:
    """Incrementally build a script with stable serialization.

    Labels are tracked in insertion order and serialized in sorted order to keep
    JSON output stable across runs.
    """

    def __init__(self) -> None:
        self._events: List[Event] = []
        self._labels: Dict[str, int] = {}

    @property
    def events(self) -> List[Event]:
        """Current list of events (read-only snapshot)."""

        return list(self._events)

    @property
    def labels(self) -> Dict[str, int]:
        """Current label map (read-only snapshot)."""

        return dict(self._labels)

    def label(self, name: str) -> None:
        """Record a label at the current event index."""

        self._labels[name] = len(self._events)

    def add_event(self, event: Event) -> None:
        """Append a pre-built event object."""

        self._events.append(event)

    def dialogue(self, speaker: str, text: str) -> None:
        """Append a dialogue event."""

        self._events.append(Dialogue(speaker=speaker, text=text))

    def choice(self, prompt: str, options: Iterable[ChoiceOptionInput]) -> None:
        """Append a choice event."""

        normalized = normalize_choice_options(options)
        self._events.append(Choice(prompt=prompt, options=normalized))

    def scene(
        self,
        background: Optional[str] = None,
        music: Optional[str] = None,
        characters: Iterable[CharacterInput] = (),
    ) -> None:
        """Append a scene update event."""

        normalized = normalize_characters(characters)
        self._events.append(
            Scene(background=background, music=music, characters=normalized)
        )

    def jump(self, target: str) -> None:
        """Append a jump event."""

        self._events.append(Jump(target=target))

    def set_flag(self, key: str, value: bool) -> None:
        """Append a set-flag event."""

        self._events.append(SetFlag(key=key, value=value))

    def set_var(self, key: str, value: int) -> None:
        """Append a set-var event."""

        self._events.append(SetVar(key=key, value=value))

    def jump_if_flag(self, key: str, is_set: bool, target: str) -> None:
        """Append a conditional jump on a flag."""

        self._events.append(JumpIf(cond=CondFlag(key=key, is_set=is_set), target=target))

    def jump_if_var(self, key: str, op: str, value: int, target: str) -> None:
        """Append a conditional jump on a variable comparison."""

        self._events.append(
            JumpIf(cond=CondVarCmp(key=key, op=op, value=value), target=target)
        )

    def patch(
        self,
        background: Optional[str] = None,
        music: Optional[str] = None,
        add: Iterable[CharacterInput] = (),
        update: Iterable[CharacterPatchInput] = (),
        remove: Iterable[str] = (),
    ) -> None:
        """Append a scene patch event."""

        normalized_add = normalize_characters(add)
        normalized_update = normalize_character_patches(update)
        self._events.append(
            Patch(
                background=background,
                music=music,
                add=normalized_add,
                update=normalized_update,
                remove=list(remove),
            )
        )

    def build(self) -> Script:
        """Finalize and return a Script object."""

        return Script(events=list(self._events), labels=dict(self._labels))

    def to_dict(self) -> Dict[str, object]:
        """Serialize the script into a stable dict."""

        return self.build().to_dict()

    def to_json(self) -> str:
        """Serialize the script into stable JSON."""

        return self.build().to_json()
