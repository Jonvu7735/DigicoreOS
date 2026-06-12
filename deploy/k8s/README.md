# DigicoreOS — Kubernetes manifests

Deploys the full platform into the `digicore` namespace: a shared Postgres and
NATS (JetStream), plus a Deployment + Service for each of the six services.

| File | Resources |
|---|---|
| `00-namespace.yaml` | `digicore` namespace |
| `10-config.yaml` | `digicore-config` ConfigMap, `digicore-postgres` Secret |
| `20-postgres.yaml` | Postgres Deployment + Service + PVC |
| `30-nats.yaml` | NATS (JetStream) Deployment + Service |
| `40-services.yaml` | the six services (Deployment + ClusterIP Service each) |
| `50-ingress.yaml` | API edge: one HTTPS host routing `/api/v1/<domain>/` to each service, with edge rate limiting |
| `60-network-policy.yaml` | Default-deny ingress + allowlist (ingress→services, services→postgres/nats) |

Each service applies its own migrations on boot (creating its schema first — one
schema per service in the shared database) and verifies RS256 tokens with the
public key. auth-svc additionally signs with the private key.

## 1. Build & push images

The manifests reference `digicore/<svc>:latest`. Build each from the shared
Dockerfile and push to a registry your cluster can pull from (then update the
`image:` fields, or pre-load them into the node, e.g. `kind load` / `minikube image load`):

```bash
REG=your-registry.example.com   # or omit for a local single-node cluster
for s in auth-svc core-erp-svc crm-svc hrm-svc reporting-svc ai-svc; do
  docker build -f deploy/Dockerfile --build-arg SERVICE=$s -t ${REG:+$REG/}digicore/$s:latest .
  [ -n "$REG" ] && docker push $REG/digicore/$s:latest
done
```

## 2. Create the JWT secret (never committed)

```bash
bash scripts/gen-dev-jwt-keys.sh   # writes .dev/jwt_{private,public}.pem (gitignored)
kubectl create namespace digicore --dry-run=client -o yaml | kubectl apply -f -
kubectl -n digicore create secret generic digicore-jwt \
  --from-file=jwt_public.pem=.dev/jwt_public.pem \
  --from-file=jwt_private.pem=.dev/jwt_private.pem
```

For production, generate a fresh keypair and store it in your secret manager.

## 3. Apply

```bash
kubectl apply -k deploy/k8s            # or: kubectl apply -f deploy/k8s/
kubectl -n digicore get pods -w
```

## 4. API edge (Ingress)

`50-ingress.yaml` fronts the six ClusterIP services with a single HTTPS host
that serves the public `/api/v1/<domain>/` surface (`API-GATEWAY.md`) and
rate-limits per client IP at the edge (`SECURITY.md §5`). It assumes the
[ingress-nginx](https://kubernetes.github.io/ingress-nginx/) controller; for a
different controller, translate the `nginx.ingress.kubernetes.io/*` annotations.

Before applying, set your host and TLS:

```bash
# 1. Point the host at your domain (edit both the rule and tls hosts), e.g.:
sed -i 's/api.digicore.example.com/api.acme.com/g' deploy/k8s/50-ingress.yaml

# 2. Provide the TLS cert as secret `digicore-tls` — either let cert-manager
#    issue it (uncomment the cluster-issuer annotation) or create it directly:
kubectl -n digicore create secret tls digicore-tls --cert=tls.crt --key=tls.key
```

Routing preserves the full path, so each request reaches its service unchanged
(`/api/v1/erp/orders` → `core-erp-svc`). JWT is still verified per service; the
edge adds TLS termination and throttling. `/api/v1/auth/login` and
`/api/v1/auth/refresh` get a tighter limit than the rest of the surface to blunt
credential stuffing. Tune the `limit-rps` / `limit-rpm` annotations for your
traffic.

## 5. Smoke test

```bash
# Through the edge (once DNS + TLS are set up):
curl https://api.acme.com/api/v1/auth/health

# Or bypass the edge and hit a service directly:
kubectl -n digicore port-forward svc/auth-svc 8081:8081 &
curl localhost:8081/api/v1/auth/health
```

## Notes

- **Dev credentials**: `10-config.yaml` ships `digicore/digicore` Postgres creds
  and a matching `DATABASE_URL`. Change both for any real deployment.
- **Database**: a single in-cluster Postgres is included for convenience; for
  production prefer a managed/HA database and delete `20-postgres.yaml`, pointing
  `DATABASE_URL` at it.
- **Ingress**: services are `ClusterIP`, fronted by `50-ingress.yaml` (see §4).
  It needs an ingress controller (ingress-nginx) plus a real host and the
  `digicore-tls` secret; without a controller the Ingress object is created but
  inert.
- **Network policy**: `60-network-policy.yaml` enforces `SECURITY.md §5.1`.
  **Ingress** is default-deny + an allowlist (ingress→services, services→
  postgres/nats). **Egress** is also default-deny: only DNS, services→postgres/
  nats, and ai-svc→external HTTPS are permitted. It needs a CNI that enforces
  NetworkPolicy (Calico, Cilium, …). Add new vertical services to the
  `app In (...)` selectors when they need the database or event bus; if the DNS
  allow doesn't match your cluster (CoreDNS not in `kube-system`), adjust
  `allow-dns-egress`.
