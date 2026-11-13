#!/usr/bin/env bash
# Install the EqnTide bridge for the current user's graphical login session.
set -euo pipefail
repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cli_bin="${repo_dir}/target/release/eqntide-cli"
desktop_bin="${EQNTIDE_DESKTOP_BIN:-/usr/bin/eqntide-desktop}"
unit_dir="${HOME}/.config/systemd/user"
if [[ ! -x "${cli_bin}" || ! -x "${desktop_bin}" ]]; then
  echo 'Build eqntide-cli in release mode and install EqnTide Desktop first.' >&2
  exit 1
fi
mkdir -p "${unit_dir}"
# Keep the legacy unit available for rollback, but stop enabling/restarting it.
systemctl --user disable --now matheditor.service 2>/dev/null || true
cat > "${unit_dir}/eqntide.service" <<EOF
[Unit]
Description=EqnTide local bridge for Word and WPS
PartOf=graphical-session.target
After=graphical-session.target

[Service]
Type=simple
ExecStart="${cli_bin}" --server
WorkingDirectory=${repo_dir}
Environment="EQNTIDE_DESKTOP_BIN=${desktop_bin}"
Restart=on-failure
RestartSec=3

[Install]
WantedBy=graphical-session.target
EOF
systemctl --user daemon-reload
systemctl --user enable --now eqntide.service
systemctl --user status eqntide.service --no-pager
