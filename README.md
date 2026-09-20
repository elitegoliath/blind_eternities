# Blind Eternities - Open Core

Blind Eternities is an open-source, stateless, and headless MTG (Magic: The Gathering) Rules Engine and AI Judge Orchestrator. Designed to be completely client-agnostic, the core engine relies on a strict FFI boundary and structural abstractions to act as the ultimate source of truth for MTG game states, interactions, and priority.

## 🏗️ Architecture

The Open Core operates across a 4-tier pipeline:

1. **Rust (Rules Engine):** Compiled via PyO3, the Rust library (`mtg_logic_core`) contains the strict invariants for the MTG State Machine. It manages the LIFO stack, Layer calculation (CR 613), Target Validation (CR 114 / CR 608.2b), State-Based Actions (SBAs), and returns typed `EngineError` permutations upon illegal moves.
2. **Pydantic/PyO3 (FFI Boundary):** Python and Rust communicate entirely via sanitized, stateless JSON payloads deserialized into strict Structs/Models, ensuring no memory leaks or proprietary front-end state is entangled with the core rules.
3. **Python/LangGraph (AI Orchestration):** Using local LLMs (Qwen 2.5 via Ollama) and a LangGraph tool-calling loop, user natural language input is routed using NLP classification (`DEFINITION`, `INTERACTION`, `SIMULATION`). The AI determines the user's intent, interacts with an offline Vector/SQLite RAG pipeline (LanceDB + FastEmbed) for rule synthesis, and transforms moves into rigid JSON actions for the engine to execute.
4. **FastAPI (WebSockets):** The application exposes persistent WebSocket endpoints that stream `PlayerPerspective` (hidden information sanitized) game states and chat messages directly to external clients (like a React Native frontend).

## 🚀 Getting Started (Docker Compose)

The easiest way to boot the entire ecosystem (FastAPI Backend, SQLite Cache, LanceDB Vector Store, and the Local Ollama LLM) is using Docker Compose.

Ensure you have Docker and Docker Compose installed.

```bash
# Boot the entire infrastructure
docker-compose up --build
```

The system will spin up:

- The **FastAPI** web server (Port: `8000`)
- The **Ollama** LLM container (Port: `11434`)

Wait until the Ollama container successfully pulls the `qwen2.5:7b` model and starts accepting requests. You can then connect a WebSocket client to `ws://localhost:8000/ws/{session_id}/{player_id}` to interact with the engine.

## 🤝 Contribution Guidelines

This repository represents the **Open-Source Core**.

- Never add proprietary monetization or cloud analytics directly to the `rust_core` or `python_agent` packages. 
- Build via structural plugins, injecting proprietary dependencies via the DI (Dependency Injection) containers provided in the python abstractions.

## ⚖️ Legal Disclaimer

Blind Eternities is unofficial Fan Content permitted under the Fan Content Policy. Not approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the Coast. ©Wizards of the Coast LLC.