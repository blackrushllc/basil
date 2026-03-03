# Phase 2 Wiring Documentation

## Environment Variables
- `OPENAI_API_KEY`: Required for AI queries.
- `TELEEMC_DSN`: MySQL DSN (e.g., `mysql://user:pass@host:3306/slim?ssl-mode=REQUIRED`).
- `TELEEMC_RDS_CA_PATH`: (Optional) Path to RDS CA certificate.
- `TELEEMC_API_TOKEN`: API Token for authentication.
- `TELEEMC_MAX_ROWS`: (Optional) Default 200, cap 1000.

## Read-only DB User setup
```sql
CREATE USER 'teleemc_readonly'@'%' IDENTIFIED BY 'password';
GRANT SELECT ON slim.* TO 'teleemc_readonly'@'%';
-- Ensure SSL is required if connecting over public networks
ALTER USER 'teleemc_readonly'@'%' REQUIRE SSL;
```

## Web Server Routing (Apache/Nginx)
### Apache (.htaccess or config)
```apache
ScriptAlias /api/query /var/www/html/web/api/teleemc-dashboard/cgi/api/query.basil
ScriptAlias /api/export /var/www/html/web/api/teleemc-dashboard/cgi/api/export.basil
```

### Nginx
```nginx
location /api/query {
    fastcgi_pass unix:/run/basil-cgi.sock;
    include fastcgi_params;
    fastcgi_param SCRIPT_FILENAME /var/www/html/web/api/teleemc-dashboard/cgi/api/query.basil;
}
```

## Security
- LLM sees ONLY schema + user prompt, not actual row data.
- Blocked columns: `ecw_password`, `ecw_username`, `gmail_password`, `gmail_username`, `apple_password`, `apple_id`, `remember_token`, and anything containing `password`, `secret`, `token`, etc.
- Soft deletes are enforced for `patients` and `attorney_masters`.
- Audit logging: All queries are logged as artifacts in `.basil/teleemc/artifacts/`.

## Testing
To run smoke tests in test mode:
`basilc test examples\teleemc\01_smoketest_api_plan.basil`
