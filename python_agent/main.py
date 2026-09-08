# python_agent/main.py
# This file is the main entry point for the Python Agent that interacts with the LLM and tools.
import json
import ast

from typing import TypedDict, Annotated, Sequence

from dotenv import load_dotenv
from langchain_core.messages import BaseMessage, HumanMessage, SystemMessage
from langgraph.graph import StateGraph, END, add_messages
from langgraph.prebuilt import ToolNode
from langchain_core.messages import AIMessage
from langchain_core.messages.tool import ToolCall

# Import local modules
from python_agent.llm_engine import get_llm, SYSTEM_PROMPT
from python_agent.tools import fetch_card, cast_spell, pass_priority

load_dotenv()

def qwen_tool_interceptor(state: dict) -> dict:
    """
    Extracts tool calls buried inside conversational text, XML tags, or markdown.
    """
    msg_key = "messages" if "messages" in state else "chat_history"
    
    if msg_key not in state or not state[msg_key]:
        return state

    messages = state[msg_key]
    last_message = messages[-1]

    if isinstance(last_message, AIMessage) and not last_message.tool_calls and last_message.content:
        content_str = last_message.content.strip()
        
        # Find the boundaries of the dictionary, ignoring surrounding text/XML
        start_idx = content_str.find("{")
        end_idx = content_str.rfind("}")
        
        if start_idx != -1 and end_idx != -1:
            dict_str = content_str[start_idx:end_idx+1]
            
            parsed_dict = None
            try:
                # Try strict JSON first
                parsed_dict = json.loads(dict_str)
            except json.JSONDecodeError:
                try:
                    # Fall back to AST for single-quote hallucinations
                    parsed_dict = ast.literal_eval(dict_str)
                except (ValueError, SyntaxError):
                    pass
            
            if parsed_dict and "name" in parsed_dict and "arguments" in parsed_dict:
                print(f"\n[DEBUG] 🪝 Intercepted buried tool call: {parsed_dict['name']}")
                
                injected_tool_call = ToolCall(
                    name=parsed_dict["name"],
                    args=parsed_dict["arguments"],
                    id=f"call_{hash(dict_str) % 10000}" 
                )
                
                last_message.tool_calls = [injected_tool_call]
                last_message.content = "" 
                
                return {msg_key: messages}
                
    return {msg_key: messages}

# --- 1. Define the State ---
# This acts like a Redux store for the conversation.
# 'add_messages' tells the graph to append new messages rather than overwriting.
class AgentState(TypedDict):
    messages: Annotated[Sequence[BaseMessage], add_messages]

def main():
    print(">>> Initializing Blind Eternities Agent (Explicit StateGraph)...")
    
    # --- 2. Setup Resources ---
    # We bind the tools to the LLM so it knows it CAN use them.
    # tools = [
    #     Tool(
    #         name="The_Judge",
    #         func=validate_move,
    #         description="Checks Magic: The Gathering rule legality. Input: JSON string."
    #     )
    # ]
    tools = [fetch_card, cast_spell, pass_priority]
    llm = get_llm()
    # Ensure the LLM cannot attempt parallel tool execution, 
    # forcing it to wait for the result of the first tool before calling the next.
    llm_with_tools = llm.bind_tools(tools)
    tool_node = ToolNode(tools)

    # --- 3. Define Nodes (The Logic) ---
    
    def call_model(state: AgentState):
        """Node 1: The Brain. Decides what to do next."""
        # Get conversation history
        messages = state["messages"]
        # Invoke the LLM
        response = llm_with_tools.invoke([SystemMessage(content=SYSTEM_PROMPT)] + list(messages))
        # Return the new message to update state
        return {"messages": [response]}

    # Node 2: The Tools.
    # We use the prebuilt ToolNode because it's just a simple executor.
    tool_node = ToolNode(tools)

    # --- 4. Define Edges (The Flow Control) ---

    def should_continue(state: dict) -> str:
        """Routes to the tool node if a tool call exists, otherwise ends the loop."""
        # Find the correct key for your state
        msg_key = "messages" if "messages" in state else "chat_history"
        last_message = state[msg_key][-1]
        
        # If the LLM (or our interceptor) added tool calls, execute them
        if hasattr(last_message, "tool_calls") and last_message.tool_calls:
            return "action"
        
        # Otherwise, the response is finished
        return END

    # --- 5. Build the Graph ---
    # 1. Build the graph
    workflow = StateGraph(AgentState)

    # 2. Add the nodes
    workflow.add_node("agent", call_model)
    workflow.add_node("interceptor", qwen_tool_interceptor)
    workflow.add_node("action", tool_node)

    # 3. Set the entry point
    workflow.set_entry_point("agent")

    # 4. Route LLM output through the interceptor
    workflow.add_edge("agent", "interceptor")

    # 5. Connect the interceptor to the router
    workflow.add_conditional_edges(
        "interceptor",
        should_continue
    )

    # 6. Complete the loop
    workflow.add_edge("action", "agent")

    app = workflow.compile()

    # --- 6. Interactive Loop ---
    print(">>> Agent Ready. Ask a question (or 'q' to quit).")
    while True:
        user_input = input("Player: ")

        # Handle exit commands
        if user_input.lower() in ['q', 'quit', 'exit']:
            break
        
        # Handle empty inputs or accidental presses of the Enter key
        if not user_input.strip():
            continue

        try:
            inputs = {"messages": [HumanMessage(content=user_input)]}
            
            # CHANGE: Use stream_mode="updates" to see steps as they happen
            print("\n--- Thinking Process ---")
            for event in app.stream(inputs, stream_mode="updates"):
                # 'event' is a dict like {'agent': {...}} or {'tools': {...}}
                for node_name, update in event.items():
                    # The update contains the new keys added to state (e.g. "messages")
                    if "messages" in update:
                        last_message = update["messages"][-1]
                        print_stream_update(node_name, last_message)
            print("------------------------\n")
            
        except Exception as e:
            print(f"Error: {e}")

def print_stream_update(node_name, message):
    """
    Pretty-prints the agent's thought process based on the node that just finished.
    """
    # 1. The "Brain" (LLM) Node
    if node_name == "agent":
        # Case A: Agent decided to call a tool
        if message.tool_calls:
            for tc in message.tool_calls:
                print(f"🤖 Agent: I need to use tool '{tc['name']}'.")
                print(f"   Args: {tc['args']}")
        # Case B: Agent has the final answer
        else:
            print(f"🤖 Agent: {message.content}")

    # 2. The "Tools" Node
    elif node_name == "tools":
        # This is the result coming back from Rust
        print(f"🛠️  Tool '{message.name}' Output: {message.content}")

if __name__ == "__main__":
    main()
