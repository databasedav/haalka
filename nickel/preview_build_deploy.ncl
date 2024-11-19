let { REPO, checkout, rust_toolchain, bevy_dependencies, cached_target_populate, just, binstall, .. } = import "common.ncl" in
let { sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
{
  name = "preview_build_deploy",

  on = {
    workflow_call = {
      inputs = {
        pr = {
          required = true,
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
          uses = "databasedav/github-pages-deploy-action@ec82a366a3ef75d20483861daeedcdb2c09a3fd5",
          with = {
            folder = "dist",
            force = false,
            attempt-limit = 1000,
            # double quoted because this must be a string https://github.com/JamesIves/github-pages-deploy-action/blob/3c4f14a9dd16589e133efef395ad4f3e79d1f277/src/constants.ts#L56 https://github.com/JamesIves/github-pages-deploy-action/blob/3c4f14a9dd16589e133efef395ad4f3e79d1f277/src/constants.ts#L144
            # TODO: figure out webgl2 compile shenanigans and add to matrix
            target-folder = m%""pr_previews/${{ inputs.pr }}/${{ inputs.api }}/${{ inputs.example }}""%,
            commit-message = "%{ REPO }/pull/${{ inputs.pr }}/commits/${{ github.sha }}"
          }
        },
      ]
    }
  }
  |> sync_name_field
  |> std.record.map_values run_on_ubuntu_latest
}
