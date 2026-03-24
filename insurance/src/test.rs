#[test]
fn test_premium_execution_idempotency() {
    let env = Env::default();
    let client = InsuranceClient::new(&env, &env.register_contract(None, InsuranceContract));
    
    // Setup policy...
    
    // First execution: Should succeed
    client.execute_due_premium_schedules(&policy_id);

    // Second execution (Immediate): Should Panic/Fail due to idempotency check
    let result = client.try_execute_due_premium_schedules(&policy_id);
    assert!(result.is_err(), "Should prevent duplicate payment in the same window");
}