import { type ReactNode, useCallback, useMemo, useState } from "react";

import { createApi } from "../api/client";
import { AuthContext, type User } from "./context";

const STORAGE_KEY = "digicore.auth";

interface Stored {
  token: string;
  user: User;
}

function load(): Stored | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Stored) : null;
  } catch {
    return null;
  }
}

/** Read the JWT payload (sub/tenant_id/roles) for display. Not a security check
 * — the backend verifies the RS256 signature on every request. */
function decodeClaims(token: string): User {
  const payload = token.split(".")[1] ?? "";
  const json = atob(payload.replace(/-/g, "+").replace(/_/g, "/"));
  const claims = JSON.parse(json) as {
    sub?: string;
    tenant_id?: string;
    roles?: string[];
  };
  return {
    id: claims.sub ?? "",
    tenant_id: claims.tenant_id ?? "",
    roles: claims.roles ?? [],
  };
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [stored, setStored] = useState<Stored | null>(load);

  const login = useCallback(
    async (email: string, password: string, tenantId?: string) => {
      const { data, error } = await createApi().POST("/api/v1/auth/login", {
        body: { email, password, tenant_id: tenantId || undefined },
      });
      if (error || !data) {
        throw new Error("Email hoặc mật khẩu không đúng.");
      }
      const next: Stored = {
        token: data.access_token,
        user: decodeClaims(data.access_token),
      };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      setStored(next);
    },
    [],
  );

  const logout = useCallback(() => {
    localStorage.removeItem(STORAGE_KEY);
    setStored(null);
  }, []);

  const value = useMemo(
    () => ({
      token: stored?.token ?? null,
      user: stored?.user ?? null,
      login,
      logout,
    }),
    [stored, login, logout],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
