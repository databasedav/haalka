let { get_examples, build_examples, deploy_examples, assert_nickels_synced, .. } = import "common.ncl" in
let { sync_name_field, .. } = import "utils.ncl" in
fun repo =>
  let get_examples = get_examples & { needs = "assert_nickels_synced" } in
  let build_examples = build_examples & { needs = "get_examples" } in
  let deploy_examples = deploy_examples repo & { needs = "build_examples" } in
  {
    name = "examples on main",

    on = {
      push = {
        branches = ["main"],
      },
    },

    concurrency = {
      group = "examples-on-main",
      cancel-in-progress = true,
    },

    jobs =
      { include [assert_nickels_synced, get_examples, build_examples, deploy_examples] }
      |> sync_name_field
  }
