variable(
  name: "some_name",
  default: "some default",
  readers: ["foo", "bar"],
  writers: ["foo", "bar"],
  cli_flag: "--some-name",
  env: "ENV_VAR",
)

variable(
  name: "foo",
)