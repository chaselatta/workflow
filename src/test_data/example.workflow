var_1 = variable(
  name =  "var_1",
  default =  "some default",
  readers =  ["foo", "bar"],
  writers =  ["foo", "bar"],
  cli_flag =  "--foo",
  env =  "VAR_ONE",
)

var_2 = variable(
  name =  "var_2",
  default =  "some default",
  readers =  ["foo", "bar"],
  writers =  ["foo", "bar"],
  env =  "VAR_TWO",
)

var_3 = variable(
  name =  "var_3",
)

builtin_tool(
  name = "echo",
)

tool(
  name = "foo",
  path = "foo.sh",
)

HOME = variable(
  name =  "HOME",
  env =  "HOME",
)

tool(
  name = "orchestral",
  path = "{variable(HOME)}/bin/orchestral",
)
