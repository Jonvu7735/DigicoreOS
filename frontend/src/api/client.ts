import createClient, { type Client } from "openapi-fetch";

import { API_BASE_URL } from "../config";
import type { paths } from "./schema";

/**
 * A fully-typed platform API client (every path/param/response is checked
 * against `docs/openapi.yaml` via the generated `schema.d.ts`). Pass the RS256
 * access token from `POST /api/v1/auth/login` to send it as `Bearer`.
 */
export function createApi(token?: string): Client<paths> {
  return createClient<paths>({
    baseUrl: API_BASE_URL,
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  });
}
