import re

with open("python_agent/tools.py", "r") as f:
    content = f.read()

# Replace else blocks with un-indented returns
# Pattern for 'else: return {"status": "illegal"...}'
pattern1 = r'        else:\n\s+return \{"status": "illegal", "reason": ruling\.get\("reason", "Unknown legality error\."\)\}'
repl1 = r'        return {"status": "illegal", "reason": ruling.get("reason", "Unknown legality error.")}'

# Pattern for 'else: return {"status": "error", "message": ruling.get("message", "Engine error")}'
pattern2 = r'        else:\n\s+return \{"status": "error", "message": ruling\.get\("message", "Engine error"\)\}'
repl2 = r'        return {"status": "error", "message": ruling.get("message", "Engine error")}'

content = re.sub(pattern1, repl1, content)
content = re.sub(pattern2, repl2, content)

with open("python_agent/tools.py", "w") as f:
    f.write(content)
