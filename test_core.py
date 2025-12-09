import mtg_logic_core
import json
import time

def test_judge():
    print("--- Testing The Judge ---")
    # A simple mock state
    game_state = {
        "active_player": "Player A",
        "phase": "Main Phase 1",
        "stack": [],
        "battlefield": []
    }
    
    payload = json.dumps(game_state)
    try:
        result = mtg_logic_core.check_board_state(payload)
        print(f"✅ Judge Result: {result}")
    except Exception as e:
        print(f"❌ Judge Failed: {e}")

def test_librarian():
    print("\n--- Testing The Librarian ---")
    query = "destroy all creatures"
    
    # First Run (Model Initialization + DB Connection)
    print("1. Running first search (Cold Start)...")
    start = time.time()
    try:
        # Limit 3 cards
        results_json = mtg_logic_core.search_cards(query, 3)
        duration = time.time() - start
        
        results = json.loads(results_json)
        
        if "status" in results and results["status"] == "error":
            print(f"❌ Search Error: {results['message']}")
            return

        print(f"✅ Found {len(results)} cards in {duration:.4f}s")
        for card in results:
            print(f"   - {card['name']} ({card['type']})")
            
    except Exception as e:
        print(f"❌ Librarian Failed: {e}")
        return

    # Second Run (Hot Cache)
    # This verifies the 'OnceLock' is working. It should be much faster
    # because it won't reload the 30MB embedding model or spin up a new Tokio Runtime.
    print("\n2. Running second search (Warm Cache)...")
    start = time.time()
    mtg_logic_core.search_cards("draw two cards", 3)
    duration = time.time() - start
    print(f"✅ Second search completed in {duration:.4f}s")

if __name__ == "__main__":
    test_judge()
    test_librarian()
