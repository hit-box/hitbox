mod common;

use hitbox_protobuf::{DescriptorPool, ProtoError};

#[test]
fn loads_pool_and_resolves_message() {
    let pool = common::test_pool();
    let desc = pool.get_message("test.User").unwrap();
    assert_eq!(desc.full_name(), "test.User");
}

#[test]
fn get_message_not_found() {
    let pool = common::test_pool();
    let result = pool.get_message("test.NonExistent");
    assert!(matches!(result, Err(ProtoError::DescriptorNotFound(_))));
}

#[test]
fn get_input_type_by_simple_name() {
    let pool = common::test_pool();
    let desc = pool.get_input_type("UserService", "GetUser").unwrap();
    assert_eq!(desc.full_name(), "test.GetUserRequest");
}

#[test]
fn get_input_type_by_full_name() {
    let pool = common::test_pool();
    let desc = pool.get_input_type("test.UserService", "GetUser").unwrap();
    assert_eq!(desc.full_name(), "test.GetUserRequest");
}

#[test]
fn get_output_type() {
    let pool = common::test_pool();
    let desc = pool.get_output_type("UserService", "GetUser").unwrap();
    assert_eq!(desc.full_name(), "test.GetUserResponse");
}

#[test]
fn get_input_type_unknown_service() {
    let pool = common::test_pool();
    let result = pool.get_input_type("NonExistent", "GetUser");
    assert!(matches!(result, Err(ProtoError::DescriptorNotFound(_))));
}

#[test]
fn get_input_type_unknown_method() {
    let pool = common::test_pool();
    let result = pool.get_input_type("UserService", "NonExistent");
    assert!(matches!(result, Err(ProtoError::DescriptorNotFound(_))));
}

#[test]
fn from_file_descriptor_set_invalid_bytes() {
    let result = DescriptorPool::from_file_descriptor_set(b"not valid protobuf");
    assert!(result.is_err());
}
