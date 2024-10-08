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

def _name_updater(ctx):
  return ctx.stdout + ctx.stderr + str(ctx.exit_code)

def _next_impl(ctx, args):
  if ctx.exit_code == args.y:
    return args.x
  return "end"

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

end = action(
  tool = echo,
  args = [
    "This is the end",
  ]
)

say_bye = action(
  tool = echo,
  args = [
    format("goodbye, {}", name),
  ]
)

simple_next = next(
  implementation = _next_impl,
  args = {
    "x": args.string(),
    "y": args.int(),
  }
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
      ],
      next = simple_next(x = "bark", y = 1)
    ),
    node(
      name = "bark",
      action = bark,
    ),
    node(
      name = "end",
      action = end,
    )
  ]
)
