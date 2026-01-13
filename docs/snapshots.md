# Snapshots

Snapshots capture the structured output of an action and compare future runs against a stored baseline to detect regressions.

They are optional but strongly recommended for non-trivial logic.

---

## What a Snapshot Is

A snapshot is the **full JSON output envelope** produced by a successful action run.

It includes:

- `ok` status
- `meta` (timings, memory, fixture context)
- `output` (the action’s returned payload)
- `failures` (if present)

Snapshots are compared **byte-for-byte at the JSON structure level**, not textually.

---

## Enabling Snapshots

Snapshots can be enabled in three ways:

### Via `config.yaml`

```yaml
snapshots:
  enabled: true
```

---

### Via CLI

```bash
hsemulate run --snapshot
```

This forces `snapshots.enabled = true` for the run.

---

### Automatically in CI Mode

```bash
hsemulate test
```

CI mode always enables snapshots.

---

## Snapshot Storage Location

Snapshots are stored in the `snapshots/` directory at the project root.

Example:

```text
snapshots/
└── <snapshot-key>.json
```

The directory is created automatically if it does not exist.

---

## Snapshot Key Generation

Each snapshot is uniquely keyed by:

- The canonical action file path
- The fixture path

This means:

- Each action + fixture pair has its own snapshot
- Different fixtures never share snapshots
- Changing the action file path results in a new snapshot

---

## Baseline Creation

On the **first run** with snapshots enabled:

- If no snapshot exists:

  - The current output is written as the baseline
  - The run passes

- No comparison is performed

This allows snapshots to be adopted incrementally.

---

## Snapshot Comparison

On subsequent runs:

- The current output is compared against the stored snapshot
- Any difference causes a failure

Example failure:

```
Snapshot mismatch (snapshots/abc123.json): value changed at output.result.total
```

---

## Snapshot Comparison Rules

- Comparison is structural (parsed JSON)
- Ordering differences are detected
- Missing or additional fields cause failure
- All differences are treated as regressions

There is no fuzzy or tolerance-based comparison.

---

## Ignore Rules (Current Status)

`config.yaml` supports an `ignore` field:

```yaml
snapshots:
  enabled: true
  ignore:
    - output.timestamp
    - meta.runId
```

However, **ignore rules are not yet applied during comparison**.

They are reserved for future implementation and currently have no effect.

You should treat all snapshot comparisons as strict.

---

## Interaction With Repeats

When `repeat > 1`:

- The snapshot baseline is created from the **first run**
- Subsequent runs are compared against that baseline
- Any variation between repeats causes failure

This makes snapshots a powerful flaky-behaviour detector.

---

## Interaction With Assertions and Budgets

Snapshot comparison occurs:

1. After action execution
2. After assertions
3. After budgets
4. Before output emission

A snapshot mismatch is treated the same as an assertion or budget failure.

---

## Updating Snapshots

There is no automatic snapshot update mode.

To update snapshots intentionally:

1. Delete the relevant snapshot file(s)
2. Re-run with snapshots enabled
3. New baselines will be created

This makes snapshot updates explicit and deliberate.

---

## When to Use Snapshots

Use snapshots when:

- Output is complex or nested
- Multiple fields change together
- You want regression protection without many assertions

Avoid snapshots when:

- Output contains unavoidable non-determinism
- You only care about a small number of invariants

In those cases, prefer assertions.

---

## Summary

- Snapshots are strict and deterministic
- Baselines are created automatically
- Comparisons are structural and exact
- Ignore rules are planned but not yet enforced
- Snapshot mismatches are fatal
