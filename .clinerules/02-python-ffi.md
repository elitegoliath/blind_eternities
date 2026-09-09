---
description: Python bindings, local AI orchestration, and FFI bridging
globs: ["**/*.py", "requirements.txt"]
alwaysApply: false
---

# Python FFI and AI Integration

The Python layer serves as the orchestrator, bridging local AI models with the Rust-compiled rules engine.

## Interoperability

* **Type Safety:** Ensure all data crossing the boundary uses strict typing. Use Pydantic models in Python that directly map to the serialized Rust struct representations.
* **FFI Boundaries:** When calling Rust from Python, ensure memory is safely handled and avoid leaking pointers. 
* **AI Orchestration:** The Python layer handles prompting the local AI. The AI may propose moves, but the Rust engine validates them. Python must gracefully handle and log illegal move rejections from Rust.