import { Navigate, Outlet } from "react-router-dom";

import { useAuth } from "../auth/useAuth";

/** Gate for authenticated routes: bounce to /login when there's no token. */
export function ProtectedRoute() {
  const { token } = useAuth();
  if (!token) {
    return <Navigate to="/login" replace />;
  }
  return <Outlet />;
}
