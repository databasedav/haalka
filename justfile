KAAJ_HAALKA_COMMIT := "011daf1ae141dc0d1d10d00d4cfa0f065de60d13"

fetch_kaaj_justfile:
  curl https://raw.githubusercontent.com/databasedav/haalka/{{ KAAJ_HAALKA_COMMIT }}/kaaj/justfile > kaaj.just

import? 'kaaj.just'

# TODO: use an actual list https://github.com/casey/just/issues/2458
exclude_examples := '"accordion", "challenge07", "draggable", "many_buttons", "utils"'

# TODO: use an actual list https://github.com/casey/just/issues/2458
export_nickels := "ci build_example pr_previews examples_on_main cleanup_pr_previews release"

sync_counter_example_readme:
  uv run python sync_counter_example_readme.py

repo_prompt:
  @nickel eval repo_prompt.ncl | sed 's/^"//; s/"$//; s/\\"/"/g; s/\\n/\n/g'
