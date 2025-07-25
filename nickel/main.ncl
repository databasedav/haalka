let { build_example, ci, cleanup_pr_previews, examples_on_main, pr_previews, release, wasm_example_index_template, .. } = import kaaj in
let REPO = "https://github.com/databasedav/haalka" in
let GITHUB_PAGES_URL = "https://databasedav.github.io/haalka" in
{
    build_example_ = build_example true "bevy/webgpu",
    ci_ = ci,
    cleanup_pr_previews_ = cleanup_pr_previews REPO,
    pr_previews_ = pr_previews REPO GITHUB_PAGES_URL,
    examples_on_main_ = examples_on_main REPO,
    release_ = release,
}
