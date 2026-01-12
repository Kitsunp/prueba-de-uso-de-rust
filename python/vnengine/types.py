"""Typed, stable serialization helpers for VN scripts."""

from __future__ import annotations

from dataclasses import dataclass, field
import json
from typing import Any, Dict, Iterable, List, Mapping, Optional, Sequence, Tuple, Union


@dataclass(frozen=True)
class Dialogue:
    """Dialogue event.

    Args:
        speaker: Character name.
        text: Dialogue text.
    """

    speaker: str
    text: str

    def to_dict(self) -> Dict[str, Any]:
        return {"type": "dialogue", "speaker": self.speaker, "text": self.text}

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "Dialogue":
        return cls(speaker=str(data["speaker"]), text=str(data["text"]))


@dataclass(frozen=True)
class ChoiceOption:
    """Choice option entry."""

    text: str
    target: str

    def to_dict(self) -> Dict[str, Any]:
        return {"text": self.text, "target": self.target}

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "ChoiceOption":
        return cls(text=str(data["text"]), target=str(data["target"]))


@dataclass(frozen=True)
class Choice:
    """Choice event containing a prompt and options."""

    prompt: str
    options: List[ChoiceOption] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "type": "choice",
            "prompt": self.prompt,
            "options": [option.to_dict() for option in self.options],
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "Choice":
        options = [ChoiceOption.from_dict(item) for item in data.get("options", [])]
        return cls(prompt=str(data["prompt"]), options=options)


@dataclass(frozen=True)
class CharacterPlacement:
    """Character placement in a scene update."""

    name: str
    expression: Optional[str] = None
    position: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "expression": self.expression,
            "position": self.position,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "CharacterPlacement":
        return cls(
            name=str(data["name"]),
            expression=data.get("expression"),
            position=data.get("position"),
        )


@dataclass(frozen=True)
class Scene:
    """Scene update event."""

    background: Optional[str] = None
    music: Optional[str] = None
    characters: List[CharacterPlacement] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "type": "scene",
            "background": self.background,
            "music": self.music,
            "characters": [character.to_dict() for character in self.characters],
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "Scene":
        characters = [
            CharacterPlacement.from_dict(item) for item in data.get("characters", [])
        ]
        return cls(
            background=data.get("background"),
            music=data.get("music"),
            characters=characters,
        )


@dataclass(frozen=True)
class Jump:
    """Jump event."""

    target: str

    def to_dict(self) -> Dict[str, Any]:
        return {"type": "jump", "target": self.target}

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "Jump":
        return cls(target=str(data["target"]))


@dataclass(frozen=True)
class SetFlag:
    """Set-flag event."""

    key: str
    value: bool

    def to_dict(self) -> Dict[str, Any]:
        return {"type": "set_flag", "key": self.key, "value": self.value}

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "SetFlag":
        return cls(key=str(data["key"]), value=bool(data["value"]))


Event = Union[Dialogue, Choice, Scene, Jump, SetFlag]


@dataclass(frozen=True)
class Script:
    """Script container with stable JSON serialization.

    Args:
        events: Ordered list of events.
        labels: Mapping from label name to event index.
    """

    events: List[Event] = field(default_factory=list)
    labels: Dict[str, int] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        ordered_labels = {key: self.labels[key] for key in sorted(self.labels)}
        return {
            "events": [event.to_dict() for event in self.events],
            "labels": ordered_labels,
        }

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), separators=(",", ":"), sort_keys=True)

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "Script":
        events = [event_from_dict(item) for item in data.get("events", [])]
        labels = {str(key): int(value) for key, value in data.get("labels", {}).items()}
        return cls(events=events, labels=labels)

    @classmethod
    def from_json(cls, raw: str) -> "Script":
        return cls.from_dict(json.loads(raw))


def event_from_dict(data: Mapping[str, Any]) -> Event:
    event_type = data.get("type")
    if event_type == "dialogue":
        return Dialogue.from_dict(data)
    if event_type == "choice":
        return Choice.from_dict(data)
    if event_type == "scene":
        return Scene.from_dict(data)
    if event_type == "jump":
        return Jump.from_dict(data)
    if event_type == "set_flag":
        return SetFlag.from_dict(data)
    raise ValueError(f"Unknown event type: {event_type}")


def normalize_choice_options(
    options: Iterable[Union[ChoiceOption, Tuple[str, str]]],
) -> List[ChoiceOption]:
    normalized: List[ChoiceOption] = []
    for option in options:
        if isinstance(option, ChoiceOption):
            normalized.append(option)
        else:
            text, target = option
            normalized.append(ChoiceOption(text=text, target=target))
    return normalized


def normalize_characters(
    characters: Iterable[Union[CharacterPlacement, Tuple[str, Optional[str], Optional[str]]]],
) -> List[CharacterPlacement]:
    normalized: List[CharacterPlacement] = []
    for character in characters:
        if isinstance(character, CharacterPlacement):
            normalized.append(character)
        else:
            name, expression, position = character
            normalized.append(
                CharacterPlacement(
                    name=name, expression=expression, position=position
                )
            )
    return normalized
