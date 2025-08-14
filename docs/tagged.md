# Tagged Maps - Sending Rust Data to Erlang

The `tagged` module provides automatic serialization of Rust types into Erlang maps with type discriminators, creating a seamless bridge between Rust ports/NIFs and Erlang processes.

## Quick Start

### 1. Basic Usage

```rust,ignore
use avmnif::tagged::{TaggedMap, TaggedResult};

// Any type implementing TaggedMap can be sent to Erlang
let data = 42i32;
let erlang_map = data.to_tagged_map(&ctx)?;

// Becomes this Erlang map:
// #{type => i32, value => 42}
```

### 2. With Custom Structs (using derive macro)

```rust,ignore
use avmnif::tagged::TaggedMap;

#[derive(TaggedMap)]
struct SensorReading {
    temperature: f32,
    humidity: f32,
    timestamp: u64,
    active: bool,
}

let reading = SensorReading {
    temperature: 23.5,
    humidity: 45.2,
    timestamp: 1634567890,
    active: true,
};

let erlang_map = reading.to_tagged_map(&ctx)?;
```

This creates an Erlang map like:
```erlang
#{
  type => sensor_reading,
  temperature => 23.5,
  humidity => 45.2, 
  timestamp => 1634567890,
  active => true
}
```

## Type Mapping

### Primitive Types

| Rust Type | Erlang Map | Notes |
|-----------|------------|-------|
| `i32` | `#{type => i32, value => 42}` | Small integers |
| `String` | `#{type => string, value => <<"hello">>}` | UTF-8 binary |
| `bool` | `#{type => bool, value => true}` | Atoms `true`/`false` |
| `f64` | `#{type => f64, value => 3.14}` | Floating point |

### Container Types

| Rust Type | Erlang Map | Notes |
|-----------|------------|-------|
| `Option<T>` | `#{type => option, variant => some, value => T}` | Some variant |
| `Option<T>` | `#{type => option, variant => nil}` | None variant |
| `Vec<T>` | `#{type => vec, elements => [T1, T2, ...]}` | List of tagged elements |

### Custom Types

```rust,ignore
#[derive(TaggedMap)]
struct Point {
    x: f32,
    y: f32,
}

// Becomes: #{type => point, x => 1.0, y => 2.0}
```

```rust,ignore
#[derive(TaggedMap)]
enum Status {
    Ready,
    Processing { progress: u8 },
    Error { code: u32, message: String },
}

// Examples:
// #{type => status, variant => ready}
// #{type => status, variant => processing, progress => 75}
// #{type => status, variant => error, code => 404, message => <<"Not found">>}
```

## Real-World Examples

### 1. GPIO Pin Management

```rust,ignore
use avmnif::tagged::TaggedMap;

#[derive(TaggedMap)]
struct GpioState {
    pins: Vec<PinState>,
    configuration: GpioConfig,
    interrupt_status: InterruptStatus,
}

#[derive(TaggedMap)]
struct PinState {
    pin_number: u8,
    mode: PinMode,
    value: bool,
    pull_resistor: PullResistor,
}

#[derive(TaggedMap)]
enum PinMode {
    Input,
    Output,
    Pwm { frequency: u32, duty_cycle: u8 },
    Analog,
}

#[derive(TaggedMap)]
enum PullResistor {
    None,
    PullUp,
    PullDown,
}

#[derive(TaggedMap)]
struct GpioConfig {
    board_type: String,
    voltage_level: f32,
    max_current: u16,
}

#[derive(TaggedMap)]
enum InterruptStatus {
    None,
    Triggered { pin: u8, edge: EdgeType, timestamp: u64 },
}

#[derive(TaggedMap)]
enum EdgeType {
    Rising,
    Falling,
    Both,
}

// In your port handler:
fn handle_gpio_message(ctx: &mut Context, message: &Message) -> PortResult {
    let (pid, reference, _command) = parse_gen_message(message)?;
    
    // Read current GPIO state
    let state = GpioState {
        pins: vec![
            PinState {
                pin_number: 18,
                mode: PinMode::Pwm { frequency: 1000, duty_cycle: 75 },
                value: true,
                pull_resistor: PullResistor::None,
            },
            PinState {
                pin_number: 22,
                mode: PinMode::Input,
                value: false,
                pull_resistor: PullResistor::PullUp,
            }
        ],
        configuration: GpioConfig {
            board_type: "ESP32".to_string(),
            voltage_level: 3.3,
            max_current: 500,
        },
        interrupt_status: InterruptStatus::Triggered {
            pin: 22,
            edge: EdgeType::Falling,
            timestamp: get_timestamp(),
        },
    };
    
    // Convert to Erlang-compatible format
    let erlang_data = state.to_tagged_map(ctx)?;
    
    // Send to Erlang
    send_reply(ctx, pid, reference, erlang_data);
    
    PortResult::Continue
}
```

### 2. I2C Device Communication

```rust,ignore
#[derive(TaggedMap)]
enum I2cResponse {
    Success {
        device_address: u8,
        register: u8,
        data: Vec<u8>,
    },
    Error {
        error_type: I2cErrorType,
        device_address: u8,
        details: String,
    },
}

#[derive(TaggedMap)]
struct I2cDeviceInfo {
    address: u8,
    name: String,
    registers: Vec<RegisterInfo>,
    status: DeviceStatus,
}

#[derive(TaggedMap)]
struct RegisterInfo {
    address: u8,
    name: String,
    access: RegisterAccess,
    value: Option<u8>,
}

#[derive(TaggedMap)]
enum RegisterAccess {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[derive(TaggedMap)]
enum I2cErrorType {
    NoAcknowledge,
    BusError,
    TimeoutError,
    InvalidAddress,
}

#[derive(TaggedMap)]
enum DeviceStatus {
    Connected,
    Disconnected,
    Error { code: u8 },
}

// Usage:
let response = I2cResponse::Success {
    device_address: 0x48,
    register: 0x00,
    data: vec![0x12, 0x34, 0x56],
};

let erlang_response = response.to_tagged_map(ctx)?;
```

## Erlang Side Usage

### Pattern Matching

```erlang
handle_message({Port, {data, Data}}) ->
    case Data of
        #{type := gpio_state, pins := Pins, interrupt_status := InterruptStatus} ->
            process_gpio_data(Pins, InterruptStatus);
        
        #{type := i2c_response, variant := success, device_address := Addr, data := Data} ->
            handle_i2c_success(Addr, Data);
            
        #{type := i2c_response, variant := error, error_type := ErrorType} ->
            handle_i2c_error(ErrorType);
            
        _ ->
            {error, unknown_message_type}
    end.
```

### Extracting Hardware Data

```erlang
process_gpio_data(Pins, InterruptStatus) ->
    % Extract individual pin states
    PinsList = maps:get(elements, Pins),
    
    ProcessedPins = [
        begin
            #{pin_number := Num, mode := Mode, value := Value} = Pin,
            case Mode of
                #{variant := pwm, frequency := Freq, duty_cycle := Duty} ->
                    {Num, pwm, Value, {Freq, Duty}};
                #{variant := input} ->
                    {Num, input, Value};
                #{variant := output} ->
                    {Num, output, Value}
            end
        end || Pin <- PinsList
    ],
    
    % Check interrupt status
    case InterruptStatus of
        #{variant := none} ->
            {ok, ProcessedPins};
        #{variant := triggered, pin := Pin, edge := Edge, timestamp := Time} ->
            {interrupt, ProcessedPins, {Pin, Edge, Time}}
    end.
```

### Creating Hardware Commands

```erlang
% Send GPIO command to Rust port
Command = #{
    type => gpio_command,
    action => set_pin,
    pin_number => 18,
    mode => #{variant => output},
    value => true
},

port_command(Port, term_to_binary(Command)).

% Send I2C read command
I2cCommand = #{
    type => i2c_command,
    action => read_register,
    device_address => 16#48,
    register => 16#00,
    byte_count => 2
},

port_command(Port, term_to_binary(I2cCommand)).
```

## Advanced Patterns

### 1. ADC/DAC Operations

```rust,ignore
#[derive(TaggedMap)]
struct AdcReading {
    channel: u8,
    raw_value: u16,
    voltage: f32,
    timestamp: u64,
    gain: AdcGain,
}

#[derive(TaggedMap)]
enum AdcGain {
    Gain1x,
    Gain2x,
    Gain4x,
    Gain8x,
}

#[derive(TaggedMap)]
struct DacOutput {
    channel: u8,
    value: u16,
    voltage: f32,
    enabled: bool,
}
```

### 2. SPI Device Configuration

```rust,ignore
#[derive(TaggedMap)]
enum SpiOperation {
    Transfer { 
        device_id: u8, 
        tx_data: Vec<u8>, 
        rx_data: Vec<u8> 
    },
    Configure { 
        device_id: u8, 
        config: SpiConfig 
    },
    SetChipSelect { 
        device_id: u8, 
        active: bool 
    },
}

#[derive(TaggedMap)]
struct SpiConfig {
    clock_speed: u32,
    mode: SpiMode,
    bit_order: BitOrder,
    chip_select_polarity: bool,
}

#[derive(TaggedMap)]
enum SpiMode {
    Mode0,  // CPOL=0, CPHA=0
    Mode1,  // CPOL=0, CPHA=1
    Mode2,  // CPOL=1, CPHA=0
    Mode3,  // CPOL=1, CPHA=1
}

#[derive(TaggedMap)]
enum BitOrder {
    MsbFirst,
    LsbFirst,
}
```

### 3. Timer and PWM Control

```rust,ignore
#[derive(TaggedMap)]
enum TimerOperation {
    Start { timer_id: u8, period_us: u32 },
    Stop { timer_id: u8 },
    GetElapsed { timer_id: u8 },
    SetCallback { timer_id: u8, callback_type: CallbackType },
}

#[derive(TaggedMap)]
enum CallbackType {
    None,
    Interrupt,
    Message { target_pid: u32 },
}

#[derive(TaggedMap)]
struct PwmConfig {
    channel: u8,
    frequency: u32,
    duty_cycle: u8,
    polarity: PwmPolarity,
    enabled: bool,
}

#[derive(TaggedMap)]
enum PwmPolarity {
    Normal,
    Inverted,
}
```

### 3. Error Handling for Hardware Operations

```rust,ignore
#[derive(TaggedMap)]
enum HardwareResult {
    Success { data: Vec<u8> },
    Warning { data: Vec<u8>, message: String },
    Error { error_type: HardwareErrorType, details: String },
}

#[derive(TaggedMap)]
enum HardwareErrorType {
    InvalidPin,
    BusError,
    DeviceNotFound,
    ConfigurationError,
    TimeoutError,
}

// In port handler:
let result = match configure_spi_device(device_id, config) {
    Ok(data) => HardwareResult::Success { data },
    Err(e) => HardwareResult::Error {
        error_type: HardwareErrorType::ConfigurationError,
        details: e.to_string(),
    },
};

send_reply(ctx, pid, reference, result.to_tagged_map(ctx)?);
```

## Best Practices

### 1. Naming Conventions

- **Rust structs** → **snake_case atoms**: `SensorReading` becomes `sensor_reading`
- **Enum variants** → **snake_case atoms**: `NetworkError` becomes `network_error`
- **Field names** → **snake_case atoms**: `lastLogin` becomes `last_login`

### 2. Type Design for Hardware

```rust,ignore
// ✅ Good - Clear, hardware-specific types
#[derive(TaggedMap)]
enum GpioResponse {
    PinState { pin: u8, value: bool },
    ConfigChanged { pin: u8, mode: PinMode },
    InterruptTriggered { pin: u8, edge: EdgeType, timestamp: u64 },
}

// ❌ Avoid - Ambiguous, hard to pattern match
#[derive(TaggedMap)]
struct GenericHardwareResponse {
    success: bool,
    pin: Option<u8>,
    value: Option<bool>,
    error: Option<String>,
}
```

### 3. Error Handling Strategy for Embedded Systems

```rust,ignore
// Consistent error types across hardware operations
#[derive(TaggedMap)]
enum HardwareError {
    InvalidConfiguration { component: String, details: String },
    BusError { bus_type: String, address: u8, operation: String },
    TimeoutError { operation: String, timeout_ms: u32 },
    ResourceBusy { resource: String },
}

// Always wrap results in a standard envelope
#[derive(TaggedMap)]
enum HardwareMessage {
    Success { operation_id: String, data: HardwareData },
    Error { operation_id: String, error: HardwareError },
}

#[derive(TaggedMap)]
enum HardwareData {
    GpioState(GpioState),
    AdcReading(AdcReading),
    I2cResponse(I2cResponse),
    SpiResponse(Vec<u8>),
}
```

### 4. Performance Considerations for Embedded Systems

- **Cache common atoms**: The atom table is global, so frequently used hardware atoms are automatically cached
- **Minimize nesting**: Deep hardware structures create more allocation overhead
- **Use appropriate types**: `u8` for pin numbers is more efficient than `String`
- **Batch operations**: Group multiple GPIO operations into single messages when possible

## Troubleshooting

### Common Issues

1. **Type mismatch errors**: Ensure your hardware types implement `TaggedMap`
2. **Atom creation failures**: Very long field names may hit atom table limits
3. **Memory issues**: Large data structures (like big ADC buffers) may exceed heap limits

### Debug Tips for Hardware Development

```rust,ignore
// Enable debug output to see generated maps
let tagged = gpio_state.to_tagged_map(ctx)?;
println!("Generated GPIO map: {:?}", tagged);

// Test roundtrip conversion for hardware data
let original = AdcReading { channel: 0, raw_value: 2048, voltage: 3.3, timestamp: 12345, gain: AdcGain::Gain1x };
let tagged = original.to_tagged_map(ctx)?;
let roundtrip = AdcReading::from_tagged_map(tagged)?;
assert_eq!(original, roundtrip);
```

## Integration Examples

### GenServer Integration for Hardware Control

```erlang
-module(gpio_server).
-behaviour(gen_server).

init([]) ->
    Port = open_port({spawn, "gpio_port"}, [binary]),
    {ok, #{port => Port}}.

handle_call({read_pin, PinNumber}, _From, #{port := Port} = State) ->
    Command = #{type => gpio_command, action => read_pin, pin_number => PinNumber},
    port_command(Port, term_to_binary(Command)),
    
    receive
        {Port, {data, Data}} ->
            case binary_to_term(Data) of
                #{type := gpio_response, variant := pin_state, pin := Pin, value := Value} ->
                    {reply, {ok, {Pin, Value}}, State};
                #{type := hardware_error, error_type := ErrorType} ->
                    {reply, {error, ErrorType}, State}
            end
    after 5000 ->
        {reply, {error, timeout}, State}
    end.

handle_call({configure_pwm, Channel, Frequency, DutyCycle}, _From, #{port := Port} = State) ->
    PwmConfig = #{
        channel => Channel,
        frequency => Frequency,
        duty_cycle => DutyCycle,
        polarity => #{variant => normal},
        enabled => true
    },
    Command = #{type => pwm_command, action => configure, config => PwmConfig},
    port_command(Port, term_to_binary(Command)),
    
    receive
        {Port, {data, Data}} ->
            case binary_to_term(Data) of
                #{type := hardware_message, variant := success} ->
                    {reply, ok, State};
                #{type := hardware_message, variant => error, error := Error} ->
                    {reply, {error, Error}, State}
            end
    after 2000 ->
        {reply, {error, timeout}, State}
    end.
```

### Supervision Tree Integration for Hardware Processes

```erlang
% Child spec for hardware port process
#{
  id => hardware_port,
  start => {hardware_port_worker, start_link, []},
  restart => permanent,
  shutdown => 5000,
  type => worker
}
```

This tagged map system provides a robust, type-safe way to communicate complex data structures between Rust and Erlang while maintaining the flexibility and pattern-matching capabilities that make Erlang powerful.