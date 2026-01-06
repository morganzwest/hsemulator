# Local HubSpot Emulator

> ⚠️ **Pre-release (v0.1.0)**
>
> This is an early release intended for developer use.
> While the core execution, assertions, and snapshot features are stable,
> the CLI surface and configuration schema may change without notice.
>
> Use in production pipelines at your own discretion.
>
> Feedback and bug reports are encouraged.


`hsemulate` is a lightweight CLI tool that lets you run **HubSpot Workflow Custom Code Actions**
(JavaScript or Python) **locally on your computer**, using the *exact same file* you would paste into HubSpot.

It is designed for:

- Local testing
- Debugging before deploying to HubSpot
- Adding assertions, budgets, and snapshots
- Avoiding slow trial-and-error in the HubSpot UI

Once installed, `hsemulate` is available globally from your terminal.

---

## Requirements

You must have the following installed:

- **Node.js** (for JavaScript actions)
- **Python 3** (for Python actions)
- **Git** (optional, but recommended)

Check versions:

```bash
node --version
python --version
hsemulate --version
````

---

## Installation

Download and run the **Windows installer (`.exe`)**.

The installer will:

* Install `hsemulate`
* Add it to your system `PATH`
* Allow clean uninstall via Windows Apps

After installation, restart your terminal and verify:

```bash
hsemulate --help
```

---

## Getting Started

### 1. Create a new project

From any directory:

JavaScript action:

```bash
hsemulate init js
```

Python action:

```bash
hsemulate init python
```

This creates a ready-to-run structure:

```
.
├─ actions/
│  └─ action.js | action.py
├─ fixtures/
│  └─ event.json
├─ config.yaml
├─ assertions.json
```

---

## Running an action

### Basic run

JavaScript:

```bash
hsemulate run actions/action.js --config config.yaml
```

Python:

```bash
hsemulate run actions/action.py --config config.yaml
```

This will:

* Load the fixture as the HubSpot `event`
* Run the action locally
* Stream logs
* Print a clear summary
* Exit with `0` (success) or `1` (failure)

---

## Fixtures (input data)

Fixtures simulate the HubSpot event payload.

Default location:

```
fixtures/event.json
```

To use a different fixture:

```bash
hsemulate run actions/action.js --config config.yaml --fixture other_event.json
```

---

## Configuration (`config.yaml`)

`config.yaml` controls how your action is executed locally.
This is where you configure **environment variables, budgets, output behaviour, and runtime options**.

### Example

```yaml
env:
  HUBSPOT_ACCESS_TOKEN: test-token
  API_BASE_URL: https://example.com
  DEBUG: "true"

budgets:
  duration_ms: 500
  memory_mb: 64

output:
  mode: simple
```

---

### Environment variables

Values under `env` are injected at runtime.

```yaml
env:
  MY_SECRET_KEY: abc123
  FEATURE_FLAG: "true"
```

Access them exactly as you would in HubSpot:

* JavaScript:

  ```js
  process.env.MY_SECRET_KEY
  ```

* Python:

  ```py
  os.environ["MY_SECRET_KEY"]
  ```

---

### Budgets (time & memory)

```yaml
budgets:
  duration_ms: 500
  memory_mb: 64
```

If exceeded:

* The run fails
* The exceeded limit is reported
* Exit code is `1`

---

### Output configuration

```yaml
output:
  mode: simple # simple | pretty | stdout | file
```

Write output to a file:

```yaml
output:
  mode: file
  file: results.json
```

---

## Assertions

Assertions let you fail a run if output is not what you expect.

Example `assertions.json`:

```json
{
  "callback.outputFields.success": true
}
```

Run with assertions:

```bash
hsemulate run actions/action.js --config config.yaml --assert assertions.json
```

---

## Snapshots

Snapshots store full output and compare future runs.

Create a snapshot:

```bash
hsemulate run actions/action.js --config config.yaml --snapshot
```

Snapshots are stored in:

```
snapshots/
```

Future runs must match the snapshot or fail.

---

## Flaky detection

Detect non-deterministic behaviour:

```bash
hsemulate run actions/action.js --config config.yaml --repeat 3
```

If outputs differ, the action is marked **flaky**.

---

## Common failure types

### Runtime error

* Syntax error
* File cannot be loaded
* Node or Python crashes

### Action error

* Exception thrown inside `main()`

### Assertion failure

* Output does not match expectations

### Budget failure

* Took too long
* Used too much memory

---

## Exit codes

* `0` → success
* `1` → failure

Safe for CI and automation.

---

## Typical workflow

1. Write or edit your action
2. Update `fixtures/event.json`
3. Update `config.yaml`
4. Run locally
5. Fix errors
6. Add assertions
7. Add snapshot (optional)
8. Paste into HubSpot with confidence

---

## Notes

* Action files are executed **exactly as written**
* No HubSpot APIs are mocked
* Environment variables come from `config.yaml`
* This tool is intentionally stricter than HubSpot’s UI

---

## Summary

`hsemulate` lets you treat HubSpot custom code like real software:

* testable
* repeatable
* debuggable
* predictable

Run locally. Ship once. Paste into HubSpot with confidence.
