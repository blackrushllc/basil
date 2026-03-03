# TeleEMC Schema Quick Reference

## Tables

### users
- **PK**: id
- **Fields**: email (unique), name, ...
- **Relationships**: links to patients via fields like user_id, owner_id, atty_id, chiro_id, ortho_id.

### patients (soft delete: deleted_at)
- **PK**: id
- **Relationships**:
  - pip_atty -> attorney_masters.id
  - chiroid -> chiropractors.id
  - homevisit_providerid -> providers.id
- **Critical Fields**: balance, amount_billed, payment1, payment2, payment3, payment4, date1, date2, date3, date4, status.

### chiropractors
- **PK**: id
- **Fields**: name, status (1 = active)
- **Sensitive Fields**: ecw_password, ecw_username, apple_password, gmail_password (NEVER SELECT)

### attorney_masters (soft delete: deleted_at)
- **PK**: id
- **Fields**: name, status (1 = active)
- **Sensitive Fields**: ecw_password, ecw_username (NEVER SELECT)
- **Notes**: multiple contact points (primary attorney, co-counsel, paralegals)

## Allowed query patterns
- Always specify explicit column list (no SELECT *).
- Use JOINs with ON.
- For “counts” use COUNT(*).
- For date ranges use CURDATE(), DATE_SUB, DATEDIFF, etc.
- Always include `deleted_at IS NULL` where applicable (patients, attorney_masters).
