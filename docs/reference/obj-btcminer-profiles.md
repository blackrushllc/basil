# BTC Miner Profiles

Profiles are JSON configurations that tell the `generic-http-json` adapter how to talk to a specific miner and extract data.

## Schema

```json
{
  "profile_name": "string",
  "kind": "generic-http-json",
  "auth": {
    "type": "basic | none | cookie",
    "username_key": "string (optional)",
    "password_key": "string (optional)"
  },
  "endpoints": {
    "info":  { "method": "GET | POST", "path": "string" },
    "stats": { "method": "GET | POST", "path": "string" },
    "reboot": { "method": "GET | POST", "path": "string" }
  },
  "extract": {
    "model": "dot.path",
    "firmware": "dot.path",
    "uptime_s": "dot.path",
    "hashrate_ghs": "dot.path",
    "temp_max_c": "dot.path",
    "fan_rpm_avg": "dot.path",
    "accepted": "dot.path",
    "rejected": "dot.path",
    "pool_url": "dot.path",
    "pool_user": "dot.path"
  }
}
```

## Built-in Profiles

1. `antminer_http_v1`: Template for Bitmain Antminers using the CGI API.
2. `whatsminer_http_v1`: Template for WhatsMiner devices.
3. `braiins_os_v1`: Template for devices running Braiins OS.

## Customizing Profiles

You can create your own profile by copying one of the templates and adjusting the `endpoints` and `extract` paths. Use browser developer tools to inspect the JSON returned by your miner's web interface.
