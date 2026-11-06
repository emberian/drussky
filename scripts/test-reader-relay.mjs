#!/usr/bin/env node
// Reader originals/excerpts/dialogue anchors through real relay/restore/empty app.
process.env.RUFREN_RELAY_QA_MODE ||= 'reader';
await import('./test-library-relay.mjs');
