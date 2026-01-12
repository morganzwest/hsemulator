# Getting Started

This guide gets you from zero to a successful local run as quickly as possible.

---

## Installation

Download and run the Windows installer from the GitHub releases page.

The installer:

* Installs `hsemulator.exe`
* Adds it to your `PATH`

Verify installation:

```bash
hsemulator --version
```

---

## Initialise a Project

In an empty directory:

```bash
hsemulator init js
hsemulator init python
```

This scaffolds a minimal project structure with:

* Configuration
* Example action depending on js/python parameter
* Example fixture
* Assertions and snapshot support

---

## Run the Example Action

From the project root:

```bash
hsemulator run
```

You should see:

* The action being executed
* Logs streamed to stdout
* A success summary

If this works, your environment is correctly set up.

---

## Minimal Workflow

The typical workflow is:

1. Paste your HubSpot custom code into `actions/`
2. Capture a real HubSpot event as a fixture
3. Run locally with `hsemulator run`
4. Add assertions or snapshots
5. Iterate until deterministic and correct
6. Paste the same code back into HubSpot

No code changes are required between local and HubSpot execution.

---

## Requirements

* **Node.js** for JavaScript actions
* **Python 3** for Python actions

The runtimes must be available on your system `PATH`.

---

## Next Steps

* Review **Project Structure** to understand generated files
* Configure `config.yaml` for your action
* Add **assertions** and **snapshots** to lock in behaviour
* Use **budgets** to prevent performance regressions
