import { createHmac, randomBytes } from "node:crypto";

const username = process.env.REYN_YC_USERNAME?.trim();
const password = process.env.REYN_YC_PASSWORD;

if (
  !username ||
  username.length > 128 ||
  !password ||
  password.length > 256
) {
  console.error(
    "Set REYN_YC_USERNAME and REYN_YC_PASSWORD, then pipe this script directly to `wrangler secret bulk`.",
  );
  process.exit(2);
}

const authPepper =
  process.env.REYN_AUTH_PEPPER ?? randomBytes(32).toString("base64url");
const eventHashKey =
  process.env.REYN_EVENT_HASH_KEY ?? randomBytes(32).toString("base64url");
const validKey = (value) =>
  value.length >= 43 &&
  value.length <= 256 &&
  /^[A-Za-z0-9_-]+$/u.test(value);
if (!validKey(authPepper) || !validKey(eventHashKey)) {
  console.error(
    "REYN_AUTH_PEPPER and REYN_EVENT_HASH_KEY must be 43–256-character base64url strings.",
  );
  process.exit(2);
}
const passwordDigest = createHmac("sha256", authPepper)
  .update(`${username}\0${password}`, "utf8")
  .digest("hex");

process.stdout.write(
  `${JSON.stringify({
    YC_USERNAME: username,
    YC_PASSWORD_DIGEST_HEX: passwordDigest,
    AUTH_PEPPER: authPepper,
    EVENT_HASH_KEY: eventHashKey,
  })}\n`,
);
