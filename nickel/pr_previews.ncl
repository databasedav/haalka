let { REPO, checkout, rust_toolchain, bevy_dependencies, cached_target_populate, just, binstall, cargo_build_env, .. } = import "common.ncl" in
let { uppercase_fields, sync_name_field, run_on_ubuntu_latest, name_id, .. } = import "utils.ncl" in
{
  name = "pr_previews",

  on = ["pull_request"],

  jobs = {
    get_examples = {
      outputs = {
        examples = "${{ steps.output_examples.outputs.examples }}"
      },
      steps = [
        checkout,
        just,
        {
          name = "output_examples",
          run = m%"
            examples=$(just list_examples)
            echo "examples=$examples"
            echo "examples=$examples" >> $GITHUB_OUTPUT
          "%
        } |> name_id
      ]
    },
    build_examples = {
      needs = "get_examples",
      strategy = {
        matrix = {
          example = "${{ fromJson(needs.get_examples.outputs.examples) }}"
        }
      },
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
          run = "just build_wasm_example ${{ matrix.example }}"
        },
        {
          name = "upload dist",
          uses = "actions/upload-artifact@v4",
          with = {
            path = "dist",
            name = "${{ matrix.example }}",
          }
        },
      ]
    },
    deployment_comment = {
      needs = ["get_examples", "deploy_examples"],
      steps = [
        {
          name = "deployment_timestamp",
          run = m%"
            timestamp=$(date -u +'%Y-%m-%dT%H:%M:%SZ')
            echo "timestamp=$timestamp"
            echo "timestamp=$timestamp" >> $GITHUB_OUTPUT
          "%
        } |> name_id,
        {
          name = "links_markdown_list",
          run = m%"
            examples=${{ needs.get_examples.outputs.examples }}
            url_base="https://databasedav.github.io/haalka/${{ github.event.number }}/"
            markdown_list=""
            for example in $(echo "${examples}" | jq -r '.[]'); do
              markdown_list+="\n- [$example](${url_base}${example})"
            done
            echo "markdown_list=$markdown_list" >> $GITHUB_OUTPUT
          "%
        } |> name_id,
        {
          uses = "thollander/actions-comment-pull-request@v2",
          with = {
            message = m%"
              **previews:**
              ${{ steps.links_markdown_list.outputs.markdown_list }}
              **commit:** %{ REPO }/pull/${{ github.event.number }}/commits/${{ github.sha }}
              **deployed:** ${{ steps.deployment_timestamp.outputs.timestamp }}
            "%,
            pr_number = "${{ github.event.number }}",
            comment_tag = "preview"
          }
        }
      ]
    }
  }
  |> std.record.map_values run_on_ubuntu_latest
  & {  # can't use runs-on for a workflow_call job
    deploy_examples = {
      needs = ["get_examples", "build_examples"],
      permissions = {
        contents = "write",
        pull-requests = "write"
      },
      strategy = {
        matrix = {
          example = "${{ fromJson(needs.get_examples.outputs.examples) }}"
        }
      },
      uses = "./.github/workflows/preview_deploy.yaml",
      with = {
        pr = "${{ github.event.number }}",
        example = "${{ matrix.example }}",
        parent_workflow_id = "${{ github.run_id }}"
      }
    }
  }
  |> sync_name_field
}
