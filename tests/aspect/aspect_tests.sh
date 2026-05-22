#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
set -euo pipefail

echo "--- Aspect: License headers ---"
grep -r "SPDX-License-Identifier" . --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=_build | head -n 1 | grep -q "."

echo "--- Aspect: No secrets in source ---"
! grep -rE "AI_KEY|API_KEY|SECRET_KEY" . --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=_build | grep -v "PLACEHOLDER" | grep -v "EXAMPLE" | grep -q "."

echo "--- Aspect: No Node.js artifacts ---"
[ ! -d "node_modules" ]
[ ! -f "package-lock.json" ]

echo "All aspect checks passed!"

