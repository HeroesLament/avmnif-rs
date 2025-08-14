//! Test fixtures and common test data
//! 
//! This module provides pre-built test data structures and scenarios
//! that are commonly used across different test modules.
//!
//! # Design Philosophy
//!
//! All fixtures are generic and work with any AtomTableOps implementation.
//! Every fixture function takes an atom table parameter for complete genericity.

use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

use crate::term::TermValue;
use crate::atom::AtomTableOps;

// ── Core Fixtures ──────────────────────────────────────────────────────────

/// Simple user data for testing
pub fn user_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::map(vec![
        (TermValue::atom("id", table), TermValue::int(123)),
        (TermValue::atom("name", table), TermValue::atom("john_doe", table)),
        (TermValue::atom("email", table), TermValue::atom("john@example.com", table)),
        (TermValue::atom("active", table), TermValue::atom("true", table)),
        (TermValue::atom("role", table), TermValue::atom("user", table)),
    ])
}

/// Admin user data for testing
pub fn admin_user_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::map(vec![
        (TermValue::atom("id", table), TermValue::int(1)),
        (TermValue::atom("name", table), TermValue::atom("admin", table)),
        (TermValue::atom("email", table), TermValue::atom("admin@example.com", table)),
        (TermValue::atom("active", table), TermValue::atom("true", table)),
        (TermValue::atom("role", table), TermValue::atom("admin", table)),
        (
            TermValue::atom("permissions", table),
            TermValue::list(vec![
                TermValue::atom("read", table),
                TermValue::atom("write", table),
                TermValue::atom("delete", table),
                TermValue::atom("admin", table),
            ])
        ),
    ])
}

/// Configuration data for testing
pub fn config_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::map(vec![
        (TermValue::atom("database_url", table), TermValue::atom("postgres://localhost", table)),
        (TermValue::atom("port", table), TermValue::int(8080)),
        (TermValue::atom("debug", table), TermValue::atom("false", table)),
        (TermValue::atom("max_connections", table), TermValue::int(100)),
        (
            TermValue::atom("features", table),
            TermValue::list(vec![
                TermValue::atom("auth", table),
                TermValue::atom("logging", table),
                TermValue::atom("metrics", table),
            ])
        ),
    ])
}

/// Error response fixture
pub fn error_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::tuple(vec![
        TermValue::atom("error", table),
        TermValue::atom("not_found", table),
        TermValue::atom("Resource not found", table),
    ])
}

/// Success response fixture
pub fn success_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::tuple(vec![
        TermValue::atom("ok", table),
        TermValue::map(vec![
            (TermValue::atom("status", table), TermValue::atom("success", table)),
            (TermValue::atom("code", table), TermValue::int(200)),
            (TermValue::atom("data", table), TermValue::atom("operation_completed", table)),
        ])
    ])
}

/// List of various data types for comprehensive testing
pub fn mixed_data_list_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::list(vec![
        TermValue::int(42),
        TermValue::atom("hello", table),
        TermValue::atom("true", table),
        TermValue::float(3.14),
        TermValue::tuple(vec![
            TermValue::atom("coord", table),
            TermValue::int(10),
            TermValue::int(20),
        ]),
        TermValue::list(vec![
            TermValue::int(1),
            TermValue::int(2),
            TermValue::int(3),
        ]),
        TermValue::map(vec![
            (TermValue::atom("key", table), TermValue::atom("value", table)),
            (TermValue::atom("count", table), TermValue::int(5)),
        ]),
        TermValue::binary(b"binary_data".to_vec()),
    ])
}

/// Nested data structure for complex testing scenarios
pub fn nested_structure_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::map(vec![
        (
            TermValue::atom("level1", table),
            TermValue::map(vec![
                (
                    TermValue::atom("level2", table),
                    TermValue::map(vec![
                        (
                            TermValue::atom("level3", table),
                            TermValue::tuple(vec![
                                TermValue::atom("deep", table),
                                TermValue::int(42),
                                TermValue::list(vec![
                                    TermValue::atom("nested", table),
                                    TermValue::atom("list", table),
                                ]),
                            ])
                        ),
                        (TermValue::atom("sibling", table), TermValue::atom("value", table)),
                    ])
                ),
                (TermValue::atom("other", table), TermValue::int(123)),
            ])
        ),
        (
            TermValue::atom("parallel", table),
            TermValue::list(vec![
                TermValue::tuple(vec![TermValue::atom("item", table), TermValue::int(1)]),
                TermValue::tuple(vec![TermValue::atom("item", table), TermValue::int(2)]),
                TermValue::tuple(vec![TermValue::atom("item", table), TermValue::int(3)]),
            ])
        ),
    ])
}

/// Database record fixture
pub fn db_record_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::map(vec![
        (TermValue::atom("id", table), TermValue::int(456)),
        (TermValue::atom("created_at", table), TermValue::int(1640995200)), // Unix timestamp
        (TermValue::atom("updated_at", table), TermValue::int(1640995300)),
        (
            TermValue::atom("data", table),
            TermValue::map(vec![
                (TermValue::atom("title", table), TermValue::atom("test_record", table)),
                (TermValue::atom("description", table), TermValue::atom("A test database record", table)),
                (TermValue::atom("version", table), TermValue::int(1)),
                (TermValue::atom("published", table), TermValue::atom("false", table)),
            ])
        ),
        (
            TermValue::atom("tags", table),
            TermValue::list(vec![
                TermValue::atom("test", table),
                TermValue::atom("fixture", table),
                TermValue::atom("database", table),
            ])
        ),
    ])
}

/// API request fixture
pub fn api_request_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::map(vec![
        (TermValue::atom("method", table), TermValue::atom("POST", table)),
        (TermValue::atom("path", table), TermValue::atom("/api/v1/users", table)),
        (
            TermValue::atom("headers", table),
            TermValue::map(vec![
                (TermValue::atom("content_type", table), TermValue::atom("application/json", table)),
                (TermValue::atom("authorization", table), TermValue::atom("Bearer token123", table)),
                (TermValue::atom("user_agent", table), TermValue::atom("test_client/1.0", table)),
            ])
        ),
        (
            TermValue::atom("body", table),
            TermValue::map(vec![
                (TermValue::atom("name", table), TermValue::atom("new_user", table)),
                (TermValue::atom("email", table), TermValue::atom("user@test.com", table)),
                (TermValue::atom("password", table), TermValue::atom("secret123", table)),
            ])
        ),
        (TermValue::atom("timestamp", table), TermValue::int(1640995400)),
    ])
}

/// Large list for performance testing
pub fn large_list_fixture(size: usize) -> TermValue {
    let elements: Vec<TermValue> = (0..size)
        .map(|i| TermValue::int(i as i32))
        .collect();
    TermValue::list(elements)
}

/// Large map for performance testing
pub fn large_map_fixture<T: AtomTableOps>(size: usize, table: &T) -> TermValue {
    let pairs: Vec<(TermValue, TermValue)> = (0..size)
        .map(|i| {
            let key = TermValue::atom(&format!("key_{}", i), table);
            let value = TermValue::int(i as i32);
            (key, value)
        })
        .collect();
    TermValue::map(pairs)
}

// ── Binary Data Fixtures ───────────────────────────────────────────────────

/// Binary data fixtures for different scenarios
pub mod binary_fixtures {
    use alloc::vec;
    use alloc::vec::Vec;
    use crate::term::TermValue;
    
    pub fn empty_binary() -> TermValue {
        TermValue::binary(vec![])
    }
    
    pub fn text_binary() -> TermValue {
        TermValue::binary(b"Hello, World!".to_vec())
    }
    
    pub fn numeric_binary() -> TermValue {
        TermValue::binary(vec![0, 1, 2, 3, 4, 5, 255, 254, 253])
    }
    
    pub fn large_binary(size: usize) -> TermValue {
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        TermValue::binary(data)
    }
}

// ── Process ID Fixtures ────────────────────────────────────────────────────

/// Process ID fixtures for testing
pub mod pid_fixtures {
    use crate::term::TermValue;
    
    pub fn self_pid() -> TermValue {
        TermValue::pid(0)
    }
    
    pub fn parent_pid() -> TermValue {
        TermValue::pid(1)
    }
    
    pub fn worker_pid() -> TermValue {
        TermValue::pid(100)
    }
    
    pub fn supervisor_pid() -> TermValue {
        TermValue::pid(200)
    }
}

// ── Reference Fixtures ─────────────────────────────────────────────────────

/// Reference fixtures for testing
pub mod ref_fixtures {
    use crate::term::TermValue;
    
    pub fn local_ref() -> TermValue {
        TermValue::reference(12345)
    }
    
    pub fn remote_ref() -> TermValue {
        TermValue::reference(67890)
    }
    
    pub fn monitor_ref() -> TermValue {
        TermValue::reference(999999)
    }
}

// ── Function Reference Fixtures ────────────────────────────────────────────

/// Function reference fixtures
pub mod function_fixtures {
    use crate::term::{TermValue, FunctionRef};
    use crate::atom::AtomTableOps;
    
    pub fn simple_function<T: AtomTableOps>(table: &T) -> TermValue {
        TermValue::Function(FunctionRef {
            module: TermValue::atom("test_module", table).as_atom().unwrap(),
            function: TermValue::atom("test_function", table).as_atom().unwrap(),
            arity: 2,
        })
    }
    
    pub fn callback_function<T: AtomTableOps>(table: &T) -> TermValue {
        TermValue::Function(FunctionRef {
            module: TermValue::atom("callbacks", table).as_atom().unwrap(),
            function: TermValue::atom("handle_event", table).as_atom().unwrap(),
            arity: 3,
        })
    }
}

// ── Complex Scenarios ──────────────────────────────────────────────────────

/// Test scenarios that combine multiple fixtures
pub mod scenarios {
    use super::*;
    
    /// Complete user session scenario
    pub fn user_session_scenario<T: AtomTableOps>(table: &T) -> TermValue {
        TermValue::map(vec![
            (TermValue::atom("user", table), user_fixture(table)),
            (TermValue::atom("session", table), session_fixture(table)),
            (TermValue::atom("permissions", table), permissions_fixture(table)),
            (TermValue::atom("last_activity", table), TermValue::int(1640995500)),
        ])
    }
    
    /// Error handling scenario
    pub fn error_scenario<T: AtomTableOps>(table: &T) -> TermValue {
        TermValue::tuple(vec![
            TermValue::atom("error", table),
            TermValue::map(vec![
                (TermValue::atom("type", table), TermValue::atom("validation_error", table)),
                (TermValue::atom("message", table), TermValue::atom("Invalid input data", table)),
                (
                    TermValue::atom("details", table),
                    TermValue::list(vec![
                        TermValue::tuple(vec![
                            TermValue::atom("field", table),
                            TermValue::atom("email", table),
                            TermValue::atom("required", table),
                        ]),
                        TermValue::tuple(vec![
                            TermValue::atom("field", table),
                            TermValue::atom("age", table),
                            TermValue::atom("must_be_positive", table),
                        ]),
                    ])
                ),
                (TermValue::atom("code", table), TermValue::int(400)),
            ])
        ])
    }
    
    /// Server state scenario
    pub fn server_state_scenario<T: AtomTableOps>(table: &T) -> TermValue {
        TermValue::map(vec![
            (TermValue::atom("uptime", table), TermValue::int(86400)), // 1 day in seconds
            (TermValue::atom("connections", table), TermValue::int(42)),
            (TermValue::atom("memory_usage", table), TermValue::float(0.75)),
            (
                TermValue::atom("active_processes", table),
                TermValue::list(vec![
                    pid_fixtures::worker_pid(),
                    pid_fixtures::supervisor_pid(),
                ])
            ),
            (TermValue::atom("config", table), config_fixture(table)),
            (
                TermValue::atom("stats", table),
                TermValue::map(vec![
                    (TermValue::atom("requests_total", table), TermValue::int(10000)),
                    (TermValue::atom("errors_total", table), TermValue::int(42)),
                    (TermValue::atom("avg_response_time", table), TermValue::float(125.5)),
                ])
            ),
        ])
    }
}

// ── Helper Functions ───────────────────────────────────────────────────────

fn session_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::map(vec![
        (TermValue::atom("id", table), TermValue::atom("session_abc123", table)),
        (TermValue::atom("created", table), TermValue::int(1640995000)),
        (TermValue::atom("expires", table), TermValue::int(1640998600)), // 1 hour later
        (TermValue::atom("authenticated", table), TermValue::atom("true", table)),
    ])
}

fn permissions_fixture<T: AtomTableOps>(table: &T) -> TermValue {
    TermValue::list(vec![
        TermValue::atom("read", table),
        TermValue::atom("write", table),
        TermValue::atom("create", table),
        TermValue::atom("update", table),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::helpers::*;
    use crate::testing::mocks::MockAtomTable;

    #[test]
    fn test_user_fixture() {
        let table = MockAtomTable::new();
        let user = user_fixture(&table);
        
        // Should be a map with expected fields
        let id_key = TermValue::atom("id", &table);
        let name_key = TermValue::atom("name", &table);
        let email_key = TermValue::atom("email", &table);
        let role_key = TermValue::atom("role", &table);
        
        assert!(user.map_get(&id_key).is_some());
        assert!(user.map_get(&name_key).is_some());
        assert!(user.map_get(&email_key).is_some());
        
        // Verify specific values
        assert_int(user.map_get(&id_key).unwrap(), 123);
        assert_atom_str(user.map_get(&role_key).unwrap(), "user", &table);
    }

    #[test]
    fn test_admin_user_fixture() {
        let table = MockAtomTable::new();
        let admin = admin_user_fixture(&table);
        
        let role_key = TermValue::atom("role", &table);
        let permissions_key = TermValue::atom("permissions", &table);
        
        assert_atom_str(admin.map_get(&role_key).unwrap(), "admin", &table);
        
        let permissions = admin.map_get(&permissions_key).unwrap();
        assert_list_length(permissions, 4);
    }

    #[test]
    fn test_config_fixture() {
        let table = MockAtomTable::new();
        let config = config_fixture(&table);
        
        let port_key = TermValue::atom("port", &table);
        let debug_key = TermValue::atom("debug", &table);
        let features_key = TermValue::atom("features", &table);
        
        assert_int(config.map_get(&port_key).unwrap(), 8080);
        assert_atom_str(config.map_get(&debug_key).unwrap(), "false", &table);
        
        let features = config.map_get(&features_key).unwrap();
        assert_list_length(features, 3);
    }

    #[test]
    fn test_nested_structure_fixture() {
        let table = MockAtomTable::new();
        let nested = nested_structure_fixture(&table);
        
        // Navigate deep into structure
        let level1_key = TermValue::atom("level1", &table);
        let level2_key = TermValue::atom("level2", &table);
        let level3_key = TermValue::atom("level3", &table);
        
        let level1 = nested.map_get(&level1_key).unwrap();
        let level2 = level1.map_get(&level2_key).unwrap();
        let level3 = level2.map_get(&level3_key).unwrap();
        
        assert_tuple_arity(level3, 3);
    }

    #[test]
    fn test_mixed_data_list_fixture() {
        let table = MockAtomTable::new();
        let mixed_list = mixed_data_list_fixture(&table);
        
        // Should contain various data types
        assert_list_length(&mixed_list, 8);
        
        let items = mixed_list.list_to_vec();
        assert_int(&items[0], 42);
        assert_atom_str(&items[1], "hello", &table);
        assert!(matches!(items[3], TermValue::Float(_)));
        assert_tuple_arity(&items[4], 3);
    }

    #[test]
    fn test_binary_fixtures() {
        let empty = binary_fixtures::empty_binary();
        let text = binary_fixtures::text_binary();
        let numeric = binary_fixtures::numeric_binary();
        
        match empty {
            TermValue::Binary(data) => assert_eq!(data.len(), 0),
            _ => panic!("Expected binary"),
        }
        
        match text {
            TermValue::Binary(data) => assert_eq!(data, b"Hello, World!"),
            _ => panic!("Expected binary"),
        }
        
        match numeric {
            TermValue::Binary(data) => {
                assert_eq!(data.len(), 9);
                assert_eq!(data[0], 0);
                assert_eq!(data[8], 253);
            }
            _ => panic!("Expected binary"),
        }
    }

    #[test]
    fn test_scenarios() {
        let table = MockAtomTable::new();
        
        let user_session = scenarios::user_session_scenario(&table);
        let error_scenario = scenarios::error_scenario(&table);
        let server_state = scenarios::server_state_scenario(&table);
        
        // Verify user session has all expected components
        let user_key = TermValue::atom("user", &table);
        let session_key = TermValue::atom("session", &table);
        let permissions_key = TermValue::atom("permissions", &table);
        
        assert!(user_session.map_get(&user_key).is_some());
        assert!(user_session.map_get(&session_key).is_some());
        assert!(user_session.map_get(&permissions_key).is_some());
        
        // Verify error scenario structure
        assert_tuple_arity(&error_scenario, 2);
        
        // Verify server state has stats
        let stats_key = TermValue::atom("stats", &table);
        let requests_key = TermValue::atom("requests_total", &table);
        
        let stats = server_state.map_get(&stats_key).unwrap();
        assert!(stats.map_get(&requests_key).is_some());
    }

    #[test]
    fn test_large_fixtures() {
        let table = MockAtomTable::new();
        
        let large_list = large_list_fixture(1000);
        let large_map = large_map_fixture(100, &table);
        
        assert_list_length(&large_list, 1000);
        
        // Large map should have 100 key-value pairs
        match large_map {
            TermValue::Map(pairs) => assert_eq!(pairs.len(), 100),
            _ => panic!("Expected map"),
        }
    }

    #[test]
    fn test_pid_fixtures() {
        let self_pid = pid_fixtures::self_pid();
        let worker_pid = pid_fixtures::worker_pid();
        
        assert!(matches!(self_pid, TermValue::Pid(_)));
        assert!(matches!(worker_pid, TermValue::Pid(_)));
        assert_ne!(self_pid, worker_pid);
    }
}