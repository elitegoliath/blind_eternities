from typing import List, Dict, Optional, Any, Union, Literal
from pydantic import BaseModel, Field


class Card(BaseModel):
    name: str
    type_line: List[str]
    mana_cost: str
    oracle_text: str
    effects: List[Any] = []


class Target(BaseModel):
    type: str
    id: Optional[str] = None


class StackObject(BaseModel):
    id: str
    card: Card
    controller: str
    targets: List[Target] = []
    source_id: Optional[str] = None


class Permanent(BaseModel):
    id: str
    name: str
    types: List[str]
    controller: str
    owner: str
    is_tapped: bool = False
    power: int = 0
    toughness: int = 0
    base_power: int = 0
    base_toughness: int = 0
    damage_marked: int = 0
    counters: Dict[str, int] = {}
    oracle_text: str = ""


class CastSpellPayload(BaseModel):
    card: Card
    targets: List[Target] = Field(default_factory=list)


class CastSpellAction(BaseModel):
    type: Literal["CastSpell"] = "CastSpell"
    payload: CastSpellPayload


class PlayLandAction(BaseModel):
    type: Literal["PlayLand"] = "PlayLand"
    payload: Card


class ActivateAbilityPayload(BaseModel):
    source_id: str
    ability_index: int
    targets: List[Target] = Field(default_factory=list)


class ActivateAbilityAction(BaseModel):
    type: Literal["ActivateAbility"] = "ActivateAbility"
    payload: ActivateAbilityPayload


class DeclareAttackersPayload(BaseModel):
    attackers: List[str]


class DeclareAttackersAction(BaseModel):
    type: Literal["DeclareAttackers"] = "DeclareAttackers"
    payload: DeclareAttackersPayload


class DeclareBlockersPayload(BaseModel):
    blockers: Dict[str, List[str]]


class DeclareBlockersAction(BaseModel):
    type: Literal["DeclareBlockers"] = "DeclareBlockers"
    payload: DeclareBlockersPayload


class PassPriorityAction(BaseModel):
    type: Literal["PassPriority"] = "PassPriority"


Action = Union[
    CastSpellAction,
    PlayLandAction,
    ActivateAbilityAction,
    DeclareAttackersAction,
    DeclareBlockersAction,
    PassPriorityAction,
]


class GameState(BaseModel):
    active_player: str
    priority_player: str
    turn_number: int
    phase: str
    step: Optional[str] = None
    stack: List[StackObject] = Field(default_factory=list)
    battlefield: List[Permanent] = Field(default_factory=list)
    graveyard: List[Card] = Field(default_factory=list)
    exile: List[Card] = Field(default_factory=list)
    hand: Dict[str, List[Card]] = Field(default_factory=dict)
    life_totals: Dict[str, int] = Field(default_factory=dict)
    mana_pool: Dict[str, Dict[str, int]] = Field(default_factory=dict)
    continuous_effects: List[Any] = Field(default_factory=list)
    pending_action: Optional[Action] = None


class EngineResponse(BaseModel):
    success: bool
    state: Optional[GameState] = None
    message: Optional[str] = None
    error: Optional[str] = None
    logs: List[str] = Field(default_factory=list)
