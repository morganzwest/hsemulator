# GitHub Actions Integration

`hsemulator` integrates cleanly with **GitHub Actions** to provide a **test-gated, deterministic promotion pipeline** for HubSpot custom code actions.

The GitHub Actions workflow is intentionally **thin**.
All behaviour, safety rules, and deployment logic live in `.hsemulator/cicd.yaml`.

GitHub Actions is used only to **orchestrate**:

1. Checkout code
2. Run tests
3. Promote on success

---

## Philosophy

The GitHub Actions integration follows these principles:

* **Configuration lives in Git**, not YAML sprawl
* **No flags in CI** — everything is declarative
* **Promotion only happens after tests pass**
* **The workflow should be boring and obvious**
* **Local and CI behaviour must be identical**

If it works locally, it works in CI.

---

## Generating a Workflow

You can scaffold a GitHub Actions workflow using:

```bash
hsemulate cicd init action
```

Optionally specify a branch:

```bash
hsemulate cicd init action --branch main
```

This creates:

```
.github/
  workflows/
    hsemulator.yml
```

The workflow is only generated when explicitly requested.

---

## Generated Workflow (Example)

The default generated workflow looks like this:

```yaml
name: hsemulator

on:
  push:
    branches: [main]

jobs:
  test-and-promote:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install hsemulator (latest Linux)
        shell: bash
        run: |
          set -e

          echo "Fetching latest hsemulator Linux release…"

          DOWNLOAD_URL=$(curl -s https://api.github.com/repos/morganzwest/hsemulator/releases/latest \
            | jq -r '.assets[] | select(.name | test("linux-x64")) | .browser_download_url')

          if [ -z "$DOWNLOAD_URL" ]; then
            echo "❌ Linux x64 asset not found in latest release"
            exit 1
          fi

          curl -L "$DOWNLOAD_URL" -o hsemulator
          chmod +x hsemulator

      - name: Run tests
        run: ./hsemulator test
        env:
          HUBSPOT_TOKEN: ${{ secrets.HUBSPOT_TOKEN }}

      - name: Promote
        run: ./hsemulator promote production
        env:
          HUBSPOT_TOKEN: ${{ secrets.HUBSPOT_TOKEN }}
```

There is **no environment-specific logic** inside the workflow itself.

---

## Secrets Configuration

### Required Secret

The workflow requires a HubSpot Private App token:

```
HUBSPOT_TOKEN
```

Configure this in GitHub:

**Repository → Settings → Secrets and variables → Actions → New repository secret**

```
Name:  HUBSPOT_TOKEN
Value: pat-xxxxxxxx
```

This token must have permission to update workflows containing the target custom code action.

---

## How Promotion Is Gated in CI

Promotion in GitHub Actions is automatically gated by:

1. `hsemulator test`
2. `.hsemulator/last-test.json`
3. Safety rules defined in `cicd.yaml`

Promotion will **not run** if:

* Tests fail
* Snapshots mismatch
* Duration or memory exceed configured limits
* The target action cannot be resolved
* Drift protection fails

There is **no need** to add conditional logic in GitHub Actions.

---

## Why No `--force` in CI

The generated workflow **never uses `--force`**.

This is intentional.

`--force` exists only for:

* Emergency recovery
* Manual ownership takeover
* Explicit local overrides

Using `--force` in CI defeats the purpose of test-gated promotion and is strongly discouraged.

---

## Branch Strategy

A typical setup:

* `main` → production promotion
* Feature branches → tests only
* Promotion only happens on `main`

Example:

```yaml
on:
  push:
    branches: [main]
```

If you want environment-based promotion (e.g. staging vs production), define **multiple targets** in `cicd.yaml` and multiple workflows or jobs — not flags.

---

## Multiple Environments

Use **targets**, not workflows:

```yaml
targets:
  staging:
    workflow_id: "123"
    selector: { ... }

  production:
    workflow_id: "456"
    selector: { ... }
```

Then promote explicitly:

```bash
hsemulate promote staging
hsemulate promote production
```

CI should remain declarative and explicit.

---

## Dry-Run in CI

You may enable dry-run for validation-only pipelines:

```yaml
deploy:
  dry_run: true
```

In this mode:

* Tests still run
* Promotion logic executes
* No PUT request is sent to HubSpot
* A machine-readable summary is printed

This is useful for pull-request validation workflows.

---

## Execution Budgets (Time & Memory)

`hsemulator` enforces **execution budgets** during CI promotion to prevent performance regressions and unsafe deployments.

These budgets are evaluated from the **last test run** and enforced during promotion.

### Supported Budgets

Budgets are defined per target in `cicd.yaml`:

```yaml
safety:
  max_duration_ms: 4000
  max_memory_mb: 128
```

* **`max_duration_ms`**
  Maximum allowed wall-clock execution time for the action.

* **`max_memory_mb`**
  Maximum allowed resident memory usage.

If a test run exceeds either budget, promotion is blocked.

---

### How Budgets Work in CI

During `hsemulator test`, execution metrics are captured and written to:

```
.hsemulator/last-test.json
```

During `hsemulator promote`, those metrics are compared against the configured budgets:

* If execution time exceeds `max_duration_ms` → promotion fails
* If memory usage exceeds `max_memory_mb` → promotion fails
* If no budget is defined → the check is skipped

Budgets are **enforced identically** locally and in GitHub Actions.

---

### Recommended Budget Values

Typical guidance:

| Environment | `max_duration_ms` | `max_memory_mb` |
| ----------- | ----------------- | --------------- |
| Local dev   | 4000              | 128             |
| CI          | 20000–50000       | 128–256         |

CI environments are noisier and slower, so higher duration budgets are recommended.

---

### Relationship to `--force`

Execution budgets are **hard safety gates**.

* Budgets are enforced by default
* Budgets can be bypassed **only** with `--force`
* `--force` should never be used in CI

---

### Why Budgets Matter

Budgets exist to:

* Catch accidental infinite loops
* Prevent performance regressions
* Enforce predictable runtime behaviour
* Make action performance reviewable in Git

They are **not** meant to exactly replicate HubSpot’s runtime limits — only to detect regressions early.

---

## Failure Behaviour

GitHub Actions will fail immediately if:

* `hsemulator test` fails
* Promotion safety checks fail
* HubSpot returns an error
* The workflow revision guard rejects the update

There are **no retries**, rollbacks, or partial success states.

Failure is explicit and loud by design.

---

## Best Practices

* Keep GitHub Actions minimal
* Never commit tokens
* Never promote from feature branches
* Do not use `--force` in CI
* Treat `.hsemulator/cicd.yaml` as the source of truth
* Keep promotion deterministic and reviewable