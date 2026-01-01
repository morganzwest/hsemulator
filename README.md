# Local HubSpot Emulator

`hsemulate` is a small CLI tool that lets you run **HubSpot Workflow Custom Code Actions**
(JavaScript or Python) **locally on your computer**, using the _exact same file_ you would paste into HubSpot.

It is designed for:

- Local testing
- Debugging errors before deploying
- Adding assertions, budgets, and snapshots
- Avoiding trial-and-error in the HubSpot UI

---

## Requirements

You must have the following installed:

- **Node.js** (for JavaScript actions)
- **Python 3** (for Python actions)
- **Git**
- **Rust** (only if running from source)

Check versions:

```bash
node --version
python --version
cargo --version
```

---

## Getting Started

### 1. Clone the repository

```bash
git clone <repo-url>
cd hsemulate
```

---

### 2. Initialise a project

This creates:

- `config.yaml`
- `fixtures/event.json`
- `actions/action.js` or `actions/action.py`
- `assertions.json`

JavaScript:

```bash
cargo run -- init js
```

Python:

```bash
cargo run -- init python
```

---

## Running an action

### Basic run

```bash
cargo run -- run actions/action.js --config config.yaml
```

or

```bash
cargo run -- run actions/action.py --config config.yaml
```

This:

- Loads the fixture as the HubSpot `event`
- Runs the action locally
- Shows logs
- Prints a result summary
- Exits with `0` (success) or `1` (failure)

---

## Fixtures (input data)

Fixtures simulate the HubSpot event payload.

Default location:

```
fixtures/event.json
```

To use a different fixture:

```bash
cargo run -- run actions/action.js --config config.yaml --fixture other_event.json
```

---

## Output modes

Controlled in `config.yaml`:

```yaml
output:
  mode: simple # stdout | pretty | simple | file
```

### Modes

- **simple** (recommended):
  Human-friendly summary with colours

- **pretty**:
  Pretty-printed JSON output

- **stdout**:
  Compact JSON (best for CI)

- **file**:
  Writes JSON to a file

```yaml
output:
  mode: file
  file: results.json
```

---

## Assertions

Assertions let you fail runs if output is not what you expect.

Example `assertions.json`:

```json
{
  "callback.outputFields.success": true
}
```

Run with assertions:

```bash
cargo run -- run actions/action.js --config config.yaml --assert assertions.json
```

If any assertion fails, the run fails.

---

## Snapshots

Snapshots store the full output and compare future runs against it.

Create a snapshot (first run):

```bash
cargo run -- run actions/action.js --config config.yaml --snapshot
```

On future runs:

- Output must match the snapshot
- Differences cause a failure

Snapshots are stored in:

```
snapshots/
```

---

## Budgets (time & memory)

Set limits in `config.yaml`:

```yaml
budgets:
  duration_ms: 500
  memory_mb: 64
```

If the action exceeds these:

- The run fails
- You see which budget was exceeded

---

## Flaky detection

To detect non-deterministic behaviour:

```bash
cargo run -- run actions/action.js --config config.yaml --repeat 3
```

If outputs differ between runs, the action is marked **flaky** and fails.

---

## Common failure types

### Runtime error

- Syntax error
- File cannot be loaded
- Node/Python crashes

### Action error

- Exception thrown inside `main()`

### Assertion failure

- Output does not match expectations

### Budget failure

- Took too long
- Used too much memory

All errors are shown clearly in the output.

---

## Exit codes

- `0` → success
- `1` → failure

This makes the tool safe for:

- CI pipelines
- Scripts
- Automation

---

## Typical workflow

1. Write or edit your action (`actions/action.js`)
2. Adjust `fixtures/event.json`
3. Run locally
4. Fix errors
5. Add assertions
6. Add snapshot (optional)
7. Paste into HubSpot with confidence

---

## Notes

- The action file is run **exactly as written**
- No HubSpot APIs are mocked
- Environment variables come from `config.yaml`
- This tool is stricter (and more helpful) than HubSpot’s UI

---

## Summary

`hsemulate` lets you treat HubSpot custom code like real code:

- testable
- repeatable
- debuggable
- predictable

Use it before every deploy.
