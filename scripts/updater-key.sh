#!/usr/bin/env bash
# Creates the key that signs Singline's updates, once, on the maintainer's own
# computer. The private key stays in ~/.tauri and goes to the repository's
# secrets, where the CI signs every build with it; the public key is printed
# for the app, which checks each update against it. Needs Node.js and the
# GitHub CLI, logged in:
#   curl -fsSL https://raw.githubusercontent.com/JulianRudrich/Singline/main/scripts/updater-key.sh | bash
set -euo pipefail

REPO=JulianRudrich/Singline
KEY="$HOME/.tauri/singline.key"

for tool in npx gh; do
  if ! command -v "$tool" > /dev/null; then
    echo "✗ $tool is missing: brew install $([ "$tool" = npx ] && echo node || echo gh)" >&2
    exit 1
  fi
done

if [ -e "$KEY" ]; then
  echo "✓ Using the key that exists at $KEY"
else
  mkdir -p "$HOME/.tauri"
  npx --yes @tauri-apps/cli@2.12.5 signer generate --ci --write-keys "$KEY" > /dev/null
  chmod 600 "$KEY"
  echo "✓ Created the key at $KEY"
fi

gh secret set TAURI_SIGNING_PRIVATE_KEY --repo "$REPO" < "$KEY"
echo "✓ Stored the private key as the TAURI_SIGNING_PRIVATE_KEY secret of $REPO"
echo
echo "Keep $KEY safe and never share it: whoever has it can sign updates."
echo "The public key below is no secret. The app needs it to check updates:"
echo
cat "$KEY.pub"
echo
