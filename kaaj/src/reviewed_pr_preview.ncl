let { get_examples_for, build_examples_for, github_script, .. } = import "common.ncl" in
let { deploy_command, deploy_permission_helper, reviewed_preview_condition, .. } = import "pr_preview_common.ncl" in
let { sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
let authorize =
  {
    "if" = reviewed_preview_condition,
    permissions = {
      contents = "read",
      pull-requests = "read",
    },
    outputs = {
      pr = "${{ github.event.pull_request.number }}",
      ref = "${{ github.event.review.commit_id }}",
      repository = "${{ github.event.pull_request.head.repo.full_name }}",
    },
    steps = [
      github_script
      & {
        name = "verify reviewer permission",
        with = {
          script = m%"
          %{ deploy_permission_helper }
          if (!(await canDeployPreview(context.actor))) {
            core.setFailed(`${context.actor} does not have permission to deploy pull request previews.`);
          }
        "%
        }
      },
      {
        name = "write preview request",
        env = {
          PR = "${{ github.event.pull_request.number }}",
          REF = "${{ github.event.review.commit_id }}",
          REPOSITORY = "${{ github.event.pull_request.head.repo.full_name }}",
          REVIEW_ID = "${{ github.event.review.id }}",
          REVIEWER = "${{ github.event.review.user.login }}",
        },
        run = m%"
        jq -n \
          --argjson pr "$PR" \
          --arg ref "$REF" \
          --arg repository "$REPOSITORY" \
          --argjson review_id "$REVIEW_ID" \
          --arg reviewer "$REVIEWER" \
          '{pr: $pr, ref: $ref, repository: $repository, review_id: $review_id, reviewer: $reviewer}' \
          > preview-request.json
      "%
      },
      {
        name = "upload preview request",
        uses = "actions/upload-artifact@v4",
        with = {
          name = "preview-request",
          path = "preview-request.json",
          retention-days = 1,
        }
      },
    ]
  }
  |> run_on_ubuntu_latest
in
let get_examples =
  get_examples_for "${{ needs.authorize.outputs.repository }}" "${{ needs.authorize.outputs.ref }}"
  & {
    needs = "authorize",
    permissions = {
      contents = "read",
    },
  }
in
let build_examples =
  build_examples_for "${{ needs.authorize.outputs.repository }}" "${{ needs.authorize.outputs.ref }}"
  & {
    needs = ["authorize", "get_examples"],
    permissions = {
      contents = "read",
    },
  }
in
{
  name = "reviewed pr preview",

  on = {
    pull_request_review = {
      types = ["submitted"],
    },
  },

  concurrency = {
    group = "${{ github.event.review.body == '%{ deploy_command }' && format('pr-preview-{0}', github.event.pull_request.number) || format('non-deploy-review-{0}', github.run_id) }}",
    cancel-in-progress = true,
  },

  jobs =
    { include [authorize, get_examples, build_examples] }
    |> sync_name_field
}
