# Budgets & Limits

Budgets enforce execution limits on actions and cause runs to fail when those limits are exceeded.

They are designed to catch performance regressions early, **not** to precisely emulate HubSpot’s runtime limits.

---

## Supported Budgets

hsemulator currently supports two budget types:

* **Time budget** (`duration_ms`)
* **Memory budget** (`memory_mb`)

Both budgets are optional and may be enabled independently.

---

## Configuring Budgets

Budgets can be defined in `config.yaml`:

```yaml
budgets:
  duration_ms: 500
  memory_mb: 128
```

Or overridden via CLI:

```bash
hsemulator run --budget-time 500 --budget-mem 128
```

CLI values override any budgets defined in `config.yaml`.

---

## Time Budget (`duration_ms`)

### What It Measures

The time budget measures:

* Wall-clock execution time of the action process
* From process spawn to completion
* Per run, per fixture

It includes:

* Runtime startup
* Shim execution
* User code execution

---

### Important Caveats

Time budgets:

* **Are not a precise match for HubSpot execution limits**
* **Vary significantly by local machine performance**
* **Have not been extensively benchmarked against HubSpot**

Factors that affect timing:

* CPU speed and load
* Disk performance
* Node/Python runtime version
* OS scheduling

As a result, time budgets should be treated as **relative guards**, not absolute guarantees.

---

### Recommended Usage

Use time budgets to:

* Detect accidental slowdowns
* Prevent runaway loops
* Enforce “reasonable” execution bounds

Do **not** use them to:

* Assert compliance with HubSpot’s exact limits
* Compare performance across machines

---

## Memory Budget (`memory_mb`)

### What It Measures

The memory budget measures:

* Peak resident set size (RSS)
* In kilobytes, converted from `memory_mb`
* Per run, per fixture

---

### Experimental Status

⚠️ **Memory budgets are experimental.**

Important limitations:

* Measurement relies on OS-level process inspection
* Accuracy varies by platform
* Short-lived spikes may be missed
* Behaviour may differ between local and CI environments

Memory budgets should be considered **best-effort signals**, not strict enforcement.

---

### Recommended Usage

Use memory budgets to:

* Catch obvious leaks or runaway allocations
* Flag unexpectedly large memory usage

Do **not** rely on them for:

* Precise memory profiling
* Hard enforcement against HubSpot limits

---

## Enforcement Behaviour

Budgets are enforced **after execution**, in this order:

1. Time budget
2. Memory budget

If a budget is exceeded:

* The run is marked as failed
* A failure message is recorded
* In CI mode, execution stops immediately

Example failure:

```
[fixtures/event.json] Budget failed: duration exceeded (742ms > 500ms)
```

---

## Interaction With Repeats

When `repeat > 1`:

* Budgets are enforced on **every run**
* A single budget violation fails the entire run
* Useful for detecting intermittent performance spikes

---

## Budgets vs Assertions and Snapshots

Budgets are evaluated:

1. After assertions
2. Before snapshots are compared

A budget failure is treated the same as:

* Assertion failure
* Snapshot mismatch

All are fatal.

---

## Best Practices

* Start with generous limits
* Tighten budgets gradually
* Prefer assertions for correctness
* Prefer snapshots for regression detection
* Treat budgets as **early warning systems**, not exact replicas of HubSpot

---

## Summary

* Budgets are optional but powerful
* Time budgets are approximate and machine-dependent
* Memory budgets are experimental
* CLI overrides always win
* Any budget violation fails the run