from pydantic import BaseModel, Field
from langchain_core.prompts import ChatPromptTemplate
from python_agent.models import GameState, Action
from python_agent.llm_engine import get_llm


class ActionWrapper(BaseModel):
    action: Action = Field(..., description="The translated game action.")


def translate_to_action(user_prompt: str, current_state: GameState) -> Action:
    llm = get_llm().with_structured_output(ActionWrapper)

    prompt = ChatPromptTemplate.from_messages(
        [
            (
                "system",
                "You are an MTG Action Translator. You map user intent into exactly one of our strictly typed Action models.\n"
                "The current game state is:\n{current_state}\n\n"
                "Based on the user's prompt, output the proper Action JSON payload. Use the exact types defined: CastSpell, PlayLand, ActivateAbility, DeclareAttackers, DeclareBlockers, PassPriority.\n"
                "Ensure the payload structure strictly matches the required Action Type payload.",
            ),
            ("human", "{user_prompt}"),
        ]
    )

    chain = prompt | llm
    result = chain.invoke(
        {
            "current_state": current_state.model_dump_json(indent=2),
            "user_prompt": user_prompt,
        }
    )
    return result.action
