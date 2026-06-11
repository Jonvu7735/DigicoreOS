//! Default RBAC policy applied to a freshly provisioned tenant.
//!
//! Roles and the role→permission matrix are a HUMAN-decided policy
//! (SECURITY.md §8.3); this module encodes the matrix approved for the platform
//! (SECURITY.md §4.3). `register` / `create_tenant` seed these per tenant.

/// The five default roles, most-privileged first.
pub const DEFAULT_ROLES: [&str; 5] = ["OWNER", "ADMIN", "MANAGER", "STAFF", "VIEWER"];

/// Every permission code in the catalogue (mirrors the seed in `0001_init.sql`).
pub const ALL_PERMISSIONS: [&str; 32] = [
    "auth_user_read",
    "auth_user_create",
    "auth_user_update",
    "auth_user_assign_role",
    "auth_tenant_read",
    "auth_tenant_update_plan",
    "erp_order_read",
    "erp_order_create",
    "erp_order_update",
    "erp_order_cancel",
    "erp_invoice_read",
    "erp_invoice_create",
    "erp_invoice_cancel",
    "erp_product_read",
    "erp_product_create",
    "erp_product_update",
    "crm_customer_read",
    "crm_customer_create",
    "crm_customer_update",
    "crm_deal_read",
    "crm_deal_create",
    "crm_deal_update",
    "crm_deal_move_stage",
    "hrm_employee_read",
    "hrm_employee_create",
    "hrm_employee_update",
    "hrm_attendance_read",
    "hrm_attendance_create",
    "reporting_dashboard_view",
    "reporting_report_export",
    "ai_assistant_use",
    "ai_config_manage",
];

/// Read-only business permissions shared by every role from VIEWER up.
const BUSINESS_READS: [&str; 8] = [
    "erp_order_read",
    "erp_invoice_read",
    "erp_product_read",
    "crm_customer_read",
    "crm_deal_read",
    "hrm_employee_read",
    "hrm_attendance_read",
    "reporting_dashboard_view",
];

/// Human-readable description stored on the seeded role row.
pub fn role_description(role: &str) -> &'static str {
    match role {
        "OWNER" => "Tenant owner — full control",
        "ADMIN" => "Tenant administrator",
        "MANAGER" => "Department manager",
        "STAFF" => "Operational staff",
        "VIEWER" => "Read-only access",
        _ => "",
    }
}

/// Permission codes granted to `role` by the default matrix. An unknown role
/// gets no permissions.
pub fn permissions_for(role: &str) -> Vec<&'static str> {
    match role {
        "OWNER" => ALL_PERMISSIONS.to_vec(),
        "ADMIN" => ALL_PERMISSIONS
            .iter()
            .copied()
            .filter(|p| *p != "auth_tenant_update_plan")
            .collect(),
        "MANAGER" => {
            let mut p = vec!["auth_user_read", "auth_tenant_read"];
            p.extend_from_slice(&BUSINESS_READS);
            p.extend_from_slice(&[
                "erp_order_create",
                "erp_order_update",
                "erp_order_cancel",
                "erp_product_create",
                "erp_product_update",
                "erp_invoice_create",
                "crm_customer_create",
                "crm_customer_update",
                "crm_deal_create",
                "crm_deal_update",
                "crm_deal_move_stage",
                "hrm_employee_create",
                "hrm_employee_update",
                "hrm_attendance_create",
                "reporting_report_export",
                "ai_assistant_use",
            ]);
            p
        }
        "STAFF" => {
            let mut p = BUSINESS_READS.to_vec();
            p.extend_from_slice(&[
                "erp_order_create",
                "crm_customer_create",
                "crm_customer_update",
                "crm_deal_create",
                "crm_deal_update",
                "crm_deal_move_stage",
                "hrm_attendance_create",
                "ai_assistant_use",
            ]);
            p
        }
        "VIEWER" => {
            let mut p = vec!["auth_user_read", "auth_tenant_read"];
            p.extend_from_slice(&BUSINESS_READS);
            p
        }
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_has_every_permission() {
        assert_eq!(permissions_for("OWNER").len(), ALL_PERMISSIONS.len());
    }

    #[test]
    fn admin_lacks_only_plan_change() {
        let admin = permissions_for("ADMIN");
        assert!(!admin.contains(&"auth_tenant_update_plan"));
        assert_eq!(admin.len(), ALL_PERMISSIONS.len() - 1);
    }

    #[test]
    fn viewer_is_read_only() {
        let viewer = permissions_for("VIEWER");
        assert!(viewer
            .iter()
            .all(|p| p.ends_with("_read") || *p == "reporting_dashboard_view"));
        assert!(viewer.contains(&"crm_customer_read"));
        assert!(!viewer.contains(&"crm_customer_create"));
    }

    #[test]
    fn staff_creates_orders_but_cannot_cancel_or_admin() {
        let staff = permissions_for("STAFF");
        assert!(staff.contains(&"erp_order_create"));
        assert!(!staff.contains(&"erp_order_cancel"));
        assert!(!staff.contains(&"auth_user_read"));
    }

    #[test]
    fn manager_cancels_orders_not_invoices() {
        let manager = permissions_for("MANAGER");
        assert!(manager.contains(&"erp_order_cancel"));
        assert!(!manager.contains(&"erp_invoice_cancel"));
        assert!(!manager.contains(&"auth_user_create"));
    }

    #[test]
    fn every_granted_code_exists_in_catalogue() {
        for role in DEFAULT_ROLES {
            for code in permissions_for(role) {
                assert!(
                    ALL_PERMISSIONS.contains(&code),
                    "{code} missing from catalogue"
                );
            }
        }
    }

    #[test]
    fn unknown_role_has_no_permissions() {
        assert!(permissions_for("ROOT").is_empty());
    }
}
