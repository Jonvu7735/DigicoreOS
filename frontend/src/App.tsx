import { Navigate, Route, Routes } from "react-router-dom";

import { ProtectedRoute } from "./components/ProtectedRoute";
import { DemoPage } from "./pages/DemoPage";
import { HomePage } from "./pages/HomePage";
import { LoginPage } from "./pages/LoginPage";
import { LoyaltyPage } from "./pages/LoyaltyPage";
import { ShipmentsPage } from "./pages/ShipmentsPage";

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route element={<ProtectedRoute />}>
        <Route path="/" element={<HomePage />} />
        <Route path="/loyalty" element={<LoyaltyPage />} />
        <Route path="/shipments" element={<ShipmentsPage />} />
        <Route path="/demo" element={<DemoPage />} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
