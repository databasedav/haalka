# kaaj [কাজ](https://translate.google.com/?sl=bn&tl=en&text=%E0%A6%95%E0%A6%BE%E0%A6%9C&op=translate)

```text
in bengali, kaaj means "work"
```

kaaj is both a nickel package that provides output targets for github actions workflows and an attached justfile for Bevy-related CI, including building examples, serving examples on the web, releasing crates, etc.

kaaj is used in [haalka](https://github.com/databasedav/haalka), [aalo](https://github.com/databasedav/aalo), and [jonmo](https://github.com/databasedav/jonmo)

## usage

add kaaj to your `Nickel-pkg.ncl`

```
dependencies = {
  kaaj = 'Git { url = "https://github.com/databasedav/haalka", ref = 'Tag "v0.1" },
}
```

then add the desired targets to your `.ncl`, for example

```ncl
let { build_example, ci, cleanup_pr_previews, deploy_reviewed_pr_preview, examples_on_main, pr_previews, release, reviewed_pr_preview, wasm_example_index_template, .. } = import kaaj in
let REPO = "https://github.com/databasedav/haalka" in
let GITHUB_PAGES_URL = "https://databasedav.github.io/haalka" in
{
    build_example_ = build_example "webgpu",
    ci_ = ci,
    cleanup_pr_previews_ = cleanup_pr_previews REPO,
    pr_previews_ = pr_previews,
    reviewed_pr_preview_ = reviewed_pr_preview,
    deploy_reviewed_pr_preview_ = deploy_reviewed_pr_preview REPO GITHUB_PAGES_URL,
    examples_on_main_ = examples_on_main REPO,
    release_ = release,
}

```

then in your `justfile` add the following, adjusting `exclude_examples` and `export_nickels` as desired

```just
KAAJ_HAALKA_COMMIT := "5091123d38b0a0eafd3b7d6b46350f6072748f0b"

fetch_kaaj_justfile:
  curl https://raw.githubusercontent.com/databasedav/haalka/{{ KAAJ_HAALKA_COMMIT }}/kaaj/justfile > kaaj.just

import? 'kaaj.just'

exclude_examples := '"example1", "example2"'

export_nickels := "ci build_example pr_previews reviewed_pr_preview deploy_reviewed_pr_preview examples_on_main cleanup_pr_previews release"
```

and finally, generate the github actions workflows with just
```
just sync_nickels
```

Pull request previews are deployed after a maintainer submits a `/deploy` comment review from the pull request's **Files changed → Review changes** dialog. This applies to both fork and same-repository pull requests. The review is bound to the exact reviewed commit; if the pull request changes before deployment, a new `/deploy` review is required. Submitting a newer `/deploy` review cancels the older preview build or deployment for that pull request. New commits also cancel older in-progress preview workflows for the same PR, and new pushes to `main` cancel older in-progress example deployments.
