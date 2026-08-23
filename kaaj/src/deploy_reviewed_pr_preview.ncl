let { checkout, github_script, GITHUB_PAGES_DEPLOY_ACTION_VERSION, GITHUB_PAGES_DEPLOY_ACTION_ATTEMPT_LIMIT, .. } = import "common.ncl" in
let { active_deploy_review_helper, .. } = import "pr_preview_common.ncl" in
let { discover_request, validate_request, .. } = import "pr_preview_scripts.ncl" in
let { sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
fun repo github_pages_url =>
  {
    name = "deploy reviewed pr preview",

    on = {
      workflow_run = {
        workflows = ["reviewed pr preview"],
        types = ["completed"],
      },
    },

    jobs =
      let discover_request =
        {
          "if" = "github.event.workflow_run.conclusion == 'success'",
          permissions = {
            actions = "read",
          },
          outputs = {
            artifact_ids = "${{ steps.discover.outputs.artifact_ids }}",
            requested = "${{ steps.discover.outputs.requested }}",
          },
          steps = [
            github_script
            & {
              id = "discover",
              name = "discover preview request",
              with = {
                script = discover_request
              }
            },
          ]
        }
        |> run_on_ubuntu_latest
      in
      let validate_request =
        {
          "if" = "needs.discover_request.outputs.requested == 'true'",
          needs = "discover_request",
          permissions = {
            actions = "read",
            contents = "read",
            pull-requests = "read",
          },
          outputs = {
            examples = "${{ steps.validate.outputs.examples }}",
            pr = "${{ steps.validate.outputs.pr }}",
            ref = "${{ steps.validate.outputs.ref }}",
            review_id = "${{ steps.validate.outputs.review_id }}",
          },
          steps = [
            {
              name = "download reviewed preview artifacts",
              uses = "actions/download-artifact@v4",
              with = {
                artifact-ids = "${{ needs.discover_request.outputs.artifact_ids }}",
                path = "artifacts",
                run-id = "${{ github.event.workflow_run.id }}",
                github-token = "${{ secrets.GITHUB_TOKEN }}",
              }
            },
            github_script
            & {
              id = "validate",
              name = "validate preview request",
              with = {
                script = validate_request
              }
            },
            {
              name = "organize preview artifacts",
              run = m%"
                set -euo pipefail
                if find artifacts -type l -print -quit | grep -q .; then
                  echo 'Preview artifacts must not contain symbolic links.' >&2
                  exit 1
                fi
                mkdir pages
                for artifact in artifacts/webgl2_* artifacts/webgpu_*; do
                  api_and_example=$(basename "$artifact")
                  api=${api_and_example%%_*}
                  example=${api_and_example#*_}
                  mkdir -p "pages/$api/$example"
                  cp -R "$artifact"/. "pages/$api/$example"
                done
              "%
            },
            {
              name = "upload validated preview",
              uses = "actions/upload-artifact@v4",
              with = {
                name = "reviewed-preview-pages",
                path = "pages",
                retention-days = 1,
              }
            },
          ]
        }
        |> run_on_ubuntu_latest
      in
      let deploy =
        {
          needs = "validate_request",
          concurrency = {
            group = "pr-preview-${{ needs.validate_request.outputs.pr }}",
            cancel-in-progress = false,
          },
          permissions = {
            actions = "read",
            contents = "write",
            pull-requests = "read",
          },
          steps = [
            checkout,
            {
              name = "remove gitignore",
              run = "rm -f .gitignore"
            },
            {
              name = "download validated preview",
              uses = "actions/download-artifact@v4",
              with = {
                name = "reviewed-preview-pages",
                path = "pages",
              }
            },
            github_script
            & {
              name = "verify preview request is still current",
              env = {
                PR = "${{ needs.validate_request.outputs.pr }}",
                REF = "${{ needs.validate_request.outputs.ref }}",
                REVIEW_ID = "${{ needs.validate_request.outputs.review_id }}",
              },
              with = {
                script = m%"
                  %{ active_deploy_review_helper }
                  const pullNumber = Number(process.env.PR);
                  const reviewId = Number(process.env.REVIEW_ID);
                  const { data: pullRequest } = await github.rest.pulls.get({
                    owner: context.repo.owner,
                    repo: context.repo.repo,
                    pull_number: pullNumber,
                  });
                  const { data: review } = await github.rest.pulls.getReview({
                    owner: context.repo.owner,
                    repo: context.repo.repo,
                    pull_number: pullNumber,
                    review_id: reviewId,
                  });
                  if (pullRequest.state !== 'open' || pullRequest.head.sha !== process.env.REF) {
                    core.setFailed('The pull request changed or closed before deployment. Submit a new /deploy review.');
                    return;
                  }
                  if (!isActiveDeployReview(review, process.env.REF)) {
                    core.setFailed('The /deploy review is no longer valid.');
                  }
                "%
              }
            },
            {
              name = "deploy",
              uses = "JamesIves/github-pages-deploy-action@v%{ GITHUB_PAGES_DEPLOY_ACTION_VERSION }",
              with = {
                folder = "pages",
                attempt-limit = GITHUB_PAGES_DEPLOY_ACTION_ATTEMPT_LIMIT,
                target-folder = "pr_previews/${{ needs.validate_request.outputs.pr }}",
                commit-message = "%{ repo }/pull/${{ needs.validate_request.outputs.pr }}/commits/${{ needs.validate_request.outputs.ref }}",
                single-commit = true,
              }
            },
          ]
        }
        |> run_on_ubuntu_latest
      in
      let deployment_comment =
        {
          needs = ["validate_request", "deploy"],
          permissions = {
            pull-requests = "write",
          },
          steps = [
            {
              id = "deployment_timestamp",
              name = "record deployment timestamp",
              run = m%"
                timestamp=$(date -u +'%Y-%m-%dT%H:%M:%SZ')
                echo "timestamp=$timestamp"
                echo "timestamp=$timestamp" >> $GITHUB_OUTPUT
              "%
            },
            {
              id = "links_markdown_list",
              name = "build preview links",
              env = {
                EXAMPLES = "${{ needs.validate_request.outputs.examples }}",
              },
              run = m%"
                base_url="%{ github_pages_url }/pr_previews/${{ needs.validate_request.outputs.pr }}"
                markdown_list=""
                for example in $(echo "$EXAMPLES" | jq -r '.[]'); do
                  markdown_list="${markdown_list}- $example: [webgl2](${base_url}/webgl2/${example}) [webgpu](${base_url}/webgpu/${example})"$'\n'
                done

                {
                  echo "markdown_list<<EOF"
                  echo "$markdown_list"
                  echo "EOF"
                } >> $GITHUB_OUTPUT
              "%
            },
            {
              uses = "thollander/actions-comment-pull-request@v2",
              with = {
                message = m%"
                  **commit:** %{ repo }/pull/${{ needs.validate_request.outputs.pr }}/commits/${{ needs.validate_request.outputs.ref }}
                  **deployed:** ${{ steps.deployment_timestamp.outputs.timestamp }}
                  **previews:**
                  ${{ steps.links_markdown_list.outputs.markdown_list }}
                "%,
                pr_number = "${{ needs.validate_request.outputs.pr }}",
                comment_tag = "preview",
                mode = "recreate",
              }
            },
          ]
        }
        |> run_on_ubuntu_latest
      in
      { include [discover_request, validate_request, deploy, deployment_comment] }
      |> sync_name_field
  }
