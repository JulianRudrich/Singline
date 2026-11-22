#!/usr/bin/env bash
# Sets the GitHub settings of the Singline repository: description and topics,
# security features, Actions permissions, the website on GitHub Pages and the
# rules for main. Run it once as the repository's owner, with the GitHub CLI
# logged in:
#   brew install gh && gh auth login
#   bash scripts/github-settings.sh
# Every step can run again; a step that fails is reported and the rest go on.
# The functions below run through step, which shellcheck cannot follow.
# shellcheck disable=SC2317
set -uo pipefail

REPO=JulianRudrich/Singline
API=repos/$REPO
failed=0

# step NAME COMMAND...: runs one change and says how it went.
step() {
  local name=$1
  shift
  if "$@" > /dev/null; then
    echo "✓ $name"
  else
    echo "✗ $name: set it by hand in the repository settings"
    failed=1
  fi
}

step "Description, Discussions and branch cleanup" \
  gh api -X PATCH "$API" \
  -f description="Synced lyrics in a small window above everything else, for Spotify, Apple Music, Amazon Music and YouTube Music on Windows, macOS and Linux." \
  -F has_discussions=true -F delete_branch_on_merge=true

step "Topics" gh api -X PUT "$API/topics" --input - <<'JSON'
{"names": ["lyrics", "synced-lyrics", "karaoke", "lrc", "lrclib", "overlay", "spotify",
  "apple-music", "youtube-music", "tauri", "rust", "svelte", "desktop-app", "mpris",
  "windows", "macos", "linux"]}
JSON

step "Private vulnerability reporting" gh api -X PUT "$API/private-vulnerability-reporting"
step "Dependabot alerts" gh api -X PUT "$API/vulnerability-alerts"
step "Dependabot security updates" gh api -X PUT "$API/automated-security-fixes"
step "Secret scanning with push protection" gh api -X PATCH "$API" --input - <<'JSON'
{"security_and_analysis": {"secret_scanning": {"status": "enabled"},
  "secret_scanning_push_protection": {"status": "enabled"}}}
JSON
step "CodeQL default setup" gh api -X PATCH "$API/code-scanning/default-setup" -f state=configured

# Every workflow asks for the permissions it needs, so the default can be read-only.
step "Read-only workflow token" gh api -X PUT "$API/actions/permissions/workflow" \
  -f default_workflow_permissions=read -F can_approve_pull_request_reviews=false
step "Actions pinned to full commit hashes" gh api -X PUT "$API/actions/permissions" \
  -F enabled=true -f allowed_actions=all -F sha_pinning_required=true

enable_pages() {
  gh api -X POST "$API/pages" -f build_type=workflow 2> /dev/null ||
    gh api -X PUT "$API/pages" -f build_type=workflow
}
step "GitHub Pages from the Website workflow" enable_pages
step "Website as the repository's homepage" \
  gh api -X PATCH "$API" -f homepage="https://julianrudrich.github.io/Singline/"
step "First publication of the website" gh workflow run pages.yml --repo "$REPO"

# Force pushes and deleting main are refused, and changes from others need the
# checks to pass. Repository admins may push directly, as the maintainer does.
add_ruleset() {
  if gh api "$API/rulesets" --jq '.[].name' | grep -qx main; then
    echo "  (a ruleset named main exists already; left as it is)" >&2
    return 0
  fi
  gh api -X POST "$API/rulesets" --input - <<'JSON'
{
  "name": "main",
  "target": "branch",
  "enforcement": "active",
  "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
  "bypass_actors": [{"actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always"}],
  "rules": [
    {"type": "deletion"},
    {"type": "non_fast_forward"},
    {"type": "required_status_checks", "parameters": {
      "strict_required_status_checks_policy": false,
      "required_status_checks": [
        {"context": "Linux checks and end-to-end test"},
        {"context": "Workflows"},
        {"context": "Spelling"},
        {"context": "Links"},
        {"context": "Rust dependencies (bans licenses sources)"}
      ]}}
  ]
}
JSON
}
step "Rules for main" add_ruleset

echo
echo "Left to do by hand: upload docs/images/social-preview.png under"
echo "Settings → General → Social preview (GitHub has no API for it)."
exit "$failed"
