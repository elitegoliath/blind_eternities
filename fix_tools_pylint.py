import re

with open("python_agent/tools.py", "r") as f:
    content = f.read()

# Fix re-imports
content = content.replace("    import json\n", "")
content = content.replace("    import re\n", "")
content = content.replace("    import mtg_logic_core\n", "")

# Remove local import of LocalJSONStateStore and replace it
content = content.replace("        from .state_store import LocalJSONStateStore\n", "")

# Remove unnecessary else after return
content = re.sub(r'        else:\n\s+return \{"status": "illegal", "reason": ruling\.get\("reason", "Unknown legality error\."\)\}', r'        return {"status": "illegal", "reason": ruling.get("reason", "Unknown legality error.")}', content)
content = re.sub(r'        else:\n\s+return \{"status": "error", "message": ruling\.get\("message", "Engine error"\)\}', r'        return {"status": "error", "message": ruling.get("message", "Engine error")}', content)


with open("python_agent/tools.py", "w") as f:
    f.write(content)
