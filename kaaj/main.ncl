{
  build_example = import "src/build_example.ncl",
  ci = import "src/ci.ncl",
  examples_on_main = import "src/examples_on_main.ncl",
  pr_previews = import "src/pr_previews.ncl",
  reviewed_pr_preview = import "src/reviewed_pr_preview.ncl",
  deploy_reviewed_pr_preview = import "src/deploy_reviewed_pr_preview.ncl",
  cleanup_pr_previews = import "src/cleanup_pr_previews.ncl",
  release = import "src/release.ncl",
  wasm_example_index_template = import "src/wasm_example_index_template.ncl",
}
