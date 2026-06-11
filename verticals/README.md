# verticals/

Vertical modules (retail, trade-export, insurance, banking, manufacturing) live here
(or in separate repos) **outside** the core workspace.

Hard rules (see ARCHITECTURE.md §7, AI-FIRST-ARCHITECTURE.md):
- Verticals consume the core ONLY via public HTTP APIs (`/api/v1/...`) and business events (NATS).
- Verticals NEVER import core service crates, query core schemas, or change core code.
- The core stays sector-agnostic: generic ERP/CRM/HRM/Reporting/AI only.
