with open("python_agent/tools.py", "r") as f:
    content = f.read()

import re

old_else_illegal = """        else:
            return {
                "status": "illegal",
                "reason": ruling.get("reason", "Unknown legality error."),
            }"""
new_else_illegal = """        return {
            "status": "illegal",
            "reason": ruling.get("reason", "Unknown legality error."),
        }"""
content = content.replace(old_else_illegal, new_else_illegal)

old_else_error = """        else:
            return {"status": "error", "message": ruling.get("message", "Engine error")}"""
new_else_error = """        return {"status": "error", "message": ruling.get("message", "Engine error")}"""
content = content.replace(old_else_error, new_else_error)

with open("python_agent/tools.py", "w") as f:
    f.write(content)
