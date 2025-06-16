let { checkout, rust_toolchain, bevy_dependencies, cached_target_populate, just, binstall, .. } = import "common.ncl" in
let { sync_name_field, run_on_ubuntu_latest, .. } = import "utils.ncl" in
fun build_wasm_webgpu_features => {
  name = "build_example",

  on = {
    workflow_call = {
      inputs = {
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

  jobs =
    {
      build = {
        steps = [
          checkout,
          rust_toolchain & { with = { targets = "wasm32-unknown-unknown" } },
          bevy_dependencies,
          cached_target_populate,
          just,
          binstall,
          {
            name = "install build dependencies",
            run = "cargo binstall --no-confirm --continue-on-failure wasm-bindgen-cli wasm-opt nickel-lang-cli trunk",
          },
          {
            name = "build example",
            "if" = "${{ inputs.api == 'webgl2' }}",
            run = "just build_wasm_example ${{ inputs.example }}"
          },
          {
            name = "build example",
            "if" = "${{ inputs.api == 'webgpu' }}",
            run = "just build_wasm_example ${{ inputs.example }} %{ build_wasm_webgpu_features }"
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
      }
    }
    |> sync_name_field
    |> std.record.map_values run_on_ubuntu_latest
}
