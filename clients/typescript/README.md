# @digicore/api-client

Typed TypeScript client for the DigicoreOS platform API, **generated from the
single source of truth** — [`docs/openapi.yaml`](../../docs/openapi.yaml).

## Use

```ts
import { createApiClient } from "@digicore/api-client";

const api = createApiClient({
  baseUrl: "https://api.digicore.example.com",
  token, // RS256 access token from POST /api/v1/auth/login
});

const { data, error } = await api.GET("/api/v1/erp/orders", {
  params: { query: { page: 1, page_size: 20 } },
});

await api.POST("/api/v1/trade-export/shipments", {
  body: { destination_country: "VN", incoterm: "FOB" },
});
```

Every path, method, parameter and response is typed from the OpenAPI contract
(via [`openapi-typescript`](https://openapi-ts.dev) for the types and
[`openapi-fetch`](https://openapi-ts.dev/openapi-fetch/) for the runtime), so the
compiler flags any drift between the frontend and the API.

## Regenerate

The generated types live in `src/schema.d.ts`. After editing `docs/openapi.yaml`:

```bash
cd clients/typescript
npm install
npm run generate    # rewrites src/schema.d.ts from docs/openapi.yaml
npm run typecheck
```

CI (the **`openapi client (typescript)`** job) runs `npm run generate` and fails
if the committed `src/schema.d.ts` is stale — keeping the client and the spec in
lockstep.
