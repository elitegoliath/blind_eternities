import streamlit as st
import mtg_logic_core
import json
import time

st.set_page_config(page_title="Blind Eternities Core", layout="wide")

st.title("⚡ Blind Eternities: Rust Core")

# --- SIDEBAR: THE JUDGE ---
st.sidebar.header("⚖️ The Judge")

# 1. Global State
with st.sidebar.expander("Game State", expanded=True):
    phase = st.selectbox("Current Phase", 
        ["Untap", "Upkeep", "Draw", "Main Phase 1", "Combat", "Main Phase 2", "End"])
    is_my_turn = st.toggle("Is Active Player?", value=True)
    stack_depth = st.number_input("Items on Stack", min_value=0, value=0)
    lands_played = st.number_input("Lands Played", min_value=0, value=0)

# 2. Mana Pool (NEW)
with st.sidebar.expander("Mana Pool", expanded=True):
    col1, col2, col3 = st.columns(3)
    with col1:
        w = st.number_input("White ({W})", 0, 10, 0)
        u = st.number_input("Blue ({U})", 0, 10, 0)
    with col2:
        b = st.number_input("Black ({B})", 0, 10, 0)
        r = st.number_input("Red ({R})", 0, 10, 0)
    with col3:
        g = st.number_input("Green ({G})", 0, 10, 0)
        c = st.number_input("Colorless ({C})", 0, 10, 0)

# 3. Define the Action
st.sidebar.divider()
action_type = st.sidebar.radio("Attempt Action:", ["Play Land", "Cast Spell"])
card_name = st.sidebar.text_input("Card Name", "Counterspell")

if action_type == "Play Land":
    default_cost = ""
    default_types = ["Land"]
else:
    default_cost = "{U}{U}"
    default_types = ["Instant"]

card_cost = st.sidebar.text_input("Mana Cost", default_cost, help="Use {W}, {1}, etc.")
card_types = st.sidebar.multiselect("Card Types", 
    ["Land", "Creature", "Artifact", "Instant", "Sorcery", "Enchantment"],
    default=default_types
)

# 4. Construct Payload & Call Rust
if st.sidebar.button("Check Legality", type="primary"):
    # Map UI to Rust Enums
    action_payload = {
        "type": "PlayLand" if action_type == "Play Land" else "CastSpell",
        "payload": {
            "name": card_name,
            "type_line": card_types,
            "mana_cost": card_cost 
        }
    }
    
    state = {
        "active_player": "Hero",
        "is_active_player": is_my_turn,
        "phase": phase,
        "battlefield": [],
        "stack": ["Spell"] * stack_depth,
        "lands_played": lands_played,
        "mana_pool": {
            "white": w, "blue": u, "black": b, 
            "red": r, "green": g, "colorless": c
        },
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
    start_t = time.perf_counter()
    results_json = mtg_logic_core.search_cards(query, 5)
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