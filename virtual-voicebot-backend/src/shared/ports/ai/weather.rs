use crate::shared::error::ai::WeatherError;

use super::{AiFuture, WeatherQuery, WeatherResponse};

pub trait WeatherPort: Send + Sync {
    fn handle_weather(
        &self,
        query: WeatherQuery,
    ) -> AiFuture<Result<WeatherResponse, WeatherError>>;
}
