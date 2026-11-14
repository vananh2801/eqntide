#!/usr/bin/env bash
# Install the EqnTide bridge for the current user's graphical login session.
set -euo pipefail
repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "$(uname -s)" != "Linux" ]]; then echo 'This service installer requires Linux/systemd.' >&2; exit 1; fi
cli_bin="${EQNTIDE_CLI_BIN:-${repo_dir}/eqntide-cli}"
if [[ ! -x "$cli_bin" ]]; then cli_bin="${repo_dir}/target/release/eqntide-cli"; fi
if [[ ! -x "$cli_bin" ]] && command -v eqntide-cli >/dev/null; then cli_bin="$(command -v eqntide-cli)"; fi
desktop_bin="${EQNTIDE_DESKTOP_BIN:-${repo_dir}/eqntide-desktop}"
if [[ ! -x "$desktop_bin" ]]; then desktop_bin="${repo_dir}/target/release/eqntide-desktop"; fi
if [[ ! -x "$desktop_bin" ]] && command -v eqntide-desktop >/dev/null; then desktop_bin="$(command -v eqntide-desktop)"; fi
unit_dir="${HOME}/.config/systemd/user"
if [[ "${1:-}" == "--uninstall" ]]; then
  systemctl --user disable --now eqntide.service 2>/dev/null || true
  # Offline users may have no user bus; remove our known enablement link too.
  rm -f "${unit_dir}/graphical-session.target.wants/eqntide.service" "${unit_dir}/eqntide.service"
  systemctl --user daemon-reload 2>/dev/null || true
  exit 0
fi
if [[ ! -x "${cli_bin}" || ! -x "${desktop_bin}" ]]; then
  echo 'Build eqntide-cli in release mode and install EqnTide Desktop first.' >&2
  exit 1
fi
write_unit() {
cat <<EOF
[Unit]
Description=EqnTide local bridge for Word and WPS
PartOf=graphical-session.target
After=graphical-session.target

[Service]
Type=simple
ExecStart="${cli_bin}" --server
WorkingDirectory=$(dirname "${cli_bin}")
Environment="EQNTIDE_DESKTOP_BIN=${desktop_bin}"
Restart=on-failure
RestartSec=3

[Install]
WantedBy=graphical-session.target
EOF
}
if [[ "${1:-}" == "--print-unit" ]]; then write_unit; exit 0; fi
if [[ "${1:-}" == "--ensure" && -f "${unit_dir}/eqntide.service" ]] && cmp -s <(write_unit) "${unit_dir}/eqntide.service"; then
  systemctl --user daemon-reload
  systemctl --user enable --now eqntide.service
  exit 0
fi
mkdir -p "${unit_dir}"
systemctl --user disable --now matheditor.service 2>/dev/null || true
write_unit > "${unit_dir}/eqntide.service"
systemctl --user daemon-reload
systemctl --user enable eqntide.service
systemctl --user restart eqntide.service
systemctl --user status eqntide.service --no-pager
