# hsemulator

Local runner for HubSpot Workflow Custom Code Actions.

hsemulator lets you run the exact JavaScript or Python code used in HubSpot workflows locally, with support for fixtures, assertions, snapshots, execution budgets, and flaky-run detection.

The goal is fast iteration and deterministic testing without relying on the HubSpot UI.

---

## When to Use It

Use hsemulator when you want to:

- Develop and debug HubSpot custom code locally
- Validate logic using real workflow event payloads
- Catch regressions with assertions and snapshots
- Enforce execution limits during development or CI
- Reduce copy–paste cycles into HubSpot

---

## Contents

```{toctree}
:maxdepth: 2

getting-started
project-structure
configuration
running-actions
assertions
snapshots
budgets
```

---

## Scope and Non-Goals

hsemulator is intentionally focused.

It does **not** attempt to:

- Fully emulate HubSpot’s runtime environment
- Mock HubSpot APIs or infrastructure
- Replace end-to-end or integration tests

---

## Links

- GitHub repository: [https://github.com/morganzwest/Local-HubSpot-Emulator](https://github.com/morganzwest/Local-HubSpot-Emulator)
- Issues and feature requests: [https://github.com/morganzwest/Local-HubSpot-Emulator/issues](https://github.com/morganzwest/Local-HubSpot-Emulator/issues)

```

```
