#!/usr/bin/env node
// Actual Sound facts + linked originals through disposable relay/restore/empty app.
process.env.RUFREN_RELAY_QA_MODE ||= 'sound';
await import('./test-library-relay.mjs');
