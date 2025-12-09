import streamlit as st
import mtg_logic_core
import json
import time

st.set_page_config(page_title="Blind Eternities Core", layout="wide")

st.title("⚡ Blind Eternities: Rust Core")

# --- SIDEBAR: THE JUDGE ---
st.sidebar.header("⚖️ The Judge")
st.sidebar.markdown("Define the current game state to check legality.")

# 1. Build the State
phase = st.sidebar.selectbox("Current Phase", 
    ["Untap", "Upkeep", "Draw", "Main Phase 1", "Combat", "Main Phase 2", "End"])

is_my_turn = st.sidebar.toggle("Is Active Player?", value=True)
stack_depth = st.sidebar.number_input("Items on Stack", min_value=0, value=0)
lands_played = st.sidebar.number_input("Lands Played", min_value=0, value=0)

# 2. Define the Action
action_type = st.sidebar.radio("Attempt Action:", ["Play Land", "Cast Spell"])

card_name = st.sidebar.text_input("Card Name (for Action)", "Mountain")
card_type_input = st.sidebar.multiselect(
    "Card Types", 
    ["Land", "Creature", "Artifact", "Instant", "Sorcery", "Enchantment"],
    default=["Land"] if action_type == "Play Land" else ["Creature"]
)

# 3. Construct Payload & Call Rust
if st.sidebar.button("Check Legality"):
    # Map UI to Rust Enums
    action_payload = {
        "type": "PlayLand" if action_type == "Play Land" else "CastSpell",
        "payload": {
            "name": card_name,
            "type_line": card_type_input,
            "mana_cost": None # Simplified for now
        }
    }
    
    state = {
        "active_player": "Hero",
        "is_active_player": is_my_turn,
        "phase": phase,
        "battlefield": [],
        "stack": ["Spell"] * stack_depth, # Dummy stack items
        "lands_played": lands_played,
        "pending_action": action_payload
    }

    # CALL RUST
    start_t = time.perf_counter()
    response = mtg_logic_core.check_board_state(json.dumps(state))
    duration = (time.perf_counter() - start_t) * 1000

    results = json.loads(response)
    ruling = results[0]

    if ruling["status"] == "legal":
        st.success(f"✅ Action Legal ({duration:.2f}ms)")
    else:
        st.error(f"❌ Illegal: {ruling.get('reason')} ({duration:.2f}ms)")


# --- MAIN AREA: THE LIBRARIAN ---
st.header("📚 The Librarian")
st.markdown("Semantic search powered by `fastembed` + `lancedb` in Rust.")

query = st.text_input("Describe a card (e.g., 'destroy all creatures'):")

if query:
    # CALL RUST
    start_t = time.perf_counter()
    results_json = mtg_logic_core.search_cards(query, 5) # Limit 5
    duration = (time.perf_counter() - start_t) * 1000
    
    results = json.loads(results_json)
    
    st.caption(f"Search completed in **{duration:.2f}ms**")

    if "status" in results and results["status"] == "error":
        st.error(results["message"])
    else:
        cols = st.columns(len(results))
        for i, card in enumerate(results):
            with cols[i]:
                st.subheader(card["name"])
                st.caption(card["type"])
                st.info(card["text"])