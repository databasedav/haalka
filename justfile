# TODO: this prolly doesn't work on windows
example name *extras:
    {{ if path_exists("examples/" + name + ".rs") == "true" { "cargo run --example " + name } else { "cd examples/" + name + " && cargo run" } }} {{ extras }}
