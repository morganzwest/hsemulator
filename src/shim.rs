// src/shim.rs

//! Runtime shims for executing HubSpot Custom Code Actions.
//!
//! The Rust CLI does NOT execute JavaScript or Python directly.
//! Instead, it spawns Node or Python and runs a tiny shim script.
//!
//! Responsibilities of a shim:
//! - Load the action file exactly as written (no modification)
//! - Call the correct HubSpot entrypoint
//! - Stream all action logs to STDERR (so they show in the terminal)
//! - Emit ONE clean JSON object to STDOUT at the very end
//!
//! Keeping STDOUT clean is critical so the Rust side can safely parse
//! the final result for assertions, snapshots, and flaky detection.

/// Node.js shim (ESM-compatible).
///
/// Usage (internal):
/// node hs_node_runner.mjs <actionFile> <event.json>
///
/// Expected action shape:
/// exports.main = async (event, callback) => { ... }
pub fn node_shim() -> &'static str {
    r#"
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

// Arguments passed by the Rust runner
const [, , actionFile, eventPath] = process.argv;

function fatal(message, error = null) {
  process.stdout.write(JSON.stringify({
    ok: false,
    language: "node",
    callback: null,
    error: {
      type: "runtime",
      message,
      stack: error?.stack || null
    }
  }));
  process.exit(1);
}

if (!actionFile || !eventPath) {
  fatal("Usage: node hs_node_runner.mjs <actionFile> <event.json>");
}

let event;
try {
  event = JSON.parse(fs.readFileSync(eventPath, "utf8"));
} catch (e) {
  fatal("Failed to read or parse event.json", e);
}

// Route logs to STDERR so STDOUT stays clean JSON
const stderr = console.error;
const rawStderr = console.error;

console.log = (...args) => {
  rawStderr("__HSE_LOG__ " + args.join(" "));
};

console.error = (...args) => {
  rawStderr("__HSE_ERR__ " + args.join(" "));
};


let callbackPayload = null;
const callback = (payload) => { callbackPayload = payload; };

// Normalize path, then convert to file:// URL (Windows-safe)
const resolvedPath = path.resolve(actionFile);
const actionUrl = pathToFileURL(resolvedPath).href;

// IMPORTANT:
// - Do NOT fall back to `require()` here.
// - If the file has a syntax error, `import()` will throw a SyntaxError,
//   which is exactly what you want to see.
// - Falling back to require on Windows can introduce the EISDIR 'C:' issue
//   when paths include \\?\ prefixes.
let mod;
try {
  mod = await import(actionUrl);
} catch (e) {
  fatal("Failed to load action file", e);
}

// HubSpot expects `main` to be exported
const fn = mod?.main || mod?.default?.main;
if (typeof fn !== "function") {
  fatal("Action file must export main(event, callback)");
}

let ok = true;
let error = null;

try {
  await fn(event, callback);
} catch (e) {
  ok = false;
  error = {
    type: "action",
    message: e?.message || String(e),
    stack: e?.stack || null
  };
}

process.stdout.write(JSON.stringify({
  ok,
  language: "node",
  callback: callbackPayload,
  error
}));
"#
}

/// Python shim.
///
/// Usage (internal):
/// python hs_python_runner.py <actionFile.py> <event.json>
///
/// Expected action shape:
/// def main(event): ...
pub fn python_shim() -> &'static str {
    r#"# hs_python_runner.py
#
# Runtime shim for executing HubSpot Custom Code Actions (Python).
#
# Contract:
# - User code may print/log freely
# - ALL user output is redirected to STDERR
# - STDOUT emits EXACTLY ONE JSON object at the end
# - Any deviation is a hard error

import importlib.util
import json
import sys
import traceback
from contextlib import redirect_stdout
from io import StringIO


def fatal(message, stack=None):
    sys.stdout.write(
        json.dumps(
            {
                "ok": False,
                "language": "python",
                "result": None,
                "error": {
                    "type": "runtime",
                    "message": message,
                    "stack": stack,
                },
            }
        )
    )
    sys.exit(1)


def import_python_file(file_path: str):
    spec = importlib.util.spec_from_file_location("hs_action_module", file_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to import file: {file_path}")

    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def emit_log(line: str):
    print("__HSE_LOG__", line, file=sys.stderr)


def emit_err(line: str):
    print("__HSE_ERR__", line, file=sys.stderr)


def main():
    if len(sys.argv) < 3:
        fatal("Usage: python hs_python_runner.py <actionFile.py> <event.json>")

    action_file = sys.argv[1]
    event_path = sys.argv[2]

    # Load event
    try:
        with open(event_path, "r", encoding="utf-8") as f:
            event = json.load(f)
    except Exception:
        fatal("Failed to read or parse event.json", traceback.format_exc())

    # Import action module
    try:
        module = import_python_file(action_file)
    except Exception:
        fatal("Failed to load action file", traceback.format_exc())

    if not hasattr(module, "main") or not callable(module.main):
        fatal("Action file must define: def main(event)")

    ok = True
    result = None
    error = None

    # Capture ALL stdout produced by user code
    stdout_buffer = StringIO()
    old_stdout = sys.stdout

    try:
        sys.stdout = stdout_buffer
        with redirect_stdout(stdout_buffer):
            result = module.main(event)
    except Exception as e:
        ok = False
        error = {
            "type": "action",
            "message": str(e),
            "stack": traceback.format_exc(),
        }
    finally:
        sys.stdout = old_stdout

    # Flush captured stdout as structured logs
    captured = stdout_buffer.getvalue()
    if captured:
        for line in captured.splitlines():
            if line.strip():
                emit_log(line)

    # Emit final JSON — MUST be the only STDOUT output
    sys.stdout.write(
        json.dumps(
            {
                "ok": ok,
                "language": "python",
                "result": result,
                "error": error,
            }
        )
    )


if __name__ == "__main__":
    main()

"#
}
