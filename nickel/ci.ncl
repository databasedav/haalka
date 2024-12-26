# TODO: ci should run for the wasm32 target as well
let { checkout, rust_toolchain, bevy_dependencies, cached_target_populate, just, binstall, cargo_build_env, .. } = import "common.ncl" in
let { uppercase_fields, sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
{
  name = "ci",

  on = "push",

  env = cargo_build_env |> uppercase_fields,

  jobs = {

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
        rust_toolchain & { with = { components = "clippy" } },
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
        {
          name = "install rust toolchain",
          uses = "dtolnay/rust-toolchain@nightly",
          with = { components = "rustfmt" }
        },
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
        rust_toolchain,
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
          run = "cargo binstall --no-confirm cargo-all-features"
        },
        {
          name = "check all features",
          run = "just check_all_features"
        }
      ]
    }
  }
  |> sync_name_field
  |> std.record.map_values run_on_ubuntu_latest
}
