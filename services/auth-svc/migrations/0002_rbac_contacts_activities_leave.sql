-- Extend the global permission catalogue with CRM contacts/activities and HRM
-- leave (keeps auth_svc.permissions in sync with platform_auth::rbac
-- ALL_PERMISSIONS). Idempotent; tenant role_permissions for new tenants are
-- seeded from the shared matrix at registration.
INSERT INTO permissions (code, description) VALUES
    ('crm_contact_read',     'Read contacts'),
    ('crm_contact_create',   'Create contacts'),
    ('crm_contact_update',   'Update contacts'),
    ('crm_activity_read',    'Read CRM activities'),
    ('crm_activity_create',  'Log a CRM activity'),
    ('crm_activity_update',  'Update a CRM activity'),
    ('hrm_leave_read',       'Read leave requests'),
    ('hrm_leave_request',    'Request leave'),
    ('hrm_leave_approve',    'Approve or reject leave requests')
ON CONFLICT (code) DO NOTHING;
