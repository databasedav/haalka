let DEPLOY_COMMAND = "/deploy" in
{
  deploy_command = DEPLOY_COMMAND,
  reviewed_preview_condition = "github.event.review.body == '%{ DEPLOY_COMMAND }' && github.event.review.state == 'commented' && github.event.review.user.login == github.actor && github.event.review.commit_id == github.event.pull_request.head.sha",
  active_deploy_review_helper = m%"
  function isActiveDeployReview(review, ref) {
    return review.commit_id === ref
      && review.body?.trim() === '%{ DEPLOY_COMMAND }'
      && review.state?.toLowerCase() === 'commented';
  }
"%,
  deploy_permission_helper = m%"
  async function canDeployPreview(username) {
    const { data } = await github.rest.repos.getCollaboratorPermissionLevel({
      owner: context.repo.owner,
      repo: context.repo.repo,
      username,
    });
    return ['admin', 'write'].includes(data.user.permission)
      || ['admin', 'maintain', 'write'].includes(data.user.role_name);
  }
"%,
}
