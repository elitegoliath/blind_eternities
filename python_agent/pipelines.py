import sqlite3
import os
import lancedb
from fastembed import TextEmbedding
from langchain_core.messages import SystemMessage, HumanMessage
from python_agent.models import Action
from python_agent.llm_engine import get_llm

DB_PATH = os.path.join(
    os.path.dirname(os.path.dirname(__file__)), "data", "mtg_database.sqlite"
)
LANCEDB_PATH = os.path.join(
    os.path.dirname(os.path.dirname(__file__)), "data", "lancedb"
)


def get_db_connection():
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    return conn


def handle_definition(query: str, entities: list[str]) -> str:
    print(f"[Pipeline] Routing to DEFINITION. Entities: {entities}")
    results = []

    if not entities:
        return "No specific cards or keywords were identified in your query."

    try:
        conn = get_db_connection()
        cursor = conn.cursor()

        for entity in entities:
            # Try exact match first
            cursor.execute(
                "SELECT name, type_line, mana_cost, oracle_text FROM cards WHERE name LIKE ? LIMIT 1",
                (entity,),
            )
            row = cursor.fetchone()

            # If no exact match, try fuzzy match
            if not row:
                cursor.execute(
                    "SELECT name, type_line, mana_cost, oracle_text FROM cards WHERE name LIKE ? LIMIT 1",
                    (f"%{entity}%",),
                )
                row = cursor.fetchone()

            if row:
                mana_cost = row["mana_cost"] if row["mana_cost"] else "No mana cost"
                results.append(
                    f"Card: {row['name']}\nType: {row['type_line']}\nMana Cost: {mana_cost}\nOracle Text: {row['oracle_text']}\n"
                )
            else:
                results.append(
                    f"Could not find exact details for '{entity}' in the database."
                )

        conn.close()

    except Exception as e:
        print(f"Database error: {e}")
        return f"Error accessing the MTG database: {str(e)}"

    if results:
        return "\n".join(results)
    return "No definitions found."


def handle_interaction(query: str, entities: list[str]) -> str:
    print(f"[Pipeline] Routing to INTERACTION. Entities: {entities}")

    # 1. Fetch Scryfall rulings for entities
    rulings_text = ""
    try:
        conn = get_db_connection()
        cursor = conn.cursor()

        for entity in entities:
            cursor.execute(
                "SELECT oracle_id, name FROM cards WHERE name LIKE ? LIMIT 1",
                (f"%{entity}%",),
            )
            card_row = cursor.fetchone()
            if card_row:
                oracle_id = card_row["oracle_id"]
                card_name = card_row["name"]
                cursor.execute(
                    "SELECT date, text FROM rulings WHERE oracle_id = ?", (oracle_id,)
                )
                rulings = cursor.fetchall()
                if rulings:
                    rulings_text += f"\n--- Rulings for {card_name} ---\n"
                    for r in rulings:
                        rulings_text += f"[{r['date']}] {r['text']}\n"
        conn.close()
    except Exception as e:
        print(f"SQLite error: {e}")

    # 2. Query LanceDB for CR vectors
    cr_text = ""
    try:
        if os.path.exists(LANCEDB_PATH):
            db = lancedb.connect(LANCEDB_PATH)
            if "cr_vectors" in db.table_names():
                table = db.open_table("cr_vectors")
                embedding_model = TextEmbedding(
                    "sentence-transformers/all-MiniLM-L6-v2"
                )
                query_vector = list(embedding_model.embed([query]))[0]

                results = table.search(query_vector).limit(5).to_list()
                if results:
                    cr_text += "\n--- Relevant Comprehensive Rules ---\n"
                    for res in results:
                        cr_text += f"{res['rule_number']} {res['text']}\n"
    except Exception as e:
        print(f"LanceDB error: {e}")

    # 3. LLM Synthesis
    llm = get_llm()
    system_prompt = f"""You are a Level 3 MTG Judge. Answer the user's question using ONLY the provided Comprehensive Rules and Card Rulings. Cite the specific rule numbers in your explanation.

{cr_text}
{rulings_text}"""

    print(">>> Generating Judge Synthesis...")
    response = llm.invoke(
        [SystemMessage(content=system_prompt), HumanMessage(content=query)]
    )

    return response.content


def handle_simulation(query: str, action: Action):
    print(f"[Pipeline] Routing to SIMULATION. Action: {action.type}")
    if hasattr(action, "payload"):
        print(f"Payload: {action.payload.model_dump_json(indent=2)}")
    else:
        print(f"Payload: {action.model_dump_json(indent=2)}")
    # Route over FFI to rust_core apply_action()
