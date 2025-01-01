let { REPO, checkout, cargo_build_env, just, get_examples, build_and_deploy_examples, assert_nickels_synced, .. } = import "common.ncl" in
let { sync_name_field, run_on_ubuntu_latest, name_id, .. } = import "utils.ncl" in
{
  name = "examples_on_main",

  on = {
    push = {
      branches = ["main"],
    },
  },

  jobs =
    {
      # TODO https://github.com/tweag/nickel/issues/747
      assert_nickels_synced_ = assert_nickels_synced,
      get_examples_ = get_examples & { needs = "assert_nickels_synced_" },
      build_and_deploy_examples_ = build_and_deploy_examples "" & { needs = "get_examples_" },
    }
    |> sync_name_field
}
