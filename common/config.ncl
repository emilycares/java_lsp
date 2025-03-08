let data = {
  formatter = "Topiary"
} in
let Schema = {
  formatter
    | std.enum.TagOrString
    | [| 'Topiary, 'Intelij, 'None |],
} in
data | Schema
