#!/bin/bash
# Automated Stitch Design Generation + GitHub Discussion Posting
# Usage: ./scripts/stitch-generate.sh "prompt text" [DESKTOP|MOBILE|TABLET] [project-name]
#
# Creates a SEPARATE Stitch project per design for unique thumbnails.
# Uses DESIGN.md context for consistent design system.
#
# Prerequisites:
# - stitch-mcp installed globally: npm i -g @_davideast/stitch-mcp
# - Timeout patched to 300s (see feedback_stitch_workflow.md)
# - gcloud ADC configured: gcloud auth application-default login
# - gh CLI authenticated

set -euo pipefail

REPO="nash87/parkhub-rust"
DISCUSSION_NUM=174
DESIGNS_DIR="$(dirname "$0")/../.stitch/designs"
PROMPT="${1:?Usage: $0 'prompt' [DESKTOP|MOBILE|TABLET] [project-name]}"
DEVICE="${2:-DESKTOP}"
PROJECT_NAME="${3:-ParkHub $(echo "$PROMPT" | head -c 30)}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
SLUG=$(echo "$PROMPT" | tr '[:upper:]' '[:lower:]' | tr -cs '[:alnum:]' '-' | head -c 40)

echo "=== Stitch Design Generator v2 ==="
echo "Prompt: $PROMPT"
echo "Device: $DEVICE"
echo "Project: $PROJECT_NAME"
echo ""

# Step 1: Create a SEPARATE project for unique thumbnail
echo "[1/6] Creating Stitch project '$PROJECT_NAME'..."
PROJECT_ID=$(stitch-mcp tool create_project -d "{\"title\":\"$PROJECT_NAME\"}" 2>/dev/null | \
  python3 -c "import json,sys; print(json.load(sys.stdin).get('name','').split('/')[-1])" 2>/dev/null)

if [ -z "$PROJECT_ID" ]; then
    echo "ERROR: Failed to create Stitch project"
    exit 1
fi
echo "  Project ID: $PROJECT_ID (PRIVATE)"

# Step 2: Enhance prompt with DESIGN.md context
echo "[2/6] Enhancing prompt with design system..."
DESIGN_MD=""
if [ -f "$(dirname "$0")/../.stitch/DESIGN.md" ]; then
    # Extract key tokens from DESIGN.md
    DESIGN_MD="\n\nDESIGN SYSTEM (from DESIGN.md):\n- Primary: Teal oklch(0.66 0.13 168) / #0d9488\n- Secondary: Amber oklch(0.74 0.16 75) / #f59e0b\n- Surface: Deep Slate #0f172a\n- Glass: backdrop-blur 24px, 60% opacity, ghost borders 15%\n- Typography: Manrope headlines, Inter body, tabular-nums for data\n- Corners: rounded-2xl cards, rounded-lg buttons\n- Motion: spring stiffness 300, stagger 50ms\n- Rule: No 1px borders, use tonal shifts only"
fi

ENHANCED_PROMPT="${PROMPT}${DESIGN_MD}"

# Step 3: Generate screen
echo "[3/6] Generating design via Stitch MCP..."
RESULT=$(stitch-mcp tool generate_screen_from_text -d "{\"projectId\":\"$PROJECT_ID\",\"prompt\":\"$ENHANCED_PROMPT\",\"deviceType\":\"$DEVICE\"}" 2>/dev/null)

if echo "$RESULT" | grep -q "Error\|error"; then
    echo "ERROR: Stitch generation failed"
    echo "$RESULT" | head -5
    exit 1
fi

COMP_COUNT=$(echo "$RESULT" | python3 -c "import json,sys; d=json.load(sys.stdin); print(len(d.get('outputComponents',[])))" 2>/dev/null || echo "0")
echo "  Generated $COMP_COUNT components"

# Step 4: Download thumbnail (unique per project)
echo "[4/6] Downloading screenshot..."
mkdir -p "$DESIGNS_DIR"
FILENAME="${SLUG}-${TIMESTAMP}.png"

THUMB_URL=$(stitch-mcp tool get_project -d "{\"name\":\"projects/$PROJECT_ID\"}" 2>/dev/null | \
  python3 -c "import json,sys; print(json.load(sys.stdin).get('thumbnailScreenshot',{}).get('downloadUrl',''))" 2>/dev/null)

if [ -n "$THUMB_URL" ]; then
    curl -sL "${THUMB_URL}=w1440" -o "$DESIGNS_DIR/$FILENAME"
    SIZE=$(file "$DESIGNS_DIR/$FILENAME" | grep -oP '\d+ x \d+' || echo "unknown")
    echo "  Saved: $FILENAME ($SIZE)"
else
    echo "  WARNING: No thumbnail available yet"
    exit 1
fi

# Step 5: Commit to repo
echo "[5/6] Committing to repo..."
cd "$(dirname "$0")/.."
git add ".stitch/designs/$FILENAME" 2>/dev/null || true
git commit -m "stitch: $SLUG" 2>/dev/null || true
flatpak-spawn --host git -C "$(pwd)" push github main 2>/dev/null || true

# Step 6: Post to GitHub Discussion (verify ID first!)
echo "[6/6] Posting to Design Lab discussion..."
DISC_ID=$(flatpak-spawn --host gh api graphql -f query="query { repository(owner: \"nash87\", name: \"parkhub-rust\") { discussion(number: $DISCUSSION_NUM) { id } } }" 2>/dev/null | \
  python3 -c "import json,sys; print(json.load(sys.stdin)['data']['repository']['discussion']['id'])" 2>/dev/null)

# Verify it's our repo
if [ -z "$DISC_ID" ]; then
    echo "  WARNING: Could not find discussion #$DISCUSSION_NUM"
else
    BODY="## Auto-Generated: $PROJECT_NAME\n\n**Prompt**: $PROMPT\n**Device**: $DEVICE\n**Generated**: $(date -Iseconds)\n**Stitch Project**: $PROJECT_ID (PRIVATE)\n\n![Design](https://raw.githubusercontent.com/$REPO/main/.stitch/designs/$FILENAME)\n\nReact with 👍 to adopt, 🎨 for variations!"

    URL=$(flatpak-spawn --host gh api graphql -f query="mutation { addDiscussionComment(input: { discussionId: \"$DISC_ID\", body: \"$BODY\" }) { comment { url } } }" 2>/dev/null | \
      python3 -c "import json,sys; print(json.load(sys.stdin)['data']['addDiscussionComment']['comment']['url'])" 2>/dev/null)
    echo "  Posted: $URL"
fi

echo ""
echo "=== Done! ==="
echo "Screenshot: .stitch/designs/$FILENAME"
echo "Stitch Project: $PROJECT_ID (PRIVATE)"
