---
description: Magic: The Gathering rules engine invariants and Rust core logic
globs: ["**/*.rs", "Cargo.toml"]
alwaysApply: false
---

# Blind Eternities: Rust Core Engine

The Rust core handles all state management, priority passes, and rule enforcement for the MTG engine.

## Core Invariants

* **Strict State Ownership:** The engine state must be immutable once the stack resolves. Use strict struct definitions for game zones (Library, Hand, Battlefield, Graveyard, Exile, Command).
* **Priority Passes:** Priority must explicitly pass between the Active Player and Non-Active Player before any spell or ability on the stack resolves.
* **The Stack:** Implement the stack as a strict LIFO (Last In, First Out) queue.
* **Error Handling:** Never use `unwrap()` or `expect()` in core logic. Always bubble up `Result<T, EngineError>` so the Python wrapper can catch and display errors gracefully.

## Architecture

* The local AI integration handles card parsing and strategic decisions, but the Rust core is the absolute source of truth for move legality.