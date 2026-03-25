#!/usr/bin/env bash
set -euo pipefail

MAX_BYTES=102400
CONTRACTS=(arena factory payout staking)
TARGET_DIR="target/wasm32-unknown-unknown/release"
METRICS_DIR="target/wasm-metrics"
METRICS_FILE="${METRICS_DIR}/sizes.md"

mkdir -p "${METRICS_DIR}"

echo "Building release WASM artifacts..."
cargo build --release --target wasm32-unknown-unknown --workspace

echo "| Contract | Raw Bytes | Optimized Bytes | Under 100KB |" > "${METRICS_FILE}"
echo "|---|---:|---:|:---:|" >> "${METRICS_FILE}"

failed=0

find_raw_wasm() {
  local contract="$1"
  local direct_path="${TARGET_DIR}/${contract}.wasm"
  if [[ -f "${direct_path}" ]]; then
    echo "${direct_path}"
    return 0
  fi

  # Fallback for toolchains that emit into deps/ with hashed filenames.
  if [[ -d "${TARGET_DIR}/deps" ]]; then
    find "${TARGET_DIR}/deps" -maxdepth 1 -type f -name "${contract}-*.wasm" | head -n 1
  fi
}

for contract in "${CONTRACTS[@]}"; do
  raw_wasm="$(find_raw_wasm "${contract}")"
  optimized_wasm="${TARGET_DIR}/${contract}.optimized.wasm"

  if [[ -z "${raw_wasm}" || ! -f "${raw_wasm}" ]]; then
    echo "Missing wasm artifact for contract: ${contract}" >&2
    failed=1
    continue
  fi

  if command -v soroban >/dev/null 2>&1; then
    soroban contract optimize --wasm "${raw_wasm}" --wasm-out "${optimized_wasm}" >/dev/null
  else
    # Local fallback to keep size-report generation possible when CLI is absent.
    cp "${raw_wasm}" "${optimized_wasm}"
  fi

  raw_size=$(wc -c < "${raw_wasm}" | tr -d ' ')
  optimized_size=$(wc -c < "${optimized_wasm}" | tr -d ' ')

  under_limit="yes"
  if (( optimized_size > MAX_BYTES )); then
    under_limit="no"
    failed=1
  fi

  echo "| ${contract} | ${raw_size} | ${optimized_size} | ${under_limit} |" >> "${METRICS_FILE}"
done

echo "WASM size report"
cat "${METRICS_FILE}"

if (( failed == 1 )); then
  echo "One or more optimized WASM binaries exceed 100KB or failed to build/optimize." >&2
  exit 1
fi
