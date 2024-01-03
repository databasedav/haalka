example name:
    {{ if path_exists("examples/" + name + ".rs") == "true" { "cargo run --example " + name } else { "cd examples/" + name + " && cargo run" } }}
