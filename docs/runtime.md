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
