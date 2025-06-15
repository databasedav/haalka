let { REPO, GITHUB_PAGES_DEPLOY_ACTION_VERSION, GITHUB_PAGES_DEPLOY_ACTION_ATTEMPT_LIMIT, checkout, .. } = import "common.ncl" in
let { sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
{
  name = "cleanup_pr_previews",

  permissions = {
    contents = "write"
  },

  on = {
    pull_request_target = {
      types = ["closed"],
    }
  },

  env = {
    PR = "${{ github.event.pull_request.number }}"
  },

  jobs =
    {
      cleanup = {
        steps = [
          checkout,
          {
            name = "make empty dir",
            run = "mkdir -p empty_dir"
          },
          {
            name = "cleanup",
            uses = "JamesIves/github-pages-deploy-action@v%{ GITHUB_PAGES_DEPLOY_ACTION_VERSION }",
            with = {
              folder = "empty_dir",
              force = false,
              attempt-limit = GITHUB_PAGES_DEPLOY_ACTION_ATTEMPT_LIMIT,
              target-folder = "pr_previews/${{ env.PR }}",
              commit-message = "cleaning up pr preview for %{ REPO }/pull/${{ env.PR }}",
            }
          },
        ]
      }
    }
    |> sync_name_field
    |> std.record.map_values run_on_ubuntu_latest
}
