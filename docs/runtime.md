# Runtime HTTP API

`hsemulator` exposes a **minimal, engine-backed HTTP runtime** for **validating and executing HubSpot custom code actions**.

The runtime is intentionally **thin**.

All logic, validation rules, execution semantics, and determinism guarantees live in the **engine**, not the API layer.
The HTTP API is a transport and orchestration surface only.

---

## Purpose of the Runtime

The HTTP runtime exists to:

1. Validate action configurations safely
2. Execute actions deterministically
3. Support UI-driven and remote execution
4. Act as a control-plane target for orchestration

It does **not** introduce new behaviour beyond the CLI.

---

## Philosophy

The runtime follows these principles:

* **Single execution engine**
* **No duplicated logic**
* **Validation is mandatory**
* **No implicit execution**
* **No hidden defaults**
* **Inline and filesystem execution behave identically**
* **CLI, CI, and HTTP are behaviourally equivalent**

If a configuration is invalid locally, it is invalid over HTTP.

---

## Starting the Runtime

Start the runtime server with:

```bash
hsemulate runtime
```

Specify a listen address:

```bash
hsemulate runtime --listen 0.0.0.0:8080
```

Default address:

```
http://127.0.0.1:8080
```

---

## Authentication

All endpoints except `/health` are protected by **API key authentication**.

### Header

```http
Authorization: Bearer <API_KEY>
```

Requests without a valid API key are rejected.

Authentication is enforced via middleware before request handling.

---

## Endpoints Overview

| Endpoint    | Purpose                          |
| ----------- | -------------------------------- |
| `/health`   | Liveness probe                   |
| `/validate` | Validate filesystem config only  |
| `/execute`  | Validate + execute inline config |

---

## `GET /health`

Liveness probe.

### Request

```http
GET /health
```

### Response

```text
ok
```

* No authentication required
* No checks performed
* Intended for probes and load balancers only

---

## `POST /validate`

Validate a **filesystem-based config** without executing any code.

This endpoint performs **static validation only**:

* Schema correctness
* Required fields
* Action entry existence
* Fixture existence and JSON validity
* Runtime configuration
* Budget sanity checks

No runtimes are spawned.

---

### Request Body

`/validate` accepts a **filesystem-backed config object**.

```json
{
  "version": 1,
  "action": {
    "type": "js",
    "entry": "actions/action.js"
  },
  "fixtures": [
    "fixtures/event.json"
  ],
  "env": {
    "HUBSPOT_TOKEN": "pat-test-token",
    "HUBSPOT_BASE_URL": "https://api.hubapi.com"
  },
  "runtime": {
    "node": "node",
    "python": "python"
  },
  "snapshots": {
    "enabled": true
  },
  "repeat": 1
}
```

Notes:

* Paths are resolved relative to the runtime working directory
* No `mode` field is accepted
* No execution occurs

---

### Successful Validation Response

```json
{
  "execution_id": "exec_abc123",
  "mode": "validate",
  "valid": true,
  "errors": []
}
```

---

### Validation Failure Response

```json
{
  "execution_id": "exec_abc123",
  "mode": "validate",
  "valid": false,
  "errors": [
    {
      "code": "ACTION_NOT_FOUND",
      "message": "Action entry does not exist: actions/action.js"
    }
  ]
}
```

Validation errors are:

* Deterministic
* Structured
* Stable across CLI, CI, and HTTP

---

## `POST /execute`

Validate and execute an **inline configuration**.

This is the **canonical execution API**.

Execution always runs **validation first**.

---

### Request Body

`/execute` accepts an **inline execution request**.

```json
{
  "mode": "execute",
  "config": {
    "version": 1,

    "action": {
      "language": "js",
      "entry": "actions/action.js",
      "source": "exports.main = async (event) => { return { ok: true }; }"
    },

    "fixtures": [
      {
        "name": "fixtures/event.json",
        "source": "{ \"input\": \"hello\" }"
      }
    ],

    "env": {
      "HUBSPOT_TOKEN": "pat-test-token",
      "HUBSPOT_BASE_URL": "https://api.hubapi.com"
    },

    "runtime": {
      "node": "node",
      "python": "python"
    },

    "snapshots": {
      "enabled": true
    },

    "repeat": 1
  }
}
```

---

### Inline Execution Model

For `/execute`:

1. Inline config is validated (no filesystem access)
2. A temporary workspace is created
3. Action and fixtures are materialised
4. Execution runs via the same engine as CLI
5. All events are collected and returned

There is **no behavioural difference** between inline and filesystem execution.

---

### Execution Modes

| Mode       | Behaviour               |
| ---------- | ----------------------- |
| `execute`  | Validate + execute      |
| `validate` | Validate only (dry-run) |

If `mode` is omitted, it defaults to `execute`.

---

### Dry-Run Example

```json
{
  "mode": "validate",
  "config": { ... }
}
```

* Performs full inline validation
* Does not execute code
* No runtimes spawned

---

### Successful Execution Response

```json
{
  "summary": {
    "status": "executed",
    "execution_id": "exec_abc123",
    "duration_ms": 42
  },
  "events": [
    {
      "kind": "execution_created"
    },
    {
      "kind": "validation_started"
    },
    {
      "kind": "execution_started"
    },
    {
      "kind": "stdout",
      "data": "..."
    },
    {
      "kind": "execution_completed"
    }
  ]
}
```

---

### Validation Failure via `/execute`

If validation fails, execution is **short-circuited**:

```json
{
  "summary": {
    "status": "validation_failed",
    "execution_id": "exec_abc123"
  },
  "events": [
    {
      "kind": "validation_failed"
    }
  ]
}
```

No code is executed.

---


# Promotion (`/promote`)

The `/promote` endpoint updates an existing **HubSpot CUSTOM_CODE workflow action** with tested source code.

It is designed to be:

* Deterministic
* Safe by default
* Fully automatable from CI or a UI
* Compatible with `hsemulate test` + snapshot gating

Promotion **does not create workflows or actions**.
It updates an existing action in place.

---

## What Promotion Does

A promotion performs the following steps:

1. Receives raw action source code (JS or Python)
2. Computes a canonical SHA-256 hash of the source
3. Injects a `hsemulator-sha` marker comment
4. Fetches the target HubSpot workflow
5. Locates the target `CUSTOM_CODE` action by selector
6. Applies drift protection
7. Updates the action source (and runtime if provided)
8. Writes the updated workflow back to HubSpot

All steps are atomic from the caller’s perspective.

---

## Drift Protection

Promotion uses a **hash marker** to ensure ownership and prevent accidental overwrites.

Injected marker (example):

```js
// hsemulator-sha: a3f4c1...
```

or (Python):

```py
# hsemulator-sha: a3f4c1...
```

Rules:

* If the existing action has the same hash → **no-op**
* If the existing hash differs → update proceeds
* If no marker exists:

  * Promotion fails by default
  * `force: true` overrides this protection

This prevents overwriting manually-edited or externally-managed actions.

---

## Endpoint

```
POST /promote
```

This endpoint is protected by the runtime API key middleware.

---

## Request Body

```json
{
  "hubspot_token": "pat-xxxx",
  "workflow_id": "123456789",
  "selector": {
    "type": "secret",
    "value": "HUBSPOT_PRIVATE_APP_TOKEN"
  },
  "runtime": "nodejs18.x",
  "source_code": "// action source here",
  "force": false,
  "dry_run": false
}
```

---

## Request Fields

### `hubspot_token` (required)

HubSpot **private app token** used to fetch and update the workflow.

* Must belong to the portal containing the workflow
* Must have automation/workflow scopes

This token is **not stored** by the runtime.

---

### `workflow_id` (required)

The HubSpot workflow ID containing the target action.

This must be a valid workflow accessible by the token.

---

### `selector` (required)

Identifies the `CUSTOM_CODE` action to update.

Currently supported selector:

```json
{
  "type": "secret",
  "value": "HUBSPOT_PRIVATE_APP_TOKEN"
}
```

This matches against `action.secretNames[]`.

Rules:

* Exactly **one** matching action must be found
* Multiple matches cause failure
* Zero matches cause failure

---

### `runtime` (optional)

Overrides the action runtime in HubSpot.

Example values:

* `nodejs18.x`
* `python3.11`

If omitted, the existing runtime is preserved.

---

### `source_code` (required)

Raw JavaScript or Python source code to promote.

* Hash marker is injected automatically
* Existing marker is replaced if present

---

### `force` (optional, default `false`)

Overrides safety checks.

Effects:

* Allows overwriting actions without a hash marker
* Skips drift ownership protection

Use with caution.

---

### `dry_run` (optional, default `false`)

If `true`:

* No PUT request is sent to HubSpot
* Full validation and hashing still occur
* A summary response is returned

Recommended for CI validation and previews.

---

## Responses

### Success (Dry Run)

```json
{
  "ok": true,
  "dry_run": true,
  "workflow_id": "123456789",
  "hash": "a3f4c1...",
  "action_index": 4
}
```

---

### Success (Promotion Applied)

```json
{
  "ok": true,
  "workflow_id": "123456789",
  "hash": "a3f4c1...",
  "revision_id": "987654321"
}
```

---

### No-Op (Already Up To Date)

```json
{
  "ok": true,
  "status": "noop",
  "hash": "a3f4c1..."
}
```

---

### Failure Examples

**Unauthorized (API key):**

```json
{
  "ok": false,
  "error": "Unauthorized"
}
```

**Invalid selector:**

```json
{
  "ok": false,
  "error": "Only selector.type = 'secret' is supported"
}
```

**Workflow fetch failure:**

```json
{
  "ok": false,
  "error": "HubSpot GET flow failed: 400 Bad Request Invalid request"
}
```

---

## Interaction With Tests and Snapshots

The runtime `/promote` endpoint itself does **not** execute tests.

However, it is designed to be called **only after**:

* `hsemulate test`
* Snapshot comparison
* Budget enforcement
* Assertion validation

The CLI `hsemulate promote` command enforces these gates automatically.

When using `/promote` directly, the caller is responsible for enforcing test discipline.

---

## Safety Guarantees

* No workflow creation
* No action creation
* Deterministic selection
* Ownership protection via hash
* Explicit override required for unsafe writes

Promotion is intentionally strict.

---

## Relationship Between `/validate` and `/execute`

| Endpoint                      | Behaviour                    |
| ----------------------------- | ---------------------------- |
| `/validate`                   | Filesystem config validation |
| `/execute { mode: validate }` | Canonical inline dry-run     |
| `/execute {}`                 | Validate + execute inline    |

Both paths share the **same validation and execution engine**.

---

## Error Handling

The runtime returns:

* `200 OK` for successful validation or execution
* `400 Bad Request` for invalid configs or execution errors
* `401 Unauthorized` for missing or invalid API keys

Errors are structured and machine-readable.

---

## Determinism Guarantees

The runtime guarantees:

* Validation precedes execution
* No execution during dry-run
* No implicit retries
* No hidden side effects
* Identical behaviour across CLI, CI, and HTTP

---

## When to Use the Runtime

The HTTP runtime is intended for:

* UI-driven execution
* Control-plane orchestration
* Managed runners
* Preflight validation
* Remote execution environments

It is **not** intended to replace the CLI for local development.

---

## Best Practices

* Use `/validate` for filesystem-based preflight checks
* Use `/execute { mode: validate }` for inline validation
* Treat `/execute` as the canonical execution API
* Never bypass validation
* Never rely on implicit defaults
