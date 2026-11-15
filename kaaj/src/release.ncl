# TODO: only release if ci passes
let { checkout, rust_toolchain, bevy_dependencies, cargo_build_env, .. } = import "common.ncl" in
let { uppercase_fields, sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
let IMPORT_GPG_VERSION = "7" in
let RELEASE_PLZ_ACTION_VERSION = "0.5" in
{
  name = "release",

  permissions = {
    contents = "write"
  },

  on = {
    push = {
      branches = ["[0-9]+.[0-9]+"],
    }
  },

  concurrency = {
    group = "release",
    cancel-in-progress = false,
  },

  env = cargo_build_env |> uppercase_fields,

  jobs =
    {
      release = {
        steps = [
          checkout
          & {
            with = {
              fetch-depth = 0,
              persist-credentials = true,
            }
          },
          {
            name = "import release gpg key",
            uses = "crazy-max/ghaction-import-gpg@v%{ IMPORT_GPG_VERSION }",
            with = {
              gpg_private_key = "${{ secrets.RELEASE_GPG_PRIVATE_KEY }}",
              passphrase = "${{ secrets.RELEASE_GPG_PASSPHRASE }}",
              fingerprint = "${{ secrets.RELEASE_GPG_FINGERPRINT }}",
              trust_level = "5",
              git_user_signingkey = true,
              git_tag_gpgsign = true,
              git_config_global = false,
            }
          },
          {
            name = "verify git tag signing preflight",
            shell = "bash",
            run = m%"
              set -euo pipefail
              [[ "$(git config --default false --get tag.gpgSign)" == "true" ]]
              tmp_tag="__release_signing_preflight_$$"
              trap 'git tag -d "$tmp_tag" >/dev/null 2>&1 || true' EXIT
              git tag -m "preflight signing check" "$tmp_tag" HEAD
              git verify-tag "$tmp_tag"
            "%
          },
          rust_toolchain,
          bevy_dependencies,
          {
            name = "run release-plz",
            uses = "release-plz/action@v%{ RELEASE_PLZ_ACTION_VERSION }",
            with = {
              command = "release"
            },
            env = {
              GITHUB_TOKEN = "${{ secrets.GITHUB_TOKEN }}",
              CARGO_REGISTRY_TOKEN = "${{ secrets.CARGO_REGISTRY_TOKEN }}"
            }
          }
        ]
      }
    }
    |> sync_name_field
    |> std.record.map_values run_on_ubuntu_latest
}
