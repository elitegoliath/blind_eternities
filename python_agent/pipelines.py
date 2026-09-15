from python_agent.models import Action

def handle_definition(query: str, entities: list[str]):
    print(f"[Pipeline] Routing to DEFINITION. Entities: {entities}")
    # TODO: Query Scryfall/SQLite DB

def handle_interaction(query: str, entities: list[str]):
    print(f"[Pipeline] Routing to INTERACTION. Entities: {entities}")
    # TODO: Synthesize rulings

def handle_simulation(query: str, action: Action):
    print(f"[Pipeline] Routing to SIMULATION. Action: {action.type}")
    if hasattr(action, 'payload'):
        print(f"Payload: {action.payload.model_dump_json(indent=2)}")
    else:
        print(f"Payload: {action.model_dump_json(indent=2)}")
    # TODO: Route over FFI to rust_core apply_action()
