/** API base URL. Empty in dev — Vite proxies `/api/v1/*` to each service
 * (see `vite.config.ts`). In a deployed build, set `VITE_API_BASE_URL` to the
 * API edge/gateway origin. */
export const API_BASE_URL =
  (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? "";
