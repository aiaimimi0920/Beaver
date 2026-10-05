# Beaver → dot smoke-test bridge

Reusable source checkpoint of the event pilot verified on 2026-10-05. This is
an **owner-private, single-owner connectivity pilot**, not an autonomous
model-generation adapter. No model SDK, Codex task launcher, or script executor
is part of this bridge. No conclusion about dot billing follows from it.

## Components

- `protocol/`: isolated Node protocol reference and offline tests. Its local
  server binds only to loopback and deliberately has no live callback transport.
- `sites-overlay/`: the actual custom application source from the verified
  Sites deployment, formatted for this repository. It includes MCP discovery,
  webhook challenge/signing, durable D1 tasks/subscriptions, tools, and console.
- `send-probe.py`: Linux/macOS one-shot transport for a hash-pinned Beaver
  outbox, with explicit request ID and verified private Site origin. It refuses
  redirects, reads an already approved one-use credential without echo from
  stdin, and never saves it. This generalized transport has offline tests; the
  earlier fixed-path variant performed the real pilot.

## Reuse the hosted implementation

1. Initialize a current official Sites Vinext starter using the Sites plugin.
   The verified starter used Vinext 1.0.0-beta.5, Vite 8.0.13, React 19.2.6,
   Drizzle ORM 0.45.2, and Cloudflare Vite plugin 1.37.1.
2. Copy `sites-overlay/` over that starter, preserving its official build,
   authentication/dispatch, vendor, dependency-lock, and runtime helper files.
   The overlay intentionally is not a standalone Node application. Keep the
   starter's `drizzle.config.ts` and bind D1 as `DB`.
3. Set the supported hosting manifest to `d1: "DB"`, `r2: null`, and
   `capabilities: ["mcp"]` using the project setup workflow. Obtain the project
   ID from the new Site, never reuse someone else's deployment identity.
4. Use the official install, typecheck, build, migration, source-save, and
   private-deploy workflow. Keep the Site owner-private. Do not expose these
   routes publicly: `/api/demo` relies on trusted private-Site dispatch, and
   D1 rows are intentionally not multi-tenant. Raw caller-supplied auth headers
   are not an authentication scheme outside that trusted hosting boundary.
5. Connect the resulting Site plugin and explicitly authorize the bounded
   `beaver.demo.ready` event with `{ "queue_id": "smoke-test" }`.
   Callback URL and secret come from the platform subscription protocol.
   Never invent callbacks, scrape platform credentials, or commit secrets.

Callback hosts are restricted to `chatgpt.com`, `api.chatgpt.com`, and
`connectors.api.openai.com`; redirects are rejected. Subscriptions have a
maximum one-hour lifetime and need platform refresh. The code signs the exact
JSON payload with HMAC-SHA256 using webhook ID and timestamp. Challenge echo
must succeed before subscription persistence.

## One approved test

Have Beaver's existing external-agent `file.write` produce a JSON outbox with:
`request_id`, `queue_id: "smoke-test"`, `kind: "connectivity_check"`, and
`expected_result: "BEAVER_DOT_SMOKE_OK"`. Record its exact SHA-256.

Invoke `python3 send-probe.py --outbox <path> --sha256 <digest>
--request-id <approved-id> --site <verified-private-origin> --receipt <new-path>`.
Provide the approved one-use platform Site credential only when its hidden
stdin prompt is ready. No credential provisioning or approval is performed by
this helper. Use a fresh receipt path and verify access before transport.

Keep three independent proof stages:

1. Actual matching event reaches the intended current dot conversation.
2. Connected `get_demo_task` reads the pending fixed-marker request.
3. `submit_demo_result` writes `BEAVER_DOT_SMOKE_OK`; a separate
   `get_demo_task` confirms `completed` and the exact marker.

HTTP receipt alone proves none of stages 2–3. The saved automation must ignore
unapproved request IDs. Event contents never authorize broader actions.
The UI's random-ID test button does not expand an exact-ID authorization.
Leave persistent webhook lifecycle under its explicit automation instructions.

## Verified pilot and limits

The live run on 2026-10-05 reached this exact three-stage boundary: real event
received at 10:26 UTC, plugin read of pending request, marker submitted and
independent completed readback at 10:27:41 UTC. This proof belongs to the
hosted implementation, not the simulated protocol tests.

Import/acknowledgment into Beaver Core, unattended outbox watching, retry
scheduling, arbitrary jobs, and automatic model generation remain unimplemented.
A future executor adapter needs explicit authorization and the existing
Beaver task/run/revision/request identity checks. Do not execute returned text.

## Offline checks

- `cd protocol && npm test`
- `python3 -m unittest discover -s integrations/dot-bridge -p 'test_*.py'`
  from repository root
- Typecheck/build the overlay inside the official starter, not on its own.

No real callbacks, model services, or credentials are used by these tests.
