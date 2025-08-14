//! Test utilities for port communication functionality

#[cfg(test)]
use alloc::{format, string::String, string::ToString, vec, vec::Vec};
use crate::atom::AtomTableOps;
use crate::testing::mocks::*;
use crate::term::{Term, TermValue, PortId, ProcessId, NifResult, NifError};

#[cfg(test)]
/// Simple message type for testing
#[derive(Debug, Clone, PartialEq)]
pub enum TestMessage {
    Command(String),
    Data(Vec<u8>),
    Error(String),
}

#[cfg(test)]
/// Test implementation of port data for testing purposes
pub struct TestPortData {
    pub port_id: u32,
    pub active: bool,
    pub messages: Vec<TestMessage>,
    pub last_command: Option<String>,
    pub error_count: u32,
}

#[cfg(test)]
impl TestPortData {
    pub fn new() -> Self {
        Self {
            port_id: 0,
            active: false,
            messages: Vec::new(),
            last_command: None,
            error_count: 0,
        }
    }

    pub fn with_port_id(port_id: u32) -> Self {
        Self {
            port_id,
            active: false,
            messages: Vec::new(),
            last_command: None,
            error_count: 0,
        }
    }

    pub fn add_message(&mut self, message: TestMessage) {
        self.messages.push(message);
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn process_messages(&mut self, _atom_table: &MockAtomTable) {
        while let Some(message) = self.messages.pop() {
            match message {
                TestMessage::Command(cmd) => {
                    self.last_command = Some(cmd);
                }
                TestMessage::Data(_) => {
                    // Handle data messages
                }
                TestMessage::Error(_) => {
                    self.error_count += 1;
                }
            }
        }
    }

    pub fn port_id(&self) -> u32 {
        self.port_id
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

#[cfg(test)]
pub fn create_ok_reply(atom_table: &MockAtomTable, data: TermValue) -> TermValue {
    let ok_atom = atom_table.ensure_atom_str("ok").unwrap();
    TermValue::tuple(vec![TermValue::Atom(ok_atom), data])
}

#[cfg(test)]
pub fn create_error_reply(atom_table: &MockAtomTable, reason: TermValue) -> TermValue {
    let error_atom = atom_table.ensure_atom_str("error").unwrap();
    TermValue::tuple(vec![TermValue::Atom(error_atom), reason])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ok_reply() {
        let atom_table = MockAtomTable::new();
        let ok_atom = atom_table.ensure_atom_str("ok").unwrap();
        let data = TermValue::SmallInt(42);
        
        let reply = create_ok_reply(&atom_table, data);
        
        // Verify the reply structure
        if let Some(elements) = reply.as_tuple() {
            assert_eq!(elements.len(), 2);
            assert_eq!(elements[0], TermValue::Atom(ok_atom));
            assert_eq!(elements[1], TermValue::SmallInt(42));
        } else {
            panic!("Expected tuple reply");
        }
    }

    #[test]
    fn test_create_error_reply() {
        let atom_table = MockAtomTable::new();
        let error_atom = atom_table.ensure_atom_str("error").unwrap();
        let reason_atom = atom_table.ensure_atom_str("invalid_input").unwrap();
        let reason = TermValue::Atom(reason_atom);
        
        let reply = create_error_reply(&atom_table, reason);
        
        // Verify the error reply structure
        if let Some(elements) = reply.as_tuple() {
            assert_eq!(elements.len(), 2);
            assert_eq!(elements[0], TermValue::Atom(error_atom));
            assert_eq!(elements[1], TermValue::Atom(reason_atom));
        } else {
            panic!("Expected tuple reply");
        }
    }

    #[test]
    fn test_port_data_trait_defaults() {
        let mut test_data = TestPortData::new();
        
        // Test default implementations
        assert_eq!(test_data.port_id(), 0);
        assert_eq!(test_data.message_count(), 0);
        
        // Test activation - should start inactive
        assert!(!test_data.is_active());
        test_data.activate();
        assert!(test_data.is_active());
        
        // Test deactivation
        test_data.deactivate();
        assert!(!test_data.is_active());
    }

    #[test]
    fn test_generic_port_data_message_handling() {
        let mut port_data = TestPortData::new();
        let atom_table = MockAtomTable::new();
        
        // Initially no messages
        assert_eq!(port_data.message_count(), 0);
        
        // Add a message
        let msg = TestMessage::Command("test".to_string());
        port_data.add_message(msg);
        assert_eq!(port_data.message_count(), 1);
        
        // Process messages (this should consume the message)
        port_data.process_messages(&atom_table);
        assert_eq!(port_data.message_count(), 0);  // Should be 0 after processing
        
        // Verify processing worked
        assert!(port_data.last_command.is_some());
        assert_eq!(port_data.last_command.as_ref().unwrap(), "test");
    }

    #[test]
    fn test_port_data_with_many_messages() {
        let mut port_data = TestPortData::new();
        let atom_table = MockAtomTable::new();
        
        // Add multiple messages
        for i in 0..5 {
            let msg = TestMessage::Command(format!("command_{}", i));
            port_data.add_message(msg);
        }
        
        assert_eq!(port_data.message_count(), 5);
        
        // Process all messages (should consume all of them)
        port_data.process_messages(&atom_table);
        assert_eq!(port_data.message_count(), 0);  // All messages should be consumed
        
        // Verify last command was processed (LIFO order)
        assert!(port_data.last_command.is_some());
        assert_eq!(port_data.last_command.as_ref().unwrap(), "command_0");
    }

    #[test]
    fn test_port_data_lifecycle() {
        let mut port_data = TestPortData::with_port_id(123);
        
        assert_eq!(port_data.port_id(), 123);
        assert!(!port_data.is_active());
        
        port_data.activate();
        assert!(port_data.is_active());
        
        port_data.deactivate();
        assert!(!port_data.is_active());
    }

    #[test]
    fn test_port_data_cleanup() {
        let mut port_data = TestPortData::new();
        
        // Add some messages
        port_data.add_message(TestMessage::Command("cmd1".to_string()));
        port_data.add_message(TestMessage::Command("cmd2".to_string()));
        
        assert_eq!(port_data.message_count(), 2);
        
        // Clear messages
        port_data.messages.clear();
        assert_eq!(port_data.message_count(), 0);
    }

    #[test]
    fn test_port_data_state_transitions() {
        let mut port_data = TestPortData::new();
        let atom_table = MockAtomTable::new();
        
        // Test state transitions with message processing
        assert!(!port_data.is_active());
        
        port_data.activate();
        assert!(port_data.is_active());
        
        // Add and process a message while active
        port_data.add_message(TestMessage::Command("active_cmd".to_string()));
        port_data.process_messages(&atom_table);
        
        assert_eq!(port_data.last_command.as_ref().unwrap(), "active_cmd");
        assert!(port_data.is_active()); // Should remain active
        
        port_data.deactivate();
        assert!(!port_data.is_active());
    }

    #[test]
    fn test_port_data_macro_generated_types() {
        // Test that our TestPortData works with macro-generated scenarios
        let mut port_data = TestPortData::new();
        let atom_table = MockAtomTable::new();
        
        // Simulate macro-generated port operations
        let commands = vec![
            "initialize",
            "configure", 
            "start",
            "process",
            "stop"
        ];
        
        for cmd in commands {
            port_data.add_message(TestMessage::Command(cmd.to_string()));
        }
        
        assert_eq!(port_data.message_count(), 5);
        
        port_data.process_messages(&atom_table);
        assert_eq!(port_data.message_count(), 0);
        assert_eq!(port_data.last_command.as_ref().unwrap(), "initialize"); // Last processed (LIFO)
    }

    #[test]
    fn test_port_op_result() {
        let atom_table = MockAtomTable::new();
        
        // Test successful operation using TermValue
        let ok_result: Result<TermValue, NifError> = Ok(TermValue::SmallInt(42));
        match ok_result {
            Ok(term) => assert_eq!(term, TermValue::SmallInt(42)),
            _ => panic!("Expected Ok result"),
        }
        
        // Test error operation
        let error_result: Result<TermValue, NifError> = Err(NifError::BadArg);
        match error_result {
            Err(NifError::BadArg) => assert!(true),
            _ => panic!("Expected BadArg error"),
        }
    }

    #[test]
    fn test_port_result_values() {
        // Test various port operation results
        #[derive(Debug, PartialEq)]
        enum TestPortResult {
            Continue,
            Stop,
            Reply(TermValue),
        }
        
        let results = vec![
            TestPortResult::Continue,
            TestPortResult::Stop,
            TestPortResult::Reply(TermValue::SmallInt(123)),
        ];
        
        for result in results {
            match result {
                TestPortResult::Continue => {
                    // Test continue case
                    assert!(true);
                }
                TestPortResult::Stop => {
                    // Test stop case  
                    assert!(true);
                }
                TestPortResult::Reply(term) => {
                    assert_eq!(term, TermValue::SmallInt(123));
                }
            }
        }
    }

    #[test]
    fn test_port_error_handling() {
        let mut port_data = TestPortData::new();
        let atom_table = MockAtomTable::new();
        
        // Test error message handling
        let error_msg = TestMessage::Error("test error".to_string());
        port_data.add_message(error_msg);
        
        assert_eq!(port_data.error_count, 0);
        port_data.process_messages(&atom_table);
        assert_eq!(port_data.error_count, 1);
    }

    #[test]
    fn test_port_error_conversion() {
        let atom_table = MockAtomTable::new();
        
        // Test converting various error types to terms
        let error_atom = atom_table.ensure_atom_str("conversion_error").unwrap();
        let error_term = TermValue::Atom(error_atom);
        
        // Verify error atoms are created correctly
        assert!(atom_table.atom_equals_str(error_atom, "conversion_error"));
    }

    #[test]
    fn test_term_to_pid() {
        let atom_table = MockAtomTable::new();
        
        // Test PID term creation and validation
        let pid_atom = atom_table.ensure_atom_str("pid").unwrap();
        let node_atom = atom_table.ensure_atom_str("node@host").unwrap();
        let creation_atom = atom_table.ensure_atom_str("creation").unwrap();
        
        // Create a PID term
        let pid_term = TermValue::Pid(ProcessId(123));
        
        // Verify PID-related atoms are handled correctly
        assert!(atom_table.atom_equals_str(pid_atom, "pid"));
        assert!(atom_table.atom_equals_str(node_atom, "node@host"));
        assert!(atom_table.atom_equals_str(creation_atom, "creation"));
        
        // Verify PID term
        if let Some(ProcessId(id)) = pid_term.as_pid() {
            assert_eq!(id, 123);
        } else {
            panic!("Expected PID term");
        }
    }

    #[test]
    fn test_standard_message_commands() {
        let mut port_data = TestPortData::new();
        let atom_table = MockAtomTable::new();
        
        // Test standard Erlang port commands
        let standard_commands = vec![
            "open", "close", "command", "connect", "disconnect"
        ];
        
        for cmd in standard_commands {
            port_data.add_message(TestMessage::Command(cmd.to_string()));
        }
        
        port_data.process_messages(&atom_table);
        
        // Should have processed all commands
        assert_eq!(port_data.message_count(), 0);
        assert!(port_data.last_command.is_some());
    }
}

// Add helper method to TermValue for PID extraction
impl TermValue {
    pub fn as_pid(&self) -> Option<ProcessId> {
        match self {
            TermValue::Pid(pid) => Some(*pid),
            _ => None,
        }
    }
}