# Runtime HTTP API

`hsemulator` exposes a **minimal HTTP runtime** for **validation and execution** of HubSpot custom code actions.

The runtime is intentionally **thin**.
All logic, safety rules, and execution behaviour live in the **engine**, not the API layer.

The HTTP server exists to:

1. Validate configuration safely
2. Execute actions deterministically
3. Act as a control-plane target for orchestration

---

## Philosophy

The runtime follows these principles:

* **One execution engine**
* **No duplicated logic**
* **Validation is first-class**
* **No implicit execution**
* **No hidden defaults**
* **Local, CLI, and HTTP behave identically**

If a config is invalid locally, it is invalid over HTTP.

---

## Starting the Runtime

Start the runtime server using:

```bash
hsemulate runtime
```

Or specify a listen address:

```bash
hsemulate runtime --listen 0.0.0.0:8080
```

By default, the server listens on:

```
http://127.0.0.1:8080
```

---

## Authentication

All runtime endpoints (except `/health`) are protected by **API key authentication**.

### Header

```http
Authorization: Bearer <API_KEY>
```

The API key is validated by the runtime middleware.

Requests without a valid API key will be rejected.

---

## Endpoints Overview

| Endpoint    | Purpose                             |
| ----------- | ----------------------------------- |
| `/health`   | Liveness probe                      |
| `/validate` | Validate config only (no execution) |
| `/execute`  | Validate + execute (or dry-run)     |

---

## `GET /health`

Health check endpoint.

### Request

```http
GET /health
```

### Response

```text
ok
```

This endpoint:

* Requires **no authentication**
* Performs **no checks**
* Exists only for liveness probes

---

## `POST /validate`

Validate a configuration **without executing any code**.

This endpoint performs **static validation only**:

* Schema correctness
* Required fields
* File existence
* Runtime configuration
* Budget sanity checks

No runtimes are spawned.

---

### Request Body

`/validate` accepts a **raw config object**.

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

No `mode` field is supported here.

---

### Successful Response

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

Validate and execute a configuration.

This endpoint is the **canonical execution API** and supports **execution modes**.

Execution **always runs validation first**.

---

### Request Body

`/execute` accepts a **wrapped request object**.

```json
{
  "mode": "execute",
  "config": {
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
}
```

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

This performs **full validation** and **returns immediately** without executing code.

---

### Successful Execution Response

```json
{
  "mode": "execute",
  "execution_id": "exec_abc123",
  "result": {
    "ok": true,
    "runs": 1,
    "failures": [],
    "max_duration_ms": 842,
    "max_memory_kb": 43120,
    "snapshots_ok": true
  }
}
```

---

### Validation Failure via `/execute`

If validation fails, execution is **short-circuited** automatically:

```json
{
  "mode": "validate",
  "execution_id": "exec_abc123",
  "valid": false,
  "errors": [
    {
      "code": "FIXTURE_INVALID_JSON",
      "message": "Fixture is not valid JSON: fixtures/event.json"
    }
  ]
}
```

No code is executed.

---

## Relationship Between `/validate` and `/execute`

| Endpoint                      | Behaviour            |
| ----------------------------- | -------------------- |
| `/validate`                   | Convenience endpoint |
| `/execute { mode: validate }` | Canonical dry-run    |
| `/execute {}`                 | Validate + execute   |

Both use the **same validation engine**.

There is **no behavioural drift** between them.

---

## Error Handling

The runtime returns:

* `200 OK` for successful validation or execution
* `400 Bad Request` for invalid requests or execution errors
* `401 Unauthorized` for missing or invalid API keys

Errors are returned in a structured JSON format suitable for automation.

---

## Determinism Guarantees

The runtime guarantees:

* Validation and execution logic is identical to CLI
* No execution without prior validation
* No runtime processes spawned during dry-run
* No implicit retries or side effects

---

## When to Use the Runtime

The HTTP runtime is intended for:

* Control-plane orchestration
* Managed runners
* UI-driven configuration flows
* Preflight validation
* Remote execution

It is **not** intended to replace the CLI for local development.

---

## Best Practices

* Use `/validate` for UI or CI preflight checks
* Use `/execute { mode: validate }` when building orchestration systems
* Treat `/execute` as the canonical API
* Never rely on implicit defaults
* Do not bypass validation