# Specular server

The Cloudflare Worker behind share links: owner accounts, capability links,
content-addressed assets in R2, published versions of a canvas, and the
comments people leave on them. It also serves the web viewer (`../web`),
which is the page a link opens. The design is
[ADR 0049](../docs/adr/0049-view-and-comment-web-client.md), on top of
[ADR 0018](../docs/adr/0018-cloud-sync-and-canvas-sharing.md).

```
desktop ──share.ts publish──▶ POST /docs/:id/assets, /docs/:id/versions
browser ──/c/:id#t=<link>──▶ POST /redeem ─▶ GET /docs/:id/manifest ─▶ GET /assets/:hash
          comments        ◀─▶ GET|POST /docs/:id/comments, …/replies, PATCH …/:cid
desktop ◀─share.ts pull─────  GET /docs/:id/comments (merged into the .canvas)
```

## Routes

| Route | Who | What |
| --- | --- | --- |
| `/api/auth/*` | anyone | better-auth: anonymous sign-in, sessions, API keys |
| `POST /docs` | signed in | a new doc owned by the caller |
| `POST\|GET /docs/:id/links`, `…/links/:grant/reset`, `DELETE …/links/:grant` | owner | capability links (`view`, `comment`, `edit`) |
| `POST /docs/:id/connect` | owner | an edit connection token |
| `POST /redeem` | a link token | a short-lived connection token |
| `POST /docs/:id/assets` | edit | upload bytes; answers their SHA-256 |
| `GET /assets/:hash` | anyone | the bytes (the hash is the secret) |
| `POST /docs/:id/versions` | edit | publish a manifest; every asset it names must exist |
| `GET /docs/:id/manifest` | any scope | the newest version and the caller's scope |
| `GET /docs/:id/comments` | any scope | every comment, as `.canvas` annotations |
| `POST /docs/:id/comments` | comment, edit | `{ annotation, authorName }` |
| `POST /docs/:id/comments/:cid/replies` | comment, edit | `{ text, authorName }` |
| `PATCH /docs/:id/comments/:cid` | comment, edit | `{ status }`: resolve or reopen |
| `DELETE /docs/:id/comments/:cid` | edit | remove a comment |

Connection tokens go in `Authorization: Bearer <token>`.

## Run it locally

```sh
pnpm install --ignore-workspace
../web/build.sh                      # the viewer's WebAssembly, into web/pkg
printf 'BETTER_AUTH_SECRET=dev\nBETTER_AUTH_URL=http://localhost:8787\n' > .dev.vars
npx wrangler d1 migrations apply specular-auth --local
npx wrangler dev --port 8787
```

Then publish a canvas to it, with the app open on that canvas so its pages
get snapshots:

```sh
node scripts/share.ts publish ~/Specular/board.canvas --server http://localhost:8787
node scripts/share.ts pull ~/Specular/board.canvas     # comments into the .canvas
node scripts/share.ts links ~/Specular/board.canvas    # print the links again
```

`build.sh` needs `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-bindgen-cli --version 0.2.129 --locked` (the version
`native/Cargo.toml` pins).

## Test

```sh
pnpm test        # vitest against the worker under miniflare
pnpm typecheck
```

## Deploy

```sh
npx wrangler secret put BETTER_AUTH_SECRET
npx wrangler d1 migrations apply specular-auth --remote
../web/build.sh && npx wrangler deploy
```
