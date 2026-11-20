#!/usr/bin/env bash
# ==============================================================================
# EqnTide Add-in Sideloading Script (Linux)
# Registers the WPS Office add-in for the EqnTide Equation Editor & Converter
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

WPS_ADDIN_DIR="${REPO_DIR}/wps-addin"

ACTION="install"
if [[ "${1:-}" == "--uninstall" || "${1:-}" == "uninstall" ]]; then
  ACTION="uninstall"
fi

echo "=========================================================="
echo " EqnTide Add-in Installer (${ACTION})"
echo "=========================================================="

OS="$(uname -s)"

ensure_linux_bridge_token() {
  local DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/eqntide"
  local TOKEN_FILE="${DATA_DIR}/bridge.token"
  mkdir -p "${DATA_DIR}"
  chmod 700 "${DATA_DIR}"
  if [[ ! -s "${TOKEN_FILE}" ]]; then
    python3 -c 'import secrets, sys; sys.stdout.write(secrets.token_hex(32) + "\n")' > "${TOKEN_FILE}"
  fi
  chmod 600 "${TOKEN_FILE}"
  tr -d '\r\n' < "${TOKEN_FILE}"
}

write_wps_token_script() {
  local DEST_DIR="$1"
  local TOKEN="$2"
  local TOKEN_JS="${DEST_DIR}/bridge-token.js"
  python3 - "$TOKEN_JS" "$TOKEN" <<'PY'
import json, os, sys
path, token = sys.argv[1], sys.argv[2]
with open(path, "w", encoding="utf-8") as fh:
    fh.write("// Generated per-user EqnTide bridge token\nwindow.EQNTIDE_BRIDGE_TOKEN = " + json.dumps(token) + ";\n")
os.chmod(path, 0o600)
PY
}

# ------------------------------------------------------------------------------
# Linux Installation (WPS Office JS Add-in)
# ------------------------------------------------------------------------------
install_linux_wps() {
  local DIRS=(
    "${HOME}/.local/share/Kingsoft/wps/jsaddons"
    "${HOME}/.local/share/Kingsoft/wps_intl/jsaddons"
  )
  local BRIDGE_TOKEN=""
  if [[ "${ACTION}" != "uninstall" ]]; then
    BRIDGE_TOKEN="$(ensure_linux_bridge_token)"
  fi

  for TARGET_BASE in "${DIRS[@]}"; do
    local TARGET_DIR="${TARGET_BASE}/EqnTide"
    local TARGET_VERSION_DIR="${TARGET_BASE}/EqnTide_1.0.0"
    local LEGACY_DIR="${TARGET_BASE}/MathEditor"
    local LEGACY_VERSION_DIR="${TARGET_BASE}/MathEditor_1.0.0"

    if [[ "${ACTION}" == "uninstall" ]]; then
      for ADDIN_DIR in "${TARGET_DIR}" "${TARGET_VERSION_DIR}" "${LEGACY_DIR}" "${LEGACY_VERSION_DIR}"; do
        if [[ -e "${ADDIN_DIR}" || -L "${ADDIN_DIR}" ]]; then
          echo "--> Removing WPS Add-in from ${ADDIN_DIR}..."
          rm -rf "${ADDIN_DIR}"
        fi
      done
      if [[ -d "${TARGET_BASE}" ]]; then python3 "${SCRIPT_DIR}/configure_wps.py" "${TARGET_BASE}" --uninstall; fi
      echo "✓ EqnTide and legacy MathEditor WPS add-ins removed."
      continue
    fi

    echo "--> Registering WPS Office JS Add-in into ${TARGET_BASE}..."
    rm -rf "${LEGACY_DIR}" "${LEGACY_VERSION_DIR}"
    mkdir -p "${TARGET_DIR}"
    if [[ -d "${TARGET_VERSION_DIR}" && ! -L "${TARGET_VERSION_DIR}" ]]; then
      cp -r "${WPS_ADDIN_DIR}"/* "${TARGET_VERSION_DIR}/"
      write_wps_token_script "${TARGET_VERSION_DIR}" "${BRIDGE_TOKEN}"
    else
      ln -sfn EqnTide "${TARGET_VERSION_DIR}"
    fi
    cp -r "${WPS_ADDIN_DIR}"/* "${TARGET_DIR}/"
    write_wps_token_script "${TARGET_DIR}" "${BRIDGE_TOKEN}"

    python3 "${SCRIPT_DIR}/configure_wps.py" "${TARGET_BASE}"

    echo "✓ WPS Add-in registered in ${TARGET_BASE}."
  done

  if [[ "${ACTION}" != "uninstall" ]]; then
    echo "ℹ Please completely exit WPS Office (close all windows) and reopen it."
    echo "  The 'EqnTide' tab will now appear on the Ribbon."
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
    echo "macOS support is temporarily disabled (no test hardware)." >&2
    exit 1
    ;;
  *)
    echo "Unsupported OS: ${OS}. Please use install-addin.ps1 on Windows."
    exit 1
    ;;
esac

echo ""
echo "Done! Make sure EqnTide / local bridge is running on port 17890."
