# The workflow starlark stdlib

## Variable
The variable is a built in function which allows users to define variables
that can be used at later times. The variables are referenced by name in
the workflow.

Variables have the following properties.
* name: The name of the variable which can be used to reference it later
* default: The default value to use
* cli_flag: The command line flag that can be used to set the value
* env: The environment variable that can be used to set the variable
* readers: A list of scopes specifying who can read the variable. Do not specify
a value to make this globally readable
* writers: A list of scopes specifying who can write the variable. Do not specify
a value to make this globally writeable


### Using variables (not yet implemented)
Variables can be used from within the workflow by using string interpolation. To
use a variable in a string you place the name between `{}` inside the string. The
variable will be evaluated at the time of using the string.

```
args = [ "--foo={foo}" ]
```

In order to use the variable the calling context must be available in the `readers`
scope of the variable or the scope must be global.

### Updating variables
Variables will originally take their value from one of the following places
in the given order:
1. The value from `cli_flag` if present
1. The `env` variable if present
1. The `default` value
1. A value later updated in the workflow (not yet implemented)

In order to update a variable a user must define a `variable_modidifer` which
can update the variable from within an action.

A variable modifier takes an implementation which is a function that takes a
`modifier_ctx` which can be used to get the results of an action and can update
the value.

```
def _my_modifier_impl(ctx):
  result = json.decode(ctx.stdout);
  ctx.udpate_variable(value = result[ctx.params.key])

my_modifier = variable_modifier(
  implementation = _my_modifier_impl,
  params = {
    "key": param.string(),
  }
)

# my_modifier is a global function that can later be used like the following:
# note that variable is present on all of the modifiers.

my_modifier(variable = "foo", key = "result")
```

This modifier can later be added to the `variable_modifiers` on an `action` to
update the variable.

The following builtin variable modifiers are avaialable:

* variable_modifier_from_exit_code(variable, exit_code_map, default). The variable
is the name of the variable to update, the exit_code_map is a map of exit codes to
values to set {0: "foo", 1: "bar"} and the default is a value that will be used if
none of the exit codes are in the map.
* variable_modifier_from_json(variable, path, default). The variable is the name of
the variable to update, the path is a period delimited path into the json result and
the default is what will be used if the path is not present.

## Tool
A tool is specified with the `tool` or `builtin_tool` rules. A tool must be
defined before it can be used from an action.

To declare a tool that is globally available use the `builtin_tool` rule. The
name of the tool will be the same as the name you would use on the command line.
```
builtin_tool(
  name = "echo",
)
```

For a tool that is specified on a path do the following:

```
tool(
  name = "my-tool",
  path = "some/path/my-tool"
)
```

If a path is relative, the path should be specified relative to the
workflow file. Paths can take part in variable interpolation so the
following is valid.

```
tool(
  name = "my-tool",
  path = "{build-dir}/path/my-tool",
)
```

### Running workflows before tool instantiation. (Not implemented)
Tools can optionally declare a workflow to run before they are
validated. This can allow a user to run a series of actions to build
a tool, set variables or download a tool
```
tool(
  name = "my-tool",
  path = "{build-dir}/path/my-tool",
  setup = "my-tool-setup-workflow",
)
```