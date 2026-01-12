## HubSpot Emulator

**hsemulator** is a local test runner for **HubSpot Workflow Custom Code Actions**.

It allows you to run the *exact JavaScript or Python code* you paste into HubSpot locally, using fixture events, with support for assertions, snapshots, execution budgets, and flaky-run detection.

The goal is to make developing HubSpot custom code feel closer to Lambda-style local development—without relying on the HubSpot UI for iteration.

---

## When to Use It

Use hsemulator when you want to:

* Develop and debug HubSpot custom code locally
* Validate logic using real event payloads
* Detect regressions with assertions and snapshots
* Enforce performance limits (time / memory)
* Reduce copy–paste cycles into the HubSpot UI

Do **not** use it to:

* Fully emulate HubSpot’s infrastructure
* Mock HubSpot APIs or rate limits
* Replace integration or end-to-end tests _(in development)_

---

## Core Capabilities

* Local execution of JS and Python custom code actions
* Fixture-based event input
* Assertions against output and metadata
* Snapshot testing with ignore rules
* Execution budgets (time and memory)
* Flaky-run detection via repeat execution
* Machine-readable summary output (CI-friendly)

---

## Quick Links

* **Getting Started** → first successful run
* **Project Structure** → generated files and folders
* **Configuration** → `config.yaml` reference
* **Assertions / Snapshots** → correctness and regression control
* **Troubleshooting** → common issues and fixes