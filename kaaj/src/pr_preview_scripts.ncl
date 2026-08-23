let { active_deploy_review_helper, deploy_permission_helper, .. } = import "pr_preview_common.ncl" in
{
  discover_request = m%"
  const MAX_PREVIEW_ARTIFACTS = 100;
  const MAX_TOTAL_ARTIFACT_BYTES = 2 * 1024 * 1024 * 1024;
  const PREVIEW_ARTIFACT_PATTERN = /^(webgl2|webgpu)_[A-Za-z0-9][A-Za-z0-9_-]*$/;

  function requireCondition(condition, message) {
    if (!condition) throw new Error(message);
  }

  async function listActiveArtifacts(runId) {
    const artifacts = await github.paginate(
      github.rest.actions.listWorkflowRunArtifacts,
      {
        owner: context.repo.owner,
        repo: context.repo.repo,
        run_id: runId,
        per_page: 100,
      },
      response => response.data.artifacts,
    );
    return artifacts.filter(artifact => !artifact.expired);
  }

  function validateArtifacts(artifacts) {
    const requestArtifacts = artifacts.filter(artifact => artifact.name === 'preview-request');
    if (requestArtifacts.length === 0) return null;

    requireCondition(
      requestArtifacts.length === 1,
      'Expected exactly one preview-request artifact.',
    );

    const previewArtifacts = artifacts.filter(artifact => artifact.name !== 'preview-request');
    const unexpectedArtifact = previewArtifacts.find(
      artifact => !PREVIEW_ARTIFACT_PATTERN.test(artifact.name),
    );
    requireCondition(
      !unexpectedArtifact,
      `Unexpected preview artifact name: ${unexpectedArtifact?.name}`,
    );
    requireCondition(
      previewArtifacts.length > 0 && previewArtifacts.length <= MAX_PREVIEW_ARTIFACTS,
      'The preview artifact count is outside the allowed range.',
    );

    const totalSize = artifacts.reduce((sum, artifact) => sum + artifact.size_in_bytes, 0);
    requireCondition(
      totalSize <= MAX_TOTAL_ARTIFACT_BYTES,
      'The preview artifacts exceed the 2 GiB size limit.',
    );

    return artifacts.map(artifact => artifact.id).join(',');
  }

  async function discoverRequest() {
    const artifacts = await listActiveArtifacts(context.payload.workflow_run.id);
    const artifactIds = validateArtifacts(artifacts);
    if (artifactIds === null) {
      core.setOutput('requested', 'false');
      return;
    }

    core.setOutput('artifact_ids', artifactIds);
    core.setOutput('requested', 'true');
  }

  await discoverRequest();
"%,
  validate_request = m%"
  %{ active_deploy_review_helper }
  %{ deploy_permission_helper }
  const fs = require('fs');
  const COMMIT_SHA_PATTERN = /^[0-9a-f]{40}$/;
  const PREVIEW_ARTIFACT_PATTERN = /^(webgl2|webgpu)_([A-Za-z0-9][A-Za-z0-9_-]*)$/;
  const REQUEST_PATH = 'artifacts/preview-request/preview-request.json';
  const REVIEWED_WORKFLOW_PATH = '.github/workflows/reviewed_pr_preview.yaml';

  function requireCondition(condition, message) {
    if (!condition) throw new Error(message);
  }

  function readPreviewRequest() {
    return JSON.parse(fs.readFileSync(REQUEST_PATH, 'utf8'));
  }

  function validateRequestShape(request) {
    requireCondition(
      Number.isInteger(request.pr) && request.pr > 0,
      'Invalid pull request number in preview request.',
    );
    requireCondition(
      Number.isInteger(request.review_id) && request.review_id > 0,
      'Invalid review ID in preview request.',
    );
    requireCondition(
      COMMIT_SHA_PATTERN.test(request.ref),
      'Invalid commit SHA in preview request.',
    );
  }

  function validateWorkflowRun(workflowRun, request) {
    requireCondition(
      workflowRun.event === 'pull_request_review',
      'Preview request did not originate from a pull request review.',
    );
    requireCondition(
      workflowRun.path === REVIEWED_WORKFLOW_PATH,
      'Preview request originated from an unexpected workflow path.',
    );
    requireCondition(
      workflowRun.head_sha === request.ref
        && workflowRun.head_repository?.full_name === request.repository,
      'The preview request does not match the triggering workflow head.',
    );

    const associatedPullRequests = workflowRun.pull_requests ?? [];
    requireCondition(
      associatedPullRequests.length <= 1
        && (associatedPullRequests.length === 0 || associatedPullRequests[0].number === request.pr),
      'The preview request does not match the triggering workflow pull request.',
    );
  }

  async function validateSourceWorkflow(workflowRun) {
    const { data: sourceWorkflow } = await github.rest.actions.getWorkflow({
      owner: context.repo.owner,
      repo: context.repo.repo,
      workflow_id: REVIEWED_WORKFLOW_PATH,
    });
    requireCondition(
      workflowRun.workflow_id === sourceWorkflow.id,
      'Preview request originated from an unexpected workflow.',
    );
  }

  async function fetchPullRequestAndReview(request) {
    const [{ data: pullRequest }, { data: review }] = await Promise.all([
      github.rest.pulls.get({
        owner: context.repo.owner,
        repo: context.repo.repo,
        pull_number: request.pr,
      }),
      github.rest.pulls.getReview({
        owner: context.repo.owner,
        repo: context.repo.repo,
        pull_number: request.pr,
        review_id: request.review_id,
      }),
    ]);
    return { pullRequest, review };
  }

  function validatePullRequest(pullRequest, workflowRun, request) {
    requireCondition(
      pullRequest.state === 'open',
      'The pull request is no longer open.',
    );
    requireCondition(
      pullRequest.head.repo?.full_name === request.repository
        && pullRequest.head.ref === workflowRun.head_branch,
      'The pull request source does not match the triggering workflow.',
    );
    requireCondition(
      pullRequest.head.sha === request.ref,
      'The pull request has changed since /deploy was submitted. Submit a new /deploy review.',
    );
  }

  function validateReview(review, workflowRun, request) {
    requireCondition(
      review.commit_id === request.ref,
      'The pull request has changed since /deploy was submitted. Submit a new /deploy review.',
    );
    requireCondition(
      isActiveDeployReview(review, request.ref),
      'The referenced review is not an active /deploy comment review.',
    );
    requireCondition(
      review.user.login === request.reviewer
        && workflowRun.actor.login === request.reviewer,
      'The preview request reviewer does not match the workflow actor.',
    );
  }

  function collectExamples() {
    const artifactNames = fs.readdirSync('artifacts', { withFileTypes: true })
      .filter(entry => entry.isDirectory() && entry.name !== 'preview-request')
      .map(entry => entry.name);
    const examples = new Map();

    for (const artifactName of artifactNames) {
      const match = PREVIEW_ARTIFACT_PATTERN.exec(artifactName);
      requireCondition(Boolean(match), `Unexpected preview artifact name: ${artifactName}`);
      const [, api, example] = match;
      if (!examples.has(example)) examples.set(example, new Set());
      examples.get(example).add(api);
    }

    requireCondition(examples.size > 0, 'No preview artifacts were produced.');
    for (const [example, apis] of examples) {
      requireCondition(
        apis.has('webgl2') && apis.has('webgpu'),
        `Preview artifacts are incomplete for ${example}.`,
      );
    }

    return [...examples.keys()].sort();
  }

  async function validateRequest() {
    const request = readPreviewRequest();
    const workflowRun = context.payload.workflow_run;
    validateRequestShape(request);
    validateWorkflowRun(workflowRun, request);
    await validateSourceWorkflow(workflowRun);

    const { pullRequest, review } = await fetchPullRequestAndReview(request);
    validatePullRequest(pullRequest, workflowRun, request);
    validateReview(review, workflowRun, request);
    requireCondition(
      await canDeployPreview(request.reviewer),
      `${request.reviewer} no longer has permission to deploy pull request previews.`,
    );

    core.setOutput('examples', JSON.stringify(collectExamples()));
    core.setOutput('pr', String(request.pr));
    core.setOutput('ref', request.ref);
    core.setOutput('review_id', String(request.review_id));
  }

  await validateRequest();
"%,
}
