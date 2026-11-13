#!/usr/bin/env bash
# ==============================================================================
# EqnTide Add-in Sideloading Script (Linux & macOS)
# Registers Word & WPS Office Add-ins for the EqnTide Equation Editor & Converter
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
echo " EqnTide Add-in Installer (${ACTION})"
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
    local TARGET_DIR="${TARGET_BASE}/EqnTide"
    local TARGET_VERSION_DIR="${TARGET_BASE}/EqnTide_1.0.0"
    local LEGACY_DIR="${TARGET_BASE}/MathEditor"
    local LEGACY_VERSION_DIR="${TARGET_BASE}/MathEditor_1.0.0"

    if [[ "${ACTION}" == "uninstall" ]]; then
      for ADDIN_DIR in "${TARGET_DIR}" "${TARGET_VERSION_DIR}" "${LEGACY_DIR}" "${LEGACY_VERSION_DIR}"; do
        if [[ -e "${ADDIN_DIR}" ]]; then
          echo "--> Removing WPS Add-in from ${ADDIN_DIR}..."
          rm -rf "${ADDIN_DIR}"
        fi
      done
      echo "✓ EqnTide and legacy MathEditor WPS add-ins removed."
      continue
    fi

    echo "--> Registering WPS Office JS Add-in into ${TARGET_BASE}..."
    rm -rf "${LEGACY_DIR}" "${LEGACY_VERSION_DIR}"
    mkdir -p "${TARGET_DIR}"
    ln -sfn EqnTide "${TARGET_BASE}/EqnTide_1.0.0"
    cp -r "${WPS_ADDIN_DIR}"/* "${TARGET_DIR}/"

    # Write publish.xml using standard WPS jsplugins schema
    local PUBLISH_XML="${TARGET_BASE}/publish.xml"
    cat << 'EOF' > "${PUBLISH_XML}"
<?xml version="1.0" encoding="UTF-8"?>
<jsplugins>
  <jsplugin name="EqnTide" type="wps" version="1.0.0" enable="enable_dev" url="EqnTide_1.0.0/" />
  <jspluginonline name="EqnTide" type="wps" enable="enable_dev" debug="true" url="http://127.0.0.1:17890/wps-addin/" />
</jsplugins>
EOF

    # Write authaddin.json to authorize EqnTide execution
    local AUTH_JSON="${TARGET_BASE}/authaddin.json"
    cat << 'EOF' > "${AUTH_JSON}"
{
  "wps": {
    "namelist": "EqnTide",
    "EqnTide": {
      "name": "EqnTide",
      "enable": true,
      "isload": true,
      "path": "EqnTide_1.0.0/",
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
    echo "  The 'EqnTide' tab will now appear on the Ribbon."
  fi
}

# ------------------------------------------------------------------------------
# macOS Installation (Word Add-in & WPS Office)
# ------------------------------------------------------------------------------
install_macos_word() {
  local WEF_DIR="${HOME}/Library/Containers/com.microsoft.Word/Data/Documents/wef"
  local MANIFEST_SRC="${WORD_ADDIN_DIR}/manifest.xml"
  local MANIFEST_DST="${WEF_DIR}/manifest-eqntide.xml"

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
      rm -rf "${WPS_MAC_DIR}/EqnTide" "${WPS_MAC_DIR}/EqnTide_1.0.0" \
        "${WPS_MAC_DIR}/MathEditor" "${WPS_MAC_DIR}/MathEditor_1.0.0"
    else
      echo "--> Registering WPS Office JS Add-in for Mac..."
      rm -rf "${WPS_MAC_DIR}/MathEditor" "${WPS_MAC_DIR}/MathEditor_1.0.0"
      mkdir -p "${WPS_MAC_DIR}/EqnTide"
      cp -r "${WPS_ADDIN_DIR}"/* "${WPS_MAC_DIR}/EqnTide_1.0.0/"
      cat << 'EOF' > "${WPS_MAC_DIR}/publish.xml"
<?xml version="1.0" encoding="UTF-8"?>
<jsplugins>
  <jsplugin name="EqnTide" type="wps" version="1.0.0" enable="enable_dev" url="EqnTide_1.0.0/" />
  <jspluginonline name="EqnTide" type="wps" enable="enable_dev" debug="true" url="http://127.0.0.1:17890/wps-addin/" />
</jsplugins>
EOF
      cat << 'EOF' > "${WPS_MAC_DIR}/authaddin.json"
{
  "wps": {
    "namelist": "EqnTide",
    "EqnTide": {
      "name": "EqnTide",
      "enable": true,
      "isload": true,
      "path": "EqnTide_1.0.0/",
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
echo "Done! Make sure EqnTide / local bridge is running on port 17890."
