let { GITHUB_PAGES_DEPLOY_ACTION_VERSION, GITHUB_PAGES_DEPLOY_ACTION_ATTEMPT_LIMIT, checkout, .. } = import "common.ncl" in
let { sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
fun repo =>
  {
    name = "cleanup pr previews",

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
        cancel_deployments = {
          concurrency = {
            group = "pr-preview-${{ github.event.pull_request.number }}",
            cancel-in-progress = true,
          },
          permissions = {},
          steps = [
            {
              name = "cancel in-progress preview deployments",
              run = "true",
            },
          ]
        },
        cleanup = {
          needs = "cancel_deployments",
          concurrency = {
            group = "pr-preview-cleanup-${{ github.event.pull_request.number }}",
            cancel-in-progress = true,
          },
          permissions = {
            contents = "write",
          },
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
                attempt-limit = GITHUB_PAGES_DEPLOY_ACTION_ATTEMPT_LIMIT,
                target-folder = "pr_previews/${{ env.PR }}",
                commit-message = "cleaning up pr preview for %{ repo }/pull/${{ env.PR }}",
                single-commit = true,
              }
            },
          ]
        }
      }
      |> sync_name_field
      |> std.record.map_values run_on_ubuntu_latest
  }
