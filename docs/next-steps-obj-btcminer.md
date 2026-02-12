# Next Steps: obj-btcminer

## Version 1.0 (Current)
- Generic HTTP/JSON adapter with profile-based extraction.
- Support for Antminer, WhatsMiner, and Braiins OS via templates.
- Snapshot and Alert evaluation logic.
- SQLite logging for historical stats and alerts.

## Proposed v1.1 Milestones
1. **Stratum Visibility**: Add visibility into worker/pool status from the miner's perspective.
2. **Vendor-specific Adapters**: Add native adapters for Antminer (e.g. using SSH or specialized API) to handle more complex auth and edge cases.
3. **Advanced Auth**: Support Digest auth and session-based (cookie) auth renewal.
4. **Local Monitoring**: Add a small local HTTP status server or Prometheus exporter.

## Testing Plan
- Use `examples/btcminer_profile_test.basil` with a real or mock device to verify field extraction.
- Validate SQLite database entries after running `examples/btcminer_monitor.basil`.
- Verify alerts are triggered correctly by setting artificially low/high thresholds.
