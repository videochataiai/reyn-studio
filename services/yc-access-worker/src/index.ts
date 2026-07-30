const LOGIN_PATH = "/api/yc-access/v1/session";
const HEALTH_PATH = "/api/yc-access/v1/health";
const MAX_REQUEST_BYTES = 4 * 1024;
const MAX_SESSION_SECONDS = 24 * 60 * 60;
const MAX_EVENT_RETENTION_SECONDS = 30 * 24 * 60 * 60;
const encoder = new TextEncoder();

type ClientInfo = {
  appVersion: string;
  platform: string;
  architecture: string;
};

type LoginInput = {
  username: string;
  password: string;
  termsVersion: string;
  privacyVersion: string;
  client: ClientInfo;
};

type AuthOutcome = "success" | "failure" | "rate_limited";
type WorkerConfiguration = {
  sessionTtlSeconds: number;
  eventRetentionSeconds: number;
};

class BodyTooLargeError extends Error {}
class InvalidBodyError extends Error {}

export default {
  async fetch(request, env, ctx): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname === HEALTH_PATH && request.method === "GET") {
      if (validateConfiguration(env) === null) {
        return jsonResponse(
          { ok: false, error: { code: "service_unavailable" } },
          503,
        );
      }
      return jsonResponse({ ok: true, service: "reyn-yc-access", version: 1 }, 200);
    }
    if (url.pathname !== LOGIN_PATH) {
      return jsonResponse({ ok: false, error: { code: "not_found" } }, 404);
    }
    if (request.method !== "POST") {
      return jsonResponse(
        { ok: false, error: { code: "method_not_allowed" } },
        405,
        { Allow: "POST" },
      );
    }

    const configuration = validateConfiguration(env);
    if (configuration === null) {
      return jsonResponse(
        { ok: false, error: { code: "service_unavailable" } },
        503,
      );
    }

    let input: LoginInput;
    try {
      input = parseLoginInput(await readBoundedJson(request, MAX_REQUEST_BYTES));
    } catch (error) {
      if (error instanceof BodyTooLargeError) {
        return jsonResponse(
          { ok: false, error: { code: "request_too_large" } },
          413,
        );
      }
      return jsonResponse({ ok: false, error: { code: "invalid_request" } }, 400);
    }

    if (
      input.termsVersion !== env.TERMS_VERSION ||
      input.privacyVersion !== env.PRIVACY_VERSION
    ) {
      return jsonResponse(
        {
          ok: false,
          error: {
            code: "legal_version_outdated",
            required_terms_version: env.TERMS_VERSION,
            required_privacy_version: env.PRIVACY_VERSION,
          },
        },
        409,
      );
    }

    const rateLimitKey = `yc-login:${await sha256Hex(input.username.toLowerCase())}`;
    const rateLimit = await env.LOGIN_RATE_LIMITER.limit({ key: rateLimitKey });
    if (!rateLimit.success) {
      ctx.waitUntil(
        recordAuthEvent(
          env,
          configuration,
          request,
          input,
          "rate_limited",
        ).catch(() => undefined),
      );
      return jsonResponse(
        { ok: false, error: { code: "rate_limited" } },
        429,
        { "Retry-After": "60" },
      );
    }

    const valid = await credentialsMatch(env, input);
    ctx.waitUntil(
      recordAuthEvent(
        env,
        configuration,
        request,
        input,
        valid ? "success" : "failure",
      ).catch(() => undefined),
    );
    if (!valid) {
      return jsonResponse(
        { ok: false, error: { code: "invalid_credentials" } },
        401,
      );
    }

    const expiresAtUtcUnix =
      Math.floor(Date.now() / 1000) + configuration.sessionTtlSeconds;
    return jsonResponse(
      {
        ok: true,
        session_token: randomToken(32),
        expires_at_utc_unix: expiresAtUtcUnix,
        terms_version: env.TERMS_VERSION,
        privacy_version: env.PRIVACY_VERSION,
      },
      200,
    );
  },
} satisfies ExportedHandler<Env>;

function validateConfiguration(env: Env): WorkerConfiguration | null {
  const values = env as unknown as Record<string, unknown>;
  const username = values.YC_USERNAME;
  const passwordDigest = values.YC_PASSWORD_DIGEST_HEX;
  const authPepper = values.AUTH_PEPPER;
  const eventHashKey = values.EVENT_HASH_KEY;
  const termsVersion = values.TERMS_VERSION;
  const privacyVersion = values.PRIVACY_VERSION;
  const sessionTtlSeconds = strictBoundedSeconds(
    values.SESSION_TTL_SECONDS,
    60,
    MAX_SESSION_SECONDS,
  );
  const eventRetentionSeconds = strictBoundedSeconds(
    values.EVENT_RETENTION_SECONDS,
    60,
    MAX_EVENT_RETENTION_SECONDS,
  );
  const authEvents = values.AUTH_EVENTS as
    | { put?: unknown }
    | null
    | undefined;
  const rateLimiter = values.LOGIN_RATE_LIMITER as
    | { limit?: unknown }
    | null
    | undefined;
  if (
    typeof username !== "string" ||
    username.length === 0 ||
    username.length > 128 ||
    username !== username.trim() ||
    typeof passwordDigest !== "string" ||
    hexToBytes(passwordDigest) === null ||
    !validKeySecret(authPepper) ||
    !validKeySecret(eventHashKey) ||
    typeof termsVersion !== "string" ||
    termsVersion.length === 0 ||
    termsVersion.length > 16 ||
    typeof privacyVersion !== "string" ||
    privacyVersion.length === 0 ||
    privacyVersion.length > 16 ||
    sessionTtlSeconds === null ||
    eventRetentionSeconds === null ||
    typeof authEvents?.put !== "function" ||
    typeof rateLimiter?.limit !== "function"
  ) {
    return null;
  }
  return { sessionTtlSeconds, eventRetentionSeconds };
}

function validKeySecret(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length >= 43 &&
    value.length <= 256 &&
    /^[A-Za-z0-9_-]+$/u.test(value)
  );
}

async function readBoundedJson(
  request: Request,
  maximumBytes: number,
): Promise<unknown> {
  const declaredLength = request.headers.get("content-length");
  if (declaredLength !== null) {
    const parsedLength = Number(declaredLength);
    if (!Number.isInteger(parsedLength) || parsedLength < 0) {
      throw new InvalidBodyError();
    }
    if (parsedLength > maximumBytes) {
      throw new BodyTooLargeError();
    }
  }
  if (request.body === null) {
    throw new InvalidBodyError();
  }

  const reader = request.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  while (true) {
    const result = await reader.read();
    if (result.done) {
      break;
    }
    total += result.value.byteLength;
    if (total > maximumBytes) {
      await reader.cancel();
      throw new BodyTooLargeError();
    }
    chunks.push(result.value);
  }

  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  try {
    const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: false }).decode(
      bytes,
    );
    return JSON.parse(text) as unknown;
  } catch {
    throw new InvalidBodyError();
  }
}

function parseLoginInput(value: unknown): LoginInput {
  if (!isRecord(value) || !isRecord(value.client)) {
    throw new InvalidBodyError();
  }
  const username = requiredString(value.username, 128).trim();
  if (username.length === 0) {
    throw new InvalidBodyError();
  }
  return {
    username,
    password: requiredString(value.password, 256),
    termsVersion: requiredString(value.terms_version, 16),
    privacyVersion: requiredString(value.privacy_version, 16),
    client: {
      appVersion: requiredString(value.client.app_version, 32),
      platform: requiredString(value.client.platform, 32),
      architecture: requiredString(value.client.architecture, 32),
    },
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function requiredString(value: unknown, maximumLength: number): string {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.length > maximumLength
  ) {
    throw new InvalidBodyError();
  }
  return value;
}

async function credentialsMatch(env: Env, input: LoginInput): Promise<boolean> {
  const suppliedUsernameDigest = await hmac(
    env.AUTH_PEPPER,
    input.username,
  );
  const expectedUsernameDigest = await hmac(env.AUTH_PEPPER, env.YC_USERNAME);
  const suppliedPasswordDigest = await hmac(
    env.AUTH_PEPPER,
    `${input.username}\0${input.password}`,
  );
  const parsedExpectedPassword = hexToBytes(env.YC_PASSWORD_DIGEST_HEX);
  const expectedPasswordDigest =
    parsedExpectedPassword ?? new Uint8Array(suppliedPasswordDigest.byteLength);

  const usernameMatches = crypto.subtle.timingSafeEqual(
    suppliedUsernameDigest,
    expectedUsernameDigest,
  );
  const passwordMatches = crypto.subtle.timingSafeEqual(
    suppliedPasswordDigest,
    expectedPasswordDigest,
  );
  return parsedExpectedPassword !== null && usernameMatches && passwordMatches;
}

async function recordAuthEvent(
  env: Env,
  configuration: WorkerConfiguration,
  request: Request,
  input: LoginInput,
  outcome: AuthOutcome,
): Promise<void> {
  const nowUtcUnix = Math.floor(Date.now() / 1000);
  const ipAddress = request.headers.get("cf-connecting-ip") ?? "unknown";
  const ipHash = bytesToHex(await hmac(env.EVENT_HASH_KEY, ipAddress));
  const event = {
    schema_version: 1,
    outcome,
    occurred_at_utc_unix: nowUtcUnix,
    ip_hash: ipHash,
    client: input.client,
    terms_version: input.termsVersion,
    privacy_version: input.privacyVersion,
  };
  await env.AUTH_EVENTS.put(
    `event/${nowUtcUnix}/${crypto.randomUUID()}`,
    JSON.stringify(event),
    {
      expirationTtl: configuration.eventRetentionSeconds,
      metadata: { schema_version: 1, outcome },
    },
  );
}

async function hmac(secret: string, value: string): Promise<Uint8Array> {
  const key = await crypto.subtle.importKey(
    "raw",
    encoder.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  return new Uint8Array(
    await crypto.subtle.sign("HMAC", key, encoder.encode(value)),
  );
}

async function sha256Hex(value: string): Promise<string> {
  return bytesToHex(
    new Uint8Array(await crypto.subtle.digest("SHA-256", encoder.encode(value))),
  );
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function hexToBytes(value: string): Uint8Array | null {
  if (!/^[0-9a-f]{64}$/i.test(value)) {
    return null;
  }
  const bytes = new Uint8Array(value.length / 2);
  for (let index = 0; index < value.length; index += 2) {
    bytes[index / 2] = Number.parseInt(value.slice(index, index + 2), 16);
  }
  return bytes;
}

function randomToken(byteLength: number): string {
  const bytes = crypto.getRandomValues(new Uint8Array(byteLength));
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary)
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replace(/=+$/u, "");
}

function strictBoundedSeconds(
  value: unknown,
  minimum: number,
  maximum: number,
): number | null {
  if (typeof value !== "string" || !/^[1-9][0-9]*$/u.test(value)) {
    return null;
  }
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
    return null;
  }
  return parsed;
}

function jsonResponse(
  body: unknown,
  status: number,
  extraHeaders: Record<string, string> = {},
): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "Cache-Control": "no-store, max-age=0",
      "Content-Type": "application/json; charset=utf-8",
      "Cross-Origin-Resource-Policy": "same-origin",
      Pragma: "no-cache",
      "Referrer-Policy": "no-referrer",
      "X-Content-Type-Options": "nosniff",
      ...extraHeaders,
    },
  });
}
