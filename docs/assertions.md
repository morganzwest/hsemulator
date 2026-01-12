# Assertions

Assertions validate that an action’s output matches expected conditions.

They are evaluated **after execution** and **before snapshots**, and any failure causes the run to fail.

---

## What Assertions Apply To

Assertions are evaluated against the **parsed JSON output** produced by the runtime shim.

This is the same structured output used for snapshots and file output.

---

## Assertion Sources and Precedence

Assertions can come from one of three places, resolved in this order:

1. **CLI override**

   ```bash
   hsemulator run --assert assertions.json
   ```

2. **`assertions_file` in `config.yaml`**

   ```yaml
   assertions_file: assertions.json
   ```

3. **Inline assertions in `config.yaml`**

   ```yaml
   assertions:
     callback.outputFields.success:
       eq: true
   ```

If no assertions are provided, no assertion checks are performed.

---

## Basic Example

`assertions.json`:

```json
{
  "callback.outputFields.success": { "eq": true }
}
```

This asserts that:

* `callback.outputFields.success` exists
* Its value is exactly `true`

---

## Assertion Structure

Assertions are defined as a JSON object:

```json
{
  "<json-path>": { "<operator>": <value> }
}
```

* The key is a **dot-separated JSON path**
* The value defines one or more assertion operators

---

## Supported Operators

### Equality

```json
{
  "language": { "eq": "node" }
}
```

Fails if the value is not exactly equal.

---

### Regex Match

```json
{
  "language": { "regex": "node|python" }
}
```

Fails if the value does not match the regex.

---

## JSON Path Resolution

Paths are resolved using simple dot notation:

```text
callback.outputFields.success
```

This resolves to:

```json
{
  "callback": {
    "outputFields": {
      "success": true
    }
  }
}
```

Rules:

* Paths must exist
* Missing paths cause an assertion failure
* Arrays must be indexed explicitly (if present)

---

## Multiple Assertions

Multiple assertions are evaluated independently.

```json
{
  "callback.outputFields.success": { "eq": true },
  "language": { "regex": "node|python" }
}
```

* All assertions must pass
* Failures are aggregated per run

---

## Failure Behaviour

When an assertion fails:

* The failure is recorded with context:

  * Fixture path
  * Assertion error
* The run is marked as failed
* In CI mode, execution stops immediately

Example failure message:

```
[fixtures/event.json] Assertion failed: expected callback.outputFields.success == true
```

---

## Assertions vs Snapshots

Assertions and snapshots serve different purposes:

* **Assertions**: explicit, intentional correctness checks
* **Snapshots**: regression detection for broader output changes

Best practice:

* Use assertions for invariants
* Use snapshots for complex or evolving output

---

## When Assertions Run

Assertions are evaluated:

1. After the action completes
2. After `ok == true` is verified
3. Before budgets and snapshots are enforced

If the action returns `ok=false`, assertions are still evaluated.

---

## Default Scaffolded Assertions

When running `hsemulator init`, a default `assertions.json` is created:

```json
{
  "callback.outputFields.success": { "eq": true },
  "language": { "regex": "node|python" }
}
```

This is intended as a minimal correctness check and should be adapted.

---

## Summary

* Assertions are strict and fatal
* CLI assertions override all others
* Missing paths fail
* All assertions must pass
* Assertions are evaluated per fixture and per repeat
