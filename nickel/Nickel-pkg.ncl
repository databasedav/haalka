{
  name = "haalka_ci",
  version = "0.1.0",
  authors = ["databasedav <31483365+databasedav@users.noreply.github.com>"],
  minimal_nickel_version = "1.12.2",
  description = "github actions ci generation for haalka",
  dependencies = {
    # kaaj = 'Git { url = "..", path = "kaaj" },
    kaaj = 'Path "../kaaj",
  }
} | std.package.Manifest
