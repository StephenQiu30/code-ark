// Maintainer-only source check: no consent/input means no SDK call and no success output.
import { readFile, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
const root = resolve(import.meta.dirname, "java");
const lock = JSON.parse(
  await readFile(resolve(import.meta.dirname, "clients.lock.json"), "utf8"),
);
const directory = await mkdtemp(
  join(tmpdir(), "codeark-example-input-guards-"),
);
const environment = Object.fromEntries(
  Object.entries(process.env).filter(([key]) => !key.startsWith("CODEARK_")),
);
const results = [];
try {
  for (const module of Object.keys(lock.modules).filter(
    (name) => name !== "common",
  )) {
    if (!/^[a-z][a-z0-9-]*$/.test(module)) throw Error("InvalidModule");
    const result = spawnSync(
      "java",
      [
        `-Duser.home=${directory}`,
        "-cp",
        join(root, module, "target/*"),
        "io.codeark.tutorial.Main",
      ],
      {
        env: environment,
        cwd: directory,
        encoding: "utf8",
        timeout: 10000,
        maxBuffer: 131072,
      },
    );
    if (
      result.status !== 1 ||
      result.stdout.trim() ||
      result.stderr.trim() !== "EXAMPLE_FAILED IllegalArgumentException"
    )
      throw Error(`InputGuardFailed:${module}`);
    results.push({ module, status: "refused-without-consent" });
  }
  console.log(
    JSON.stringify(
      { ok: true, checks: results, runtimeAccepted: false },
      null,
      2,
    ),
  );
} finally {
  await rm(directory, { recursive: true, force: true });
}
