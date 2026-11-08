#!/usr/bin/env bash
# ==============================================================================
# Math Editor Add-in Sideloading Script (Linux & macOS)
# Registers Word & WPS Office Add-ins for the Math Equation Editor & Converter
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

WORD_ADDIN_DIR="${REPO_DIR}/word-addin"
WPS_ADDIN_DIR="${REPO_DIR}/wps-addin"

ACTION="install"
if [[ "${1:-}" == "--uninstall" || "${1:-}" == "uninstall" ]]; then
  ACTION="uninstall"
fi

echo "=========================================================="
echo " Math Editor Add-in Installer (${ACTION})"
echo "=========================================================="

OS="$(uname -s)"

# ------------------------------------------------------------------------------
# Linux Installation (WPS Office JS Add-in)
# ------------------------------------------------------------------------------
install_linux_wps() {
  local TARGET_BASE="${HOME}/.local/share/Kingsoft/wps/jsaddons"
  local TARGET_DIR="${TARGET_BASE}/MathEditor"

  if [[ "${ACTION}" == "uninstall" ]]; then
    if [[ -d "${TARGET_DIR}" ]]; then
      echo "--> Removing WPS Add-in from ${TARGET_DIR}..."
      rm -rf "${TARGET_DIR}"
      echo "✓ WPS Add-in successfully uninstalled."
    else
      echo "ℹ WPS Add-in directory not found, skipping."
    fi
    return
  fi

  echo "--> Registering WPS Office JS Add-in..."
  mkdir -p "${TARGET_DIR}"
  cp -r "${WPS_ADDIN_DIR}"/* "${TARGET_DIR}/"

  # Ensure publish.xml exists so WPS automatically loads the addin
  local PUBLISH_XML="${TARGET_BASE}/publish.xml"
  if [[ ! -f "${PUBLISH_XML}" ]]; then
    cat << 'EOF' > "${PUBLISH_XML}"
<jsaddons>
  <jsaddon name="MathEditor" enable="true" />
</jsaddons>
EOF
  fi

  echo "✓ WPS Add-in successfully installed to:"
  echo "    ${TARGET_DIR}"
  echo "ℹ Restart WPS Office Writer. The 'Math Editor' tab will appear on the ribbon."
}

# ------------------------------------------------------------------------------
# macOS Installation (Word Add-in & WPS Office)
# ------------------------------------------------------------------------------
install_macos_word() {
  local WEF_DIR="${HOME}/Library/Containers/com.microsoft.Word/Data/Documents/wef"
  local MANIFEST_SRC="${WORD_ADDIN_DIR}/manifest.xml"
  local MANIFEST_DST="${WEF_DIR}/manifest-matheditor.xml"

  if [[ "${ACTION}" == "uninstall" ]]; then
    if [[ -f "${MANIFEST_DST}" ]]; then
      echo "--> Removing Word Add-in manifest from ${MANIFEST_DST}..."
      rm -f "${MANIFEST_DST}"
      echo "✓ Word Add-in manifest uninstalled."
    fi
  else
    echo "--> Sideloading Word Add-in manifest for Office for Mac..."
    mkdir -p "${WEF_DIR}"
    cp "${MANIFEST_SRC}" "${MANIFEST_DST}"
    echo "✓ Word Add-in manifest installed to:"
    echo "    ${MANIFEST_DST}"
    echo "ℹ Restart Microsoft Word for Mac. Check the 'Insert' -> 'My Add-ins' -> 'Developer Add-ins' or Ribbon."
  fi

  # Also install WPS if installed on macOS
  local WPS_MAC_DIR="${HOME}/Library/Containers/com.kingsoft.wpsoffice.mac/Data/Library/Application Support/Kingsoft/wps/jsaddons/MathEditor"
  if [[ -d "$(dirname "${WPS_MAC_DIR}")" || -d "/Applications/wpsoffice.app" ]]; then
    if [[ "${ACTION}" == "uninstall" ]]; then
      rm -rf "${WPS_MAC_DIR}"
    else
      echo "--> Registering WPS Office JS Add-in for Mac..."
      mkdir -p "${WPS_MAC_DIR}"
      cp -r "${WPS_ADDIN_DIR}"/* "${WPS_MAC_DIR}/"
      echo "✓ WPS Add-in installed to ${WPS_MAC_DIR}."
    fi
  fi
}

# ------------------------------------------------------------------------------
# Main Dispatch
# ------------------------------------------------------------------------------
case "${OS}" in
  Linux*)
    install_linux_wps
    ;;
  Darwin*)
    install_macos_word
    ;;
  *)
    echo "Unsupported OS: ${OS}. Please use install-addin.ps1 on Windows."
    exit 1
    ;;
esac

echo ""
echo "Done! Make sure the desktop editor / local bridge is running on port 17890."
