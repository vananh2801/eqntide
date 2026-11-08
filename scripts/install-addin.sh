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
  local DIRS=(
    "${HOME}/.local/share/Kingsoft/wps/jsaddons"
    "${HOME}/.local/share/Kingsoft/wps_intl/jsaddons"
  )

  for TARGET_BASE in "${DIRS[@]}"; do
    local TARGET_DIR="${TARGET_BASE}/MathEditor"

    if [[ "${ACTION}" == "uninstall" ]]; then
      if [[ -d "${TARGET_DIR}" ]]; then
        echo "--> Removing WPS Add-in from ${TARGET_DIR}..."
        rm -rf "${TARGET_DIR}"
        echo "✓ WPS Add-in successfully uninstalled."
      fi
      continue
    fi

    echo "--> Registering WPS Office JS Add-in into ${TARGET_BASE}..."
    mkdir -p "${TARGET_DIR}"
    cp -r "${WPS_ADDIN_DIR}"/* "${TARGET_DIR}/"

    # Write publish.xml using standard WPS jsplugins schema
    local PUBLISH_XML="${TARGET_BASE}/publish.xml"
    cat << 'EOF' > "${PUBLISH_XML}"
<?xml version="1.0" encoding="UTF-8"?>
<jsplugins>
  <jsplugin name="MathEditor" type="wps" version="1.0.0" enable="enable_dev" url="MathEditor/" />
  <jspluginonline name="MathEditor" type="wps" enable="enable_dev" debug="true" url="http://127.0.0.1:17890/wps-addin/" />
</jsplugins>
EOF

    # Write authaddin.json to authorize MathEditor execution
    local AUTH_JSON="${TARGET_BASE}/authaddin.json"
    cat << 'EOF' > "${AUTH_JSON}"
{
  "wps": {
    "namelist": "MathEditor",
    "MathEditor": {
      "name": "MathEditor",
      "enable": true,
      "isload": true,
      "path": "MathEditor/",
      "md5": "",
      "mode": 2
    }
  }
}
EOF

    echo "✓ WPS Add-in registered in ${TARGET_BASE}."
  done

  if [[ "${ACTION}" != "uninstall" ]]; then
    echo "ℹ Please completely exit WPS Office (close all windows) and reopen it."
    echo "  The 'Math Editor' tab will now appear on the Ribbon."
  fi
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
  local WPS_MAC_DIR="${HOME}/Library/Containers/com.kingsoft.wpsoffice.mac/Data/.kingsoft/wps/jsaddons"
  if [[ -d "$(dirname "${WPS_MAC_DIR}")" ]]; then
    if [[ "${ACTION}" == "uninstall" ]]; then
      rm -rf "${WPS_MAC_DIR}/MathEditor"
    else
      echo "--> Registering WPS Office JS Add-in for Mac..."
      mkdir -p "${WPS_MAC_DIR}/MathEditor"
      cp -r "${WPS_ADDIN_DIR}"/* "${WPS_MAC_DIR}/MathEditor/"
      cat << 'EOF' > "${WPS_MAC_DIR}/publish.xml"
<?xml version="1.0" encoding="UTF-8"?>
<jsplugins>
  <jsplugin name="MathEditor" type="wps" version="1.0.0" enable="enable_dev" url="MathEditor/" />
  <jspluginonline name="MathEditor" type="wps" enable="enable_dev" debug="true" url="http://127.0.0.1:17890/wps-addin/" />
</jsplugins>
EOF
      cat << 'EOF' > "${WPS_MAC_DIR}/authaddin.json"
{
  "wps": {
    "namelist": "MathEditor",
    "MathEditor": {
      "name": "MathEditor",
      "enable": true,
      "isload": true,
      "path": "MathEditor/",
      "md5": "",
      "mode": 2
    }
  }
}
EOF
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
