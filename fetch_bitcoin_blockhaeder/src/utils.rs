use std::env;
use dotenv::dotenv;

pub fn get_env_var(key: &str) -> String {
    dotenv().ok();
    env::var(key).expect(&format!("Environment variable {} not found", key))
}