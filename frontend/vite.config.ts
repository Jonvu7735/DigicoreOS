import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// No API gateway yet, so in dev we proxy each `/api/v1/<service>` prefix to that
// service's local HTTP port (see deploy/docker-compose.dev.yml + verticals).
const SERVICE_PORTS: Record<string, number> = {
  auth: 8081,
  erp: 8082,
  crm: 8083,
  hrm: 8084,
  reporting: 8085,
  ai: 8086,
  "trade-export": 8087,
  retail: 8088,
};

const proxy = Object.fromEntries(
  Object.entries(SERVICE_PORTS).map(([name, port]) => [
    `/api/v1/${name}`,
    { target: `http://localhost:${port}`, changeOrigin: true },
  ]),
);

export default defineConfig({
  plugins: [react()],
  server: { port: 5173, proxy },
});
