# python_agent/main.py
import json
import ast
import asyncio
from typing import TypedDict, Annotated, Sequence, AsyncGenerator

from dotenv import load_dotenv
from langchain_core.messages import BaseMessage, HumanMessage, SystemMessage, AIMessage
from langchain_core.runnables import RunnableConfig
from langgraph.graph import StateGraph, END, add_messages
from langgraph.prebuilt import ToolNode

# Import local modules
from python_agent.llm_engine import get_llm, SYSTEM_PROMPT
from python_agent.tools import fetch_card, cast_spell, pass_priority
from python_agent.interfaces import StateStoreBackend, LLMInterceptor, AgentMiddleware
from python_agent.state_store import LocalJSONStateStore
from python_agent.interceptors import QwenToolInterceptor

load_dotenv()

class AgentState(TypedDict):
    messages: Annotated[Sequence[BaseMessage], add_messages]

class BlindEternitiesAgent:
    """
    Headless MTG Judge Agent capable of stateless execution.
    Outputs structured events rather than printing to CLI.
    """
    def __init__(
        self,
        state_store: StateStoreBackend = None,
        interceptor: LLMInterceptor = None,
        pre_action: AgentMiddleware = None,
        post_action: AgentMiddleware = None
    ):
        self.state_store = state_store or LocalJSONStateStore()
        self.interceptor = interceptor or QwenToolInterceptor()
        self.pre_action = pre_action
        self.post_action = post_action

        self.tools = [fetch_card, cast_spell, pass_priority]
        self.llm = get_llm().bind_tools(self.tools)
        
        self.app = self._build_graph()

    def _build_graph(self):
        def call_model(state: AgentState):
            messages = state["messages"]
            response = self.llm.invoke([SystemMessage(content=SYSTEM_PROMPT)] + list(messages))
            return {"messages": [response]}
            
        def interceptor_node(state: AgentState):
            return self.interceptor(state)
            
        def pre_action_node(state: AgentState):
            if self.pre_action:
                return self.pre_action(state)
            return state
            
        def post_action_node(state: AgentState):
            if self.post_action:
                return self.post_action(state)
            return state

        def should_continue(state: dict) -> str:
            msg_key = "messages" if "messages" in state else "chat_history"
            last_message = state[msg_key][-1]
            if hasattr(last_message, "tool_calls") and last_message.tool_calls:
                return "pre_action"
            return END

        tool_node = ToolNode(self.tools)

        workflow = StateGraph(AgentState)
        workflow.add_node("agent", call_model)
        workflow.add_node("interceptor", interceptor_node)
        workflow.add_node("pre_action", pre_action_node)
        workflow.add_node("action", tool_node)
        workflow.add_node("post_action", post_action_node)

        workflow.set_entry_point("agent")
        workflow.add_edge("agent", "interceptor")
        workflow.add_conditional_edges("interceptor", should_continue)
        
        workflow.add_edge("pre_action", "action")
        workflow.add_edge("action", "post_action")
        workflow.add_edge("post_action", "agent")

        return workflow.compile()

    async def stream_events(self, session_id: str, user_input: str) -> AsyncGenerator[dict, None]:
        """Yields JSON-serializable events meant to be piped to WebSockets or SSE."""
        inputs = {"messages": [HumanMessage(content=user_input)]}
        config = RunnableConfig(
            configurable={"session_id": session_id, "state_store": self.state_store}
        )
        
        async for event in self.app.astream(inputs, config=config, stream_mode="updates"):
            for node_name, update in event.items():
                if "messages" in update:
                    last_message = update["messages"][-1]
                    if node_name == "agent":
                        if last_message.tool_calls:
                            yield {"type": "agent_tool_call", "tool_calls": last_message.tool_calls}
                        else:
                            yield {"type": "agent_reply", "content": last_message.content}
                    elif node_name == "action":
                        yield {"type": "tool_result", "name": last_message.name, "content": last_message.content}

async def run_cli():
    print(">>> Initializing Blind Eternities Agent (Headless Backend)...")
    agent = BlindEternitiesAgent()
    session_id = "cli_dev_session"
    print(">>> Agent Ready. Ask a question (or 'q' to quit).")
    while True:
        try:
            user_input = input("Player: ")
        except EOFError:
            break
        if user_input.lower() in ['q', 'quit', 'exit']:
            break
        if not user_input.strip():
            continue

        print("\n--- Thinking Process ---")
        async for event in agent.stream_events(session_id, user_input):
            if event["type"] == "agent_tool_call":
                for tc in event["tool_calls"]:
                    print(f"🤖 Agent: I need to use tool '{tc['name']}'.\n   Args: {tc['args']}")
            elif event["type"] == "tool_result":
                print(f"🛠️  Tool '{event.get('name')}' Output: {event.get('content')}")
            elif event["type"] == "agent_reply":
                print(f"🤖 Agent: {event['content']}")
        print("------------------------\n")

def main():
    asyncio.run(run_cli())

if __name__ == "__main__":
    main()
