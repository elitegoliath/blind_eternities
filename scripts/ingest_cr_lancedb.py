import os
import re
import lancedb
from fastembed import TextEmbedding

def ingest_rules(filepath: str):
    if not os.path.exists(filepath):
        print(f"File not found: {filepath}")
        return

    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    rules_data = []
    lines = content.split('\n')
    for line in lines:
        parts = line.split(" ", 1)
        if len(parts) == 2 and re.match(r"^\d+\.[\d\.a-z]*$", parts[0]):
            rules_data.append({"rule_number": parts[0], "text": parts[1]})

    if rules_data:
        print(f"Generating embeddings for {len(rules_data)} rules...")
        embedding_model = TextEmbedding("sentence-transformers/all-MiniLM-L6-v2")
        texts = [r["text"] for r in rules_data]
        embeddings = list(embedding_model.embed(texts))
        
        for r, emb in zip(rules_data, embeddings):
            r["vector"] = emb

        db_path = os.path.join(os.path.dirname(os.path.dirname(__file__)), "data", "lancedb")
        os.makedirs(db_path, exist_ok=True)
        db = lancedb.connect(db_path)
        
        table_name = "cr_vectors"
        if table_name in db.table_names():
            db.drop_table(table_name)
            
        print("Ingesting into LanceDB...")
        db.create_table(table_name, data=rules_data)
        print("Done.")
    else:
        print("No rules matched.")

if __name__ == "__main__":
    raw_path = os.path.join(os.path.dirname(os.path.dirname(__file__)), "data", "raw", "MagicCompRules.txt")
    ingest_rules(raw_path)
