// This is where we will define the core application and all the related things
// on how to startup the application

use super::config::configuration::Configuration;

#[derive(Debug, Clone)]
pub struct Application {}

impl Application {
    pub async fn initialize(mut config: Configuration) -> Self {
        Self {}
    }
}
