# Cloudflare Worker — AI-proxy

Worker som relayar anrop från appen till Anthropic API. API-nyckeln hanteras server-sidan och exponeras aldrig för slutanvändaren.

## Var ligger koden

`/home/tubbs/code/fr-desktop/fr-ai-proxy/`

Deployad på: `https://fr-ai-proxy.tubbs.workers.dev`

## Hur det fungerar

Appen skickar anrop till workern med en inbyggd `x-app-secret`. Workern verifierar hemligheten och vidarebefordrar anropet till Anthropic med den riktiga API-nyckeln.

```
fr-app (Rust) → fr-ai-proxy (Cloudflare) → api.anthropic.com
```

Endpoints workern hanterar:
- `GET /v1/models` — lista tillgängliga Claude-modeller
- `POST /v1/messages` — skicka prompt, få svar

## Konstanter i Rust (lib.rs)

```rust
const AI_PROXY_URL: &str = "https://fr-ai-proxy.tubbs.workers.dev";
const AI_PROXY_SECRET: &str = "...";  // inbyggd i binären
```

## Secrets i Cloudflare

Två secrets satta via `wrangler secret put`:
- `ANTHROPIC_API_KEY` — Anthropic API-nyckel
- `APP_SECRET` — delas med appen via den inbyggda konstanten

## Uppdatera secrets / deploya om

Kör setup-scriptet i worker-mappen:

```bash
cd /home/tubbs/code/fr-desktop/fr-ai-proxy
./setup-secrets.sh
```

Scriptet frågar efter nya värden för båda secrets och kör `wrangler deploy` på slutet.

Kräver Node 22 via nvm — scriptet laddar det automatiskt.

## Deploya utan att byta secrets

```bash
cd /home/tubbs/code/fr-desktop/fr-ai-proxy
source ~/.nvm/nvm.sh && nvm use 22 && bunx wrangler deploy
```
