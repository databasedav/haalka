let { REPO, checkout, rust_toolchain, bevy_dependencies, cached_target_populate, just, binstall, .. } = import "common.ncl" in
let { sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
let GITHUB_PAGES_DEPLOY_ACTION = "4" in
let ATTEMPT_LIMIT = 1000 in
{
  name = "build_and_deploy_example",

  on = {
    workflow_call = {
      inputs = {
        pr = {
          required = false,
          type = "number"
        },
        api = {
          required = true,
          type = "string"
        },
        example = {
          required = true,
          type = "string"
        },
      }
    }
  },

  jobs = {
    build = {
      steps = [
        checkout,
        rust_toolchain & { with = { targets = "wasm32-unknown-unknown" } },
        bevy_dependencies,
        cached_target_populate,
        just,
        binstall,
        # TODO: just add to binstall once https://github.com/trunk-rs/trunk/pull/915 released
        {
          name = "install trunk",
          run = m%"
            wget https://github.com/databasedav/trunk/releases/download/v0.0.1/trunk
            chmod +x trunk
            mv trunk /usr/local/bin
          "%
        },
        {
          name = "install build dependencies",
          run = "cargo binstall --no-confirm wasm-bindgen-cli wasm-opt nickel-lang-cli",
        },
        {
          name = "build example",
          "if" = "${{ inputs.api == 'webgl2' }}",
          run = "just build_wasm_example ${{ inputs.example }}"
        },
        {
          name = "build example",
          "if" = "${{ inputs.api == 'webgpu' }}",
          run = "just build_wasm_example ${{ inputs.example }} webgpu"
        },
        {
          name = "upload dist",
          uses = "actions/upload-artifact@v4",
          with = {
            path = "dist",
            name = "${{ inputs.api }}_${{ inputs.example }}",
          }
        },
      ]
    },
    deploy = {
      needs = "build",
      env = {
        SUFFIX = "{{ inputs.api + '/' + inputs.example }}",
      },
      steps = [
        checkout,
        {
          name = "remove gitignore",
          run = "rm .gitignore"
        },
        {
          name = "download dist",
          uses = "actions/download-artifact@v4",
          with = {
            name = "${{ inputs.api }}_${{ inputs.example }}",
            path = "dist",
            github-token = "${{ secrets.GITHUB_TOKEN }}",
          }
        },
        {
          name = "deploy",
          uses = "JamesIves/github-pages-deploy-action@v%{ GITHUB_PAGES_DEPLOY_ACTION }",
          with = {
            folder = "dist",
            force = false,
            attempt-limit = ATTEMPT_LIMIT,
            target-folder = "${{ inputs.pr && 'pr_previews/' + inputs.pr + '/' + env.SUFFIX || 'examples/' + env.SUFFIX }}",
            commit-message = "${{ inputs.pr && '%{ REPO }' + '/pull/' + inputs.pr + '/commits/' + github.sha || '%{ REPO }' + '/commit/' + github.sha }}"
          }
        },
      ]
    }
  }
  |> sync_name_field
  |> std.record.map_values run_on_ubuntu_latest
}
