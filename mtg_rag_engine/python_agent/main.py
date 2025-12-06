# mtg_rag_engine/python_agent/main.py

# This file is the main entry point for the Python Agent that interacts with the LLM and tools.

import sys
import operator
from typing import TypedDict, Annotated, Sequence

from dotenv import load_dotenv
from langchain_core.messages import BaseMessage, HumanMessage
from langchain_core.tools import Tool
from langgraph.graph import StateGraph, END, add_messages
from langgraph.prebuilt import ToolNode

# Import local modules
from python_agent.llm_engine import get_llm, SYSTEM_PROMPT
from python_agent.tools import validate_move

load_dotenv()

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
    tools = [validate_move]
    llm = get_llm()
    llm_with_tools = llm.bind_tools(tools)

    # --- 3. Define Nodes (The Logic) ---
    
    def call_model(state: AgentState):
        """Node 1: The Brain. Decides what to do next."""
        # Get conversation history
        messages = state["messages"]
        # Invoke the LLM
        response = llm_with_tools.invoke([SYSTEM_PROMPT] + list(messages))
        # Return the new message to update state
        return {"messages": [response]}

    # Node 2: The Tools.
    # We use the prebuilt ToolNode because it's just a simple executor.
    tool_node = ToolNode(tools)

    # --- 4. Define Edges (The Flow Control) ---
    
    def should_continue(state: AgentState):
        """Decides: Do we run tools or stop?"""
        last_message = state["messages"][-1]
        
        # If the LLM returned a tool_call, go to 'tools'
        if last_message.tool_calls:
            return "tools"
        # Otherwise, stop
        return END

    # --- 5. Build the Graph ---
    workflow = StateGraph(AgentState)

    workflow.add_node("agent", call_model)
    workflow.add_node("tools", tool_node)

    workflow.set_entry_point("agent")

    # Conditional Logic: After 'agent', check if we need tools
    workflow.add_conditional_edges(
        "agent",
        should_continue,
    )

    # Loop Logic: After 'tools', always go back to 'agent' to interpret results
    workflow.add_edge("tools", "agent")

    # Compile into a Runnable
    app = workflow.compile()

    # --- 6. Interactive Loop ---
    print(">>> Agent Ready. Ask a question (or 'q' to quit).")
    while True:
        user_input = input("\nUser: ")
        if user_input.lower() in ['q', 'quit', 'exit']:
            break
            
        try:
            # Prepare input
            inputs = {"messages": [HumanMessage(content=user_input)]}
            
            # Run the graph
            # stream_mode="values" prints the state as it evolves
            final_response = None
            for event in app.stream(inputs, stream_mode="values"):
                # Capture the last message from the stream
                final_response = event["messages"][-1]
            
            # Print the final AI response
            if final_response:
                print(f"Agent: {final_response.content}")
            
        except Exception as e:
            print(f"Error: {e}")

if __name__ == "__main__":
    main()
