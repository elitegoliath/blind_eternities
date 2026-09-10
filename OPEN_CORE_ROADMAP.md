# Open Core Refactoring Roadmap

## Phase 1: Python Agent Abstractions & API Readiness (React Native Prep)
- [ ] **Decouple I/O**: Remove CLI-specific `input()` and `print()` from the core LangGraph agent. Refactor to a generator/async stream that yields structured events.
- [ ] **State Management DI**: Extract `game_session.json` into a `StateStoreBackend` protocol keyed by `session_id` (crucial for React Native stateless client requests).
- [ ] **LLM Adapter Pattern**: Move `qwen_tool_interceptor` into a generic `LLMInterceptor` base class.
- [ ] **Middleware Hooks**: Add `pre_action` and `post_action` hooks to the workflow.

## Phase 2: Rust Core Extensibility
- [ ] **Dynamic Rules Engine**: Refactor `Judge` from static methods to an instantiable struct evaluating `Box<dyn RuleValidator>`.
- [ ] **Enum Expansion**: Add `Custom(serde_json::Value)` fallback variants to Enums for proprietary frontend payloads.
- [ ] **Configurable SBAs**: Move hardcoded SBA logic into default plugins.

## Phase 3: FFI & Transport Boundaries
- [ ] **Client-Agnostic API Wrapper**: Create a FastAPI / WebSocket template around the Python agent that a React Native client can seamlessly connect to.
- [ ] **Plugin Injection**: Update PyO3 bindings to allow Python to construct specific active rulesets.
