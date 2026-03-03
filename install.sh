#!/bin/bash

# First check OS.
OS="$(uname)"
if [[ "${OS}" == "Linux" ]]
then
  TXTX_ON_LINUX=1
elif [[ "${OS}" == "Darwin" ]]
then
  TXTX_ON_MACOS=1
else
  abort "txtx is only supported on macOS and Linux."
fi

# Required installation paths.
TXTX_DL_BASE_URL="https://github.com/txtx/txtx/releases/latest/download"
# Prefer ~/.local/bin; fallback to ~/bin
if mkdir -p "$HOME/.local/bin" 2>/dev/null; then
  INSTALL_DIR="$HOME/.local/bin"
else
  mkdir -p "$HOME/bin"
  INSTALL_DIR="$HOME/bin"
fi

TXTX_DST="${INSTALL_DIR}/txtx"

if [[ -n "${TXTX_ON_MACOS-}" ]]
then
  UNAME_MACHINE="$(/usr/bin/uname -m)"

  if [[ "${UNAME_MACHINE}" == "arm64" ]]
  then
    # ARM macOS
    TXTX_DL_RELEASE_URL="${TXTX_DL_BASE_URL}/txtx-darwin-arm64.tar.gz"
  else
    # Intel macOS
    TXTX_DL_RELEASE_URL="${TXTX_DL_BASE_URL}/txtx-darwin-x64.tar.gz"
  fi
else
  UNAME_MACHINE="$(uname -m)"

  # Linux
  TXTX_DL_RELEASE_URL="${TXTX_DL_BASE_URL}/txtx-linux-x64.tar.gz"
fi

# string formatters
if [[ -t 1 ]]
then
  tty_escape() { printf "\033[%sm" "$1"; }
else
  tty_escape() { :; }
fi
tty_mkbold() { tty_escape "1;$1"; }
tty_underline="$(tty_escape "4;39")"
tty_blue="$(tty_mkbold 34)"
tty_green="$(tty_mkbold 32)"
tty_orange="$(tty_mkbold 33)"
tty_red="$(tty_mkbold 31)"
tty_bold="$(tty_mkbold 39)"
tty_reset="$(tty_escape 0)"

shell_join() {
  local arg
  printf "%s" "$1"
  shift
  for arg in "$@"
  do
    printf " "
    printf "%s" "${arg// /\ }"
  done
}

abort() {
  printf "%s\n" "$@" >&2
  exit 1
}

chomp() {
  printf "%s" "${1/"$'\n'"/}"
}

ohai() {
  printf "${tty_orange}→${tty_bold} %s${tty_reset}\n" "$(shell_join "$@")"
}

warn() {
  printf "${tty_red}Warning${tty_reset}: %s\n" "$(chomp "$1")" >&2
}

echo ""
ohai "Downloading and installing ${tty_green}txtx${tty_reset} from ${tty_orange}${TXTX_DL_RELEASE_URL}${tty_reset}"
curl -s -o txtx.tar.gz -L $TXTX_DL_RELEASE_URL
gzip -d txtx.tar.gz
tar -xf txtx.tar
if [[ -n "${TXTX_ON_MACOS-}" ]]
then
  xattr -d com.apple.quarantine ./txtx 2> /dev/null
fi

ohai "Installing ${TXTX_DST}"
install -m 0755 "txtx" "${TXTX_DST}"
rm txtx txtx.tar txtx.tar.gz

echo -e "${tty_green}✓${tty_reset} Install successful"

if ! command -v txtx >/dev/null 2>&1; then
  echo ""
  echo "⚠️  txtx is not in your PATH."
  echo "   Add this to your shell config (e.g. ~/.zshrc or ~/.bashrc):"
  echo "     export PATH=\"${INSTALL_DIR}:\$PATH\""
  echo ""
fi

cat <<EOS
${tty_blue}+-----------------+
|                 |
|                 |
|   ##            |
|  #####  ##  ##  |
|   ##     # ##   |
|   ##  #  ## #   |
|    ###  ##   #  |
|                 |
|                 |
+-----------------+
${tty_orange}→${tty_reset} Run ${tty_green}txtx new${tty_reset} to get started
${tty_orange}→${tty_reset} Further documentation: ${tty_orange}https://docs.txtx.sh${tty_reset}
EOS
