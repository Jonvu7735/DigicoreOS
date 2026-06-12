import { createContext } from "react";

/** Identity derived from the RS256 access-token claims. The login response only
 * returns the token (sub/tenant_id/roles live in the JWT), so we read these
 * client-side for display — the backend still verifies the signature per call. */
export interface User {
  id: string;
  tenant_id: string;
  roles: string[];
}

export interface AuthContextValue {
  token: string | null;
  user: User | null;
  /** Authenticate against auth-svc; throws on failure. */
  login: (email: string, password: string, tenantId?: string) => Promise<void>;
  logout: () => void;
}

export const AuthContext = createContext<AuthContextValue | null>(null);
