-- Cargo lines: the packing-list / commercial-invoice rows of a shipment.
-- Child of export_shipments; removed with its parent.

CREATE TABLE IF NOT EXISTS export_cargo_lines (
    id            UUID PRIMARY KEY,
    shipment_id   UUID        NOT NULL REFERENCES export_shipments (id) ON DELETE CASCADE,
    tenant_id     TEXT        NOT NULL,
    description   TEXT        NOT NULL,
    -- Harmonized System tariff code (6-10 digits), when the goods are classified.
    hs_code       TEXT,
    quantity      BIGINT      NOT NULL,
    unit          TEXT        NOT NULL,
    net_weight_kg DOUBLE PRECISION,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS export_cargo_lines_shipment_idx
    ON export_cargo_lines (shipment_id);
