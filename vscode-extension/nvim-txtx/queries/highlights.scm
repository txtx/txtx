; Keywords
[
  "addon"
  "signer"
  "action"
  "output"
  "variable"
  "input"
  "import"
  "flow"
  "module"
  "runbook"
] @keyword

; Comments
(comment) @comment

; Strings
(string) @string

; Numbers
(number) @number

; Booleans
(boolean) @boolean

; Null
(null) @constant.builtin

; Functions
(function_call
  name: (identifier) @function.call)

; References (variables, actions, etc.)
(reference) @variable

; Identifiers in attributes
(attribute
  key: (identifier) @property)

; Object fields
(object_field
  key: (identifier) @property)
(object_field
  key: (string) @property)

; Block names
(addon_block
  network: (string) @string.special)

(signer_block
  name: (string) @string.special
  type: (string) @type)

(action_block
  name: (string) @string.special
  type: (string) @type)

(output_block
  name: (string) @string.special)

(variable_declaration
  name: (string) @string.special)

(flow_block
  name: (string) @string.special)

(module_block
  name: (string) @string.special)

(runbook_block
  name: (string) @string.special)

(input_declaration
  name: (string) @string.special)

(import_statement
  path: (string) @string.special)

; Operators
"=" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator

; Punctuation
[
  "{"
  "}"
] @punctuation.bracket

[
  "["
  "]"
] @punctuation.bracket

[
  "("
  ")"
] @punctuation.bracket

"," @punctuation.delimiter
":" @punctuation.delimiter
"." @punctuation.delimiter
