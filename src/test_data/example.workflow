var_1 = variable(
  default =  "some default",
  readers =  ["foo", "bar"],
  writers =  ["foo", "bar"],
  cli_flag =  "--foo",
  env =  "VAR_ONE",
)

var_2 = variable(
  default =  "some default",
  readers =  ["foo", "bar"],
  writers =  ["foo", "bar"],
  env =  "VAR_TWO",
)

var_3 = variable(
)

#builtin_tool(
#  name = "echo",
#)
foo = tool(
  path = "foo.sh",
)

b = "bar.sh"
bar = tool(
  path = format("{}", b)
)

HOME = variable(
  env =  "HOME",
)

orchestral = tool(
  path = format("{}/bin/orchestral", HOME),
)
