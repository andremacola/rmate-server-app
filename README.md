# RMate Server

A lightweight, native macOS tray application that manages an `rmate` server, enabling remote file editing in popular local code editors.

## Features

- **Lightweight & Native:** Built purely in Rust for a minimal memory footprint and native macOS performance.
- **System Tray Control:** Easily start, stop, and configure the server directly from your macOS menu bar.
- **Multi-Editor Support:** Seamlessly switch the `rmate` server connection between Zed, VS Code, and Sublime Text.
- **Persistent State:** The application remembers your last selected editor, so you don't have to reconfigure it on every launch.
- **Automatic Server Management:** Automatically handles the `rmate` server process in the background.

## Building from Source

### Prerequisites

- [Rust Programming Language](https://www.rust-lang.org/tools/install)

### Build Commands

1.  **Clone the repository:**

    ```sh
    git clone https://github.com/andremacola/rmate-server-app.git
    cd rmate-server-app
    ```

2.  **Build the release executable:**
    This command compiles the application. The final binary will be located in `target/release/rmate-server`.

    ```sh
    cargo build --release
    ```

3.  **Package the `.app` bundle:**
    To create a standard macOS `.app` bundle that you can move to your `/Applications` folder, you can save the following script as `build.sh` and run it from the project root (`sh build.sh`). This script will compile the code and package everything into `build/release/RMate Server.app`.

    ```sh
    #!/bin/sh
    set -e

    APP_NAME="RMate Server.app"
    BUILD_DIR="build/release"
    EXECUTABLE_NAME="rmate-server"
    APP_PATH="$BUILD_DIR/$APP_NAME"
    CONTENTS_PATH="$APP_PATH/Contents"
    MACOS_PATH="$CONTENTS_PATH/MacOS"
    RESOURCES_PATH="$CONTENTS_PATH/Resources"

    # Read version from Cargo.toml
    VERSION=$(grep "^version" Cargo.toml | head -n 1 | cut -d '"' -f 2)

    echo "Building version $VERSION..."

    # Clean up old version
    rm -rf "$APP_PATH"

    # Create directory structure
    mkdir -p "$MACOS_PATH"
    mkdir -p "$RESOURCES_PATH"

    # Create Info.plist
    cat > "$CONTENTS_PATH/Info.plist" <<EOF
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
        <key>CFBundleDevelopmentRegion</key>
        <string>en</string>
        <key>CFBundleDisplayName</key>
        <string>RMate Server</string>
        <key>CFBundleExecutable</key>
        <string>$EXECUTABLE_NAME</string>
        <key>CFBundleIconFile</key>
        <string>icon.icns</string>
        <key>CFBundleIdentifier</key>
        <string>com.andremacola.rmate-server</string>
        <key>CFBundleInfoDictionaryVersion</key>
        <string>6.0</string>
        <key>CFBundleName</key>
        <string>RMate Server</string>
        <key>CFBundlePackageType</key>
        <string>APPL</string>
        <key>CFBundleShortVersionString</key>
        <string>$VERSION</string>
        <key>CFBundleVersion</key>
        <string>$VERSION</string>
        <key>LSMinimumSystemVersion</key>
        <string>10.13</string>
        <key>LSUIElement</key>
        <true/>
        <key>NSPrincipalClass</key>
        <string>NSApplication</string>
        <key>NSHighResolutionCapable</key>
        <true/>
    </dict>
    </plist>
    EOF

    # Compile the Rust app first
    echo "Compiling Rust executable..."
    cargo build --release

    # Move executable and copy resources
    echo "Packaging .app bundle..."
    mv "target/release/$EXECUTABLE_NAME" "$MACOS_PATH/"
    cp -r "bin" "$RESOURCES_PATH/"
    cp -r "icons" "$RESOURCES_PATH/"
    cp "icons/icon.icns" "$RESOURCES_PATH/"

    # Set permissions
    chmod +x "$MACOS_PATH/$EXECUTABLE_NAME"

    echo "Successfully created and packaged $APP_PATH"
    ```
