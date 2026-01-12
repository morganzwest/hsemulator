# Running Actions

This page describes how hsemulator executes actions, how CLI flags interact with `config.yaml`, and how runs behave internally.

All behaviour described here matches the actual execution model.

---

## Basic Execution

From the project root:

```bash
hsemulator run
```

This will:

1. Load `config.yaml`
2. Apply any CLI overrides
3. Execute the action once per fixture
4. Apply assertions, snapshots, and budgets
5. Emit output according to `output.mode`
6. Exit non-zero if any failure occurs

If any fixture or run fails, the command exits with an error.

---

## Command Variants

### `run` (default)

```bash
hsemulator run
```

* Uses a single `config.yaml`
* Human-oriented output (unless overridden)
* Fails immediately on errors

---

### `test` (CI mode)

```bash
hsemulator test
```

CI-first mode with different semantics:

* Recursively discovers **all** `config.yaml` files
* Forces:

  * `mode = ci`
  * `snapshots.enabled = true`
* Always emits **one stable JSON blob**
* Never prints human-readable logs
* Fails fast on the first failing run per config

This is the recommended entry point for CI pipelines.

---

## Fixtures and Repeats

### Multiple Fixtures

```yaml
fixtures:
  - fixtures/event_1.json
  - fixtures/event_2.json
```

Execution behaviour:

* Each fixture is executed independently
* Failures are reported with fixture context
* Any failure causes the overall run to fail

---

### Repeated Runs (Flaky Detection)

```yaml
repeat: 5
```

Or via CLI:

```bash
hsemulator run --repeat 5
```

Behaviour:

* Each fixture is executed `repeat` times
* Assertions and snapshots are applied per run
* Failures are aggregated
* Used to detect non-deterministic behaviour

There is no warning-only mode. Flaky behaviour is treated as a failure.

---

## CLI Overrides

CLI flags override `config.yaml` values at runtime.

Supported overrides:

* `--action <path>` → overrides `action.entry`
* `--fixture <path>` (repeatable) → overrides `fixtures`
* `--snapshot` → forces `snapshots.enabled = true`
* `--watch` → enables watch mode
* `--repeat <n>` → overrides `repeat`
* `--budget-time <ms>` → overrides `budgets.duration_ms`
* `--budget-mem <mb>` → overrides `budgets.memory_mb`
* `--assert <file>` → overrides the assertions source

Overrides are applied before execution and do not mutate `config.yaml`.

---

## Watch Mode

```bash
hsemulator run --watch
```

Watch mode:

* Re-runs on changes to:

  * `config.yaml`
  * Action entry file
  * Fixture files
* Clears the screen between runs
* Prints a minimal pass/fail summary
* Never exits

This is intended for tight local iteration.

---

## Execution Model (Per Run)

For each fixture and repeat iteration, hsemulator:

1. Writes the fixture JSON to a temporary directory
2. Selects runtime based on file extension:

   * `.js`, `.mjs`, `.cjs` → Node
   * `.py` → Python
3. Injects environment variables
4. Executes the runtime shim
5. Captures:

   * Structured JSON output
   * Execution time
   * Peak memory usage
6. Applies checks in this order:

   1. `ok == true`
   2. Assertions
   3. Budgets
   4. Snapshots
7. Emits output
8. Aggregates failures

---

## Output Emission Rules

Output behaviour depends on execution context and `output.mode`:

* Local runs emit human-readable output unless `output.mode = file`
* `output.mode = file` writes JSON to disk and emits nothing to stdout
* CI mode emits a single JSON blob only
* Watch mode emits minimal pass/fail status

Assertions, snapshots, and budgets always run regardless of output mode.

---

## Exit Codes

Exit codes are stable and CI-friendly:

* `0` – All fixtures and runs passed
* `1` – Any failure (assertion, snapshot, budget, or runtime error)

CI mode always exits non-zero on failure.

---

## Determinism Expectations

hsemulator is intentionally strict.

To avoid failures:

* Avoid random or time-based logic
* Ignore non-deterministic snapshot fields
* Treat flaky behaviour as a defect, not a warning

---

## Summary

* `run` is developer-focused and interactive
* `test` is CI-first and JSON-only
* CLI overrides are explicit and deterministic
* Failures are contextual, aggregated, and fatal
