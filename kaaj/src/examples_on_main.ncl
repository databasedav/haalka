let { get_examples, build_examples, deploy_examples, assert_nickels_synced, .. } = import "common.ncl" in
let { sync_name_field, .. } = import "utils.ncl" in
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
      build_examples_ = build_examples & { needs = "get_examples_" },
      deploy_examples_ = deploy_examples "" & { needs = "build_examples_" },
    }
    |> sync_name_field
}
