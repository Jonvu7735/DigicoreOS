#!/usr/bin/env bash
# Generate a LOCAL DEV RSA key pair for auth-svc RS256 JWT signing.
#
# Output: ./.dev/jwt_private.pem (PKCS#8) + ./.dev/jwt_public.pem (SPKI).
# `.dev/` is gitignored – these keys are dev-only and MUST NOT be committed or
# used in staging/prod (SECURITY.md: secrets via secret manager / K8s secrets).
#
# auth-svc (APP_ENV=dev) loads these paths automatically (bootstrap/config.rs).
set -euo pipefail

DEST="${1:-.dev}"
mkdir -p "$DEST"

if [[ -f "$DEST/jwt_private.pem" && -f "$DEST/jwt_public.pem" ]]; then
  echo "dev JWT keys already exist in $DEST/ – leaving them in place."
  exit 0
fi

openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$DEST/jwt_private.pem" 2>/dev/null
openssl rsa -in "$DEST/jwt_private.pem" -pubout -out "$DEST/jwt_public.pem" 2>/dev/null
chmod 600 "$DEST/jwt_private.pem"

echo "Generated dev RS256 keys:"
echo "  $DEST/jwt_private.pem (signing)"
echo "  $DEST/jwt_public.pem  (verification)"
