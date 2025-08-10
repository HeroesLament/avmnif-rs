## nif_collection! Macro

Creates a complete NIF module with automatic registration and initialization

### Parameters:
- \`module_name\`: String literal name for the NIF module (e.g., "display_nifs")
- \`nif_function_name\`: Rust function name that implements the NIF (e.g., display_init_nif)
- \`erlang_function_name\`: String literal name that Erlang will call (e.g., "init")
- \`arity\`: Number of arguments the function takes (e.g., 1, 2, 3)

### What it does:
- Creates a static array of NIF function entries
- Generates the module initialization function
- Handles AtomVM NIF registration boilerplate
- Sets up proper function signatures and calling conventions
- Provides error handling and type conversion infrastructure

### Generated code structure:
    // Static NIF function array
    static NIF_FUNCS: &[ErlNifFunc] = &[
        ErlNifFunc {
            name: "<erlang_function_name>",
            arity: <arity>,
            function: <nif_function_name>_wrapper,
            flags: 0,
        },
        // ... more functions
    ];
    
    // Module initialization
    #[no_mangle]
    pub extern "C" fn nif_init() -> *const ErlNifEntry {
        &NIF_ENTRY
    }
    
    // Individual function wrappers
    extern "C" fn <nif_function_name>_wrapper(
        env: *mut ErlNifEnv,
        argc: c_int,
        argv: *const ERL_NIF_TERM
    ) -> ERL_NIF_TERM {
        // Wrapper implementation with error handling
    }

### Usage:
    nif_collection! {
        name: "<module_name>",
        nifs: [
            (<nif_function_name>, "<erlang_function_name>", <arity>),
            // ... more NIFs
        ],
    }

## Example Usage:

### Basic NIF Collection
    nif_collection! {
        name: "display_nifs",
        nifs: [
            (display_init_nif, "init", 1),
            (display_clear_nif, "clear", 1),
            (display_draw_pixel_nif, "draw_pixel", 4),
            (display_get_info_nif, "get_info", 1),
        ],
    }

### Complex NIF Collection with Multiple Modules
    nif_collection! {
        name: "esp32_hardware",
        nifs: [
            // GPIO functions
            (gpio_set_direction_nif, "gpio_set_direction", 2),
            (gpio_set_level_nif, "gpio_set_level", 2),
            (gpio_get_level_nif, "gpio_get_level", 1),
            
            // ADC functions  
            (adc_init_nif, "adc_init", 1),
            (adc_read_nif, "adc_read", 1),
            (adc_read_voltage_nif, "adc_read_voltage", 1),
            
            // SPI functions
            (spi_init_nif, "spi_init", 1),
            (spi_transfer_nif, "spi_transfer", 2),
            (spi_close_nif, "spi_close", 1),
        ],
    }

### Required NIF Function Signatures

Each NIF function must follow this signature pattern:

    fn <nif_function_name>(ctx: &Context, args: &[Term]) -> NifResult<Term> {
        // Function implementation
    }

Where:
- \`ctx\`: Execution context (usually unused in simple NIFs)
- \`args\`: Array of Erlang terms passed as arguments
- \`NifResult<Term>\`: Result type that can be Ok(term) or Err(error)

### Example NIF Function Implementation
    fn display_init_nif(_ctx: &Context, args: &[Term]) -> NifResult<Term> {
        // Validate argument count
        if args.len() != 1 {
            return Err(NifError::BadArg);
        }
        
        // Extract configuration from args[0]
        let config = parse_display_config(&args[0])?;
        
        // Create and initialize display
        let display = DisplayContext::new(config)?;
        let display_ptr = create_resource!(DISPLAY_TYPE, display)?;
        
        // Return resource term to Erlang
        Ok(make_resource_term!(env, display_ptr))
    }

    fn display_draw_pixel_nif(_ctx: &Context, args: &[Term]) -> NifResult<Term> {
        // Validate argument count  
        if args.len() != 4 {
            return Err(NifError::BadArg);
        }
        
        // Extract arguments
        let display = get_resource!(env, args[0], DISPLAY_TYPE)?;
        let x = args[1].to_i32()?;
        let y = args[2].to_i32()?;
        let color = args[3].to_u32()?;
        
        // Perform operation
        display.draw_pixel(x, y, color)?;
        
        Ok(Term::atom("ok"))
    }

## Erlang Usage

After compilation, the NIFs can be used from Erlang:

    % Load the NIF module
    ok = erlang:load_nif("path/to/display_nifs", 0),
    
    % Use the functions
    Display = display_nifs:init(#{width => 240, height => 320}),
    ok = display_nifs:clear(Display),
    ok = display_nifs:draw_pixel(Display, 10, 20, 16#FF0000),
    Info = display_nifs:get_info(Display).

## Generated Module Structure

The macro generates a complete C-compatible NIF module:

    // Module entry point
    static NIF_ENTRY: ErlNifEntry = ErlNifEntry {
        major: ERL_NIF_MAJOR_VERSION,
        minor: ERL_NIF_MINOR_VERSION,
        name: "<module_name>",
        num_of_funcs: <function_count>,
        funcs: NIF_FUNCS.as_ptr(),
        load: Some(nif_load),
        reload: None,
        upgrade: None,
        unload: None,
        vm_variant: c"beam.vanilla".as_ptr(),
        options: 1,
        sizeof_ErlNifResourceTypeInit: size_of::<ErlNifResourceTypeInit>(),
        min_erts: c"erts-6.0".as_ptr(),
    };

## Parameter Summary:

- \`module_name\`           = String name for the entire NIF module
- \`nif_function_name\`     = Rust function that implements the NIF logic
- \`erlang_function_name\`  = String name that Erlang code will call
- \`arity\`                 = Number of arguments the function accepts (0-255)
- \`ctx\`                   = Execution context passed to NIF functions
- \`args\`                  = Array of Erlang terms containing function arguments
- \`NifResult<Term>\`       = Return type for NIF functions (Ok/Err)`;

console.log(nifCollectionDoc);
Output

Result

# NIF Collection Macro Template - Generic Parameter Descriptions

Shows what each parameter does in the nif_collection macro

## nif_collection! Macro

Creates a complete NIF module with automatic registration and initialization

### Parameters:
- `module_name`: String literal name for the NIF module (e.g., "display_nifs")
- `nif_function_name`: Rust function name that implements the NIF (e.g., display_init_nif)
- `erlang_function_name`: String literal name that Erlang will call (e.g., "init")
- `arity`: Number of arguments the function takes (e.g., 1, 2, 3)

### What it does:
- Creates a static array of NIF function entries
- Generates the module initialization function
- Handles AtomVM NIF registration boilerplate
- Sets up proper function signatures and calling conventions
- Provides error handling and type conversion infrastructure

### Generated code structure:
    // Static NIF function array
    static NIF_FUNCS: &[ErlNifFunc] = &[
        ErlNifFunc {
            name: "<erlang_function_name>",
            arity: <arity>,
            function: <nif_function_name>_wrapper,
            flags: 0,
        },
        // ... more functions
    ];
    
    // Module initialization
    #[no_mangle]
    pub extern "C" fn nif_init() -> *const ErlNifEntry {
        &NIF_ENTRY
    }
    
    // Individual function wrappers
    extern "C" fn <nif_function_name>_wrapper(
        env: *mut ErlNifEnv,
        argc: c_int,
        argv: *const ERL_NIF_TERM
    ) -> ERL_NIF_TERM {
        // Wrapper implementation with error handling
    }

### Usage:
    nif_collection! {
        name: "<module_name>",
        nifs: [
            (<nif_function_name>, "<erlang_function_name>", <arity>),
            // ... more NIFs
        ],
    }

## Example Usage:

### Basic NIF Collection
    nif_collection! {
        name: "display_nifs",
        nifs: [
            (display_init_nif, "init", 1),
            (display_clear_nif, "clear", 1),
            (display_draw_pixel_nif, "draw_pixel", 4),
            (display_get_info_nif, "get_info", 1),
        ],
    }

### Complex NIF Collection with Multiple Modules
    nif_collection! {
        name: "esp32_hardware",
        nifs: [
            // GPIO functions
            (gpio_set_direction_nif, "gpio_set_direction", 2),
            (gpio_set_level_nif, "gpio_set_level", 2),
            (gpio_get_level_nif, "gpio_get_level", 1),
            
            // ADC functions  
            (adc_init_nif, "adc_init", 1),
            (adc_read_nif, "adc_read", 1),
            (adc_read_voltage_nif, "adc_read_voltage", 1),
            
            // SPI functions
            (spi_init_nif, "spi_init", 1),
            (spi_transfer_nif, "spi_transfer", 2),
            (spi_close_nif, "spi_close", 1),
        ],
    }

### Required NIF Function Signatures

Each NIF function must follow this signature pattern:

    fn <nif_function_name>(ctx: &Context, args: &[Term]) -> NifResult<Term> {
        // Function implementation
    }

Where:
- `ctx`: Execution context (usually unused in simple NIFs)
- `args`: Array of Erlang terms passed as arguments
- `NifResult<Term>`: Result type that can be Ok(term) or Err(error)

### Example NIF Function Implementation
    fn display_init_nif(_ctx: &Context, args: &[Term]) -> NifResult<Term> {
        // Validate argument count
        if args.len() != 1 {
            return Err(NifError::BadArg);
        }
        
        // Extract configuration from args[0]
        let config = parse_display_config(&args[0])?;
        
        // Create and initialize display
        let display = DisplayContext::new(config)?;
        let display_ptr = create_resource!(DISPLAY_TYPE, display)?;
        
        // Return resource term to Erlang
        Ok(make_resource_term!(env, display_ptr))
    }

    fn display_draw_pixel_nif(_ctx: &Context, args: &[Term]) -> NifResult<Term> {
        // Validate argument count  
        if args.len() != 4 {
            return Err(NifError::BadArg);
        }
        
        // Extract arguments
        let display = get_resource!(env, args[0], DISPLAY_TYPE)?;
        let x = args[1].to_i32()?;
        let y = args[2].to_i32()?;
        let color = args[3].to_u32()?;
        
        // Perform operation
        display.draw_pixel(x, y, color)?;
        
        Ok(Term::atom("ok"))
    }

## Erlang Usage

After compilation, the NIFs can be used from Erlang:

    % Load the NIF module
    ok = erlang:load_nif("path/to/display_nifs", 0),
    
    % Use the functions
    Display = display_nifs:init(#{width => 240, height => 320}),
    ok = display_nifs:clear(Display),
    ok = display_nifs:draw_pixel(Display, 10, 20, 16#FF0000),
    Info = display_nifs:get_info(Display).

## Generated Module Structure

The macro generates a complete C-compatible NIF module:

    // Module entry point
    static NIF_ENTRY: ErlNifEntry = ErlNifEntry {
        major: ERL_NIF_MAJOR_VERSION,
        minor: ERL_NIF_MINOR_VERSION,
        name: "<module_name>",
        num_of_funcs: <function_count>,
        funcs: NIF_FUNCS.as_ptr(),
        load: Some(nif_load),
        reload: None,
        upgrade: None,
        unload: None,
        vm_variant: c"beam.vanilla".as_ptr(),
        options: 1,
        sizeof_ErlNifResourceTypeInit: size_of::<ErlNifResourceTypeInit>(),
        min_erts: c"erts-6.0".as_ptr(),
    };

## Parameter Summary:

- `module_name`           = String name for the entire NIF module
- `nif_function_name`     = Rust function that implements the NIF logic
- `erlang_function_name`  = String name that Erlang code will call
- `arity`                 = Number of arguments the function accepts (0-255)
- `ctx`                   = Execution context passed to NIF functions
- `args`                  = Array of Erlang terms containing function arguments
- `NifResult<Term>`       = Return type for NIF functions (Ok/Err)