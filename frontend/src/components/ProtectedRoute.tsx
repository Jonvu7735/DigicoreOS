import { Navigate } from "react-router-dom";

import { useAuth } from "../auth/useAuth";
import { AppLayout } from "./AppLayout";

/** Gate for authenticated routes: bounce to /login when there's no token,
 * otherwise render the app shell (navbar + content) around the nested routes. */
export function ProtectedRoute() {
  const { token } = useAuth();
  if (!token) {
    return <Navigate to="/login" replace />;
  }
  return <AppLayout />;
}
