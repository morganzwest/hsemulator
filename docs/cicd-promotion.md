# CI/CD and Promotion

CI/CD Promotion enables **safe, test-gated deployment of HubSpot custom code actions** directly from `hsemulator`.

It allows code that has been **tested locally and in CI** to be **promoted into existing HubSpot workflows** without manual copy/paste, while enforcing deterministic safety checks.

This feature turns `hsemulator` from a local runner into a **complete developer workflow**.

---

## What Promotion Is (and Is Not)

### Promotion **is**:

- A controlled update of **existing** HubSpot custom code actions
- Gated by local or CI test results
- Deterministic and explicit
- Driven by configuration, not flags
- Designed for Git-based workflows

### Promotion **is not**:

- A workflow designer
- A general HubSpot automation tool
- A way to create actions or workflows
- A blind deployment mechanism

Promotion assumes the workflow and action already exist.

---

## Commands

### Initialise CI/CD

```bash
hsemulator cicd init
hsemulator cicd init action
hsemulator cicd init action --branch main
```

This scaffolds:

```
.hsemulator/
  cicd.yaml
.github/
  workflows/
    hsemulator.yml   # optional
```

---

### Promote Code

```bash
hsemulator promote <target>
```

Force promotion (skip tests and drift checks):

```bash
hsemulator promote <target> --force
```

---

## High-Level Promotion Flow

When running `hsemulator promote`, the following steps occur:

1. Load `.hsemulator/cicd.yaml`
2. Resolve HubSpot authentication
3. Validate target configuration
4. (Optional) Enforce test gate
5. Load local action source
6. Compute deterministic code hash
7. Fetch the target workflow from HubSpot
8. Locate the target action via selector
9. Apply drift and safety checks
10. Update the workflow using a revision-safe PUT

If **any step fails**, promotion stops immediately.

---

## Configuration (`cicd.yaml`)

Promotion is driven entirely by `.hsemulator/cicd.yaml`.

Example:

```yaml
version: 1

hubspot:
  token: ''

targets:
  production:
    workflow_id: '3549922549'

    selector:
      type: secret
      value: HS_ACTION__CONTACT_RENAME__PROD
      require_unique: true

    runtime: PYTHON39

    safety:
      require_clean_tests: true
      require_snapshot_match: true
      max_duration_ms: 4000
      max_memory_mb: 128

    deploy:
      mode: full-flow-replace
      dry_run: false
```

---

## Authentication

Promotion requires a HubSpot Private App token.

### Preferred (CI/CD)

```bash
export HUBSPOT_TOKEN=pat-...
```

### Fallback (local only)

```yaml
hubspot:
  token: 'pat-...'
```

⚠️ **Using tokens in `cicd.yaml` is insecure** and should only be done for local testing.

---

## Target Selection

Promotion targets an action using a **selector**.

### Supported Selector

```yaml
selector:
  type: secret
  value: HS_ACTION__CONTACT_RENAME__PROD
```

The selector matches against the action’s `secretNames` field.

Promotion **fails** if:

- No actions match
- More than one action matches
- The action is not a `CUSTOM_CODE` action

This guarantees deterministic targeting.

---

## Test Gating

By default, promotion is **blocked unless tests have passed**.

Promotion requires:

- `.hsemulator/last-test.json` to exist
- `ok: true`
- `snapshots_ok: true` (unless disabled)

Safety rules can be configured per target:

```yaml
safety:
  require_clean_tests: true
  require_snapshot_match: true
  max_duration_ms: 4000
  max_memory_mb: 128
```

If safety checks fail, promotion is refused.

---

## `--force` Mode

`--force` disables **all safety gates**, including:

- Test enforcement
- Snapshot enforcement
- Hash drift protection

It still requires:

- `workflow_id`
- `selector`
- HubSpot authentication

`--force` exists for emergency recovery and manual overrides.

---

## Drift Protection (Hash Markers)

`hsemulator` embeds a deterministic hash marker into promoted code:

```python
# hsemulator-sha: abc123...
```

or

```js
// hsemulator-sha: abc123...
```

### Behaviour

- If the existing hash matches → no-op
- If the hash differs → normal update
- If no marker exists:

  - Block promotion (default)
  - Allow only with `--force`

This prevents accidental overwrites of unknown or manually edited code.

---

## Dry-Run Mode

Enable dry-run to preview changes without mutating HubSpot:

```yaml
deploy:
  dry_run: true
```

Dry-run outputs a machine-readable summary and exits without performing a PUT.

---

## Failure Modes (Intentional)

Promotion **fails loudly** when:

- Tests have not been run
- Tests failed
- Snapshots mismatch
- Selector is ambiguous
- Workflow revision conflicts
- Action origin is unknown (without `--force`)
- HubSpot API returns an error

There are **no silent retries** or auto-healing behaviours.

---

## CI/CD Usage (GitHub Actions)

Generated workflows are intentionally minimal:

```yaml
- name: Run tests
  run: ./hsemulator test

- name: Promote
  if: success()
  run: ./hsemulator promote production
  env:
    HUBSPOT_TOKEN: ${{ secrets.HUBSPOT_TOKEN }}
```

All behaviour is driven by `cicd.yaml`, not CLI flags.

---

## Best Practices

- Treat Git as the source of truth
- Never commit real tokens
- Use secrets for selectors
- Avoid `--force` in CI
- Promote only from clean `main` branches
- Keep promotion deterministic and boring

---

## Summary

- Promotion updates **existing** HubSpot custom code actions
- It is deterministic, explicit, and test-gated
- Safety checks prevent accidental overwrites
- Hash markers provide drift protection
- `--force` exists, but is intentionally dangerous
- No workflow orchestration or magic is performed

This feature is designed to feel more like **`terraform apply`** than a deployment script — explicit, safe, and developer-owned.
