# Reyn YC access service

This Cloudflare Worker unlocks official invitation-only Reyn Studio preview
binaries. Public source builds do not require the service.

## Security contract

- Credentials are never committed or embedded in the desktop binary.
- The Worker compares fixed-length HMAC-SHA-256 digests with
  `crypto.subtle.timingSafeEqual`.
- Five login attempts are allowed per username digest and Cloudflare location
  per minute.
- Responses are never cached and the endpoint does not enable browser CORS.
- Auth events contain an HMAC of the connecting IP, client build metadata,
  outcome, and legal versions. They do not contain the username, password, or
  session token.
- KV applies a 30-day expiration to every auth event.
- Cloudflare Worker logs, invocation logs, traces, and persistence are disabled
  in `wrangler.jsonc`.
- A successful login returns a random, in-memory session proof that expires
  within 24 hours. The desktop app asks again after restart.

## Provision and deploy

Requirements: Node.js 22 or newer and an authenticated Wrangler session.

1. Install dependencies:

   ```sh
   npm install
   ```

2. For a new Cloudflare account, create the event namespace:

   ```sh
   wrangler kv namespace create reyn-yc-auth-events
   ```

3. Replace the existing `AUTH_EVENTS` namespace ID in `wrangler.jsonc` with the
   returned ID. The ID is a public binding identifier, not a secret. Do not
   create a second namespace for the existing Reyn deployment.

4. Choose the shared YC credentials and configure encrypted Worker secrets:

   ```sh
   REYN_YC_USERNAME='the-username' \
   REYN_YC_PASSWORD='the-high-entropy-password' \
   npm run secrets:configure
   ```

   The helper generates independent random keys for credential hashing and
   auth-event IP pseudonymization. It writes the secret bundle directly to
   Wrangler. Do not redirect its output to a file.

5. Validate and deploy:

   ```sh
   npm run types
   npm run check
   npm test
   npm run deploy
   ```

6. Build the official desktop binary with:

   ```sh
   REYN_ACCESS_REQUIRED=1 \
   REYN_ACCESS_ENDPOINT='https://reynflow.com/api/yc-access/v1/session' \
   cargo build --release
   ```

`npm run deploy` first lists remote secret names and refuses to invoke Wrangler
unless all four required names are present. The Worker separately validates
every secret and binding at request time and returns a generic 503 for missing
or malformed configuration. `AUTH_PEPPER` and `EVENT_HASH_KEY` must be
43–256-character base64url strings; the helper generates each from 32 random
bytes. The specific Worker route is more
precise than the static-site route, so
`/api/yc-access/*` reaches this service while the rest of reynflow.com remains
on the website Worker.

## Local development

Create an untracked `.dev.vars` containing the four names listed under
`secrets.required` in `wrangler.jsonc`. Use a development KV namespace ID or
Wrangler's local KV implementation. Never copy production credentials into a
committed fixture.

Start the local Worker and run the reusable security-path smoke test:

```sh
wrangler dev --ip 127.0.0.1 --port 8793

REYN_YC_USERNAME='local-test-user' \
REYN_YC_PASSWORD='local-test-password' \
npm run test:integration
```

The smoke test covers success, invalid credentials, legal-version mismatch,
oversized bodies, absent CORS, no-store headers, and rate limiting. It refuses
to target a non-local host unless `REYN_ALLOW_REMOTE_SMOKE=1` is explicitly set.
