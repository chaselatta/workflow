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

# Leave this value here for tests
1