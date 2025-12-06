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
from langchain_core.messages import SystemMessage

# --- The Persona ---
# This is where we prompt-engineer the "Judge" behavior.
SYSTEM_PROMPT = """
You are "The Judge," an automated rules engine for Magic: The Gathering.
Your goal is to provide strictly accurate rulings based on the Comprehensive Rules (CR).

GUIDELINES:
1. DO NOT GUESS. If you are unsure of a specific interaction, use the available tools to query the Rust engine.
2. CITATIONS REQUIRED. Whenever you declare a move legal or illegal, you must cite the relevant CR rule number or interaction layer if known.
3. TONE. Be precise, concise, and professional. Avoid conversational filler.
4. LAYERS. When discussing continuous effects (Opalescence, Humility), explicitly mention which Layer (1-7) applies.

If the user provides a JSON payload or card name, pass it to your verification tools immediately.
"""

@lru_cache(maxsize=1)
def get_llm(model_name: str = "gpt-5-nano", temperature: float = 0.0) -> ChatOpenAI:
    """
    Returns a configured LLM instance. 
    Cached to prevent re-initialization overhead during high-throughput testing.
    """
    
    # Fail fast if keys are missing
    if not os.getenv("OPENAI_API_KEY"):
        raise ValueError("OPENAI_API_KEY not found in environment variables.")

    return ChatOpenAI(
        model=model_name,
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