use crate::shared::error::ai::WeatherError;

use super::{AiFuture, WeatherQuery, WeatherResponse};

pub trait WeatherPort: Send + Sync {
    fn handle_weather(
        &self,
        call_id: String,
        query: WeatherQuery,
    ) -> AiFuture<Result<WeatherResponse, WeatherError>>;
}
