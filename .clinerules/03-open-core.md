# Open-Core Boundaries and Proprietary Isolation

1. **No Proprietary Logic in Core:** Never add monetization, multi-tenant cloud logic, or proprietary analytics directly to `rust_core` or `python_agent`. These directories represent the open-source foundation.
2. **Depend on Abstractions:** Python components must use Protocols (interfaces) for state management and LLM interactions. Rust components must use traits (e.g., `RuleValidator`) for game logic.
3. **Closed for Modification, Open for Extension:** When a new MTG format or custom proprietary rule is needed, do NOT modify `rules.rs`. Instead, create a new implementation of the `RuleValidator` trait and inject it.
4. **FFI Purity:** The Rust-Python boundary must only exchange strict, agnostic JSON data payloads. Never pass cloud-specific tokens or user PII through the `mtg_logic_core` FFI.
5. **Graceful Fallbacks:** The open-core must remain fully functional locally out-of-the-box. If proprietary plugins are not injected, the engine must gracefully default to the standard open-source ruleset and local `game_session.json` storage.
6. **Client-Agnostic Architecture (React Native Ready):** The core engine must never assume its execution environment. Never use `print()` for crucial engine outputs or `input()` for state gathering in core libraries. All outputs must be structured events capable of being serialized over WebSockets or REST APIs to external clients like React Native.
