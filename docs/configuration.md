# Configuration (`config.yaml`)

`config.yaml` defines how hsemulator runs your action.

It is declarative and explicit by design—there is no hidden or global configuration.

---

## Minimal Example

```yaml
version: 1

action:
  type: js
  entry: actions/action.js

fixtures:
  - fixtures/event.json

runtime:
  node: node

output:
  mode: stdout
```

This is the smallest valid configuration.

---

## Full Example

```yaml
version: 1

action:
  type: js
  entry: actions/action.js

fixtures:
  - fixtures/event.json

env:
  HUBSPOT_TOKEN: "pat-your-token-here"
  HUBSPOT_BASE_URL: "https://api.hubapi.com"

runtime:
  node: node
  python: python

output:
  mode: stdout

snapshots:
  enabled: true
  ignore:
    - output.timestamp
    - meta.runId

budgets:
  duration_ms: 500
  memory_mb: 128
```

---

## Top-Level Fields

### `version`

Configuration schema version.

Currently must be set to `1`.

---

## `action`

Defines the custom code action to execute.

```yaml
action:
  type: js | python
  entry: actions/action.js
```

* `type` determines which runtime shim is used
* `entry` is the file executed for each fixture

The code should be identical to what is pasted into HubSpot.

---

## `fixtures`

List of event payloads to execute.

```yaml
fixtures:
  - fixtures/event.json
```

* Each fixture is run independently
* Multiple fixtures result in multiple executions
* Failures are aggregated into the final result

---

## `env`

Environment variables injected at runtime.

```yaml
env:
  HUBSPOT_TOKEN: "pat-..."
```

* Values are exposed to the action exactly as environment variables
* Useful for tokens, base URLs, or feature flags
* Secrets should **not ** be committed to version control

---

## `runtime`

Defines how the action is executed on your system.

```yaml
runtime:
  node: node
  python: python
```

* Values must resolve via your system `PATH`
* Only the runtime matching `action.type` is used
* No sandboxing is applied beyond the process boundary
* HubSpot currently only uses Python 3.9, this is not enforced locally

---

## `output`

Controls how execution output is emitted.

```yaml
output:
  mode: stdout
  # file: results.json
```

---

### `mode`

Determines how logs and the final execution summary are written.

Supported modes:

* **`stdout`**
  Writes logs and the final summary directly to standard output.
  Best for local development.

* **`simple`**
  Reduced, human-readable output focused on pass/fail status.
  Suitable for quick feedback.

* **`pretty`**
  Expanded, formatted output including execution details and timings.
  Intended for interactive debugging.

* **`file`**
  Writes output to a file instead of stdout.
  Requires `file` to be specified.

Example:

```yaml
output:
  mode: file
  file: results.json
```

---

### Notes

* All modes produce **human-readable output**
* `file` mode is recommended for CI pipelines
* Exit codes are consistent across all modes
* Output format stability is guaranteed within a major version

---

## `snapshots`

Controls snapshot testing behaviour.

```yaml
snapshots:
  enabled: true
  ignore:
    - output.timestamp
```

* When enabled, output is compared to stored snapshots
* Ignored paths are excluded from comparisons
* Snapshot mismatches fail the run

---

## `budgets`

Defines execution limits.

```yaml
budgets:
  duration_ms: 500
  memory_mb: 128
```

* Violations cause the run to fail
* Useful for detecting performance regressions
* Applied per execution, per fixture
* Memory budgets are **experimental** and may result in unexpected behavior.


---

## Validation Rules

* Unknown fields are rejected
* Missing required fields fail fast
* Paths are resolved relative to `config.yaml`

This is intentional to keep behaviour predictable.
