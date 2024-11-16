# TODO: add concurrency group
let { REPO, .. } = import "common.ncl" in
let { sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
{
  name = "preview_deploy",

  permissions = {
    contents = "write",
    pull-requests = "write"
  },

  on = {
    workflow_call = {
      inputs = {
        pr = {
          required = true,
          type = "number"
        },
        example = {
          required = true,
          type = "string"
        },
        parent_workflow_id = {
          required = true,
          type = "number"
        }
      }
    }
  },

  jobs = {
    deploy = {
      steps = [
        {
          name = "download dist",
          uses = "actions/download-artifact@v4",
          with = {
            name = "${{ inputs.example }}",
            path = "dist",
            github-token = "${{ secrets.GITHUB_TOKEN }}",
            run-id = "${{ inputs.parent_workflow_id }}",
          }
        },
        {
          name = "deploy",
          uses = "JamesIves/github-pages-deploy-action@v4",
          with = {
            folder = "dist",
            branch = "pr_previews",
            # double quoted because this must be a string https://github.com/JamesIves/github-pages-deploy-action/blob/3c4f14a9dd16589e133efef395ad4f3e79d1f277/src/constants.ts#L56 https://github.com/JamesIves/github-pages-deploy-action/blob/3c4f14a9dd16589e133efef395ad4f3e79d1f277/src/constants.ts#L144
            target-folder = m%""${{ inputs.pr }}/${{ inputs.example }}""%,
            commit-message = "%{ REPO }/pull/${{ inputs.pr }}/commits/${{ github.sha }}"
          }
        },
      ]
    }
  }
  |> sync_name_field
  |> std.record.map_values run_on_ubuntu_latest
}
