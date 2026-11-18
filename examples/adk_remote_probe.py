"""ADK's own RemoteA2aAgent -> Synapse: the view an ADK user actually gets.

This is the one interop leg that needs no model key: RemoteA2aAgent is a
*forwarder*. It never calls a model -- it hands the message to the remote
A2A server and converts what comes back into ADK events. So this probe
measures the part that only ADK can measure: ADK's own conversion layer,
including the rule that a task which never reaches a terminal state is
read as "the stream was cut short" and turned into an error.

It also runs a negative control against an in-process fake peer that
reproduces the *pre-fix* Synapse behaviour (a final artifact, but no
terminal status). That control is what makes the terminal-state fix
measurable: without it, "it worked" could just mean "ADK never looked".

Run it against a live Synapse A2A server:

    # terminal 1 -- start the Synapse side
    cargo run --example a2a_interop_server -- 9099

    # terminal 2 -- a venv with google-adk[a2a]
    pip install "google-adk[a2a]"
    python examples/adk_remote_probe.py
"""

import asyncio
import json
import os
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

from google.adk.agents.remote_a2a_agent import RemoteA2aAgent
from google.adk.runners import InMemoryRunner
from google.genai import types


CARD = os.environ.get(
    "SYNAPSE_A2A_CARD",
    "http://127.0.0.1:9099/.well-known/agent-card.json",
)
APP = "synapse_interop_probe"
USER = "probe-user"


async def one_round(remote, label):
    """Run one user turn through the remote agent and report what came back."""
    runner = InMemoryRunner(agent=remote, app_name=APP)
    session = await runner.session_service.create_session(
        app_name=APP, user_id=USER
    )
    message = types.Content(
        role="user",
        parts=[types.Part(text="interop probe from ADK RemoteA2aAgent")],
    )

    texts = []
    errors = []
    finals = 0
    async for event in runner.run_async(
        user_id=USER, session_id=session.id, new_message=message
    ):
        if event.is_final_response():
            finals += 1
        print(f"{label}_EVENT author={event.author} final={event.is_final_response()}")
        if event.content and event.content.parts:
            for part in event.content.parts:
                if part.text:
                    texts.append(part.text)
                    print(f"{label}_TEXT {part.text}")
        if event.error_message:
            errors.append(event.error_message)
            print(f"{label}_ERROR {event.error_message}")
        if event.error_code:
            print(f"{label}_ERROR_CODE {event.error_code}")

    print(f"{label}_FINAL_COUNT {finals}")
    print(f"{label}_HAS_ERROR {bool(errors)}")
    print(f"{label}_ANSWER {'|'.join(texts)}")
    return texts, errors


def start_fake_peer(omit_terminal: bool):
    """A fake A2A peer that behaves like Synapse did before the fix.

    omit_terminal=True reproduces the bug: the answer arrives as an
    artifact, but the task never reaches a terminal state.
    """

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def _send(self, body: bytes, content_type: str):
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            port = self.server.server_address[1]
            card = {
                "name": "pre_fix_synapse",
                "description": "mimics the pre-fix Synapse A2A server",
                "version": "1.0.0",
                "supportedInterfaces": [
                    {
                        "url": f"http://127.0.0.1:{port}",
                        "protocolBinding": "JSONRPC",
                        "protocolVersion": "1.0",
                    }
                ],
                "capabilities": {"streaming": True, "pushNotifications": False},
                "defaultInputModes": ["text/plain"],
                "defaultOutputModes": ["text/plain"],
                "skills": [],
            }
            self._send(json.dumps(card).encode(), "application/json")

        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            if length:
                self.rfile.read(length)
            events = [
                {
                    "artifactUpdate": {
                        "taskId": "t-pre-fix",
                        "contextId": "c-pre-fix",
                        "artifact": {
                            "artifactId": "",
                            "name": "response",
                            "parts": [{"text": "PRE_FIX_ANSWER"}],
                        },
                        "append": False,
                        "lastChunk": True,
                    }
                }
            ]
            if not omit_terminal:
                events.append(
                    {
                        "statusUpdate": {
                            "taskId": "t-pre-fix",
                            "contextId": "c-pre-fix",
                            "status": {"state": "TASK_STATE_COMPLETED"},
                        }
                    }
                )
            body = "".join(
                "data: "
                + json.dumps({"jsonrpc": "2.0", "id": 1, "result": event})
                + "\n\n"
                for event in events
            ).encode()
            self._send(body, "text/event-stream")

    server = HTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


async def main():
    # use_legacy=False makes the client request ADK's own extension URI.
    # Synapse does not implement that extension (its semantics are "run
    # ADK's new agent executor"), so the honest answer is to report it as
    # unsupported -- which is exactly what must not turn into a failure.
    remote = RemoteA2aAgent(
        name="synapse_peer",
        agent_card=CARD,
        use_legacy=False,
    )
    print("CARD", CARD)
    print("AGENT_NAME", remote.name)

    texts, errors = await one_round(remote, "LIVE")
    print("LIVE_ROUND_OK", bool(texts) and not errors)

    # ---- negative control: the pre-fix peer -------------------------
    fake = start_fake_peer(omit_terminal=True)
    port = fake.server_address[1]
    fake_remote = RemoteA2aAgent(
        name="pre_fix_peer",
        agent_card=f"http://127.0.0.1:{port}/.well-known/agent-card.json",
        use_legacy=False,
    )
    _, control_errors = await one_round(fake_remote, "PREFIX")
    print("PREFIX_HAS_ERROR", bool(control_errors))
    fake.shutdown()


if __name__ == "__main__":
    asyncio.run(main())
