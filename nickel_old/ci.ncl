# TODO: ci should run for the wasm32 target as well
let { checkout, rust_toolchain, bevy_dependencies, cached_target_populate, just, binstall, cargo_build_env, assert_nickels_synced, .. } = import "common.ncl" in
let { uppercase_fields, sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
{
  name = "ci",

  on = "push",

  env = cargo_build_env |> uppercase_fields,

  jobs =
    {
      assert_nickels_synced_ = assert_nickels_synced,
    }
    & (
      {
        test = {
          steps = [
            checkout,
            rust_toolchain,
            bevy_dependencies,
            cached_target_populate,
            just,
            {
              name = "run tests",
              run = "just test"
            }
          ]
        },
        clippy = {
          steps = [
            checkout,
            rust_toolchain & { with = { toolchain = "nightly", components = "clippy" } },
            bevy_dependencies,
            cached_target_populate,
            just,
            {
              name = "run clippy",
              run = "just clippy"
            }
          ]
        },
        format = {
          steps = [
            checkout,
            rust_toolchain & { with = { toolchain = "nightly", components = "rustfmt" } },
            just,
            {
              name = "run format",
              run = "just format -- --check"
            }
          ]
        },
        docs = {
          steps = [
            checkout,
            rust_toolchain & { with = { toolchain = "nightly" } },
            bevy_dependencies,
            cached_target_populate,
            just,
            {
              name = "check docs",
              run = "just doc"
            }
          ]
        },
        check_all_features = {
          steps = [
            checkout,
            rust_toolchain,
            bevy_dependencies,
            cached_target_populate,
            just,
            binstall,
            {
              name = "install cargo all features",
              run = "cargo binstall --no-confirm --continue-on-failure cargo-all-features"
            },
            {
              name = "check all features",
              run = "just check_all_features"
            }
          ]
        }
      }
      |> std.record.map_values (fun record => record & { needs = "assert_nickels_synced_" })
    )
    |> sync_name_field
    |> std.record.map_values run_on_ubuntu_latest
}
