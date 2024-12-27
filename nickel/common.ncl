let { uppercase_fields, sync_name_field, run_on_ubuntu_latest, name_id, .. } = import "utils.ncl" in

let CHECKOUT_VERSION = "v4" in
let RUST_TOOLCHAIN_VERSION = "stable" in
let BEVY_DEPENDENCIES = [
  "libasound2-dev",
  "libudev-dev",
  "libwayland-dev",
  "libxkbcommon-dev",
]
in
let CARGO_CACHE_VERSION = "2" in
let SETUP_JUST_VERSION = "2" in
let INSTALL_NIX_ACTION_VERSION = "30" in
let CACHIX_ACTION_VERSION = "15" in
{
  REPO = "https://github.com/databasedav/haalka",
  checkout = {
    name = "checkout repo",
    uses = "actions/checkout@%{ CHECKOUT_VERSION }",
    with = {
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
    uses = "Leafwing-Studios/cargo-cache@v%{ CARGO_CACHE_VERSION }",
    with = {
      sweep-cache = true,
    }
  },
  just = {
    name = "install just",
    uses = "extractions/setup-just@v%{ SETUP_JUST_VERSION }"
  },
  binstall = {
    name = "install binstall",
    uses = "cargo-bins/cargo-binstall@main"
  },
  assert_nickels_synced = {
    steps = [
      checkout,
      {
        name = "install nix",
        uses = "cachix/install-nix-action@v%{ INSTALL_NIX_ACTION_VERSION }",
      },
      {
        name = "nickel binary cache",
        uses = "cachix/cachix-action@v%{ CACHIX_ACTION_VERSION }",
        with = {
          name = "tweag-nickel-cache",
          extraPullNames = "tweag-nickel"
        }
      },
      {
        name = "install nickel",
        run = m%"
          nix run github:tweag/nickel
          alias nickel='nix run github:tweag/nickel'
          nickel --version
        "%
      },
      just,
      {
        name = "assert nickels synced",
        run = m%"
        just sync_nickels
        [[ -n $(git status --porcelain) ]] && echo "nickels are not synced; run `just sync_nickels`" && exit 1 || exit 0
      "%
      }
    ]
  },
  get_examples =
    {
      outputs = {
        examples = "${{ steps.output_examples.outputs.examples }}"
      },
      steps = [
        checkout,
        just,
        {
          name = "output_examples",
          run = m%"
          examples=$(just list_examples)
          echo "examples=$examples"
          echo "examples=$examples" >> $GITHUB_OUTPUT
        "%
        }
        |> name_id
      ]
    }
    |> run_on_ubuntu_latest,
  build_and_deploy_examples = fun pr_ =>
    {
      needs = "get_examples_",
      strategy = {
        matrix = {
          example = "${{ fromJson(needs.get_examples_.outputs.examples) }}",
          api = ["webgl2", "webgpu"],
        }
      },
      permissions = {
        contents = "write",
      },
      uses = "./.github/workflows/build_and_deploy_example.yaml",
      with = {
        pr = pr_,
        api = "${{ matrix.api }}",
        example = "${{ matrix.example }}",
      },
    }
}
