#!/bin/bash
set -e

if [ -f .env ]; then
  echo "Loading environment variables from .env file..."

  set -a
  source .env
  set +a
fi

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo -e "${GREEN}Fleur Build and Sign Script${NC}"
echo "This script will build, sign, and notarize your Fleur application"
echo "------------------------------------------------------------"

if [ -z "$APPLE_SIGNING_IDENTITY" ]; then
  echo -e "${RED}Error: APPLE_SIGNING_IDENTITY environment variable not set${NC}"
  echo "Example: export APPLE_SIGNING_IDENTITY=\"Developer ID Application: Acme, Inc (APPLE_TEAM_ID)\""
  exit 1
fi

if [ -z "$APPLE_TEAM_ID" ]; then
  echo -e "${YELLOW}Warning: APPLE_TEAM_ID not set, will try to extract from signing identity${NC}"

  APPLE_TEAM_ID=$(echo "$APPLE_SIGNING_IDENTITY" | grep -o '([A-Z0-9]*)' | tr -d '()')
  
  if [ -z "$APPLE_TEAM_ID" ]; then
    echo -e "${RED}Error: Could not extract team ID from signing identity${NC}"
    echo "Please set APPLE_TEAM_ID manually"
    exit 1
  else
    echo -e "${GREEN}Extracted Team ID: $APPLE_TEAM_ID${NC}"
  fi
fi

read -p "Do you want to notarize the application? (y/n): " SHOULD_NOTARIZE

if [[ "$SHOULD_NOTARIZE" =~ ^[Yy]$ ]]; then
  if [ -z "$APPLE_ID" ] || [ -z "$APPLE_PASSWORD" ]; then
    echo -e "${RED}Error: Notarization requires APPLE_ID and APPLE_PASSWORD to be set${NC}"
    echo "Example: export APPLE_ID=\"your.email@example.com\""
    echo "Example: export APPLE_PASSWORD=\"app-specific-password\""
    exit 1
  fi
fi

echo -e "${GREEN}Building Fleur with Tauri...${NC}"
cd "$(dirname "$0")"
pushd src-tauri > /dev/null

echo -e "${YELLOW}Creating build with: bun tauri build${NC}"
bun tauri build

popd > /dev/null

APP_BUNDLE_PATH="./src-tauri/target/release/bundle/macos/Fleur.app"
DMG_PATH="./src-tauri/target/release/bundle/dmg/Fleur_0.1.2_aarch64.dmg"

echo -e "${GREEN}Signing Fleur.app main executable with hardened runtime...${NC}"
codesign --force --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" \
  --entitlements ./src-tauri/macos/entitlements.plist "$APP_BUNDLE_PATH/Contents/MacOS/fleur"

echo -e "${GREEN}Signing complete Fleur.app with hardened runtime...${NC}"
codesign --force --deep --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" \
  --entitlements ./src-tauri/macos/entitlements.plist "$APP_BUNDLE_PATH"

echo -e "${GREEN}Creating DMG with signed app...${NC}"
hdiutil create -volname "Fleur" -srcfolder "$APP_BUNDLE_PATH" -ov -format UDZO "./src-tauri/target/release/bundle/dmg/Fleur_signed.dmg"

echo -e "${GREEN}Signing DMG...${NC}"
codesign --force --timestamp --sign "$APPLE_SIGNING_IDENTITY" "./src-tauri/target/release/bundle/dmg/Fleur_signed.dmg"

if [[ "$SHOULD_NOTARIZE" =~ ^[Yy]$ ]]; then
  echo -e "${GREEN}Submitting for notarization...${NC}"
  
  xcrun notarytool submit "./src-tauri/target/release/bundle/dmg/Fleur_signed.dmg" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait

  # Staple the notarization ticket to the dmg
  echo -e "${GREEN}Stapling notarization ticket...${NC}"
  xcrun stapler staple "./src-tauri/target/release/bundle/dmg/Fleur_signed.dmg"
fi

echo -e "${GREEN}Process complete!${NC}"
echo -e "Signed DMG is available at: ${YELLOW}./src-tauri/target/release/bundle/dmg/Fleur_signed.dmg${NC}"
echo ""
echo -e "${GREEN}Distribution instructions:${NC}"
echo "1. Upload the signed (and notarized) DMG to your website or GitHub releases"
echo "2. Users can download and mount the DMG, then drag the app to their Applications folder"
echo ""
echo -e "${YELLOW}For curl-based installation:${NC}"
echo "Create a simple install script that users can run with:"
echo "curl -sSL https://your-domain.com/install-fleur.sh | bash"