// k6 load/perf smoke test for the DigicoreOS API edge.
//
// Drives the real auth + read paths so you can establish a latency/throughput
// baseline and right-size the k8s resource requests/limits (the review flags
// capacity planning as a pre-GA task). Thresholds fail the run (non-zero exit)
// so this can gate a release in CI or a staging pipeline.
//
// Run:
//   k6 run -e BASE_URL=https://api.digicore.example.com \
//          -e EMAIL=owner@acme.test -e PASSWORD=secret \
//          deploy/load/k6-smoke.js
//
// Tune load with -e VUS=50 -e DURATION=2m (or edit `options.stages`).

import http from "k6/http";
import { check, sleep } from "k6";
import { Rate } from "k6/metrics";

const BASE_URL = __ENV.BASE_URL || "http://localhost:8081";
const EMAIL = __ENV.EMAIL || "owner@acme.test";
const PASSWORD = __ENV.PASSWORD || "secret";

const loginFailRate = new Rate("login_failed");

export const options = {
  stages: __ENV.VUS
    ? [{ duration: __ENV.DURATION || "1m", target: Number(__ENV.VUS) }]
    : [
        { duration: "30s", target: 20 }, // ramp up
        { duration: "1m", target: 20 }, // steady
        { duration: "30s", target: 0 }, // ramp down
      ],
  thresholds: {
    http_req_failed: ["rate<0.01"], // <1% of requests fail
    http_req_duration: ["p(95)<500"], // 95% under 500ms
    login_failed: ["rate<0.01"],
  },
};

function login() {
  const res = http.post(
    `${BASE_URL}/api/v1/auth/login`,
    JSON.stringify({ email: EMAIL, password: PASSWORD }),
    { headers: { "Content-Type": "application/json" }, tags: { name: "login" } },
  );
  const ok = check(res, { "login 200": (r) => r.status === 200 });
  loginFailRate.add(!ok);
  if (!ok) return null;
  return res.json("access_token");
}

export default function () {
  const token = login();

  // Liveness/readiness are unauthenticated and cheap — always exercise them.
  check(http.get(`${BASE_URL}/api/v1/auth/health`, { tags: { name: "health" } }), {
    "health 200": (r) => r.status === 200,
  });

  if (token) {
    const authd = { headers: { Authorization: `Bearer ${token}` } };
    // A representative authorized read (orders report). Adjust per environment.
    check(
      http.get(`${BASE_URL}/api/v1/reporting/dashboard/sales-summary`, {
        ...authd,
        tags: { name: "sales-summary" },
      }),
      { "report not 5xx": (r) => r.status < 500 },
    );
  }

  sleep(1);
}
