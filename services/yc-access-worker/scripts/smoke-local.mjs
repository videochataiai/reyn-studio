import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";

const baseUrl = new URL(
  process.env.REYN_ACCESS_SMOKE_URL ??
    "http://127.0.0.1:8793/api/yc-access/v1/",
);
if (
  !["127.0.0.1", "localhost", "::1"].includes(baseUrl.hostname) &&
  process.env.REYN_ALLOW_REMOTE_SMOKE !== "1"
) {
  throw new Error(
    "Refusing to exercise a remote access service without REYN_ALLOW_REMOTE_SMOKE=1.",
  );
}

const username = process.env.REYN_YC_USERNAME;
const password = process.env.REYN_YC_PASSWORD;
if (!username || !password) {
  throw new Error("Set REYN_YC_USERNAME and REYN_YC_PASSWORD for the smoke test.");
}

const client = {
  app_version: "0.1.1-test",
  platform: "windows",
  architecture: "x86_64",
};
const legal = {
  terms_version: "1.0",
  privacy_version: "1.0",
  client,
};

async function request(path, options = {}) {
  const response = await fetch(new URL(path, baseUrl), {
    signal: AbortSignal.timeout(5_000),
    ...options,
  });
  const body = await response.json();
  return { response, body };
}

const health = await request("health");
assert.equal(health.response.status, 200);
assert.equal(health.body.ok, true);
assert.equal(health.response.headers.get("cache-control"), "no-store, max-age=0");
assert.equal(health.response.headers.get("access-control-allow-origin"), null);

const invalid = await request("session", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    ...legal,
    username: `invalid-${randomUUID()}`,
    password: "invalid",
  }),
});
assert.equal(invalid.response.status, 401);
assert.equal(invalid.body.error.code, "invalid_credentials");

const outdated = await request("session", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    ...legal,
    terms_version: "0.9",
    username,
    password,
  }),
});
assert.equal(outdated.response.status, 409);
assert.equal(outdated.body.error.code, "legal_version_outdated");

const valid = await request("session", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ ...legal, username, password }),
});
assert.equal(valid.response.status, 200);
assert.equal(valid.body.ok, true);
assert.ok(valid.body.session_token.length >= 32);
assert.ok(valid.body.expires_at_utc_unix > Math.floor(Date.now() / 1_000));
assert.equal(valid.response.headers.get("access-control-allow-origin"), null);

const oversized = await request("session", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: `{${"x".repeat(5_000)}}`,
});
assert.equal(oversized.response.status, 413);
assert.equal(oversized.body.error.code, "request_too_large");

const rateUsername = `rate-${randomUUID()}`;
const rateStatuses = [];
for (let attempt = 0; attempt < 7; attempt += 1) {
  const limited = await request("session", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      ...legal,
      username: rateUsername,
      password: "invalid",
    }),
  });
  rateStatuses.push(limited.response.status);
}
assert.ok(rateStatuses.includes(429), `Expected a 429, got ${rateStatuses.join(", ")}`);

console.log("Local YC access smoke passed.");
