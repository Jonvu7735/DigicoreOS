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

## 4. Smoke test

```bash
kubectl -n digicore port-forward svc/auth-svc 8081:8081 &
curl localhost:8081/api/v1/auth/health
```

## Notes

- **Dev credentials**: `10-config.yaml` ships `digicore/digicore` Postgres creds
  and a matching `DATABASE_URL`. Change both for any real deployment.
- **Database**: a single in-cluster Postgres is included for convenience; for
  production prefer a managed/HA database and delete `20-postgres.yaml`, pointing
  `DATABASE_URL` at it.
- **Ingress**: services are `ClusterIP`. Front them with an Ingress/gateway that
  routes `/api/v1/<svc>/...` and terminates TLS (out of scope here).
