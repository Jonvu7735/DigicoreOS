# Load / performance testing (k6)

`k6-smoke.js` is a [k6](https://k6.io) script that drives the real auth + read
paths through the API edge. Use it to establish a latency/throughput baseline
and to right-size the k8s resource requests/limits (the go-live review flags
capacity planning as a pre-GA task).

## Run

```sh
k6 run -e BASE_URL=https://api.digicore.example.com \
       -e EMAIL=owner@acme.test -e PASSWORD='<password>' \
       deploy/load/k6-smoke.js

# Heavier, fixed load:
k6 run -e BASE_URL=... -e EMAIL=... -e PASSWORD=... -e VUS=100 -e DURATION=3m \
       deploy/load/k6-smoke.js
```

## Thresholds (fail the run)

- `http_req_failed` < 1%
- `http_req_duration` p95 < 500ms
- `login_failed` < 1%

A failing threshold exits non-zero, so this can gate a release in a staging
pipeline.

## Using the results to size resources

The services currently request `50m / 64Mi` and cap at `500m / 256Mi`
(`deploy/k8s/40-services.yaml`), with HPAs scaling 2→5 on 70% CPU
(`46-hpa.yaml`). After a representative run:

- If p95 latency climbs under load while CPU is well below the HPA target, the
  bottleneck is likely the DB or a low limit — raise `limits.cpu`/`memory`.
- If pods are CPU-throttled (HPA pinned at max), raise `maxReplicas` and/or
  per-pod CPU.
- Set `requests` near the observed steady-state so the scheduler bin-packs
  accurately; keep `limits` with headroom for spikes.
