format *extras:
  cargo +nightly fmt {{ extras }}

doc *extras:
  cargo +nightly doc --all-features --no-deps -Zunstable-options -Zrustdoc-scrape-examples --locked {{ extras }}

doctest:
  cargo test --doc --all-features --locked

# TODO: unit tests
test: doctest

clippy *extras:
  cargo clippy --all-features --all-targets --locked -- --deny warnings {{ extras }}

# TODO: --all-features flag doesn't work because examples shenanigans
check_all_features:
  cargo check-all-features --locked

example name *extras:
  cargo run --example {{ name }} {{ extras }}

# TODO: use inline module to make this cleaner https://github.com/casey/just/issues/2442
example_wasm name *extras:
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner cargo run --target wasm32-unknown-unknown --example {{ name }} {{ extras }}

release *extras:
  cargo release --unpublished --sign-tag {{ extras }}

# TODO: figure out y ci doesn't do this
sign_tag tag:
  GIT_COMMITTER_DATE="$(git log -1 --format=%aD {{ tag }})" git tag {{ tag }} {{ tag }} -f -s && git push --tags --force

sync_counter_example_readme:
  python sync_counter_example_readme.py

# TODO: use an actual list https://github.com/casey/just/issues/2458
exclude_examples := '"accordion", "challenge07", "draggable", "many_buttons", "utils"'

list_examples:
  @cargo metadata --no-deps --format-version 1 | jq -c --argjson exclude '[{{ exclude_examples }}]' '[.packages[].targets[] | select(.kind[] == "example" and (.name as $name | $exclude | index($name) | not)) | .name]'

generate_wasm_example_index pr example:
  nickel eval nickel/wasm_example_index_template.ncl --field index_html -- 'pr={{ pr }}' 'example="{{ example }}"' | sed 's/^"//; s/"$//; s/\\"/"/g; s/\\n/\n/g' > index.html

build_wasm_example example:
  just generate_wasm_example_index {{ example }}
  trunk build --locked --release --example {{ example }}

export_nickel file:
  nickel export --format yaml nickel/{{ file }}.ncl

# TODO: use an actual list https://github.com/casey/just/issues/2458
export_nickels := "ci pr_previews preview_deploy release"

# TODO: use https://github.com/rhysd/actionlint after
sync_nickels:
  @for nickel in {{ export_nickels }}; do \
    echo "# generated by nickel/$nickel.ncl; do not manually edit" > ./.github/workflows/$nickel.yaml; \
    just export_nickel $nickel >> ./.github/workflows/$nickel.yaml; \
  done
