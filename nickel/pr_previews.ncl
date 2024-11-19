let { REPO, checkout, cargo_build_env, just, .. } = import "common.ncl" in
let { uppercase_fields, sync_name_field, run_on_ubuntu_latest, name_id, .. } = import "utils.ncl" in
{
  name = "pr_previews",

  on = ["pull_request"],

  jobs = {
    get_examples = {
      outputs = {
        examples = "${{ steps.output_examples.outputs.examples }}"
      },
      steps = [
        checkout,
        just,
        {
          name = "output_examples",
          run = m%"
            examples=$(just list_examples)
            echo "examples=$examples"
            echo "examples=$examples" >> $GITHUB_OUTPUT
          "%
        } |> name_id
      ]
    },
    deployment_comment = {
      needs = ["get_examples", "build_and_deploy_examples"],
      permissions = {
        pull-requests = "write",
      },
      steps = [
        {
          name = "deployment_timestamp",
          run = m%"
            timestamp=$(date -u +'%Y-%m-%dT%H:%M:%SZ')
            echo "timestamp=$timestamp"
            echo "timestamp=$timestamp" >> $GITHUB_OUTPUT
          "%
        } |> name_id,
        {
          name = "links_markdown_list",
          run = m%"
            examples='${{ needs.get_examples.outputs.examples }}'
            base_url="https://databasedav.github.io/haalka/pr_previews/${{ github.event.number }}"
            markdown_list=""
            for example in $(echo "${examples}" | jq -r '.[]'); do
              markdown_list+="\n- $example: [webgl2](${base_url}/webgl2/${example}) [webgpu](${base_url}/webgpu/${example})"
            done
            echo "markdown_list=$markdown_list"
            echo "markdown_list=$markdown_list" >> $GITHUB_OUTPUT
          "%
        } |> name_id,
        {
          uses = "thollander/actions-comment-pull-request@v2",
          with = {
            message = m%"
              **previews:**
              ${{ steps.links_markdown_list.outputs.markdown_list }}
              **commit:** %{ REPO }/pull/${{ github.event.number }}/commits/${{ github.event.pull_request.head.sha }}
              **deployed:** ${{ steps.deployment_timestamp.outputs.timestamp }}
            "%,
            pr_number = "${{ github.event.number }}",
            comment_tag = "preview"
          }
        }
      ]
    }
  }
  |> std.record.map_values run_on_ubuntu_latest
  & {  # can't use runs-on for a reusable workflow
    build_and_deploy_examples = {
      needs = "get_examples",
      strategy = {
        matrix = {
          example = "${{ fromJson(needs.get_examples.outputs.examples) }}",
          api = ["webgl2", "webgpu"],
        }
      },
      permissions = {
        contents = "write",
      },
      uses = "./.github/workflows/preview_build_deploy.yaml",
      with = {
        pr = "${{ github.event.number }}",
        api = "${{ matrix.api }}",
        example = "${{ matrix.example }}",
        parent_workflow_id = "${{ github.run_id }}"
      },
    }
  }
  |> sync_name_field
}
