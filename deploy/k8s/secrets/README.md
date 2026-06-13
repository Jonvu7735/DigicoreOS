# Production secret management

The default install ships a **dev-grade** `digicore-postgres` Secret inline in
`10-config.yaml` and expects `digicore-jwt` to be created out of band. Neither is
acceptable for production: secrets must not live in git in plaintext, and they
should be rotated.

Pick ONE of the two GitOps-friendly approaches below. Both keep the consuming
manifests unchanged — they still reference the Secrets `digicore-postgres` and
`digicore-jwt`; only how those Secrets get created changes.

## Option A — Sealed Secrets (Bitnami)

Encrypt secrets with the cluster's public key so the *encrypted* form is safe to
commit. The controller decrypts them in-cluster into real Secrets.

```sh
# Install the controller, then seal a secret with kubeseal:
kubectl -n digicore create secret generic digicore-postgres \
  --from-literal=POSTGRES_USER=digicore \
  --from-literal=POSTGRES_PASSWORD="$(openssl rand -base64 24)" \
  --from-literal=POSTGRES_DB=digicore \
  --dry-run=client -o yaml \
  | kubeseal --format yaml > digicore-postgres.sealed.yaml   # commit this
```

`sealed-secret.example.yaml` shows the resulting shape (the `encryptedData` is
ciphertext — placeholders here). Remove the inline dev Secret from
`10-config.yaml` and apply the sealed manifests instead.

## Option B — External Secrets Operator

Keep the source of truth in a real secret manager (AWS Secrets Manager, GCP
Secret Manager, Vault, …) and sync it into k8s Secrets. `external-secret.example.yaml`
defines a `SecretStore` + two `ExternalSecret`s that materialise
`digicore-postgres` and `digicore-jwt`. No secret value ever touches git.

## Either way

- Remove the dev `digicore-postgres` Secret from `10-config.yaml`.
- Rotate the RS256 JWT keypair and DB password on a schedule.
- These manifests are intentionally NOT in the root `kustomization.yaml`
  (choose and wire the one you use).
