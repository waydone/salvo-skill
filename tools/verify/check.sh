#!/usr/bin/env bash
# Re-verify salvo-skill against the Salvo version pinned in ./Cargo.toml.
#   1. regenerate src/blocks/ from every ```rust block of skills/salvo-skill
#   2. cargo check (normal + cfg(test)) — must report 0 errors, 0 warnings (no deprecated API)
#   3. cargo test runtime — TestClient checks for behavior claims
# usage: tools/verify/check.sh            (from anywhere)
set -uo pipefail
here="$(cd -- "$(dirname -- "$0")" && pwd)"
skill="$here/../../skills/salvo-skill"
manifest="$here/Cargo.toml"

python3 -I "$here/check_edition.py" "$skill" "$manifest" || exit 1
python3 -I "$here/gen_harness.py" "$skill" "$here/src" "$here/harness.json" || exit 1

log="$here/target/check.log"
mkdir -p "$here/target"
cargo check --manifest-path "$manifest" --all-targets --message-format=short 2> "$log" > /dev/null
status=$?
errors=$(grep -c -E '^src/.*: error' "$log")
warnings=$(grep -c -E '^src/.*: warning' "$log")
echo "cargo check --all-targets: exit=$status errors=$errors warnings=$warnings"
if [ "$errors" -gt 0 ] || [ "$warnings" -gt 0 ]; then
  echo "--- per block (see the // file:line header of src/blocks/bNNN.rs):"
  grep -E '^src/.*: (error|warning)' "$log" | sed -E 's|^src/blocks/(b[0-9]+)\.rs.*|\1|; s|^(src/[a-z_]+\.rs).*|\1|' | sort | uniq -c | sort -rn
  grep -E '^src/.*: (error|warning)' "$log" | head -40
  exit 1
fi

cargo test --manifest-path "$manifest" runtime 2>&1 | grep -E '^test |test result' || exit 1
