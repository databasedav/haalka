let UBUNTU_LATEST = "ubuntu-latest" in
{
  # TODO: this should be simpler ?
  uppercase_fields = fun record => record |> std.record.to_array |> std.array.map (fun record => record |> std.record.map (fun field value => if field == "field" then std.string.uppercase value else value)) |> std.record.from_array,
  sync_name_field = std.record.map (fun field value => value & { name = field }),
  run_on_ubuntu_latest = fun record => record & { runs-on = UBUNTU_LATEST },
  name_id = fun record => record & { id = record.name },
}
