let uppercase_fields = fun record => record |> std.record.to_array |> std.array.map (fun record => record |> std.record.map (fun field value => if field == "field" then std.string.uppercase value else value)) |> std.record.from_array
# std.array.map (fun record std.record.map (fun field value => if field == "field" then std.string.uppercase value else value))
in
let test = {
  shit = "fuck"
} in
{
  env = uppercase_fields test
}
