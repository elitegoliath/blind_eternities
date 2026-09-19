# Blind Eternities: API & WebSocket Contract

The Blind Eternities Open Core acts as a stateless WebSocket API backend. This document defines the payload structures external clients (e.g. React Native) should expect when interacting with the API.

## 🔌 WebSocket Connection

**Endpoint:** `ws://<host>:8000/ws/{session_id}/{player_id}`

Upon connecting, the engine will immediately broadcast the sanitized initial game state to the client.

## 📥 Incoming Actions (From Client to Server)

The client sends JSON strings. These can either be natural language queries (for the AI to process) or strict Action payloads.

### Natural Language Query
```json
{
  "query": "Cast Lightning Bolt targeting Grizzly Bears"
}
```

### Strict Action Payload (Simulation)
```json
{
  "type": "PassPriority"
}
```

```json
{
  "type": "CastSpell",
  "payload": {
    "card": {
      "name": "Lightning Bolt",
      "type_line": ["Instant"],
      "mana_cost": "{R}",
      "oracle_text": "Lightning Bolt deals 3 damage to any target.",
      "effects": [{"type": "DealDamage", "amount": 3}]
    },
    "targets": [{"type": "Permanent", "id": "bear-1"}],
    "targeting_requirements": [{"type": "Any"}]
  }
}
```

## 📤 Outgoing Payloads (From Server to Client)

The server broadcasts structured JSON payloads back to the connected sockets. Check the `"type"` field to route it in your frontend.

### 1. Game State (`"type": "game_state"`)
The engine pushes a heavily sanitized `PlayerPerspective` view. Notice that the opponent's hand is masked as `"Hidden Card"`.

```json
{
  "type": "game_state",
  "state": {
    "active_player": "player_1",
    "priority_player": "player_2",
    "turn_number": 1,
    "phase": "Main Phase 1",
    "stack": [],
    "battlefield": [
      {
        "id": "bear-1",
        "name": "Grizzly Bears",
        "controller": "player_1",
        "is_tapped": false,
        "damage_marked": 0,
        "base_characteristics": {"types": ["Creature"], "power": 2, "toughness": 2},
        "current_characteristics": {"types": ["Creature"], "power": 2, "toughness": 2}
      }
    ],
    "hand": {
      "player_1": [{"name": "Lightning Bolt"}],
      "player_2": [{"name": "Hidden Card"}]
    },
    "life_totals": {"player_1": 20, "player_2": 20},
    "consecutive_passes": 1
  }
}
```

### 2. Engine Rejection / Error (`"type": "error"`)
If a strict action fails the Rust engine validation (e.g., trying to cast a Sorcery on an opponent's turn), it returns an `EngineError` payload.

```json
{
  "type": "error",
  "error": {
    "type": "OutOfPhase",
    "details": "Wrong Phase"
  }
}
```

### 3. AI Chat Message (`"type": "message"`)
When the AI orchestrator synthesizes definitions, interactions, or translates strict engine errors into conversational explanations.

```json
{
  "type": "message",
  "content": "You cannot cast that Sorcery right now because it's your opponent's turn!"
}
```
