# HubSpot Emulator

**hsemulator** is a local test runner for **HubSpot Workflow Custom Code Actions**.

It allows you to run the _exact JavaScript or Python code_ you paste into HubSpot locally, using fixture events, with support for assertions, snapshots, execution budgets, and flaky-run detection.

The goal is to make developing HubSpot custom code feel closer to Lambda-style local development—without relying on the HubSpot UI for iteration.

---

## When to Use It

Use hsemulator when you want to:

- Develop and debug HubSpot custom code locally
- Validate logic using real workflow event payloads
- Detect regressions with assertions and snapshots
- Enforce performance limits (time / memory)
- Reduce copy–paste cycles into the HubSpot UI

Do **not** use it to:

- Fully emulate HubSpot’s infrastructure
- Mock HubSpot APIs or rate limits
- Replace integration or end-to-end tests _(in development)_

---

## Core Capabilities

- Local execution of JS and Python custom code actions
- Fixture-based event input
- Assertions against output and metadata
- Snapshot testing for regression detection
- Execution budgets (time and memory)
- Flaky-run detection via repeat execution
- Machine-readable summary output (CI-friendly)

---

## Documentation

```{toctree}
:maxdepth: 1

getting-started
project-structure
configuration
running-actions
assertions
snapshots
budgets
cicd-promotion
```

---

## Scope and Non-Goals

hsemulator is intentionally focused.

It does **not** attempt to:

- Fully reproduce HubSpot’s runtime environment
- Provide API mocks or request replay
- Replace production monitoring or E2E tests

Its purpose is fast, deterministic local iteration.

---

## Links

- GitHub repository: [https://github.com/morganzwest/hsemulator](https://github.com/morganzwest/hsemulator)
- Issues and feature requests: [https://github.com/morganzwest/hsemulator/issues](https://github.com/morganzwest/hsemulator/issues)
