use serde::Deserialize;

#[derive(Clone)]
pub struct AppState {}

#[derive(Default)]
pub struct MetarInfo {
    pub station: String,
    pub date_time: String,
    pub zulu_day: Option<u32>,
    pub zulu_hour: Option<u32>,
    pub zulu_minute: Option<u32>,
    pub wind: String,
    pub visibility: String,
    pub weather: String,
    pub clouds: String,
    pub temperature: String,
    pub dewpoint: String,
    pub altimeter: String,
    pub altimeter_hpa: Option<u32>,
    pub altimeter_inches: Option<f32>,
    pub altimeter_default_unit: String,
    pub remarks: String,
    pub raw: String,
}

#[derive(Deserialize)]
pub struct MetarQuery {
    pub icao: String,
}

