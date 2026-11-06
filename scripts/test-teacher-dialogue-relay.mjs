#!/usr/bin/env node
// Consume final synthetic dialogue exports; optional originals.json enables full audio restore.
process.env.RUFREN_RELAY_QA_MODE ||= 'dialogue';
await import('./test-library-relay.mjs');
