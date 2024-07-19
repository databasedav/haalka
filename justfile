format:
    cargo +nightly fmt

doc *extras:
    cargo +nightly doc -Zunstable-options -Zrustdoc-scrape-examples {{ extras }}

doctest:
    cargo test --doc

example name *extras:
    cargo run --example {{ name }} {{ extras }}

release *extras:
    cargo release --unpublished --sign-tag {{ extras }}
