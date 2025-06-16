import? 'kaaj.just'

fetch_kaaj_justfile:
  # curl https://raw.githubusercontent.com/databasedav/haalka/main/justfile > haalka.just
  cp kaaj/justfile kaaj.just

# TODO: use an actual list https://github.com/casey/just/issues/2458
exclude_examples := '"accordion", "challenge07", "draggable", "many_buttons", "utils"'

# TODO: use an actual list https://github.com/casey/just/issues/2458
export_nickels := "ci build_example pr_previews examples_on_main cleanup_pr_previews release"

sync_counter_example_readme:
  uv run python sync_counter_example_readme.py
