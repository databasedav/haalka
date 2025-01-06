let { checkout, rust_toolchain, bevy_dependencies, cached_target_populate, cargo_build_env, .. } = import "common.ncl" in
let { uppercase_fields, sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
let RELEASE_PLZ_ACTION_VERSION = "0.5" in
{
  name = "release",

  permissions = {
    pull-requests = "write",
    contents = "write"
  },

  on = {
    pull_request = {
      types = ["closed"],
      branches = ["[0-9]+.[0-9]+"],
    }
  },

  env = cargo_build_env |> uppercase_fields,

  jobs =
    {
      release = {
        "if" = "${{ github.event.pull_request.merged == true }}",
        steps = [
          checkout,
          rust_toolchain,
          bevy_dependencies,
          cached_target_populate,
          {
            name = "run release-plz",
            uses = "MarcoIeni/release-plz-action@v%{ RELEASE_PLZ_ACTION_VERSION }",
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
