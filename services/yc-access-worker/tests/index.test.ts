import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHmac, timingSafeEqual as nodeTimingSafeEqual } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";
import { fileURLToPath, URL as NodeURL } from "node:url";

import worker from "../src/index.ts";
import {
  REQUIRED_SECRET_NAMES,
  missingSecretNames,
} from "../scripts/preflight-secrets.mjs";

if (typeof crypto.subtle.timingSafeEqual !== "function") {
  Object.defineProperty(crypto.subtle, "timingSafeEqual", {
    configurable: true,
    value(left: ArrayBufferView, right: ArrayBufferView): boolean {
      return nodeTimingSafeEqual(
        Buffer.from(left.buffer, left.byteOffset, left.byteLength),
        Buffer.from(right.buffer, right.byteOffset, right.byteLength),
      );
    },
  });
}

type PutCall = {
  key: string;
  value: string;
  options: KVNamespacePutOptions | undefined;
};

function invokeWorker(request: Request, env: Env, context: ExecutionContext) {
  return worker.fetch(
    request as Parameters<typeof worker.fetch>[0],
    env,
    context,
  );
}

function fixture(options: { validPassword?: boolean; rateLimited?: boolean } = {}) {
  const username = "yc-invitee";
  const password = "correct horse battery staple";
  const authPepper = "p".repeat(43);
  const eventHashKey = "e".repeat(43);
  const digest = createHmac("sha256", authPepper)
    .update(`${username}\0${password}`, "utf8")
    .digest("hex");
  const puts: PutCall[] = [];
  const waits: Promise<unknown>[] = [];
  const env = {
    AUTH_EVENTS: {
      async put(key: string, value: string, options?: KVNamespacePutOptions) {
        puts.push({ key, value, options });
      },
    } as unknown as KVNamespace,
    LOGIN_RATE_LIMITER: {
      async limit() {
        return { success: options.rateLimited !== true };
      },
    } as unknown as RateLimit,
    TERMS_VERSION: "1.0",
    PRIVACY_VERSION: "1.0",
    SESSION_TTL_SECONDS: "86340",
    EVENT_RETENTION_SECONDS: "2592000",
    YC_USERNAME: username,
    YC_PASSWORD_DIGEST_HEX: digest,
    AUTH_PEPPER: authPepper,
    EVENT_HASH_KEY: eventHashKey,
  } satisfies Env;
  const body = {
    username,
    password: options.validPassword === false ? "wrong password" : password,
    terms_version: "1.0",
    privacy_version: "1.0",
    client: {
      app_version: "0.1.0",
      platform: "windows",
      architecture: "x86_64",
    },
  };
  const request = new Request("https://reynflow.com/api/yc-access/v1/session", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "cf-connecting-ip": "203.0.113.42",
    },
    body: JSON.stringify(body),
  });
  const context = {
    waitUntil(promise: Promise<unknown>) {
      waits.push(promise);
    },
    passThroughOnException() {},
    props: {},
  } as unknown as ExecutionContext;
  return { body, context, env, puts, request, waits };
}

async function settle(waits: Promise<unknown>[]) {
  await Promise.all(waits);
}

test("missing or malformed secrets fail closed without naming them", async () => {
  for (const name of REQUIRED_SECRET_NAMES) {
    const { context, env, request, waits } = fixture();
    const broken = { ...env, [name]: undefined } as unknown as Env;
    const response = await invokeWorker(request, broken, context);
    assert.equal(response.status, 503);
    assert.deepEqual(await response.json(), {
      ok: false,
      error: { code: "service_unavailable" },
    });
    assert.equal(waits.length, 0);
  }

  for (const [name, value] of [
    ["YC_USERNAME", " padded "],
    ["YC_PASSWORD_DIGEST_HEX", "abcd"],
    ["AUTH_PEPPER", "short"],
    ["EVENT_HASH_KEY", "contains whitespace and is not a key"],
  ] as const) {
    const { context, env, request } = fixture();
    const response = await invokeWorker(
      request,
      { ...env, [name]: value },
      context,
    );
    assert.equal(response.status, 503);
  }

  const { context, env } = fixture();
  const health = await invokeWorker(
    new Request("https://reynflow.com/api/yc-access/v1/health"),
    { ...env, EVENT_HASH_KEY: "" },
    context,
  );
  assert.equal(health.status, 503);
});

test("authentication outcomes are generic and never enable CORS or caching", async () => {
  for (const [options, status] of [
    [{}, 200],
    [{ validPassword: false }, 401],
    [{ rateLimited: true }, 429],
  ] as const) {
    const { context, env, request, waits } = fixture(options);
    const response = await invokeWorker(request, env, context);
    await settle(waits);
    assert.equal(response.status, status);
    assert.equal(response.headers.get("access-control-allow-origin"), null);
    assert.equal(response.headers.get("cache-control"), "no-store, max-age=0");
  }
});

test("auth events are pseudonymous, bounded to 30 days, and omit credentials", async () => {
  const { body, context, env, puts, request, waits } = fixture();
  const response = await invokeWorker(request, env, context);
  const payload = (await response.json()) as Record<string, unknown>;
  await settle(waits);

  assert.equal(response.status, 200);
  assert.equal(puts.length, 1);
  const eventPut = puts[0];
  assert.ok(eventPut);
  assert.match(eventPut.key, /^event\/[0-9]+\/[0-9a-f-]+$/u);
  assert.equal(eventPut.options?.expirationTtl, 30 * 24 * 60 * 60);
  assert.deepEqual(eventPut.options?.metadata, {
    schema_version: 1,
    outcome: "success",
  });
  const event = JSON.parse(eventPut.value) as Record<string, unknown>;
  assert.match(String(event.ip_hash), /^[0-9a-f]{64}$/u);
  assert.notEqual(event.ip_hash, "203.0.113.42");
  assert.equal(event.outcome, "success");
  assert.equal(JSON.stringify(event).includes(body.username), false);
  assert.equal(JSON.stringify(event).includes(body.password), false);
  assert.equal(
    JSON.stringify(event).includes(String(payload.session_token)),
    false,
  );
});

test("oversized bodies are rejected before authentication", async () => {
  const { context, env } = fixture();
  const response = await invokeWorker(
    new Request("https://reynflow.com/api/yc-access/v1/session", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: "x".repeat(4097),
    }),
    env,
    context,
  );
  assert.equal(response.status, 413);
});

test("request handling emits no credential or session logs", async () => {
  const fixtureValue = fixture();
  const messages: unknown[][] = [];
  const originals = [console.log, console.warn, console.error] as const;
  console.log = (...values: unknown[]) => messages.push(values);
  console.warn = (...values: unknown[]) => messages.push(values);
  console.error = (...values: unknown[]) => messages.push(values);
  try {
    await invokeWorker(
      fixtureValue.request,
      fixtureValue.env,
      fixtureValue.context,
    );
    await settle(fixtureValue.waits);
  } finally {
    [console.log, console.warn, console.error] = originals;
  }
  assert.deepEqual(messages, []);
});

test("deployment preflight requires exactly the configured secret names", () => {
  assert.deepEqual(
    missingSecretNames(REQUIRED_SECRET_NAMES.map((name) => ({ name }))),
    [],
  );
  assert.deepEqual(
    missingSecretNames(
      REQUIRED_SECRET_NAMES.filter((name) => name !== "EVENT_HASH_KEY").map(
        (name) => ({ name }),
      ),
    ),
    ["EVENT_HASH_KEY"],
  );
  assert.throws(() => missingSecretNames({}), /did not return an array/u);
  const wrangler = JSON.parse(
    readFileSync(
      fileURLToPath(new NodeURL("../wrangler.jsonc", import.meta.url)),
      "utf8",
    ),
  ) as { secrets?: { required?: string[] } };
  assert.deepEqual(
    [...(wrangler.secrets?.required ?? [])].sort(),
    [...REQUIRED_SECRET_NAMES].sort(),
  );
});

test("production session TTL leaves a clock-skew safety margin", () => {
  const wrangler = JSON.parse(
    readFileSync(
      fileURLToPath(new NodeURL("../wrangler.jsonc", import.meta.url)),
      "utf8",
    ),
  ) as { vars?: { SESSION_TTL_SECONDS?: string } };
  const sessionTtlSeconds = Number(wrangler.vars?.SESSION_TTL_SECONDS);
  assert.ok(Number.isInteger(sessionTtlSeconds));
  assert.ok(sessionTtlSeconds > 0);
  assert.ok(sessionTtlSeconds <= 24 * 60 * 60 - 30);
});

test("secret generator rejects malformed keys without writing a bundle", () => {
  const result = spawnSync(
    process.execPath,
    [
      fileURLToPath(
        new NodeURL("../scripts/generate-secret-bundle.mjs", import.meta.url),
      ),
    ],
    {
      encoding: "utf8",
      env: {
        ...process.env,
        REYN_YC_USERNAME: "invitee",
        REYN_YC_PASSWORD: "do-not-print-this-password",
        REYN_AUTH_PEPPER: "short",
        REYN_EVENT_HASH_KEY: "also-short",
      },
    },
  );
  assert.equal(result.status, 2);
  assert.equal(result.stdout, "");
  assert.equal(result.stderr.includes("do-not-print-this-password"), false);
  assert.equal(result.stderr.includes("also-short"), false);
});
