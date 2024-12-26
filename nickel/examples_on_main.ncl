let { REPO, checkout, cargo_build_env, just, get_examples, build_and_deploy_examples, .. } = import "common.ncl" in
let { sync_name_field, run_on_ubuntu_latest, name_id, .. } = import "utils.ncl" in
{
  name = "examples_on_main",

  on = {
    push = {
      branches = ["main"],
    },
  },

  jobs = { 
    # TODO: file nickel issue, field and value matching should not cause recursion
    get_examples_ = get_examples,
    build_and_deploy_examples_ = build_and_deploy_examples "",
  }
  |> sync_name_field
}
