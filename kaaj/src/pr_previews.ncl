let { get_examples, build_examples, assert_nickels_synced, .. } = import "common.ncl" in
let { sync_name_field, .. } = import "utils.ncl" in
let get_examples = get_examples & { needs = "assert_nickels_synced" } in
let build_examples = build_examples & { needs = "get_examples" } in
{
  name = "pr previews",

  on = "pull_request",

  concurrency = {
    group = "pr-preview-${{ github.event.number }}",
    cancel-in-progress = true,
  },

  jobs =
    { include [assert_nickels_synced, get_examples, build_examples] }
    |> sync_name_field
}
