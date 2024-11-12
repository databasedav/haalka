let CHECKOUT_VERSION = "v4" in
let RUST_TOOLCHAIN_VERSION = "stable" in
let BEVY_DEPENDENCIES = [
  "libasound2-dev",
  "libudev-dev",
  "libwayland-dev",
  "libxkbcommon-dev",
] in
let CARGO_CACHE_VERSION = "v2" in
let SETUP_JUST_VERSION = "v2" in
{
  runs_on = "ubuntu-latest",
  checkout = {
    name = "checkout repo",
    uses = "actions/checkout@%{ CHECKOUT_VERSION }",
    with = {
      fetch-depth = 0,
      submodules = true
    }
  },
  rust_toolchain = {
    name = "install rust toolchain",
    uses = "dtolnay/rust-toolchain@%{ RUST_TOOLCHAIN_VERSION }"
  },
  bevy_dependencies = {
    name = "install bevy dependencies",
    shell = "bash",
    run = "sudo apt-get update && sudo apt-get install --no-install-recommends %{ BEVY_DEPENDENCIES |> std.string.join " " }"
  },
  cargo_build_env = {
    cargo_term_color = "always",
    rustflags = "--deny warnings",
    rustdocflags = "--deny warnings",
  },
  cached_target_populate = {
    name = "populated target directory from cache",
    uses = "Leafwing-Studios/cargo-cache@%{ CARGO_CACHE_VERSION }",
    with = {
      sweep-cache = true,
    }
  },
  just = {
    name = "install just",
    uses = "extractions/setup-just@%{ SETUP_JUST_VERSION }"
  }
}
