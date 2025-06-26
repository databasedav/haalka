let { uppercase_fields, sync_name_field, run_on_ubuntu_latest, name_id, .. } = import "utils.ncl" in

let CHECKOUT_VERSION = "v4" in
let DEFAULT_RUST_TOOLCHAIN_VERSION = "stable" in
let BEVY_DEPENDENCIES = [
  "libasound2-dev",
  "libudev-dev",
  "libwayland-dev",
  "libxkbcommon-dev",
]
in
let CARGO_CACHE_VERSION = "2" in
let SETUP_JUST_VERSION = "2" in
let NICKEL_VERSION = "1.12.2" in
let install_nickel = {
  name = "install nickel",
  run = m%"
    OUT="$HOME/.local/bin/nickel"
    mkdir -p "$(dirname "$OUT")"
    curl -L "https://github.com/tweag/nickel/releases/download/%{ NICKEL_VERSION }/nickel-pkg-x86_64-linux" -o "$OUT"
    chmod +x "$OUT"
    echo "$(dirname "$OUT")" >> "$GITHUB_PATH"
    nickel --version
  "%
} in
{
  REPO = "https://github.com/databasedav/haalka",
  GITHUB_PAGES_DEPLOY_ACTION_VERSION = "4.7.3",
  GITHUB_PAGES_DEPLOY_ACTION_ATTEMPT_LIMIT = 100,
  checkout = {
    name = "checkout repo",
    uses = "actions/checkout@%{ CHECKOUT_VERSION }",
    with = {
      submodules = true
    }
  },
  rust_toolchain = {
    name = "install rust toolchain",
    uses = "dtolnay/rust-toolchain@master",
    with = {
      toolchain | default = DEFAULT_RUST_TOOLCHAIN_VERSION
    }
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
  install_nickel,
  assert_nickels_synced =
    {
      steps = [
        checkout,
        install_nickel,
        just,
        {
          name = "assert nickels synced",
          run = m%"
            just sync_nickels
            [[ -n $(git status --porcelain) ]] && echo 'nickels are not synced; run `just format_nickels sync_nickels`' && exit 1 || exit 0
          "%
        }
      ]
    }
    |> run_on_ubuntu_latest,
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
  build_examples = {
    strategy = {
      matrix = {
        example = "${{ fromJson(needs.get_examples_.outputs.examples) }}",
        api = ["webgl2", "webgpu"],
      }
    },
    uses = "./.github/workflows/build_example.yaml",
    with = {
      api = "${{ matrix.api }}",
      example = "${{ matrix.example }}",
    },
  },
  deploy_examples = fun pr =>
    {
      permissions = {
        contents = "write",
      },
      steps = [
        checkout,
        {
          name = "remove gitignore",
          run = "rm .gitignore"
        },
        {
          name = "download artifacts",
          uses = "actions/download-artifact@v4",
          with = {
            path = "artifacts",
            github-token = "${{ secrets.GITHUB_TOKEN }}",
          }
        },
        {
          name = "organize artifacts",
          run = m%"
          for artifact in artifacts/*; do
            dirname=$(basename "$artifact")
            api=${dirname%%_*}
            example=${dirname#*_}
            mkdir -p "pages/$api/$example"
            mv "$artifact"/* "pages/$api/$example"
          done
        "%,
        },
        {
          name = "deploy",
          uses = "JamesIves/github-pages-deploy-action@v%{ GITHUB_PAGES_DEPLOY_ACTION_VERSION }",
          with = {
            folder = "pages",
            force = false,
            attempt-limit = GITHUB_PAGES_DEPLOY_ACTION_ATTEMPT_LIMIT,
            target-folder = if pr != "" then "pr_previews/%{ pr }" else "examples",
            commit-message = if pr != "" then "%{ REPO }/pull/%{ pr }/commits/${{ github.event.pull_request.head.sha }}" else "%{ REPO }/commit/${{ github.sha }}",
          }
        },
      ]
    }
    |> run_on_ubuntu_latest,
}
