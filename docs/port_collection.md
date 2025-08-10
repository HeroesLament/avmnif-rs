## port_collection! Macro

Creates a complete AtomVM port with lifecycle management and message handling

### Parameters:
- \`port_name\`: Identifier for the port collection (e.g., avmgl_touch)
- \`init_fn\`: Global initialization function called once at startup (e.g., avmgl_touch_init)
- \`destroy_fn\`: Global cleanup function called at shutdown (e.g., avmgl_touch_destroy)
- \`create_port_fn\`: Function that creates individual port instances (e.g., touch_create_port)
- \`handler_fn\`: Message handler function for port communication (e.g., touch_message_handler)

### What it does:
- Registers the port driver with AtomVM
- Sets up global initialization and cleanup lifecycle
- Manages individual port instance creation
- Routes messages to the appropriate handler function
- Provides infrastructure for async communication from hardware

### Generated code structure:
    // Port driver registration
    static PORT_DRIVER: AtomVMPortDriver = AtomVMPortDriver {
        name: "<port_name>",
        init: Some(<init_fn>),
        destroy: Some(<destroy_fn>),
        create_port: <create_port_fn>,
        message_handler: <handler_fn>,
    };
    
    // Module registration
    #[no_mangle]
    pub extern "C" fn port_driver_init() -> *const AtomVMPortDriver {
        &PORT_DRIVER
    }

### Usage:
    port_collection!(
        <port_name>,
        init = <init_fn>,
        destroy = <destroy_fn>,
        create_port = <create_port_fn>,
        handler = <handler_fn>
    );

## Function Signatures:

### Global Init Function
    fn <init_fn>(_global: &mut GlobalContext) {
        // One-time hardware/driver initialization
        // Called when AtomVM starts up
    }

### Global Destroy Function  
    fn <destroy_fn>(_global: &mut GlobalContext) {
        // Global cleanup when AtomVM shuts down
        // Clean up shared resources, disable interrupts, etc.
    }

### Port Creation Function
    fn <create_port_fn>(_global: &GlobalContext, opts: Term) -> *mut Context {
        // Create and configure a new port instance
        // Parse options from Erlang
        // Allocate port-specific data structure
        // Return context pointer for this port instance
    }

### Message Handler Function
    fn <handler_fn>(ctx: &mut Context, message: &Message) -> PortResult {
        // Handle messages sent to this port from Erlang
        // Extract platform_data for port state
        // Process commands and send replies
        // Return Continue or Terminate
    }

## Example Usage:

### Complete Touch Controller Port
    port_collection!(
        avmgl_touch,
        init = avmgl_touch_init,
        destroy = avmgl_touch_destroy,
        create_port = touch_create_port,
        handler = touch_message_handler
    );

### Hardware SPI Port Example
    port_collection!(
        stm32_spi,
        init = spi_driver_init,
        destroy = spi_driver_cleanup,
        create_port = spi_create_port,
        handler = spi_message_handler
    );

## Port Data Management:

### Port-Specific Data Structure
    #[repr(C)]
    struct <PortName>Data {
        // Hardware configuration
        hardware_config: HardwareConfig,
        
        // Runtime state
        active: bool,
        owner_pid: u32,
        
        // Hardware handles/timers
        timer_handle: *mut c_void,
        interrupt_handle: *mut c_void,
    }

### Storing Port Data in Context
    fn <create_port_fn>(_global: &GlobalContext, opts: Term) -> *mut Context {
        let port_data = Box::new(<PortName>Data {
            // Initialize from opts
        });
        
        unsafe {
            let ctx = create_port_context(_global);
            (*ctx).platform_data = Box::into_raw(port_data) as *mut c_void;
            ctx
        }
    }

### Accessing Port Data in Handler
    fn <handler_fn>(ctx: &mut Context, message: &Message) -> PortResult {
        let port_data = unsafe { 
            &mut *((*ctx).platform_data as *mut <PortName>Data) 
        };
        
        // Use port_data for message processing
    }

## Message Handling Patterns:

### Standard Command Processing
    fn <handler_fn>(ctx: &mut Context, message: &Message) -> PortResult {
        let port_data = unsafe { 
            &mut *((*ctx).platform_data as *mut <PortName>Data) 
        };
    
        let (pid, reference, command) = parse_gen_message(message)?;
        
        match command.atom_name()? {
            "start" => {
                // Initialize hardware for this port instance
                port_data.owner_pid = pid.to_local_process_id();
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Continue
            }
            
            "configure" => {
                // Update hardware configuration
                let config = command.get_kv("config")?;
                port_data.apply_config(config)?;
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Continue
            }
            
            "read" => {
                // Read data from hardware
                let data = port_data.read_hardware_data()?;
                send_reply(ctx, pid, reference, tuple!("ok", data));
                PortResult::Continue
            }
            
            "stop" => {
                // Clean up this port instance
                port_data.cleanup_hardware();
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Terminate
            }
            
            _ => {
                send_reply(ctx, pid, reference, tuple!("error", atom!("unknown_command")));
                PortResult::Continue
            }
        }
    }

## Async Hardware Communication:

### ISR to Erlang Message Flow
    // Hardware interrupt handler
    extern "C" fn hardware_isr_handler(arg: *mut c_void) {
        let port_data = arg as *mut <PortName>Data;
        
        unsafe {
            // Read hardware data
            let data = read_hardware_registers();
            
            // Create Erlang message
            let event_msg = create_event_message(data);
            
            // Send to owning Erlang process
            port_send_message_from_task(
                global_context_ptr(),
                (*port_data).owner_pid,
                event_msg
            );
        }
    }

### Timer-Based Events
    extern "C" fn timer_callback(arg: *mut c_void) {
        let port_data = arg as *mut <PortName>Data;
        
        unsafe {
            let tick_msg = create_tick_message();
            port_send_message_from_task(
                global_context_ptr(),
                (*port_data).owner_pid, 
                tick_msg
            );
        }
    }

## Erlang Usage:

After loading the port driver, Erlang can create and use ports:

    % Create a port instance
    Port = open_port({spawn, "avmgl_touch"}, [binary, {packet, 4}]),
    
    % Send commands to the port
    Port ! {call, self(), make_ref(), start_touch},
    Port ! {call, self(), make_ref(), {start_animation, [{fps, 60}]}},
    
    % Receive replies and async messages
    receive
        {Port, {data, {ok, Data}}} -> handle_reply(Data);
        {Port, {data, {touch, X, Y}}} -> handle_touch_event(X, Y);
        {Port, {data, animation_tick}} -> handle_animation_frame()
    end.

## Port vs NIFs - When to Use Which:

### Use Ports When:
- Hardware generates asynchronous events (interrupts, timers)
- Need to send unsolicited messages to Erlang processes
- Long-running operations that shouldn't block the scheduler
- Hardware state needs to persist between calls
- Multiple Erlang processes need to interact with the same hardware

### Use NIFs When:
- Simple synchronous function calls
- Fast operations that return immediately
- Stateless or resource-based operations
- Direct function call semantics from Erlang

## Complete Touch Controller Example:

    #[repr(C)]
    struct TouchPortData {
        i2c_port: u8,
        interrupt_pin: u8,
        touch_threshold: u16,
        last_x: u16,
        last_y: u16,
        touch_active: bool,
        animation_timer: *mut c_void,
        owner_pid: u32,
    }

    fn avmgl_touch_init(_global: &mut GlobalContext) {
        unsafe {
            touch_controller_init();
            gpio_install_isr_service(0);
        }
    }

    fn avmgl_touch_destroy(_global: &mut GlobalContext) {
        unsafe {
            gpio_uninstall_isr_service();
            touch_controller_deinit();
        }
    }

    fn touch_create_port(_global: &GlobalContext, opts: Term) -> *mut Context {
        let port_data = Box::new(TouchPortData {
            i2c_port: opts.get_kv("i2c_port").unwrap_or(Term::from_u8(0)).to_u8().unwrap(),
            interrupt_pin: opts.get_kv("interrupt_pin").unwrap_or(Term::from_u8(4)).to_u8().unwrap(),
            touch_threshold: opts.get_kv("threshold").unwrap_or(Term::from_u16(100)).to_u16().unwrap(),
            last_x: 0,
            last_y: 0,
            touch_active: false,
            animation_timer: std::ptr::null_mut(),
            owner_pid: 0,
        });

        unsafe {
            let ctx = create_port_context(_global);
            (*ctx).platform_data = Box::into_raw(port_data) as *mut c_void;
            ctx
        }
    }

    fn touch_message_handler(ctx: &mut Context, message: &Message) -> PortResult {
        let port_data = unsafe { 
            &mut *((*ctx).platform_data as *mut TouchPortData) 
        };

        let (pid, reference, command) = parse_gen_message(message)?;
        
        match command.atom_name()? {
            "start_touch" => {
                port_data.owner_pid = pid.to_local_process_id();
                // Configure hardware interrupts...
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Continue
            }
            "stop" => {
                // Cleanup hardware...
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Terminate
            }
            _ => {
                send_reply(ctx, pid, reference, tuple!("error", atom!("unknown_command")));
                PortResult::Continue
            }
        }
    }

    port_collection!(
        avmgl_touch,
        init = avmgl_touch_init,
        destroy = avmgl_touch_destroy,  
        create_port = touch_create_port,
        handler = touch_message_handler
    );

## Parameter Summary:

- \`port_name\`         = Identifier for the entire port driver
- \`init_fn\`           = Global initialization function (hardware setup)
- \`destroy_fn\`        = Global cleanup function (hardware teardown)  
- \`create_port_fn\`    = Creates individual port instances with config
- \`handler_fn\`        = Processes messages sent to port instances
- \`GlobalContext\`     = AtomVM global state (usually unused)
- \`Context\`           = Individual port instance context
- \`opts\`              = Erlang term with port configuration options
- \`Message\`           = Incoming message from Erlang process
- \`PortResult\`        = Continue or Terminate the port instance
- \`platform_data\`     = Pointer to port-specific data structure`;

console.log(portCollectionDoc);
Output

Result

# Port Collection Macro Template - Generic Parameter Descriptions

Shows what each parameter does in the port_collection macro

## port_collection! Macro

Creates a complete AtomVM port with lifecycle management and message handling

### Parameters:
- `port_name`: Identifier for the port collection (e.g., avmgl_touch)
- `init_fn`: Global initialization function called once at startup (e.g., avmgl_touch_init)
- `destroy_fn`: Global cleanup function called at shutdown (e.g., avmgl_touch_destroy)
- `create_port_fn`: Function that creates individual port instances (e.g., touch_create_port)
- `handler_fn`: Message handler function for port communication (e.g., touch_message_handler)

### What it does:
- Registers the port driver with AtomVM
- Sets up global initialization and cleanup lifecycle
- Manages individual port instance creation
- Routes messages to the appropriate handler function
- Provides infrastructure for async communication from hardware

### Generated code structure:
    // Port driver registration
    static PORT_DRIVER: AtomVMPortDriver = AtomVMPortDriver {
        name: "<port_name>",
        init: Some(<init_fn>),
        destroy: Some(<destroy_fn>),
        create_port: <create_port_fn>,
        message_handler: <handler_fn>,
    };
    
    // Module registration
    #[no_mangle]
    pub extern "C" fn port_driver_init() -> *const AtomVMPortDriver {
        &PORT_DRIVER
    }

### Usage:
    port_collection!(
        <port_name>,
        init = <init_fn>,
        destroy = <destroy_fn>,
        create_port = <create_port_fn>,
        handler = <handler_fn>
    );

## Function Signatures:

### Global Init Function
    fn <init_fn>(_global: &mut GlobalContext) {
        // One-time hardware/driver initialization
        // Called when AtomVM starts up
    }

### Global Destroy Function  
    fn <destroy_fn>(_global: &mut GlobalContext) {
        // Global cleanup when AtomVM shuts down
        // Clean up shared resources, disable interrupts, etc.
    }

### Port Creation Function
    fn <create_port_fn>(_global: &GlobalContext, opts: Term) -> *mut Context {
        // Create and configure a new port instance
        // Parse options from Erlang
        // Allocate port-specific data structure
        // Return context pointer for this port instance
    }

### Message Handler Function
    fn <handler_fn>(ctx: &mut Context, message: &Message) -> PortResult {
        // Handle messages sent to this port from Erlang
        // Extract platform_data for port state
        // Process commands and send replies
        // Return Continue or Terminate
    }

## Example Usage:

### Complete Touch Controller Port
    port_collection!(
        avmgl_touch,
        init = avmgl_touch_init,
        destroy = avmgl_touch_destroy,
        create_port = touch_create_port,
        handler = touch_message_handler
    );

### Hardware SPI Port Example
    port_collection!(
        esp32_spi,
        init = spi_driver_init,
        destroy = spi_driver_cleanup,
        create_port = spi_create_port,
        handler = spi_message_handler
    );

## Port Data Management:

### Port-Specific Data Structure
    #[repr(C)]
    struct <PortName>Data {
        // Hardware configuration
        hardware_config: HardwareConfig,
        
        // Runtime state
        active: bool,
        owner_pid: u32,
        
        // Hardware handles/timers
        timer_handle: *mut c_void,
        interrupt_handle: *mut c_void,
    }

### Storing Port Data in Context
    fn <create_port_fn>(_global: &GlobalContext, opts: Term) -> *mut Context {
        let port_data = Box::new(<PortName>Data {
            // Initialize from opts
        });
        
        unsafe {
            let ctx = create_port_context(_global);
            (*ctx).platform_data = Box::into_raw(port_data) as *mut c_void;
            ctx
        }
    }

### Accessing Port Data in Handler
    fn <handler_fn>(ctx: &mut Context, message: &Message) -> PortResult {
        let port_data = unsafe { 
            &mut *((*ctx).platform_data as *mut <PortName>Data) 
        };
        
        // Use port_data for message processing
    }

## Message Handling Patterns:

### Standard Command Processing
    fn <handler_fn>(ctx: &mut Context, message: &Message) -> PortResult {
        let port_data = unsafe { 
            &mut *((*ctx).platform_data as *mut <PortName>Data) 
        };
    
        let (pid, reference, command) = parse_gen_message(message)?;
        
        match command.atom_name()? {
            "start" => {
                // Initialize hardware for this port instance
                port_data.owner_pid = pid.to_local_process_id();
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Continue
            }
            
            "configure" => {
                // Update hardware configuration
                let config = command.get_kv("config")?;
                port_data.apply_config(config)?;
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Continue
            }
            
            "read" => {
                // Read data from hardware
                let data = port_data.read_hardware_data()?;
                send_reply(ctx, pid, reference, tuple!("ok", data));
                PortResult::Continue
            }
            
            "stop" => {
                // Clean up this port instance
                port_data.cleanup_hardware();
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Terminate
            }
            
            _ => {
                send_reply(ctx, pid, reference, tuple!("error", atom!("unknown_command")));
                PortResult::Continue
            }
        }
    }

## Async Hardware Communication:

### ISR to Erlang Message Flow
    // Hardware interrupt handler
    extern "C" fn hardware_isr_handler(arg: *mut c_void) {
        let port_data = arg as *mut <PortName>Data;
        
        unsafe {
            // Read hardware data
            let data = read_hardware_registers();
            
            // Create Erlang message
            let event_msg = create_event_message(data);
            
            // Send to owning Erlang process
            port_send_message_from_task(
                global_context_ptr(),
                (*port_data).owner_pid,
                event_msg
            );
        }
    }

### Timer-Based Events
    extern "C" fn timer_callback(arg: *mut c_void) {
        let port_data = arg as *mut <PortName>Data;
        
        unsafe {
            let tick_msg = create_tick_message();
            port_send_message_from_task(
                global_context_ptr(),
                (*port_data).owner_pid, 
                tick_msg
            );
        }
    }

## Erlang Usage:

After loading the port driver, Erlang can create and use ports:

    % Create a port instance
    Port = open_port({spawn, "avmgl_touch"}, [binary, {packet, 4}]),
    
    % Send commands to the port
    Port ! {call, self(), make_ref(), start_touch},
    Port ! {call, self(), make_ref(), {start_animation, [{fps, 60}]}},
    
    % Receive replies and async messages
    receive
        {Port, {data, {ok, Data}}} -> handle_reply(Data);
        {Port, {data, {touch, X, Y}}} -> handle_touch_event(X, Y);
        {Port, {data, animation_tick}} -> handle_animation_frame()
    end.

## Port vs NIFs - When to Use Which:

### Use Ports When:
- Hardware generates asynchronous events (interrupts, timers)
- Need to send unsolicited messages to Erlang processes
- Long-running operations that shouldn't block the scheduler
- Hardware state needs to persist between calls
- Multiple Erlang processes need to interact with the same hardware

### Use NIFs When:
- Simple synchronous function calls
- Fast operations that return immediately
- Stateless or resource-based operations
- Direct function call semantics from Erlang

## Complete Touch Controller Example:

    #[repr(C)]
    struct TouchPortData {
        i2c_port: u8,
        interrupt_pin: u8,
        touch_threshold: u16,
        last_x: u16,
        last_y: u16,
        touch_active: bool,
        animation_timer: *mut c_void,
        owner_pid: u32,
    }

    fn avmgl_touch_init(_global: &mut GlobalContext) {
        unsafe {
            touch_controller_init();
            gpio_install_isr_service(0);
        }
    }

    fn avmgl_touch_destroy(_global: &mut GlobalContext) {
        unsafe {
            gpio_uninstall_isr_service();
            touch_controller_deinit();
        }
    }

    fn touch_create_port(_global: &GlobalContext, opts: Term) -> *mut Context {
        let port_data = Box::new(TouchPortData {
            i2c_port: opts.get_kv("i2c_port").unwrap_or(Term::from_u8(0)).to_u8().unwrap(),
            interrupt_pin: opts.get_kv("interrupt_pin").unwrap_or(Term::from_u8(4)).to_u8().unwrap(),
            touch_threshold: opts.get_kv("threshold").unwrap_or(Term::from_u16(100)).to_u16().unwrap(),
            last_x: 0,
            last_y: 0,
            touch_active: false,
            animation_timer: std::ptr::null_mut(),
            owner_pid: 0,
        });

        unsafe {
            let ctx = create_port_context(_global);
            (*ctx).platform_data = Box::into_raw(port_data) as *mut c_void;
            ctx
        }
    }

    fn touch_message_handler(ctx: &mut Context, message: &Message) -> PortResult {
        let port_data = unsafe { 
            &mut *((*ctx).platform_data as *mut TouchPortData) 
        };

        let (pid, reference, command) = parse_gen_message(message)?;
        
        match command.atom_name()? {
            "start_touch" => {
                port_data.owner_pid = pid.to_local_process_id();
                // Configure hardware interrupts...
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Continue
            }
            "stop" => {
                // Cleanup hardware...
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Terminate
            }
            _ => {
                send_reply(ctx, pid, reference, tuple!("error", atom!("unknown_command")));
                PortResult::Continue
            }
        }
    }

    port_collection!(
        avmgl_touch,
        init = avmgl_touch_init,
        destroy = avmgl_touch_destroy,  
        create_port = touch_create_port,
        handler = touch_message_handler
    );

## Parameter Summary:

- `port_name`         = Identifier for the entire port driver
- `init_fn`           = Global initialization function (hardware setup)
- `destroy_fn`        = Global cleanup function (hardware teardown)  
- `create_port_fn`    = Creates individual port instances with config
- `handler_fn`        = Processes messages sent to port instances
- `GlobalContext`     = AtomVM global state (usually unused)
- `Context`           = Individual port instance context
- `opts`              = Erlang term with port configuration options
- `Message`           = Incoming message from Erlang process
- `PortResult`        = Continue or Terminate the port instance
- `platform_data`     = Pointer to port-specific data structure