#!/usr/bin/env node
// Revisit consumer entrypoint; historical protocol-4 replay uses RUFREN_RELAY_QA_PROTOCOL=4.
process.env.RUFREN_RELAY_QA_MODE ||= 'revisit';
await import('./test-library-relay.mjs');
