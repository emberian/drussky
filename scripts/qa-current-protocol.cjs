// Current-client expectations follow the checked-out core contract. Historical
// migration/input fixtures retain literal versions and never import this helper.
const fs = require('node:fs');
const path = require('node:path');
const source = fs.readFileSync(path.join(__dirname, '../crates/rufren-core/src/lib.rs'), 'utf8');
const declaration = source.match(/^pub const SYNC_VERSION: u32 = (\d+);$/m);
if (!declaration) throw Error('Cannot read the current core SYNC_VERSION declaration');
const CURRENT_PROTOCOL = Number(declaration[1]);
if (!Number.isSafeInteger(CURRENT_PROTOCOL) || CURRENT_PROTOCOL < 1) throw Error('Invalid core SYNC_VERSION');
module.exports = { CURRENT_PROTOCOL };
