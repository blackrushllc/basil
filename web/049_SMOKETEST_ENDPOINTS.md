# TeleEMC Endpoint Smoke Tests

Use these examples to verify the API endpoints. Replace `<token>` with your `TELEEMC_API_TOKEN` (default: `dev-teleemc-token`).

## 1. Query API (`/api/query`)

### Simple natural language query
```bash
curl -X POST http://localhost/api/teleemc-dashboard/cgi/api/query.basil \
     -H "Authorization: Bearer dev-teleemc-token" \
     -H "Content-Type: application/json" \
     -d '{"prompt": "How many active patients do we have?"}'
```

### Query for a specific artifact (if you know the prompt)
```bash
curl -X POST http://localhost/api/teleemc-dashboard/cgi/api/query.basil \
     -H "Authorization: Bearer dev-teleemc-token" \
     -H "Content-Type: application/json" \
     -d '{"prompt": "List the top 10 attorney masters by patient count"}'
```

## 2. Export API (`/api/export`)

### Export as CSV
Replace `A12345` with a real `artifact_id` returned from the query API.
```bash
curl -X POST http://localhost/api/teleemc-dashboard/cgi/api/export.basil \
     -H "Authorization: Bearer dev-teleemc-token" \
     -H "Content-Type: application/json" \
     -d '{"artifact_id": "A12345", "format": "csv"}' \
     --output export.csv
```

### Export as Text
```bash
curl -X POST http://localhost/api/teleemc-dashboard/cgi/api/export.basil \
     -H "Authorization: Bearer dev-teleemc-token" \
     -H "Content-Type: application/json" \
     -d '{"artifact_id": "A12345", "format": "txt"}'
```

## 3. Error Case: Unauthorized
```bash
curl -X POST http://localhost/api/teleemc-dashboard/cgi/api/query.basil \
     -H "Authorization: Bearer WRONG_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"prompt": "test"}'
```

## 4. Error Case: Blocked Column
```bash
curl -X POST http://localhost/api/teleemc-dashboard/cgi/api/query.basil \
     -H "Authorization: Bearer dev-teleemc-token" \
     -H "Content-Type: application/json" \
     -d '{"prompt": "Show me ecw_password for all chiropractors"}'
```
