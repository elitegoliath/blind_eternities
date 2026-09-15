from python_agent.llm_engine import get_llm
from pydantic import BaseModel
class Test(BaseModel):
    hello: str

llm = get_llm().with_structured_output(Test)
res = llm.invoke("Say hi")
print(res)
