## Running tests

To run the tests, run:

```bash
bun test
```

## Updating the App Icon

To update the application icon, you'll need:
1. A source image (at least 1024x1024px, PNG format recommended)
2. ImageMagick installed (`brew install imagemagick`)

Then run the icon generation script:

```bash
# Generate all required icons
scripts/generate_icons.sh public/logo.png

# You can pass a different path to the script if you want to use a different image
```

The script will automatically:
- Generate all required icon sizes for macOS, Windows, and Linux
- Create the .icns file for macOS
- Create the .ico file for Windows
- Place all icons in the correct location (src-tauri/icons)

For best results, use a square PNG image with dimensions of at least 1024x1024 pixels.
