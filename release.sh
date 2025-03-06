#!/bin/bash
set -e

# Check if version argument is provided
if [ -z "$1" ]; then
    echo "Please provide a version number (e.g. ./release.sh 1.0.0)"
    exit 1
fi

VERSION=$1

# Ensure version starts with 'v'
if [[ $VERSION != v* ]]; then
    VERSION="v$VERSION"
fi

echo "Creating release for version $VERSION..."

# Update version in package.json
echo "Updating package.json..."
jq ".version = \"${VERSION#v}\"" package.json > tmp.json && mv tmp.json package.json

# Update version in Cargo.toml
echo "Updating Cargo.toml..."
sed -i '' "s/^version = \".*\"/version = \"${VERSION#v}\"/" src-tauri/Cargo.toml

# Update version in tauri.conf.json
echo "Updating tauri.conf.json..."
jq ".version = \"${VERSION#v}\"" src-tauri/tauri.conf.json > tmp.json && mv tmp.json src-tauri/tauri.conf.json

# Install dependencies and build frontend
echo "Installing dependencies..."
bun install

echo "Building frontend..."
bun run build

# Build Tauri app
echo "Building Tauri app..."
cd src-tauri
TAURI_SIGNING_PRIVATE_KEY=~/.tauri/fleur.key cargo tauri build --release --target universal-apple-darwin

# Generate latest.json
echo "Generating latest.json..."
SIGNATURE=$(cat target/universal-apple-darwin/release/bundle/macos/Fleur.app.tar.gz.sig)
cat > latest.json << EOL
{
    "version": "${VERSION#v}",
    "notes": "See the assets to download this version and install.",
    "pub_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "platforms": {
        "darwin-aarch64": {
            "signature": "$SIGNATURE",
            "url": "https://github.com/fleuristes/fleur/releases/download/$VERSION/Fleur.app.tar.gz"
        }
    }
}
EOL

# List the generated artifacts
echo "Generated artifacts:"
echo "-------------------"
echo "Latest update info: $(ls latest.json)"
echo "DMG installer: $(ls target/universal-apple-darwin/release/bundle/dmg/*.dmg)"
echo "App archive: $(ls target/universal-apple-darwin/release/bundle/macos/*.app.tar.gz)"
