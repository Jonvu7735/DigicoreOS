# HA Postgres (CloudNativePG) — opt-in overlay

`cluster.yaml` replaces the single-pod `deploy/k8s/20-postgres.yaml` with a
3-instance, auto-failover Postgres managed by the
[CloudNativePG](https://cloudnative-pg.io) operator, with continuous WAL
archiving (PITR) + scheduled base backups to object storage.

This is the recommended path for production HA (the go-live review flags the
single primary + logical-only backup as the remaining data-tier gap).

## Use it

1. Install the operator:
   ```sh
   kubectl apply --server-side -f \
     https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/release-1.24/releases/cnpg-1.24.0.yaml
   ```
2. Create the app credentials secret CNPG expects (username/password keys):
   ```sh
   kubectl -n digicore create secret generic digicore-postgres-app \
     --from-literal=username=digicore --from-literal=password='<strong-password>'
   ```
3. Create the backup credentials secret and set `destinationPath` in `cluster.yaml`:
   ```sh
   kubectl -n digicore create secret generic digicore-pg-backup-creds \
     --from-literal=ACCESS_KEY_ID=... --from-literal=ACCESS_SECRET_KEY=...
   ```
4. Remove `20-postgres.yaml` from the root `kustomization.yaml` (don't run both),
   then apply this overlay:
   ```sh
   kubectl apply -f deploy/k8s/ha-postgres/cluster.yaml
   ```

`DATABASE_URL` is unchanged: the `postgres` Service is an ExternalName alias to
the CNPG read/write endpoint (`digicore-pg-rw`).

> Tip: keep `JETSTREAM_REPLICAS` and the Postgres instance count in sync with
> your node count and failure-domain (zone) topology.
