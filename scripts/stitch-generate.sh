#!/bin/bash
# Automated Stitch Design Generation + GitHub Discussion Posting
# Usage: ./scripts/stitch-generate.sh "prompt text" [DESKTOP|MOBILE|TABLET]
#
# Prerequisites:
# - stitch-mcp installed globally: npm i -g @_davideast/stitch-mcp
# - gcloud ADC configured: gcloud auth application-default login
# - gh CLI authenticated

set -euo pipefail

PROJECT_ID="17575216190001233216"
REPO="nash87/parkhub-rust"
DISCUSSION_NUM=174
DESIGNS_DIR="$(dirname "$0")/../.stitch/designs"
PROMPT="${1:?Usage: $0 'prompt' [DESKTOP|MOBILE|TABLET]}"
DEVICE="${2:-DESKTOP}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
SLUG=$(echo "$PROMPT" | tr '[:upper:]' '[:lower:]' | tr -cs '[:alnum:]' '-' | head -c 40)

echo "=== Stitch Design Generator ==="
echo "Prompt: $PROMPT"
echo "Device: $DEVICE"
echo ""

# Step 1: Generate screen
echo "[1/5] Generating design via Stitch MCP..."
RESULT=$(stitch-mcp tool generate_screen_from_text -d "{\"projectId\":\"$PROJECT_ID\",\"prompt\":\"$PROMPT\",\"deviceType\":\"$DEVICE\"}" 2>/dev/null)

if echo "$RESULT" | grep -q "Error"; then
    echo "ERROR: Stitch generation failed"
    echo "$RESULT"
    exit 1
fi

echo "[2/5] Extracting design system..."
echo "$RESULT" | python3 -c "
import json,sys
d=json.load(sys.stdin)
ds=d.get('outputComponents',[{}])[0].get('designSystem',{}).get('designSystem',{})
if ds:
    print(f\"Theme: {ds.get('theme',{}).get('colorMode','')} {ds.get('displayName','')}\")
    print(f\"Font: {ds.get('theme',{}).get('font','')}\")
" 2>/dev/null || true

# Step 3: Download thumbnail
echo "[3/5] Downloading screenshot..."
mkdir -p "$DESIGNS_DIR"
FILENAME="${SLUG}-${TIMESTAMP}.png"

# Get latest thumbnail from project
THUMB_URL=$(stitch-mcp tool get_project -d "{\"name\":\"projects/$PROJECT_ID\"}" 2>/dev/null | python3 -c "
import json,sys
d=json.load(sys.stdin)
print(d.get('thumbnailScreenshot',{}).get('downloadUrl',''))
" 2>/dev/null)

if [ -n "$THUMB_URL" ]; then
    curl -sL "${THUMB_URL}=w1440" -o "$DESIGNS_DIR/$FILENAME"
    echo "  Saved: $DESIGNS_DIR/$FILENAME"
    file "$DESIGNS_DIR/$FILENAME"
else
    echo "  WARNING: No thumbnail available yet"
fi

# Step 4: Commit to repo
echo "[4/5] Committing to repo..."
cd "$(dirname "$0")/.."
git add ".stitch/designs/$FILENAME" 2>/dev/null || true
git commit -m "stitch: $SLUG" 2>/dev/null || true
flatpak-spawn --host git -C "$(pwd)" push github main 2>/dev/null || true

# Step 5: Post to GitHub Discussion
echo "[5/5] Posting to Design Lab discussion..."
DISC_ID=$(flatpak-spawn --host gh api graphql -f query="query { repository(owner: \"nash87\", name: \"parkhub-rust\") { discussion(number: $DISCUSSION_NUM) { id } } }" 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin)['data']['repository']['discussion']['id'])")

BODY="## Auto-Generated Design: $SLUG\n\n**Prompt**: $PROMPT\n**Device**: $DEVICE\n**Generated**: $(date -Iseconds)\n\n![Design](https://raw.githubusercontent.com/$REPO/main/.stitch/designs/$FILENAME)\n\nReact with 👍 to adopt, 🎨 for variations!"

flatpak-spawn --host gh api graphql -f query="mutation { addDiscussionComment(input: { discussionId: \"$DISC_ID\", body: \"$BODY\" }) { comment { url } } }" 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin)['data']['addDiscussionComment']['comment']['url'])" 2>/dev/null

echo ""
echo "=== Done! Design posted to Discussion #$DISCUSSION_NUM ==="
