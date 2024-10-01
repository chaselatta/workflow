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


name = variable(
  cli_flag = "--name"
)

echo = builtin_tool(
 name = "echo",
)

def _name_updater():
  return "NEW NAME"

say_hi = action(
  tool = echo,
  args = [
    "hello",
    name,
  ],
  setters = [
    setter(
      implementation = _name_updater,
      variable = name,
    )
  ]
)

bark = action(
  tool = echo,
  args = [
    "woof, woof",
  ]
)

say_bye = action(
  tool = echo,
  args = [
    format("goodbye, {}", name),
  ]
)

main = workflow(
  entrypoint = "hi",
  graph = [
    sequence(
      name = "hi",
      actions = [
        say_hi,
        bark,
        say_bye
      ]
    )
  ]
)
