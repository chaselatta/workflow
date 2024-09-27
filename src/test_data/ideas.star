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


say_hi = action(
  tool = echo,
  args = [
    "hello",
    name,
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

single_node_workflow = workflow(
  # There is a single node in the graph so we don't need an entrypoint,
  graph = node(
      action = bark,
    ),
)

single_sequence_workflow = workflow(
  # There is a single node in the graph so we don't need an entrypoint
  graph = sequence(
      actions = [
        say_hi,
        bark,
        say_bye,
      ],
  )
)

main = workflow(
  # Entrypoints 
  entrypoint = "hi",

  # A graph can take either a list of nodes/sequences or it can take a single
  # node or sequence. If a list is passed in then the entrypoint name will be
  # used but if a single is passed in the entrypoint will not be used.
  graph = [
    node(
      # name is a name used in selecting the next node.
      # a name can be omitted but if it is it cannot be selected in a
      # next_choice function. only useful for single node graphs.
      name = "hi",
      # action is the action to run
      action =  say_hi,
      # Next is a function which chooses the next action to run.
      # the stdlib will provide some actions but users can write their
      # own by using the next_choice function.
      # def _my_next_choice(ctx):
      #   if ctx.args.default:
      #     return ctx.args.default
      #   return "foo" 
      # my_next_choice = next_choice(
      #  implementation = _my_next_choice,
      #  args = {
      #    "default": arg.string(required = True)
      #  }
      #)
      next = on_exit_code({
        0: "bark",
      }, default: "D")
    ),
    node(
      # this is a terminal node, there is no next action.
      name = "bye",
      action = say_bye,
    ),
    node(
      name = "bark",
      action = bark,
      # A next choice can just be a pure action which means that it
      # gets run automatically next.
      next = say_bye,
    ),
  ]
)

# We then need to run the workflow
run_workflow(main) #if run_workflow is called twice we error out.