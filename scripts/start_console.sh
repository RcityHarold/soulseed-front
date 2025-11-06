#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export SOULSEED_API_BASE_URL="${SOULSEED_API_BASE_URL:-http://localhost:8700/api/v1}"
export SOULSEED_DEFAULT_TENANT="${SOULSEED_DEFAULT_TENANT:-1}"
export SOULSEED_DEFAULT_SESSION="${SOULSEED_DEFAULT_SESSION:-123}"
export SOULSEED_AUTH_TOKEN="${SOULSEED_AUTH_TOKEN:-token}"

exec dx serve --platform web --package soulseed-console --port 5173
