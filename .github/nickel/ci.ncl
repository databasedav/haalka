let { runs_on, checkout, rust_toolchain, bevy_dependencies, cached_target_populate, just, cargo_build_env, .. } = import "common.ncl" in
let { uppercase_fields, sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
{
  name = "ci",

  on = {
    push = {
      branches = ["main"],
    },
    pull_request = {
      branches = ["main"],
    }
  },

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
          run = "just doctest"
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
        rust_toolchain & { with = { components = "rustfmt" } },
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
    }
  }
  |> sync_name_field
  |> std.record.map_values run_on_ubuntu_latest
}
