"""An ADK agent exposed over A2A -- the ADK side of the Synapse <-> ADK interop probe.

The mode is chosen by the `INTEROP_MODEL` environment variable:

* `offline` (the default) -- **no model call at all**. The agent answers
  from the conversation itself, so this leg runs with no key and no
  network. That is the mode CI can use: what is under test here is the
  protocol shape (the card, the artifacts, the terminal status), not the
  model.
* a model name (e.g. `gemini-3.5-flash`) -- the agent really calls the
  model and answers with the model's own words.

The key never appears in this file. ADK's own `Gemini` class builds its
`google-genai` client from the environment (`GOOGLE_API_KEY` first, then
`GEMINI_API_KEY`), so the *server process* carries the key, not the
agent.

Why `Gemini` and not `LiteLlm`: a Google AI Studio key authenticates on
the native `google-genai` path only. The OpenAI-compatible surface
rejects it ("Invalid Auth key"), so an ADK agent that speaks the OpenAI
dialect to Google cannot use such a key at all.
"""

import os
from typing import AsyncGenerator

from google.adk.agents import Agent
from google.adk.agents.base_agent import BaseAgent
from google.adk.agents.invocation_context import InvocationContext
from google.adk.events.event import Event
from google.adk.models import Gemini
from google.genai import types

# The model to call, or "offline" for the key-free branch.
MODEL = os.environ.get("INTEROP_MODEL", "offline").strip() or "offline"

# The prefixes the answers start with.
#
# The two branches must answer differently: an offline answer that looks
# like a model's answer is not evidence that a model was called.
OFFLINE_PREFIX = "ADK_OFFLINE_OK"
MODEL_PREFIX = "ADK_MODEL_OK"


class OfflineInteropAgent(BaseAgent):
  """Answers with the caller's own text -- no model, no key, no network.

  It exists so that the "Synapse as client, ADK as server" leg can be
  exercised anywhere. A real model call proves the same protocol shape
  plus one more thing (that the answer really came from the model); the
  protocol shape does not need it.
  """

  async def _run_async_impl(
      self, ctx: InvocationContext
  ) -> AsyncGenerator[Event, None]:
    question = ""
    if ctx.user_content and ctx.user_content.parts:
      question = "".join(part.text or "" for part in ctx.user_content.parts)
    answer = f"{OFFLINE_PREFIX} {question.upper()}".strip()
    yield Event(
        invocation_id=ctx.invocation_id,
        author=self.name,
        content=types.Content(
            role="model",
            parts=[types.Part(text=answer)],
        ),
    )


if MODEL == "offline":
  root_agent = OfflineInteropAgent(
      name="interop_agent",
      description="ADK agent used to test A2A interop with Synapse.",
  )
else:
  # A real model, on the native google-genai path.
  root_agent = Agent(
      name="interop_agent",
      model=Gemini(model=MODEL),
      description="ADK agent used to test A2A interop with Synapse.",
      instruction=(
          "You are the A2A interop probe. Reply with exactly one line: "
          f"{MODEL_PREFIX} followed by a space and the uppercase form of "
          "the user's text. Do not call any tool."
      ),
  )
