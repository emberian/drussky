#!/usr/bin/env node
// Producer timing observations through HTTP, coherent SQLite/audio restore, empty app.
process.env.RUFREN_RELAY_QA_MODE ||= 'timing';
await import('./test-library-relay.mjs');
