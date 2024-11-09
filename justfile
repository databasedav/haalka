format:
    cargo +nightly fmt

doc *extras:
    cargo +nightly doc -Zunstable-options -Zrustdoc-scrape-examples -Fdebug {{ extras }}

doctest:
    cargo test --doc

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
