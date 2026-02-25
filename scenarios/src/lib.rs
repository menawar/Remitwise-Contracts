pub mod tests {
    use soroban_sdk::Env;
    use testutils::set_ledger_time;

    pub fn setup_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        set_ledger_time(&env, 1, 1704067200); // Jan 1, 2024
        env
    }
}
