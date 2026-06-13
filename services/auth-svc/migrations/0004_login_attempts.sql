-- Brute-force protection for /auth/login (SECURITY.md §5.2).
-- One row per account (lowercased email): a running count of consecutive
-- failures and, once the threshold is hit, the time until which login is locked.
-- Shared across replicas so a lockout holds platform-wide.
CREATE TABLE IF NOT EXISTS login_attempts (
    email_lower    TEXT PRIMARY KEY,
    failed_count   INTEGER     NOT NULL DEFAULT 0,
    locked_until   TIMESTAMPTZ,
    last_failed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
