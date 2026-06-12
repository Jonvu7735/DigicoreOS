-- Shipment status timeline: one row per status change (and the opening
-- creation), written in the same transaction as the change it records.

CREATE TABLE IF NOT EXISTS export_shipment_status_history (
    id          UUID PRIMARY KEY,
    shipment_id UUID        NOT NULL REFERENCES export_shipments (id) ON DELETE CASCADE,
    tenant_id   TEXT        NOT NULL,
    -- NULL on the opening entry (creation of the DRAFT).
    from_status TEXT,
    to_status   TEXT        NOT NULL,
    at          TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS export_shipment_status_history_shipment_idx
    ON export_shipment_status_history (shipment_id, at);
