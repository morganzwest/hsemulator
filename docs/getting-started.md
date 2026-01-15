# Getting Started

This guide gets you from zero to a successful local run as quickly as possible.

---

## Installation

hsemulator is distributed as a standalone binary.

### Recommended: Scoop (Windows)

**Scoop is the preferred installation method** and will always have the most up-to-date releases.

```powershell
scoop bucket add hsemulate https://github.com/morganzwest/scoop-hsemulate
scoop install hsemulate
```

Upgrade with:

```powershell
scoop update hsemulate
```

---

### Winget (Windows)

hsemulator is available on Winget as:

```
MorganZWest.HSEmulate
```

Install with:

```powershell
winget install MorganZWest.HSEmulate
```

⚠️ **Important**

**NOT ALL VERSIONS WILL BE RELEASED ON WINGET.**
Winget releases are intentionally less frequent due to review latency and immutability requirements.

If you want faster access to new versions, **use Scoop instead**.

---

### Manual download

You can also download and run the Windows installer directly from GitHub Releases:

[https://github.com/morganzwest/hsemulator/releases](https://github.com/morganzwest/hsemulator/releases)

The installer:

* Installs `hsemulate.exe`
* Adds it to your `PATH`

---

### Verify installation

```bash
hsemulate --version
```

---

## Initialise a Project

In an empty directory:

```bash
hsemulate init js
hsemulate init python
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
hsemulate run
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
3. Run locally with `hsemulate run`
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
