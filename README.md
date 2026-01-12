# HubSpot Emulator (hsemulator)

**hsemulator** is a local test runner for **HubSpot Workflow Custom Code Actions**.

It allows you to run the _exact JavaScript or Python code_ you paste into HubSpot locally, using real workflow event payloads, with support for assertions, snapshots, execution budgets, and flaky-run detection.

The goal is to make developing HubSpot custom code feel closer to Lambda-style local development—without relying on the HubSpot UI for iteration.

---

## Why hsemulator exists

Developing HubSpot custom code today typically means:

- Writing code in an editor
- Copying it into the HubSpot UI
- Triggering a workflow
- Reading logs in a browser
- Repeating

hsemulator replaces that loop with a local, deterministic workflow:

- Code stays in your editor
- Events are fixtures
- Failures are explicit
- Regressions are caught automatically

No UI-driven iteration required.

---

## When to use it

Use hsemulator when you want to:

- Develop and debug HubSpot custom code locally
- Validate logic using real workflow event payloads
- Catch regressions with assertions and snapshots
- Enforce execution limits during development or CI
- Reduce copy–paste cycles into the HubSpot UI

Do **not** use it to:

- Fully emulate HubSpot’s runtime or infrastructure
- Mock HubSpot APIs or rate limits
- Replace integration or end-to-end tests

---

## Core capabilities

- Local execution of JS and Python custom code actions
- Fixture-based event input
- Assertions against output and metadata
- Snapshot testing for regression detection
- Execution budgets (time and memory)
- Flaky-run detection via repeat execution
- CI-friendly, machine-readable output

---

## Documentation

📖 **Full documentation is available on Read the Docs:**

👉 [Read our Documentation](https://hsemulator.readthedocs.io/)

Start here if you want:
- Installation instructions
- Project structure
- Configuration reference
- Assertions, snapshots, budgets, and CI usage

---

## Developer notes (Rust / Cargo)

hsemulator is written in **Rust** and distributed as a standalone binary.

### Building locally

```bash
cargo build
````

### Running from source

```bash
cargo run -- run
```

### Release builds

```bash
cargo build --release
```

The release binary is what gets packaged into the installer.
End users do **not** need Rust or Cargo installed.

---

## High-level architecture

At a high level, hsemulator works like this:

```
fixture.json
     │
     ▼
runtime shim (Node / Python)
     │
     ▼
your HubSpot custom code
     │
     ▼
structured JSON output
     │
     ├─ assertions
     ├─ budgets
     └─ snapshots
```

Each run is isolated, deterministic, and fully observable.
---

## Contributing / feedback

Issues, bug reports, and feature requests are welcome:

👉 [https://github.com/morganzwest/Local-HubSpot-Emulator/issues](https://github.com/morganzwest/Local-HubSpot-Emulator/issues)

---

## License

MIT
