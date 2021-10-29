#! /usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

usage() {
  # Local var because of grep
  declare helpdoc='HELP'
  helpdoc+='DOC'

  echo 'Usage: ./build.sh [command] ...'
  echo 'Commands:'
  echo ''
  grep "$helpdoc" "dev.sh" -B 1 | grep -E -v '^--$' | sed -e 's/^  //g' -e "s/# $helpdoc: //g"
}

function git-root() {
  cd "$(git rev-parse --show-toplevel)"
}

function write() {
  local text="$1"
  echo "${text}"
}

function warn() {
  local text="$1"
  echo "${text}"
}

function watch() {
  write "WATCH: watching the $(pwd) and testing changes"
  rg --files | entr -s "./dev.sh validate 2>&1 | head -n 40"
}

function check-rust-format {
  write "CHECK-RUST-FORMAT: checking the formatting on all rust code"
  cargo fmt --all -- --check
}

function format-rust {
  write "FORMAT-RUST: formatting all rust code"
  cargo fmt --all
}

function clean {
  write "CLEAN: removing all cargo artifacts"
  cargo clean
}

function lint-rust {
  write "LINT-RUST: running clippy on all rust code"
  cargo clippy -- -D warnings
}

function build-rust {
  write "BUILD-RUST: running 'cargo build' to build all rustn code"
  cargo build
}

function test-rust {
  write "TEST-RUST: testing all rust code"
  cargo test
}

function lint-docs {
  write "LINT-DOCS: run the vale tool on all restructured text"
  rg --files -g '*.rst' | xargs vale
}

function docs() {
  write "DOCS: building all sphinx based documentation in the docs directory"
  cd docs
  make html
}

function check-shell-format() {
  write "CHECK-SHELL-FORMAT: checking the formatting on all shell code"
  shfmt -d -i 2 .
}

function format-shell() {
  write "FORMAT-SHELL: format on all shell code"
  shfmt -i 2 -w .
}

function lint-shell() {
  write "LINT-SHELL: format on all shell code"

  rg --files -g '*.sh' | xargs shellcheck
}

function check-nix-format() {
  write "CHECK-NIX-FORMAT: check the format on all nix code"

  rg --files -g '*.nix' | xargs nixfmt -c
}

function format-nix() {
  write "FORMAT-NIX: format on all nix code"

  rg --files -g '*.nix' | xargs nixfmt
}

function lint-nix() {
  write "LINT-NIX: lint nix code"
  local result

  # the linter does not work well on nix flakes or generated nix files
  set +o errexit
  result=$(rg --files -g '*.nix' -g '!flake.nix' -g '!Cargo.nix')
  set -o errexit

  echo "${result}" | xargs -r nix-linter
}

function build-nix() {
  write "BUILD-NIX: build based on nix"

  nix build
}

function lint() {
  write "LINT: run all available lints"
  lint-rust
  lint-shell
  lint-docs
  lint-nix
}

function check-format() {
  write "CHECK-FORMAT: run all available checks on formatting"
  check-shell-format
  check-rust-format
  check-nix-format
}

function test() {
  write "TEST: run all available tests"
  test-rust
}

function build() {
  write "BUILD: run all available builds"
  build-rust
  build-nix
}

function format() {
  write "FORMAT: run all available formats"
  format-rust
  format-shell
  format-nix
}

function validate() {
  write "VALIDATE: run all available validations"
  lint
  build
  test
  check-format
}

function notify-fail() {
  local rv=$?
  if [[ $rv -ne 0 ]]; then
    warn "process failed, exiting..."
  fi
  exit $rv
}
trap "notify-fail" EXIT

function main() {
  local arg="${1:-no-arg}"

  git-root

  case "$arg" in
  clean)
    # HELPDOC: remove all the build artifacts in the system
    clean
    ;;
  validate)
    # HELPDOC: run all the checks and validations available
    validate
    ;;
  docs)
    # HELPDOC: build the sphinx based documentation in the `docs` directory
    docs
    ;;
  lint)
    # HELPDOC: run all of the lint checks
    lint
    ;;
  test)
    # HELPDOC: run all of the tests in the system
    test
    ;;
  build)
    # HELPDOC: build all the buildable code in the system
    build
    ;;
  check-format)
    # HELPDOC: check formatting on all the code in the system
    check-format
    ;;
  format)
    # HELPDOC: format all the code in the system
    format
    ;;
  watch)
    # HELPDOC: run validate any time a file chagnes
    watch
    ;;
  lint-rust)
    # HELPDOC: run all of the lint checks on rust code
    lint-rust
    ;;
  build-rust)
    # HELPDOC: build all of the rust code in the system
    build-rust
    ;;
  test-rust)
    # HELPDOC: run all of the tests in the system
    test-rust
    ;;
  check-shell-format)
    # HELPDOC: check that all the shell files in the system are formatted
    check-shell-format
    ;;
  format-shell)
    # HELPDOC: format all the shell files in the system
    format-shell
    ;;
  lint-shell)
    # HELPDOC: lint all the shell files in the system
    lint-shell
    ;;
  check-nix-format)
    # HELPDOC: check that all the nix files in the system are formatted
    check-nix-format
    ;;
  format-nix)
    # HELPDOC: format all the nix files in the system
    format-nix
    ;;
  lint-nix)
    # HELPDOC: lint all the nix files in the system
    lint-nix
    ;;
  build-nix)
    # HELPDOC: build the system using nix
    build-nix
    ;;
  lint-docs)
    # HELPDOC: lint all the docs using the vale text linter
    lint-docs
    ;;
  *)
    usage
    exit 1
    ;;
  esac
}

main "$@"
