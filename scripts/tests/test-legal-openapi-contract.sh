#!/usr/bin/env bash
#
# Static OpenAPI guard for legal-readiness and module surfaces.
#
# This checks the generated OpenAPI snapshot only; it does not start the app.
# Keep this guard cheap so docs/legal policy work can run it under pressure.
#
# Run: bash scripts/tests/test-legal-openapi-contract.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

node <<'NODE'
const fs = require('fs');

const spec = JSON.parse(fs.readFileSync('docs/openapi/rust.json', 'utf8'));
const paths = spec.paths || {};

const required = [
  ['GET', '/api/v1/legal/impressum'],
  ['GET', '/api/v1/admin/privacy'],
  ['PUT', '/api/v1/admin/privacy'],
  ['GET', '/api/v1/admin/impressum'],
  ['PUT', '/api/v1/admin/impressum'],
  ['GET', '/api/v1/users/me/export'],
  ['DELETE', '/api/v1/users/me/delete'],
  ['GET', '/api/v1/modules'],
  ['GET', '/api/v1/modules/{name}'],
  ['PATCH', '/api/v1/admin/modules/{name}'],
  ['GET', '/api/v1/admin/modules/{name}/config'],
  ['PATCH', '/api/v1/admin/modules/{name}/config'],
];

let failed = false;

for (const [method, path] of required) {
  const entry = paths[path];
  if (!entry || !entry[method.toLowerCase()]) {
    console.error(`ERROR: OpenAPI snapshot missing ${method} ${path}`);
    failed = true;
  }
}

if (failed) {
  process.exit(1);
}

console.log('ParkHub Rust legal/module OpenAPI contract OK.');
NODE
