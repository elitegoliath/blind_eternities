import streamlit as st
import mtg_logic_core
import json
import time

st.set_page_config(page_title="Blind Eternities Core", layout="wide")

st.title("⚡ Blind Eternities: Rust Core")

st.markdown("""
        <style>
        /* Compact Buttons: Reduces the gap between the Up and Down buttons */
        div[data-testid="column"] button {
            height: auto !important;
            padding: 0px 10px !important; /* Reduces internal button padding */
            min-height: 25px !important;
            margin-bottom: -15px !important; /* HACK: Pulls the next element closer */
            border: 1px solid #444; /* Optional: Adds a subtle border for definition */
        }
        
        /* Fix the vertical alignment of the arrow emojis */
        div[data-testid="column"] button p {
            font-size: 1.2rem !important;
            line-height: 1.2 !important;
            margin: 0 !important;
            padding-top: 2px !important;
        }
        </style>
    """, unsafe_allow_html=True)

# --- SIDEBAR: THE JUDGE (Stateful) ---
if "game_state" not in st.session_state:
    # Initialize Default State
    st.session_state.game_state = {
        "active_player": "Hero",
        "is_active_player": True,
        "phase": "Main Phase 1",
        "battlefield": [],
        "stack": [],
        "lands_played": 0,
        "mana_pool": {"white": 0, "blue": 0, "black": 0, "red": 0, "green": 0, "colorless": 0},
        "pending_action": None
    }

st.sidebar.header("⚖️ The Judge")

# Display Current State Logic
gs = st.session_state.game_state

# Global State
with st.sidebar.expander("Game State", expanded=True):
    phase = st.selectbox("Current Phase", 
        ["Untap", "Upkeep", "Draw", "Main Phase 1", "Combat", "Main Phase 2", "End"])
    is_my_turn = st.toggle("Is Active Player?", value=True)
    stack_depth = st.number_input("Items on Stack", min_value=0, value=0)
    lands_played = st.number_input("Lands Played", min_value=0, value=0)

# EDITABLE STATE (For prototyping, we still let you cheat/edit values)
with st.sidebar.expander("Game State Controls", expanded=True):
    gs["phase"] = st.selectbox("Phase", ["Untap", "Upkeep", "Draw", "Main Phase 1", "Combat", "Main Phase 2", "End"], 
                               index=["Untap", "Upkeep", "Draw", "Main Phase 1", "Combat", "Main Phase 2", "End"].index(gs["phase"]))
    
    # Mana Controls (Bind to session state directly)
    # Note: In a real game, you wouldn't edit these manually, you'd TAP lands.
    st.divider()
    st.caption("Mana Pool")

    # Helper to render Value (Left) + Vertical Buttons (Right)
    def mana_control(label, key_name, color_code):
        c_val, c_btns = st.columns([2, 1])
        
        current_val = gs["mana_pool"][key_name]
        
        with c_val:
            # TEXT COLOR FIX: 
            # 1. We color ONLY the Symbol ({R}) using a span.
            # 2. The number uses default theme color (white/black) to match "other section text".
            st.markdown(
                f"<div style='text-align: right; font-weight: bold; padding-right: 10px; line-height: 1.0;'>"
                f"<span style='color: {color_code}; font-size: 1.2em;'>{label}</span><br/>"
                f"<span style='font-size: 2.2em;'>{current_val}</span>"
                f"</div>", 
                unsafe_allow_html=True
            )
        
        with c_btns:
            # We use a container to try and group them tighter
            with st.container():
                if st.button("🔼", key=f"inc_{key_name}", use_container_width=True):
                    gs["mana_pool"][key_name] += 1
                    st.rerun()
                
                # The CSS margin-bottom hack above helps close this gap
                if st.button("🔽", key=f"dec_{key_name}", use_container_width=True):
                    if current_val > 0:
                        gs["mana_pool"][key_name] -= 1
                        st.rerun()

    # Render Controls in a Grid
    m1, m2 = st.columns(2)
    
    with m1:
        mana_control("White {W}", "white", "#F0F2C0")
        mana_control("Blue {U}", "blue", "#AAE0FA")
        mana_control("Black {B}", "black", "#CBC2BF")
    
    with m2:
        mana_control("Red {R}", "red", "#F9AA8F")
        mana_control("Green {G}", "green", "#9BD3AE")
        mana_control("Colorless {C}", "colorless", "#D3D6D9")

# ACTION INPUT
st.sidebar.divider()
action_type = st.sidebar.radio("Action:", ["Play Land", "Cast Spell"])
card_name = st.sidebar.text_input("Card Name", "Mountain" if action_type == "Play Land" else "Lightning Bolt")
card_cost = st.sidebar.text_input("Cost", "" if action_type == "Play Land" else "{R}")

if st.sidebar.button("APPLY ACTION"):
    # 1. Attach Action to State
    payload = {
        "type": "PlayLand" if action_type == "Play Land" else "CastSpell",
        "payload": {
            "name": card_name,
            "type_line": ["Land"] if action_type == "Play Land" else ["Instant"], 
            "mana_cost": card_cost
        }
    }
    gs["pending_action"] = payload

    # 2. Send to Rust
    json_str = json.dumps(gs)
    response = mtg_logic_core.apply_action(json_str)
    result = json.loads(response)

    if result["status"] == "success":
        st.success("✅ Resolved!")
        # 3. Update Session State with Rust's new truth
        st.session_state.game_state = result["new_state"]
        st.rerun()
    else:
        st.error(f"❌ Denied: {result.get('reason')}")

# --- MAIN DISPLAY ---
st.divider()
col1, col2 = st.columns(2)

with col1:
    st.subheader("Battlefield")
    if not gs["battlefield"]:
        st.info("Empty")
    for perm in gs["battlefield"]:
        st.button(f"{perm['name']} (ID: {perm['id']})", disabled=True)

with col2:
    st.subheader("Stack")
    if not gs["stack"]:
        st.info("Empty")
    for item in reversed(gs["stack"]):
        st.warning(f"⚡ Casting: {item}")
