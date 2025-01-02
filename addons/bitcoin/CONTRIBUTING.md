# Contributing

Thank you for contributing to txtx! This guide is intended to provide context and instructions for contributing to the Bitcoin driver.

## Adding Opcodes
Bitcoin supports many opcodes - if you'd like to see one for txtx, it's quite simple to get it added!

The opcode encoder functions live in the [opcodes](./src/functions/opcodes/) directory.
First, choose which category the new opcode should be included in - code changes should only be needed in that file.

### Defining the function
At the top of each of the opcodes files, there should be a `lazy_static` block that defines the functions exported by the file. 
Add a new `define_function!` macro block to this array.
This macro will allow you to document the opcode encoder name, the expected input arguments, the outputs, and some documentation.
Here are some notes:
 - The struct identifier should be a PascalCase version of the opcode `Name`.
 - The name should be of the format `op_<name>`
 - The documentation should be of the format "`btc::op_<name>` does some action."
 - In most cases, the example block should contain a single hcl `output` named `"opcode"` with a `value = btc::op_<name>(...args)`. After the output block, there should be a comment: `// > opcode: <expected_result>`. Note: to generate the expected result, just run the block using txtx after the opcode has been built!
 - Each input should:
   - have a descriptive name
   - have documentation explaining the arguments purpose and any restrictions on the acceptable values
   - have a typing field that defines an array of acceptable types
   - indicate if the field is optional
 - In most cases, output should have a type `Type::addon(BITCOIN_OPCODE)`

Next, we can defined the struct that will do the encoding.

### Declaring the struct

At the bottom of the file, define a scruct like so, filling in the name appropriately:
```rust
#[derive(Clone)]
pub struct Name;
impl FunctionImplementation for Name {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}
```
The `check_instantiability` fn should be left `unimplemented!()`; only the `run` function needs to be completed.

First, if the function accepts arguments, call the following at the top of the `run` function:
```rust
arg_checker(fn_spec, args)?;
```
This will verify that the caller of the function has provided the appropriate number of arguments with the appropriate types.

Then, the expected arguments can be retrieved from the `args` variable:
```rust
// note: unwraps are safe because we called `arg_checker` fn
// note: here, we are getting the 0th argument, and casting it to a i128
let my_int = args.get(0).unwrap().as_integer().unwrap();
// note: here, we are getting the 1st argument, and casting it to a &str
let my_str = args.get(1).unwrap().as_string().unwrap();
```

Now, any data manipulations that need to take place can be completed on the extracted arguments.

Finally, the appropriate data needs to be returned. 
To do this, the opcode will need to be added to the codec.

Open the [codec module](./src/codec/mod.rs).
In the enum, add the opcode name to the appropriate section.
Then, add the new enum variant to the `get_code` fn, and have it return a vec with the appropriate bytes.

Now, the return data can be prepped.
In the opcode struct, pull the opcode bytes, push to it any needed data, and return the bytes:
```rust
let mut bytes = BitcoinOpcode:MyNewOp.get_code();
bytes.push(other_data_generated_from_args);
Ok(BitcoinValue::opcode(bytes))
```


Examples:
 - [Function Definition](https://github.com/txtx/txtx/blob/1ab496c69f216733fc51644ff865cc5575865035/addons/bitcoin/src/functions/opcodes/constants.rs#L33-L60)
 - [Struct Definition](https://github.com/txtx/txtx/blob/1ab496c69f216733fc51644ff865cc5575865035/addons/bitcoin/src/functions/opcodes/constants.rs#L112-L161)