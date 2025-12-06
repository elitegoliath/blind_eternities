# Blind Eternities

**A Retrieval-Augmented Rules (RAG) Engine for Magic: The Gathering**

## Architecture

This project uses a hybrid **Python + Rust** architecture. Python handles the semantic reasoning (LLM), while a compiled Rust core enforces the strict game state and handles high-performance vector retrieval.

```mermaid
    graph TD
    %% Styling Definitions
    classDef python fill:#3776ab,stroke:#fff,stroke-width:2px,color:#fff;
    classDef rust fill:#dea584,stroke:#fff,stroke-width:2px,color:#000;
    classDef data fill:#444,stroke:#fff,stroke-width:2px,color:#fff;
    classDef user fill:#fff,stroke:#333,stroke-width:1px,color:#000;

    user([User / Developer]) -->|Natural Language Query| Agent
    
    subgraph "Python Land (The Orchestrator)"
        Agent[Agent Runtime<br/><i>LangChain + Pydantic</i>]:::python
        LLM[LLM Interface<br/><i>OpenAI / Anthropic</i>]:::python
        Agent <-->|Context & Reasoning| LLM
    end

    Agent <==>|FFI / PyO3 Bridge<br/><i>Zero-Copy Data Transfer</i>| Core
    
    subgraph "Rust Land (The Judge)"
        Core[Compiled Extension<br/><i>mtg_logic_core.so</i>]:::rust
        Rules[Rules Engine<br/><i>State Machine & Layers</i>]:::rust
        Ingest[Data Ingestion<br/><i>Streaming Parser</i>]:::rust
        
        Core --> Rules
        Ingest --> DB
    end

    subgraph "Persistence Layer"
        DB[(LanceDB / JSON<br/><i>Vector Store & Rules</i>)]:::data
        Rules <-->|High-Speed Lookup| DB
    end

    %% Legend / Connectors
    Ingest -.->|Periodic Updates| Scryfall(Scryfall API):::user
```

## Disclaimer

Unofficial Fan Content Policy This project is unofficial Fan Content permitted under the Fan Content Policy. Not approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the Coast. ©Wizards of the Coast LLC.
