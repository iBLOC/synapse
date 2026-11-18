"""Interop probe: the a2a-sdk 1.3.0 client (Google ADK's own dependency) -> Synapse.

Run it against a live Synapse A2A server:

    # terminal 1 -- start the Synapse side
    cargo run --example a2a_interop_server -- 9099

    # terminal 2 -- a venv with google-adk[a2a] (pulls a2a-sdk 1.x)
    pip install "google-adk[a2a]"
    python examples/a2a_sdk_probe.py

Measurements, in order:
  1. card resolution        -- can the Python client parse Synapse's v1.0 card?
  2. default client path    -- exactly what an a2a-sdk app does (create_client
                               + send_message), including the extension header.
  3. GetTask                -- what the server actually persisted for that task.
  4. direct streaming call  -- the same call with the transport used directly.
  5. negative control       -- the same call with the A2A-Version header removed.

Every line is a measurement, not a summary: the point is that the reading comes
from a different implementation than the one under test.
"""

import asyncio
import json

import httpx
from google.protobuf import json_format

from a2a.client import A2ACardResolver, ClientConfig, create_client
from a2a.client.transports.jsonrpc import JsonRpcTransport
from a2a.types import a2a_pb2 as pb


SYNAPSE = "http://127.0.0.1:9099"
EXT = "https://example.com/ext/v1"
UNKNOWN_EXT = "https://example.com/not-implemented/v1"


def meta_dict(md):
    if md is None:
        return None
    return json_format.MessageToDict(md)


def describe(resp, tag):
    which = resp.WhichOneof("payload")
    print(tag + "_EVENT", which)
    if which == "task":
        t = resp.task
        print(tag + "_TASK", t.id, pb.TaskState.Name(t.status.state))
        for a in t.artifacts:
            print(tag + "_ARTIFACT_TEXT", json.dumps([p.text for p in a.parts]))
        for h in t.history:
            print(tag + "_HISTORY_ROLE", pb.Role.Name(h.role),
                  json.dumps([p.text for p in h.parts]))
        print(tag + "_TASK_META", json.dumps(meta_dict(t.metadata)))
    elif which == "message":
        print(tag + "_MESSAGE_ROLE", pb.Role.Name(resp.message.role))
        print(tag + "_MESSAGE_TEXT", json.dumps([p.text for p in resp.message.parts]))
        print(tag + "_MESSAGE_META", json.dumps(meta_dict(resp.message.metadata)))
    elif which == "status_update":
        su = resp.status_update
        print(tag + "_STATUS", pb.TaskState.Name(su.status.state))
        if su.status.message is not None:
            print(tag + "_STATUS_TEXT",
                  json.dumps([p.text for p in su.status.message.parts]))
        print(tag + "_STATUS_META", json.dumps(meta_dict(su.metadata)))
    elif which == "artifact_update":
        au = resp.artifact_update
        print(tag + "_ARTIFACT_TEXT",
              json.dumps([p.text for p in au.artifact.parts]))
        print(tag + "_ARTIFACT_META", json.dumps(meta_dict(au.metadata)))


def build_request(tag):
    return pb.SendMessageRequest(
        message=pb.Message(
            message_id=tag,
            role=pb.Role.ROLE_USER,
            parts=[pb.Part(text="interop probe from a2a-sdk 1.3.0")],
        )
    )


async def main():
    headers = {
        "A2A-Extensions": EXT + ", " + UNKNOWN_EXT,
        "X-A2A-Extensions": EXT,
    }
    client_http = httpx.AsyncClient(timeout=30.0, headers=headers)

    # ---- 1. card -------------------------------------------------------
    resolver = A2ACardResolver(client_http, SYNAPSE)
    card = await resolver.get_agent_card()
    print("CARD_NAME", card.name)
    print("CARD_VERSION", card.version)
    for it in card.supported_interfaces:
        print("CARD_INTERFACE", it.url, it.protocol_binding, it.protocol_version)
    print("CARD_STREAMING", card.capabilities.streaming)
    print("CARD_EXTENSIONS", json.dumps([e.uri for e in card.capabilities.extensions]))
    print("CARD_INPUT_MODES", json.dumps(list(card.default_input_modes)))

    # ---- 2. default client path ---------------------------------------
    config = ClientConfig(httpx_client=client_http, streaming=True)
    client = await create_client(SYNAPSE, client_config=config)
    print("SENT_VERSION_HEADER", client_http.headers.get("A2A-Version"))
    task_id = None
    try:
        async for resp in client.send_message(build_request("probe-sdk-1")):
            describe(resp, "SDK")
            which = resp.WhichOneof("payload")
            if which == "task":
                task_id = resp.task.id
            elif which == "status_update":
                task_id = task_id or resp.status_update.task_id
            elif which == "artifact_update":
                task_id = task_id or resp.artifact_update.task_id
    except Exception as exc:
        print("SDK_ERROR", type(exc).__name__, str(exc))

    # ---- 3. GetTask: what did the server persist? ---------------------
    transport = JsonRpcTransport(client_http, card, SYNAPSE)
    if task_id:
        try:
            stored = await transport.get_task(pb.GetTaskRequest(id=task_id))
            print("GETTASK_STATE", pb.TaskState.Name(stored.status.state))
            print("GETTASK_ARTIFACTS",
                  json.dumps([[p.text for p in a.parts] for a in stored.artifacts]))
            print("GETTASK_META", json.dumps(meta_dict(stored.metadata)))
        except Exception as exc:
            print("GETTASK_ERROR", type(exc).__name__, str(exc))

    # ---- 4. direct streaming call (card gate bypassed) -----------------
    try:
        async for resp in transport.send_message_streaming(
            build_request("probe-sdk-stream")
        ):
            describe(resp, "STREAM")
    except Exception as exc:
        print("STREAM_ERROR", type(exc).__name__, str(exc))

    # ---- 5. negative control: no A2A-Version header --------------------
    bare_http = httpx.AsyncClient(timeout=30.0)
    bare_transport = JsonRpcTransport(bare_http, card, SYNAPSE)
    try:
        resp = await bare_transport.send_message(build_request("probe-sdk-bare"))
        describe(resp, "BARE")
    except Exception as exc:
        print("BARE_ERROR", type(exc).__name__, str(exc))
    finally:
        await bare_http.aclose()

    await client_http.aclose()


if __name__ == "__main__":
    asyncio.run(main())
