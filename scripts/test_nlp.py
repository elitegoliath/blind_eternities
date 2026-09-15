import sys
import os

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from python_agent.router import classify_query, Intent
from python_agent.translator import translate_to_action
from python_agent.models import GameState
from python_agent.pipelines import handle_simulation

def test_nlp():
    prompt = "I want to cast Lightning Bolt targeting my opponent's Tarmogoyf"
    
    print(f"\n--- Testing NLP Pipeline ---")
    print(f"User Prompt: '{prompt}'")
    
    # 1. Router
    print("\n1. Classifying Intent...")
    classification = classify_query(prompt)
    print(f"Classification: {classification.intent.value}")
    print(f"Entities: {classification.entities}")
    
    assert classification.intent == Intent.SIMULATION, "Failed to classify as SIMULATION"
    assert "Lightning Bolt" in classification.entities, "Failed to extract Lightning Bolt"
    assert "Tarmogoyf" in classification.entities, "Failed to extract Tarmogoyf"
    
    # 2. Translator
    print("\n2. Translating to Action...")
    dummy_state = GameState(
        active_player="Player",
        priority_player="Player",
        turn_number=1,
        phase="PreCombatMain"
    )
    
    action = translate_to_action(prompt, dummy_state)
    print(f"Translated Action: {action.type}")
    
    assert action.type == "CastSpell", "Failed to translate to CastSpell Action"
    assert action.payload.card.name == "Lightning Bolt", "Failed to populate CastSpell card name"
    
    # 3. Pipeline Stub
    print("\n3. Pipeline Routing...")
    handle_simulation(prompt, action)
    
    print("\n✅ NLP Pipeline Test Passed!")

if __name__ == "__main__":
    test_nlp()
