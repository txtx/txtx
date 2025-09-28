module.exports = grammar({
  name: 'txtx',

  extras: $ => [
    /\s/,
    $.comment,
  ],

  rules: {
    runbook: $ => repeat($._statement),

    _statement: $ => choice(
      $.addon_block,
      $.signer_block,
      $.action_block,
      $.output_block,
      $.variable_declaration,
      $.input_declaration,
      $.import_statement,
      $.flow_block,
      $.module_block,
      $.runbook_block,
    ),

    // Addon block: addon "network_name" { ... }
    addon_block: $ => seq(
      'addon',
      field('network', $.string),
      field('config', $.block),
    ),

    // Signer block: signer "name" "type" { ... }
    signer_block: $ => seq(
      'signer',
      field('name', $.string),
      field('type', $.string),
      field('config', $.block),
    ),

    // Action block: action "name" "type" { ... }
    action_block: $ => seq(
      'action',
      field('name', $.string),
      field('type', $.string),
      field('config', $.block),
    ),

    // Output block: output "name" { ... }
    output_block: $ => seq(
      'output',
      field('name', $.string),
      field('config', $.block),
    ),

    // Variable declaration: variable "name" { ... }
    variable_declaration: $ => seq(
      'variable',
      field('name', $.string),
      field('config', $.block),
    ),

    // Input declaration: input "name" = value
    input_declaration: $ => seq(
      'input',
      field('name', $.string),
      '=',
      field('value', $._expression),
    ),

    // Import statement: import "path"
    import_statement: $ => seq(
      'import',
      field('path', $.string),
    ),

    // Flow block: flow "name" { ... }
    flow_block: $ => seq(
      'flow',
      field('name', $.string),
      field('config', $.block),
    ),

    // Module block: module "name" { ... }
    module_block: $ => seq(
      'module',
      field('name', $.string),
      field('config', $.block),
    ),

    // Runbook block: runbook "name" { ... }
    runbook_block: $ => seq(
      'runbook',
      field('name', $.string),
      field('config', $.block),
    ),

    // Block: { key = value ... }
    block: $ => seq(
      '{',
      repeat($.attribute),
      '}',
    ),

    // Attribute: key = value
    attribute: $ => seq(
      field('key', $.identifier),
      '=',
      field('value', $._expression),
    ),

    // Expressions
    _expression: $ => choice(
      $.string,
      $.number,
      $.boolean,
      $.null,
      $.array,
      $.object,
      $.reference,
      $.function_call,
      $.binary_expression,
    ),

    // String literals
    string: $ => choice(
      seq('"', /[^"]*/, '"'),
      seq("'", /[^']*/, "'"),
      // Multi-line string
      seq('"""', /[^"]|"[^"]|""[^"]*/s, '"""'),
    ),

    // Numbers
    number: $ => {
      const decimal = /[0-9]+/;
      const hexadecimal = /0x[0-9a-fA-F]+/;
      const float = /[0-9]+\.[0-9]+/;
      
      return choice(
        hexadecimal,
        float,
        decimal,
      );
    },

    // Booleans
    boolean: $ => choice('true', 'false'),

    // Null
    null: $ => 'null',

    // Arrays: [1, 2, 3]
    array: $ => seq(
      '[',
      sepBy(',', $._expression),
      optional(','),
      ']',
    ),

    // Objects: { key: value, ... }
    object: $ => seq(
      '{',
      sepBy(',', $.object_field),
      optional(','),
      '}',
    ),

    object_field: $ => seq(
      field('key', choice($.identifier, $.string)),
      ':',
      field('value', $._expression),
    ),

    // References: input.name, action.name.field, signer.name
    reference: $ => {
      const segment = choice($.identifier, $.index_access);
      return seq(
        segment,
        repeat(seq('.', segment)),
      );
    },

    index_access: $ => seq(
      $.identifier,
      '[',
      $._expression,
      ']',
    ),

    // Function calls: function_name(arg1, arg2)
    function_call: $ => seq(
      field('name', $.identifier),
      '(',
      field('arguments', sepBy(',', $._expression)),
      ')',
    ),

    // Binary expressions: a + b, a * b
    binary_expression: $ => choice(
      prec.left(2, seq($._expression, '*', $._expression)),
      prec.left(2, seq($._expression, '/', $._expression)),
      prec.left(1, seq($._expression, '+', $._expression)),
      prec.left(1, seq($._expression, '-', $._expression)),
    ),

    // Identifiers
    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    // Comments
    comment: $ => choice(
      seq('#', /.*/),
      seq('//', /.*/),
      seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/'),
    ),
  },
});

// Helper function for comma-separated lists
function sepBy(sep, rule) {
  return optional(seq(rule, repeat(seq(sep, rule))));
}