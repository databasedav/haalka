let { REPO, checkout, cargo_build_env, just, get_examples, build_and_deploy_examples, .. } = import "common.ncl" in
let { sync_name_field, run_on_ubuntu_latest, name_id, .. } = import "utils.ncl" in
{
  name = "pr_previews",

  on = "pull_request",

  jobs = {
    # TODO: file nickel issue, field and value matching should not cause recursion
    get_examples_ = get_examples,
    build_and_deploy_examples_ = build_and_deploy_examples "${{ github.event.number }}",
    deployment_comment = {
      needs = ["get_examples_", "build_and_deploy_examples_"],
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
            examples='${{ needs.get_examples_.outputs.examples }}'
            base_url="https://databasedav.github.io/haalka/pr_previews/${{ github.event.number }}"
            markdown_list=""
            for example in $(echo "${examples}" | jq -r '.[]'); do
              markdown_list="${markdown_list}- $example: [webgl2](${base_url}/webgl2/${example}) [webgpu](${base_url}/webgpu/${example})"$'\n'
            done
            echo "markdown_list=$markdown_list"
            echo "markdown_list=$markdown_list" >> $GITHUB_OUTPUT
          "%
        } |> name_id,
        {
          uses = "thollander/actions-comment-pull-request@v2",
          with = {
            message = m%"
              **commit:** %{ REPO }/pull/${{ github.event.number }}/commits/${{ github.event.pull_request.head.sha }}
              **deployed:** ${{ steps.deployment_timestamp.outputs.timestamp }}
              **previews:**
              ${{ steps.links_markdown_list.outputs.markdown_list }}
            "%,
            pr_number = "${{ github.event.number }}",
            comment_tag = "preview"
          }
        }
      ]
    } |> run_on_ubuntu_latest
  }
  |> sync_name_field
}