# Port Data Storage Paradigms - When and How to Store Different Types of Data

Shows the different ways ports can manage data based on usage patterns

## Three Data Storage Paradigms for Ports

### 1. AtomVM Resources - Erlang-Visible Data
For data that Erlang processes need to see, modify, or that should be garbage collected

### 2. Static/ISR Buffers - High-Performance Hardware Interface  
For data that ISRs write to and needs zero-allocation, atomic operations

### 3. Port-Managed Memory - Internal Port State
For port-specific state that's neither Erlang-visible nor ISR-critical

## Paradigm 1: AtomVM Resources (Erlang-Managed)

### When to Use:
- Data that Erlang processes need to access directly
- Configuration objects that should be garbage collected
- State that multiple NIFs/ports might share
- Data with complex lifetimes tied to Erlang processes

### Example: Display Configuration Resource
    // Register the resource type
    resource_type!(DISPLAY_CONFIG_RESOURCE, DisplayConfig, display_config_destructor);
    
    #[repr(C)]
    struct DisplayConfig {
        width: u16,
        height: u16,
        color_depth: u8,
        orientation: u8,
        backlight_level: u8,
    }
    
    fn display_create_port(_global: &GlobalContext, opts: Term) -> *mut Context {
        // Parse Erlang configuration
        let config = DisplayConfig {
            width: opts.get_kv("width").unwrap().to_u16().unwrap(),
            height: opts.get_kv("height").unwrap().to_u16().unwrap(),
            color_depth: opts.get_kv("depth").unwrap_or(Term::from_u8(16)).to_u8().unwrap(),
            orientation: opts.get_kv("orientation").unwrap_or(Term::from_u8(0)).to_u8().unwrap(),
            backlight_level: opts.get_kv("backlight").unwrap_or(Term::from_u8(128)).to_u8().unwrap(),
        };
        
        // Create AtomVM-managed resource
        let config_ptr = create_resource!(DISPLAY_CONFIG_RESOURCE, config).unwrap();
        let config_term = make_resource_term!(env, config_ptr);
        
        // Port context holds reference to prevent GC
        let ctx = create_port_context(_global);
        (*ctx).set_user_data(config_term.as_raw()); // Hypothetical API
        ctx
    }
    
    fn display_message_handler(ctx: &mut Context, message: &Message) -> PortResult {
        // Extract resource from context
        let config_term = Term::from_raw((*ctx).get_user_data());
        let display_config = get_resource!(env, config_term, DISPLAY_CONFIG_RESOURCE).unwrap();
        
        // Use Erlang-managed configuration
        match command.atom_name().unwrap() {
            "set_backlight" => {
                let level = command.get_kv("level").unwrap().to_u8().unwrap();
                display_config.backlight_level = level;
                set_display_backlight(level);
                PortResult::Continue
            }
            // ...
        }
    }

## Paradigm 2: Static/ISR Buffers (Hardware-Critical)

### When to Use:
- ISR functions need to write data
- Zero-allocation requirements
- Atomic operations needed
- Ring buffers for streaming data
- Hardware event queues

### Example: Touch Event Ring Buffer (ISR → Erlang)
    use core::sync::atomic::{AtomicUsize, Ordering};
    
    // Static allocation - never freed, ISR-safe
    static mut TOUCH_RING_BUFFER: TouchRingBuffer = TouchRingBuffer::new();
    static TOUCH_BUFFER_INITIALIZED: AtomicBool = AtomicBool::new(false);
    
    #[repr(C)]
    struct TouchRingBuffer {
        events: [TouchEvent; 256],        // Fixed-size ring buffer
        write_pos: AtomicUsize,           // ISR writes here
        read_pos: AtomicUsize,            // Port reads here
        dropped_events: AtomicUsize,      // Overflow counter
    }
    
    #[repr(C)]
    struct TouchEvent {
        timestamp: u64,
        x: u16,
        y: u16,
        pressure: u8,
        event_type: u8, // touch_down, touch_up, touch_move
    }
    
    impl TouchRingBuffer {
        const fn new() -> Self {
            Self {
                events: [TouchEvent::empty(); 256],
                write_pos: AtomicUsize::new(0),
                read_pos: AtomicUsize::new(0),
                dropped_events: AtomicUsize::new(0),
            }
        }
        
        // Called from ISR - must be fast and lock-free
        unsafe fn push_from_isr(&self, event: TouchEvent) {
            let current_write = self.write_pos.load(Ordering::Relaxed);
            let next_write = (current_write + 1) % self.events.len();
            let current_read = self.read_pos.load(Ordering::Acquire);
            
            // Check if buffer is full
            if next_write == current_read {
                self.dropped_events.fetch_add(1, Ordering::Relaxed);
                return; // Drop the event
            }
            
            // Write event atomically
            self.events[current_write] = event;
            self.write_pos.store(next_write, Ordering::Release);
        }
        
        // Called from port context - can block briefly
        fn pop_events(&self, events: &mut Vec<TouchEvent>) -> usize {
            let current_read = self.read_pos.load(Ordering::Relaxed);
            let current_write = self.write_pos.load(Ordering::Acquire);
            
            if current_read == current_write {
                return 0; // No events
            }
            
            let mut read_pos = current_read;
            let mut count = 0;
            
            while read_pos != current_write {
                events.push(self.events[read_pos]);
                read_pos = (read_pos + 1) % self.events.len();
                count += 1;
            }
            
            self.read_pos.store(read_pos, Ordering::Release);
            count
        }
    }
    
    // ISR handler - hardware interrupt
    #[no_mangle]
    extern "C" fn touch_isr_handler() {
        if !TOUCH_BUFFER_INITIALIZED.load(Ordering::Acquire) {
            return;
        }
        
        let event = TouchEvent {
            timestamp: get_system_timestamp(),
            x: read_touch_x_register(),
            y: read_touch_y_register(),
            pressure: read_touch_pressure_register(),
            event_type: read_touch_event_type_register(),
        };
        
        unsafe {
            TOUCH_RING_BUFFER.push_from_isr(event);
        }
    }
    
    fn touch_create_port(_global: &GlobalContext, opts: Term) -> *mut Context {
        // Initialize the static buffer once
        if !TOUCH_BUFFER_INITIALIZED.swap(true, Ordering::AcqRel) {
            unsafe {
                TOUCH_RING_BUFFER = TouchRingBuffer::new();
            }
            
            // Set up hardware interrupt
            unsafe {
                configure_touch_interrupt(touch_isr_handler);
            }
        }
        
        create_port_context(_global)
    }
    
    fn touch_message_handler(ctx: &mut Context, message: &Message) -> PortResult {
        let (pid, reference, command) = parse_gen_message(message)?;
        
        match command.atom_name()? {
            "read_events" => {
                let mut events = Vec::new();
                unsafe {
                    let count = TOUCH_RING_BUFFER.pop_events(&mut events);
                }
                
                if events.is_empty() {
                    send_reply(ctx, pid, reference, atom!("no_events"));
                } else {
                    let event_terms: Vec<Term> = events.iter()
                        .map(|e| tuple!("touch", e.x, e.y, e.pressure, e.timestamp))
                        .collect();
                    send_reply(ctx, pid, reference, tuple!("events", Term::from_list(&event_terms)));
                }
                PortResult::Continue
            }
            
            "get_stats" => {
                let dropped = unsafe { TOUCH_RING_BUFFER.dropped_events.load(Ordering::Relaxed) };
                send_reply(ctx, pid, reference, tuple!("stats", dropped));
                PortResult::Continue
            }
            
            _ => PortResult::Continue
        }
    }

## Paradigm 3: Port-Managed Memory (Internal State)

### When to Use:
- Port-specific configuration and state
- Timers, file handles, hardware contexts
- Data that's neither Erlang-visible nor ISR-critical
- Complex objects that need custom cleanup

### Example: Audio Codec Port with Internal State
    struct AudioCodecState {
        // Hardware configuration
        sample_rate: u32,
        bit_depth: u8,
        channels: u8,
        
        // Runtime state
        is_playing: bool,
        current_volume: u8,
        
        // Hardware handles
        dma_handle: *mut c_void,
        timer_handle: *mut c_void,
        codec_i2c_handle: *mut c_void,
        
        // Internal buffers (not ISR-accessed)
        playback_buffer: Vec<u8>,
        processing_buffer: Vec<f32>,
    }
    
    impl AudioCodecState {
        fn new(opts: Term) -> Self {
            Self {
                sample_rate: opts.get_kv("sample_rate").unwrap_or(Term::from_u32(44100)).to_u32().unwrap(),
                bit_depth: opts.get_kv("bit_depth").unwrap_or(Term::from_u8(16)).to_u8().unwrap(),
                channels: opts.get_kv("channels").unwrap_or(Term::from_u8(2)).to_u8().unwrap(),
                
                is_playing: false,
                current_volume: 128,
                
                dma_handle: std::ptr::null_mut(),
                timer_handle: std::ptr::null_mut(),
                codec_i2c_handle: std::ptr::null_mut(),
                
                playback_buffer: Vec::with_capacity(4096),
                processing_buffer: Vec::with_capacity(2048),
            }
        }
        
        fn initialize_hardware(&mut self) -> Result<(), AudioError> {
            unsafe {
                self.codec_i2c_handle = i2c_init(I2C_PORT_0, self.sample_rate);
                self.dma_handle = dma_init_audio_channel();
                self.timer_handle = timer_init_audio_clock(self.sample_rate);
            }
            Ok(())
        }
        
        fn cleanup(&mut self) {
            unsafe {
                if !self.dma_handle.is_null() {
                    dma_deinit(self.dma_handle);
                    self.dma_handle = std::ptr::null_mut();
                }
                if !self.timer_handle.is_null() {
                    timer_deinit(self.timer_handle);
                    self.timer_handle = std::ptr::null_mut();
                }
                if !self.codec_i2c_handle.is_null() {
                    i2c_deinit(self.codec_i2c_handle);
                    self.codec_i2c_handle = std::ptr::null_mut();
                }
            }
        }
    }
    
    fn audio_create_port(_global: &GlobalContext, opts: Term) -> *mut Context {
        let mut audio_state = Box::new(AudioCodecState::new(opts));
        
        if let Err(_) = audio_state.initialize_hardware() {
            return std::ptr::null_mut(); // Port creation failed
        }
        
        let ctx = create_port_context(_global);
        unsafe {
            (*ctx).platform_data = Box::into_raw(audio_state) as *mut c_void;
        }
        ctx
    }
    
    fn audio_message_handler(ctx: &mut Context, message: &Message) -> PortResult {
        let audio_state = unsafe {
            &mut *((*ctx).platform_data as *mut AudioCodecState)
        };
        
        let (pid, reference, command) = parse_gen_message(message)?;
        
        match command.atom_name()? {
            "play_buffer" => {
                let audio_data = command.get_kv("data")?.to_binary()?;
                audio_state.playback_buffer.clear();
                audio_state.playback_buffer.extend_from_slice(&audio_data);
                
                // Start DMA playback
                unsafe {
                    dma_start_playback(audio_state.dma_handle, audio_state.playback_buffer.as_ptr());
                }
                audio_state.is_playing = true;
                
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Continue
            }
            
            "set_volume" => {
                let volume = command.get_kv("volume")?.to_u8()?;
                audio_state.current_volume = volume;
                
                unsafe {
                    codec_set_volume(audio_state.codec_i2c_handle, volume);
                }
                
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Continue
            }
            
            "stop" => {
                audio_state.cleanup();
                send_reply(ctx, pid, reference, atom!("ok"));
                PortResult::Terminate
            }
            
            _ => PortResult::Continue
        }
    }
    
    // Port cleanup - called when port terminates
    extern "C" fn audio_port_cleanup(ctx: *mut Context) {
        if !ctx.is_null() {
            let audio_state = unsafe {
                Box::from_raw((*ctx).platform_data as *mut AudioCodecState)
            };
            // AudioCodecState::Drop will call cleanup()
        }
    }

## Data Flow Patterns

### ISR → Static Buffer → Port → Erlang
    [Hardware] → [ISR writes to static ring buffer] → [Port polls buffer] → [Send to Erlang]

### Erlang → Resource → Port → Hardware  
    [Erlang config] → [AtomVM resource] → [Port reads resource] → [Configure hardware]

### Port Internal State Management
    [Port creation] → [Allocate internal state] → [Initialize hardware] → [Process messages] → [Cleanup on termination]

## Storage Decision Matrix

| Data Type | Erlang Visible? | ISR Access? | Storage Choice |
|-----------|-----------------|-------------|----------------|
| Configuration | Yes | No | AtomVM Resource |
| Event Queue | No | Yes (Write) | Static Ring Buffer |
| Hardware Handles | No | No | Port-Managed Memory |
| Streaming Data | Partially | Yes | Static Buffer + Resource |
| Port State | No | No | Port-Managed Memory |

## Memory Safety Considerations

### Static Buffers:
- Use atomic operations for ISR safety
- Fixed-size allocation (no malloc in ISR)
- Lock-free data structures only
- Consider cache line alignment

### Resources:
- Automatic cleanup via AtomVM GC
- Type-safe extraction
- Reference counting handled automatically
- Can be passed between processes

### Port-Managed:
- Manual cleanup required in port destructor
- Use RAII patterns (Drop trait)
- Careful pointer management
- No sharing between threads without synchronization

## Complete Example: Multi-Paradigm SPI Port

    // Paradigm 1: Erlang-visible SPI configuration
    resource_type!(SPI_CONFIG_RESOURCE, SpiConfig, spi_config_destructor);
    
    // Paradigm 2: ISR-written transaction queue
    static mut SPI_TX_QUEUE: SpiRingBuffer = SpiRingBuffer::new();
    
    // Paradigm 3: Port internal state
    struct SpiPortState {
        config_resource: Term,        // Reference to Paradigm 1
        hardware_handle: *mut c_void, // Internal hardware state
        transaction_id: AtomicU32,    // For tracking async operations
    }
    
    fn spi_create_port(_global: &GlobalContext, opts: Term) -> *mut Context {
        // Create Erlang-managed config
        let config = SpiConfig::from_opts(opts);
        let config_ptr = create_resource!(SPI_CONFIG_RESOURCE, config).unwrap();
        let config_term = make_resource_term!(env, config_ptr);
        
        // Initialize ISR buffer
        unsafe {
            SPI_TX_QUEUE.initialize();
        }
        
        // Create port-managed state
        let port_state = Box::new(SpiPortState {
            config_resource: config_term,
            hardware_handle: std::ptr::null_mut(),
            transaction_id: AtomicU32::new(0),
        });
        
        let ctx = create_port_context(_global);
        unsafe {
            (*ctx).platform_data = Box::into_raw(port_state) as *mut c_void;
        }
        ctx
    }
