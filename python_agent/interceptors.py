# python_agent/interceptors.py
import json
import ast
from langchain_core.messages import AIMessage
from langchain_core.messages.tool import ToolCall
from .interfaces import LLMInterceptor

class QwenToolInterceptor(LLMInterceptor):
    """
    Extracts tool calls buried inside conversational text, XML tags, or markdown.
    Specifically designed to handle Qwen 3's hallucination patterns.
    """
    def __call__(self, state: dict) -> dict:
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
