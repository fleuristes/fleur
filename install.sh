#!/bin/bash
set -e

# ANSI color codes and styling
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

# Function to print messages with different styling
print_message() {
    echo -e "${BLUE}${BOLD}🌸 ${1}${RESET}"
}

print_success() {
    echo -e "${GREEN}${BOLD}✅ ${1}${RESET}"
}

print_error() {
    echo -e "${RED}${BOLD}❌ ${1}${RESET}"
}

print_warning() {
    echo -e "${YELLOW}${BOLD}⚠️  ${1}${RESET}"
}

print_step() {
    echo -e "\n${CYAN}${BOLD}🔹 ${1}${RESET}"
}

# Function to check for sudo access and get it if needed
ensure_sudo() {
    if [[ $EUID -ne 0 ]]; then
        sudo -v || {
            print_error "Admin privileges are required for installation."
            exit 1
        }
        # Keep sudo active
        while true; do sudo -n true; sleep 60; kill -0 "$$" || exit; done 2>/dev/null &
    fi
}

cleanup() {
    if [[ -d "$BUILD_DIR" ]]; then
        rm -rf "$BUILD_DIR"
    fi
}

# Function to show progress bar for curl
download_with_progress() {
    curl -L "$1" -o "$2" --progress-bar
}

# Display bold block-style banner
display_banner() {
    clear
    echo
    echo -e "${MAGENTA}${BOLD}"
    echo "███████╗██╗     ███████╗██╗   ██╗██████╗ "
    echo "██╔════╝██║     ██╔════╝██║   ██║██╔══██╗"
    echo "█████╗  ██║     █████╗  ██║   ██║██████╔╝"
    echo "██╔══╝  ██║     ██╔══╝  ██║   ██║██╔══██╗"
    echo "██║     ███████╗███████╗╚██████╔╝██║  ██║"
    echo "╚═╝     ╚══════╝╚══════╝ ╚═════╝ ╚═╝  ╚═╝"
    echo -e "${RESET}"
    # Short animation
    sleep 0.7
}

# Function to check and handle existing installation
check_existing_installation() {
    if [[ -d "/Applications/Fleur.app" ]]; then
        print_warning "Fleur is already installed on this system."
        read -p "$(echo -e $YELLOW"Do you want to remove the existing installation before continuing? (y/n): "$RESET)" choice
        case "$choice" in
            y|Y)
                print_step "Removing existing installation"
                ensure_sudo
                sudo rm -rf "/Applications/Fleur.app"
                print_success "Previous installation removed successfully!"
                ;;
            n|N)
                print_warning "Installation aborted. Existing installation was not modified."
                exit 0
                ;;
            *)
                print_error "Invalid choice. Installation aborted."
                exit 1
                ;;
        esac
    fi
}

# Function to verify downloaded file
verify_download() {
    if [[ ! -f "$1" ]]; then
        print_error "Download failed: $1 not found."
        exit 1
    fi
    # Check file size to ensure it's not empty
    file_size=$(stat -f%z "$1")
    if [[ $file_size -lt 1000 ]]; then
        print_error "Download appears to be incomplete or corrupted (size: $file_size bytes)."
        exit 1
    fi
}

# Function for spinner animation during operations
spinner() {
    local pid=$1
    local delay=0.1
    local spinstr='|/-\'
    while [ "$(ps a | awk '{print $1}' | grep -w $pid)" ]; do
        local temp=${spinstr#?}
        printf " [%c]  " "$spinstr"
        local spinstr=$temp${spinstr%"$temp"}
        sleep $delay
        printf "\b\b\b\b\b\b"
    done
    printf "    \b\b\b\b"
}

# Function to get the latest release version from GitHub
get_latest_version() {
    local latest_version=$(curl -s https://api.github.com/repos/fleuristes/fleur/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [[ -z "$latest_version" ]]; then
        print_error "Failed to fetch the latest version from GitHub."
        exit 1
    fi
    echo "$latest_version"
}

# Main installation process
main() {
    # Get the latest version
    VERSION=$(get_latest_version)

    # Display banner
    display_banner
    print_message "Welcome to the Fleur installer $VERSION!"
    # Trap for cleanup
    trap cleanup EXIT INT TERM
    # System compatibility check
    if [[ "$(uname)" != "Darwin" ]]; then
        print_error "Fleur is currently only compatible with macOS."
        print_error "This installation script does not support Linux or Windows yet."
        exit 1
    fi
    # Check for existing installation
    check_existing_installation
    # Create a directory for downloads
    BUILD_DIR="$HOME/.fleur-build-$(date +%s)"
    mkdir -p "$BUILD_DIR"
    print_message "Using build directory: ${YELLOW}$BUILD_DIR${RESET}"
    # Download pre-built application
    APP_URL="https://github.com/fleuristes/fleur/releases/download/$VERSION/Fleur.app.tar.gz"
    print_message "Downloading Fleur from: ${YELLOW}$APP_URL${RESET}"
    echo -e "${YELLOW}${BOLD}Downloading...${RESET}"
    download_with_progress "$APP_URL" "$BUILD_DIR/Fleur.app.tar.gz"
    verify_download "$BUILD_DIR/Fleur.app.tar.gz"
    # Extract the application
    print_message "Extracting files..."
    tar -xzf "$BUILD_DIR/Fleur.app.tar.gz" -C "$BUILD_DIR" &
    extraction_pid=$!
    spinner $extraction_pid
    wait $extraction_pid
    # Verify extraction
    if [[ ! -d "$BUILD_DIR/Fleur.app" ]]; then
        print_error "Extraction failed. Fleur.app not found in the build directory."
        exit 1
    fi
    # Remove quarantine attribute
    xattr -rd com.apple.quarantine "$BUILD_DIR/Fleur.app" 2>/dev/null || true
    # Copy to Applications
    print_message "Installing Fleur..."
    cp -R "$BUILD_DIR/Fleur.app" /Applications/
    # Set permissions
    chmod -R 755 "/Applications/Fleur.app"
    chown -R $(whoami) "/Applications/Fleur.app"
    # Create application icon cache
    touch "/Applications/Fleur.app"
    killall Finder &>/dev/null || true
    # Display completion message with animation
    echo -e "${GREEN}${BOLD}"
    echo "Installation complete! ✨🍰✨"
    echo -e "${RESET}"
    open "/Applications/Fleur.app"
}

# Run the main installation process
main
