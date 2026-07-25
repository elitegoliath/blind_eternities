# python_agent/llm_engine.py
# This file sets up the LLM and prompt templates for the Magic: The Gathering Judge Agent.
# Crucially, it defines the System Prompt - the instructions that stop the LLM from being a
# helpful assistant and force it to be a strict Rules Lawyer.
# We use lru_cache here so that if you call get_llm() multiple times in a session,
# it doesn't re-initialize the connection object every time.

# Example prompt:
# I have an Urza, Lord High Artificer on the battlefield. I cast a second Urza, Lord High Artificer. What happens?

import os
from functools import lru_cache
from langchain_openai import ChatOpenAI
from langchain_core.prompts import ChatPromptTemplate, MessagesPlaceholder

# --- The Persona ---
# This is where we prompt-engineer the "Judge" behavior.
SYSTEM_PROMPT = """
You are the Blind Eternities Magic: The Gathering Rules Engine Agent...

GUIDELINES:
1. DO NOT GUESS. If you are unsure of a specific interaction, use the available tools to query the Rust engine.
2. CITATIONS REQUIRED. Whenever you declare a move legal or illegal, you must cite the relevant CR rule number or interaction layer if known.
3. TONE. Be precise, concise, and professional. Avoid conversational filler.
4. LAYERS. When discussing continuous effects (Opalescence, Humility), explicitly mention which Layer (1-7) applies.

If the user provides a JSON payload or card name, pass it to your verification tools immediately.

CRITICAL OPERATIONAL RULES:
1. IMPLIED MANA: If the player asks about casting a spell or activating an ability but does NOT explicitly list their available mana pool, assume they have the exact mana required to pay for it. Do not invent a restricted mana pool. In your tool call, you can omit the mana pool or populate it with the exact cost of the card.
2. CARD ACCURACY: Always verify or assume the correct oracle mana cost of a card (e.g., 'Grizzly Bears' costs {1}{G}, not {2}{G}).
3. ENGINE SUPREMACY: You MUST report every single item returned in the tool's ruling array to the user. 
   - If the engine returns a "status": "sba_trigger", you must explicitly tell the user that a State-Based Action occurs, and explain the rule associated with the "action" field (e.g., if it says "Legend Rule", explain that they must choose one and put the rest into the graveyard).
   - Never ignore an SBA trigger, even if the pending action is "legal".
"""

@lru_cache(maxsize=1)
def get_llm(temperature: float = 0.0) -> ChatOpenAI:
    """
    Returns a configured LLM instance. 
    Cached to prevent re-initialization overhead during high-throughput testing.
    """

    model_name = os.getenv("LLM_MODEL_NAME", "llama3.1")

    # Local runners usually ignore the API key, but LangChain/OpenAI SDK 
    # still require the variable to be populated with a string.
    api_key = os.getenv("OPENAI_API_KEY", "not-needed-for-local")

    # Set this to the port your host runner (Ollama, LM Studio, vLLM) is using.
    # E.g., 11434 for Ollama, 1234 for LM Studio.
    local_base_url = os.getenv("LLM_BASE_URL", "http://llm-engine:11434/v1")

    print(f"[DEBUG] 🧠 Connecting to local LLM at {local_base_url}")

    return ChatOpenAI(
        model=model_name,
        base_url=local_base_url,
        api_key=api_key,
        temperature=temperature, # Keep at 0 for deterministic rule evaluation
        streaming=True,          # Better UX for long explanations
    )

def get_prompt_template() -> ChatPromptTemplate:
    """
    Constructs the chat history structure for the agent.
    """
    return ChatPromptTemplate.from_messages([
        ("system", SYSTEM_PROMPT),
        MessagesPlaceholder(variable_name="chat_history"), # Memory injection point
        ("human", "{input}"),
        MessagesPlaceholder(variable_name="agent_scratchpad"), # Thinking space for ReAct
    ])

# Why this design?
# Temperature 0.0: Standard chatbots use 0.7 for creativity. An MTG Judge must use 0.0 because the rules are deterministic. We don't want "creative" interpretations of the stack.
# Prompt Separation: By keeping SYSTEM_PROMPT here, you can iterate on your instructions (e.g., "Add a rule to always check for state-based actions") without touching the main.py code.
# Dependency Injection: get_llm() allows you to easily swap gpt-4-turbo for claude-3-opus or a local llama-3 later just by changing one line in this file.