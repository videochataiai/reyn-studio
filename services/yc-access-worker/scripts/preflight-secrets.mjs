import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const REQUIRED_SECRET_NAMES = Object.freeze([
  "AUTH_PEPPER",
  "EVENT_HASH_KEY",
  "YC_PASSWORD_DIGEST_HEX",
  "YC_USERNAME",
]);

export function missingSecretNames(rows) {
  if (!Array.isArray(rows)) {
    throw new TypeError("Wrangler secret list did not return an array.");
  }
  const present = new Set(
    rows
      .map((row) => (row && typeof row === "object" ? row.name : undefined))
      .filter((name) => typeof name === "string"),
  );
  return REQUIRED_SECRET_NAMES.filter((name) => !present.has(name));
}

function listRemoteSecrets() {
  const executable = process.platform === "win32" ? "wrangler.cmd" : "wrangler";
  const result = spawnSync(
    executable,
    ["secret", "list", "--format", "json"],
    {
      cwd: new URL("..", import.meta.url),
      encoding: "utf8",
      shell: false,
      stdio: ["ignore", "pipe", "inherit"],
    },
  );
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`wrangler secret list exited with status ${result.status}`);
  }
  return JSON.parse(result.stdout);
}

function main() {
  const missing = missingSecretNames(listRemoteSecrets());
  if (missing.length > 0) {
    console.error(`Deployment blocked: missing Worker secrets: ${missing.join(", ")}`);
    process.exitCode = 2;
    return;
  }
  console.log(`Verified ${REQUIRED_SECRET_NAMES.length} required remote secret names.`);
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
