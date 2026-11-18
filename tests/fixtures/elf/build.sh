#!/usr/bin/env bash
set -euo pipefail
fixture_repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
exec python3 "$fixture_repo_root/scripts/build-test-fixtures.py"
