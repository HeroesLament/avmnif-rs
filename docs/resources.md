## resource_type! Resource Type Registration

Registers a new resource type with AtomVM at startup

### Parameters:
- \`resource_name\`: Global variable name for the resource type (e.g., DISPLAY_TYPE)
- \`rust_type\`: The Rust struct/type that will be stored (e.g., DisplayContext)  
- \`destructor_fn\`: Function called when AtomVM GC destroys the resource (e.g., display_cleanup)

### What it does:
- Creates a global static variable to hold the resource type pointer
- Calls enif_init_resource_type() during module initialization
- Registers the destructor callback with AtomVM
- Makes the resource type available for allocation/extraction

### Generated code:
    static mut <resource_name>: *mut ErlNifResourceType = core::ptr::null_mut();
    
    // In module init function:
    <resource_name> = enif_init_resource_type(env, "<resource_name>", &init, flags, &mut tried);

### Usage:
    resource_type!(<resource_name>, <rust_type>, <destructor_fn>);

## create_resource! Resource Allocation

Creates a new instance of a resource in AtomVM-managed memory

### Parameters:
- \`resource_name\`: The resource type to allocate (must match resource_type! name)
- \`data_expr\`: Rust expression that creates the data to store (e.g., DisplayContext::new())

### What it does:
- Evaluates the data expression to create a Rust value
- Asks AtomVM for memory of the right size via enif_alloc_resource()
- Copies the Rust data into AtomVM-managed memory
- Returns a pointer to the allocated resource
- Memory is now owned by AtomVM and will be garbage collected

### Returns:
\`Result<*mut c_void, NifError>\` - Pointer to allocated resource or error

### Usage:
    create_resource!(<resource_name>, <data_expr>);

## get_resource! Resource Extraction

Gets your Rust data back from an Erlang term containing a resource

### Parameters:
- \`env_expr\`: The NIF environment (for safety checking)
- \`term_expr\`: Erlang term that should contain a resource (e.g., args[0])
- \`resource_name\`: Expected resource type (must match what's in the term)

### What it does:
- Calls enif_get_resource() to extract the pointer from the term
- Verifies the term actually contains a resource of the expected type
- Casts the void pointer back to your Rust type
- Returns a mutable reference to your data
- Ensures type safety - can't extract wrong resource type

### Returns:
\`Result<&mut <rust_type>, NifError>\` - Mutable reference to your data or error

### Usage:
    get_resource!(<env_expr>, <term_expr>, <resource_name>);

## make_resource_term! Term Creation

Wraps a resource pointer in an Erlang term so Erlang can hold onto it

### Parameters:
- \`env_expr\`: The NIF environment (needed for term creation)
- \`resource_ptr\`: Raw pointer to allocated resource (from create_resource!)

### What it does:
- Calls enif_make_resource() to create an Erlang term from the pointer
- Increments the resource reference count (AtomVM now tracks it)
- Creates a term that Erlang can pass around, store, send to other processes
- When Erlang GC collects this term, reference count decrements
- When reference count hits zero, destructor runs and memory is freed

### Returns:
\`Term\` - Erlang term wrapping the resource

### Usage:
    make_resource_term!(<env_expr>, <resource_ptr>);

## Example Usage Flow:

### 1. Register the resource type (once at startup)
    resource_type!(DISPLAY_TYPE, DisplayContext, display_destructor);

### 2. In a NIF function - allocate and return to Erlang
    fn display_init_nif(env: Env, args: &[Term]) -> NifResult<Term> {
        // Parse config from Erlang
        let config = parse_display_config(&args[0])?;
        
        // ALLOCATE: Create resource in AtomVM memory
        let display_ptr = create_resource!(DISPLAY_TYPE, DisplayContext::new(config))?;
        
        // TERM CREATION: Wrap for Erlang
        let display_term = make_resource_term!(env, display_ptr);
        
        Ok(display_term) // Erlang now owns this resource
    }

### 3. In another NIF function - extract and use
    fn display_draw_nif(env: Env, args: &[Term]) -> NifResult<Term> {
        // EXTRACT: Get our data back from Erlang term
        let display = get_resource!(env, args[0], DISPLAY_TYPE)?;
        let x = args[1].to_i32()?;
        let y = args[2].to_i32()?;
        
        // Use the resource
        display.draw_pixel(x, y, 0xFF0000)?;
        
        Ok(Term::atom("ok"))
    }

### 4. Destructor runs automatically when Erlang GC collects the term
    unsafe extern "C" fn display_destructor(_env: *mut ErlNifEnv, obj: *mut c_void) {
        let display = obj as *mut DisplayContext;
        // Cleanup: close files, free hardware, etc.
        (*display).cleanup();
        drop(core::ptr::read(display));
    }

## Parameter Summary:

- \`resource_name\`    = Global identifier for this resource type
- \`rust_type\`        = Your Rust struct that gets stored  
- \`destructor_fn\`    = Cleanup function when GC destroys resource
- \`data_expr\`        = Expression that creates your Rust data
- \`env_expr\`         = NIF environment (for safety and term creation)
- \`term_expr\`        = Erlang term containing a resource
- \`resource_ptr\`     = Raw pointer to allocated resource memory`;

console.log(markdownDoc);
Output

Result

# Resource Macro Templates - Generic Parameter Descriptions

Shows what each parameter does in the resource macros

## MACRO 1: Resource Type Registration

Registers a new resource type with AtomVM at startup

### Parameters:
- `resource_name`: Global variable name for the resource type (e.g., DISPLAY_TYPE)
- `rust_type`: The Rust struct/type that will be stored (e.g., DisplayContext)  
- `destructor_fn`: Function called when AtomVM GC destroys the resource (e.g., display_cleanup)

### What it does:
- Creates a global static variable to hold the resource type pointer
- Calls enif_init_resource_type() during module initialization
- Registers the destructor callback with AtomVM
- Makes the resource type available for allocation/extraction

### Generated code:
    static mut <resource_name>: *mut ErlNifResourceType = core::ptr::null_mut();
    
    // In module init function:
    <resource_name> = enif_init_resource_type(env, "<resource_name>", &init, flags, &mut tried);

### Usage:
    resource_type!(<resource_name>, <rust_type>, <destructor_fn>);

## MACRO 2: Resource Allocation

Creates a new instance of a resource in AtomVM-managed memory

### Parameters:
- `resource_name`: The resource type to allocate (must match resource_type! name)
- `data_expr`: Rust expression that creates the data to store (e.g., DisplayContext::new())

### What it does:
- Evaluates the data expression to create a Rust value
- Asks AtomVM for memory of the right size via enif_alloc_resource()
- Copies the Rust data into AtomVM-managed memory
- Returns a pointer to the allocated resource
- Memory is now owned by AtomVM and will be garbage collected

### Returns:
`Result<*mut c_void, NifError>` - Pointer to allocated resource or error

### Usage:
    create_resource!(<resource_name>, <data_expr>);

## MACRO 3: Resource Extraction

Gets your Rust data back from an Erlang term containing a resource

### Parameters:
- `env_expr`: The NIF environment (for safety checking)
- `term_expr`: Erlang term that should contain a resource (e.g., args[0])
- `resource_name`: Expected resource type (must match what's in the term)

### What it does:
- Calls enif_get_resource() to extract the pointer from the term
- Verifies the term actually contains a resource of the expected type
- Casts the void pointer back to your Rust type
- Returns a mutable reference to your data
- Ensures type safety - can't extract wrong resource type

### Returns:
`Result<&mut <rust_type>, NifError>` - Mutable reference to your data or error

### Usage:
    get_resource!(<env_expr>, <term_expr>, <resource_name>);

## MACRO 4: Term Creation

Wraps a resource pointer in an Erlang term so Erlang can hold onto it

### Parameters:
- `env_expr`: The NIF environment (needed for term creation)
- `resource_ptr`: Raw pointer to allocated resource (from create_resource!)

### What it does:
- Calls enif_make_resource() to create an Erlang term from the pointer
- Increments the resource reference count (AtomVM now tracks it)
- Creates a term that Erlang can pass around, store, send to other processes
- When Erlang GC collects this term, reference count decrements
- When reference count hits zero, destructor runs and memory is freed

### Returns:
`Term` - Erlang term wrapping the resource

### Usage:
    make_resource_term!(<env_expr>, <resource_ptr>);

## Example Usage Flow:

### 1. Register the resource type (once at startup)
    resource_type!(DISPLAY_TYPE, DisplayContext, display_destructor);

### 2. In a NIF function - allocate and return to Erlang
    fn display_init_nif(env: Env, args: &[Term]) -> NifResult<Term> {
        // Parse config from Erlang
        let config = parse_display_config(&args[0])?;
        
        // ALLOCATE: Create resource in AtomVM memory
        let display_ptr = create_resource!(DISPLAY_TYPE, DisplayContext::new(config))?;
        
        // TERM CREATION: Wrap for Erlang
        let display_term = make_resource_term!(env, display_ptr);
        
        Ok(display_term) // Erlang now owns this resource
    }

### 3. In another NIF function - extract and use
    fn display_draw_nif(env: Env, args: &[Term]) -> NifResult<Term> {
        // EXTRACT: Get our data back from Erlang term
        let display = get_resource!(env, args[0], DISPLAY_TYPE)?;
        let x = args[1].to_i32()?;
        let y = args[2].to_i32()?;
        
        // Use the resource
        display.draw_pixel(x, y, 0xFF0000)?;
        
        Ok(Term::atom("ok"))
    }

### 4. Destructor runs automatically when Erlang GC collects the term
    unsafe extern "C" fn display_destructor(_env: *mut ErlNifEnv, obj: *mut c_void) {
        let display = obj as *mut DisplayContext;
        // Cleanup: close files, free hardware, etc.
        (*display).cleanup();
        drop(core::ptr::read(display));
    }

## Parameter Summary:

- `resource_name`    = Global identifier for this resource type
- `rust_type`        = Your Rust struct that gets stored  
- `destructor_fn`    = Cleanup function when GC destroys resource
- `data_expr`        = Expression that creates your Rust data
- `env_expr`         = NIF environment (for safety and term creation)
- `term_expr`        = Erlang term containing a resource
- `resource_ptr`     = Raw pointer to allocated resource memory