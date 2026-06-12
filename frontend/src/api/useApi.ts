import { useMemo } from "react";

import { useAuth } from "../auth/useAuth";
import { createApi } from "./client";

/** A typed API client bound to the current session token, recreated when the
 * token changes (login/logout). */
export function useApi() {
  const { token } = useAuth();
  return useMemo(() => createApi(token ?? undefined), [token]);
}
