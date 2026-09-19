# Blind Eternities - Open Core Roadmap

## Completed Milestones (v1.0.0)

### Phase 1: Python Agent Abstractions & API Readiness (React Native Prep)
- [x] **Decouple I/O**: Remove CLI-specific `input()` and `print()` from the core LangGraph agent. Refactor to a generator/async stream that yields structured events.
- [x] **State Management DI**: Extract `game_session.json` into a `StateStoreBackend` protocol keyed by `session_id` (crucial for React Native stateless client requests).
- [x] **LLM Adapter Pattern**: Move `qwen_tool_interceptor` into a generic `LLMInterceptor` base class.
- [x] **Middleware Hooks**: Add `pre_action` and `post_action` hooks to the workflow.

### Phase 2: Rust Core Extensibility & FSM
- [x] **Dynamic Rules Engine**: Refactor `Judge` from static methods to an instantiable struct evaluating `Box<dyn RuleValidator>`.
- [x] **Enum Expansion**: Add `Custom(serde_json::Value)` fallback variants to Enums for proprietary frontend payloads.
- [x] **Configurable SBAs**: Move hardcoded SBA logic into default plugins.
- [x] **Priority & SBA Loop (CR 117.5)**: Decouple SBAs from FFI boundaries and correctly embed them into the core priority loop.
- [x] **Triggered Abilities (CR 603)**: Scaffold GameEvent emitters, Pending Trigger queues, and APNAP stack resolution.

### Phase 3: FFI & Transport Boundaries
- [x] **Client-Agnostic API Wrapper**: Create a FastAPI / WebSocket template around the Python agent that a React Native client can seamlessly connect to.
- [x] **Plugin Injection**: Update PyO3 bindings to allow Python to construct specific active rulesets.

### Phase 4: NLP Orchestration & Tooling
- [x] **Intent Classification**: Create a router to classify user queries (`DEFINITION`, `INTERACTION`, `SIMULATION`) using Pydantic schemas.
- [x] **Action Translation**: Use structured LLM outputs to coerce natural language into strict Pydantic `Action` models (bridging to the Rust engine).
- [x] **Scryfall SQLite Integration**: Connect the `DEFINITION` pipeline branch to the local Scryfall offline cache.
- [ ] **Interactive Trigger Resolution**: Expose pending triggers over the FFI so the agent can prompt the LLM/User for target selection before placing them on the stack.

### Phase 6: Advanced Mechanics & Network Integrity
- [x] **State Sanitization**: Implement `PlayerPerspective` serialization in Rust to mask hidden zones (e.g., opponent's hand/library) before broadcasting over WebSockets.
- [x] **Target Validation & Resolution (CR 114)**: Build targeting requirements into stack objects, enforcing validation upon casting and handling the "fizzle" rule upon resolution.
- [x] **Continuous Effects (CR 613)**: Introduce a characteristic modifier pipeline ordered strictly by the 7 MTG Layers to dynamically calculate current object properties.
- [x] **Structured Error Bubbling**: Expose typed `EngineError` enums over FFI, allowing the Python LLM to translate strict engine rejections into user-friendly explanations.
- [x] **Interaction RAG Pipeline**: Combine vector search on the Comprehensive Rules (CR) with Scryfall rulings to resolve complex NLP "What happens when..." queries dynamically.

### Phase 7: Client App Integration (Upcoming)
- [x] **Frontend Connection**: Wire up the React Native frontend to the structured Python agent WebSocket.
- [x] **E2E Rules Testing**: Validate complex MTG interactions (e.g., Stack resolution, Counterspells, Simultaneous ETBs) completely end-to-end.

## Future Roadmap (v1.1.0+)

While the v1.0.0 core is feature-complete for standard rules interactions, the Magic: The Gathering ecosystem is vast. The following are planned areas of expansion:

- [ ] **Advanced Keyword Mechanics:** Implement specialized, historically complex set mechanics (e.g., Mutate, Banding, Bestow, and Companion rules).
- [ ] **Multiplayer Session Scaling:** Refactor the turn FSM and priority passes to natively support multi-opponent formats like Commander / EDH.
- [ ] **Engine Performance Optimizations:** Optimize the FFI serialization boundaries, introduce Rust-native async abstractions, and reduce memory footprints during complex stack resolutions.
- [ ] **Comprehensive Rules Update Automation:** Create a pipeline to automatically ingest and re-embed the MTG Comprehensive Rules whenever a new set releases.

## 🤝 Welcoming Community Contributions

The Blind Eternities Open Core thrives on community collaboration! If you're passionate about MTG rules engine development, AI orchestration, or Rust performance optimization, we would love your help tackling the v1.1.0+ roadmap. 

Whether it's implementing a niche card mechanic, optimizing LanceDB queries, or writing E2E tests, pull requests are warmly welcomed. Please check out the issues tab or join the community discussions to get started!
