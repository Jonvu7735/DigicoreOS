/**
 * Typed client for the DigicoreOS platform API.
 *
 * The types in `./schema.d.ts` are GENERATED from `docs/openapi.yaml`
 * (`npm run generate`) — do not edit them by hand. Every path, method,
 * parameter and response is typed from the OpenAPI contract, so the compiler
 * flags any drift between the frontend and the API.
 */
import createClient, { type Client } from "openapi-fetch";

import type { paths } from "./schema";

export type { components, operations, paths } from "./schema";

export interface ApiClientOptions {
  /** Base URL of the API edge, e.g. `https://api.digicore.example.com`. */
  baseUrl: string;
  /** RS256 access token from `POST /api/v1/auth/login` (sent as `Bearer`). */
  token?: string;
  /** Tenant id; only needed when it isn't already embedded in the JWT. */
  tenantId?: string;
}

/**
 * Create a fully-typed client for the platform API.
 *
 * ```ts
 * const api = createApiClient({ baseUrl, token });
 * const { data, error } = await api.GET("/api/v1/erp/orders", {
 *   params: { query: { page: 1, page_size: 20 } },
 * });
 * ```
 */
export function createApiClient(options: ApiClientOptions): Client<paths> {
  const headers: Record<string, string> = {};
  if (options.token) headers.Authorization = `Bearer ${options.token}`;
  if (options.tenantId) headers["X-Tenant-Id"] = options.tenantId;
  return createClient<paths>({ baseUrl: options.baseUrl, headers });
}
