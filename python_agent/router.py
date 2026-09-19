from enum import Enum
from pydantic import BaseModel, Field
from typing import List
from langchain_core.prompts import ChatPromptTemplate
from python_agent.llm_engine import get_llm


class Intent(str, Enum):
    DEFINITION = "DEFINITION"
    INTERACTION = "INTERACTION"
    SIMULATION = "SIMULATION"


class QueryClassification(BaseModel):
    intent: Intent = Field(..., description="The classification of the user's query.")
    entities: List[str] = Field(
        default_factory=list,
        description="List of card names, mechanics, or keywords extracted.",
    )


def classify_query(user_prompt: str) -> QueryClassification:
    llm = get_llm().with_structured_output(QueryClassification)
    prompt = ChatPromptTemplate.from_messages(
        [
            (
                "system",
                "You are an MTG rules classifier. Classify the user's input into one of three intents:\n"
                "- DEFINITION: Asking what a card or keyword does.\n"
                "- INTERACTION: Asking how multiple cards interact conceptually.\n"
                "- SIMULATION: Stating an action they want to take in the current game state (e.g., 'I cast Lightning Bolt').\n"
                "Also extract any MTG card names or mechanics as entities.",
            ),
            ("human", "{user_prompt}"),
        ]
    )
    chain = prompt | llm
    return chain.invoke({"user_prompt": user_prompt})
