//! The open-meteo weather API.

use std::sync::mpsc;
use serde::Deserialize;
use crate::geocoding::Place;

// Wind Speed (10m): Average wind speed at 10 meters above ground.
//   0-5 km/h -> calm
//  5-15 km/h -> light breeze
// 15-30 km/h -> moderate wind
// 30-50 km/h -> strong wind
//   50+ km/h -> very strong wind
//
// Wind Gusts: Short bursts above the average wind.
// +10-20 km/h -> noticeable
// +20-30 km/h -> strong
//   +30+ km/h -> very strong and unstable
//
// Cloud Cover
// * Low clouds (0-2 km): affect sunlight and "overcast feeling"
// * Mid clouds (2-6 km): soften sunlight, partial shading
// * High clouds (6+ km): thin clouds, often translucent
//
// Hourly Precipitation: Total amount of water falling from the sky in 1 hour.
//    0 mm -> none
//  0-1 mm -> very light
//  1-4 mm -> light
//  4-8 mm -> moderate
// 8-15 mm -> heavy
//  15+ mm -> very heavy
//
// Dew point: How much moisture is in the air.
//    <5 °C -> dry
//  5-10 °C -> comfortable
// 10-15 °C -> slightly humid
// 15-20 °C -> humid
//   >20 °C -> very humid

/// Weather dataset, returned from the server.
#[derive(Debug, Clone, Deserialize)]
struct WeatherData {
    hourly: Hourly,
}

/// Hourly values from the dataset, returned from the server.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct Hourly {
    pub time: Vec<chrono::DateTime<chrono::Utc>>,
    pub temperature_2m: Vec<f64>,
    pub apparent_temperature: Vec<f64>,
    pub wind_speed_10m: Vec<f64>,
    pub wind_gusts_10m: Vec<f64>,
    pub precipitation: Vec<f64>,
    pub dew_point_2m: Vec<f64>,
    pub cloud_cover_low: Vec<f64>,
    pub cloud_cover_mid: Vec<f64>,
    pub cloud_cover_high: Vec<f64>,
}

impl Hourly {
    /// Get the API URL arguments that will be required to parse this struct.
    fn url_args() -> &'static str {
        "temperature_2m,\
        apparent_temperature,\
        wind_speed_10m,\
        wind_gusts_10m,\
        precipitation,\
        dew_point_2m,\
        cloud_cover_low,\
        cloud_cover_mid,\
        cloud_cover_high"
    }
}

/// The system responsible for communicating with the remote open-meteo
/// weather API.
#[derive(Debug)]
pub struct Weather {
    /// The transmitting end of the channel where requests are sent to the
    /// server.
    tx: mpsc::Sender<Place>,

    /// The receiving end of the channel where responses from the server are
    /// received.
    rx: mpsc::Receiver<Hourly>,

    /// The current dataset.
    ///
    /// This can either be the dataset received from the remote server, or
    /// dataset that was loaded from the disk cache.
    pub current: Hourly,
}

impl Weather {
    /// Using `place`, get the URL of the endpoint that will service it.
    fn endpoint_url(place: &Place) -> String {
        let params = vec![
            "timezone=auto".to_string(),
            "forecast_days=3".to_string(),
            format!("hourly={}", Hourly::url_args()),
            format!("latitude={:.2}", place.latitude()),
            format!("longitude={:.2}", place.longitude()),
        ];

        format!("http://api.open-meteo.com/v1/forecast?{}", params.join("&"))
    }

    /// Spawn the background task that will handle networking, and return the
    /// interface that can be used to communicate with the task.
    pub fn spawn_background_task() -> Self {
        // Create the channels for communicating with this task.
        let (tx_req, rx_req) = mpsc::channel::<Place>();
        let (tx_res, rx_res) = mpsc::channel::<Hourly>();

        std::thread::spawn(move || {
            // Create the client we will use to make requests.
            let client = reqwest::blocking::Client::new();

            // Listen for queries from the app.
            while let Ok(query) = rx_req.recv() {
                // Get the endpoint URL for this request.
                let url = Self::endpoint_url(&query);

                // Send the request to the server and attempt to parse the json.
                let result = client.get(url).send()
                    .and_then(|r| r.json::<WeatherData>());

                // Normalize the dataset in case none were sent.
                let hourly: Hourly = match result {
                    Ok(resp) => resp.hourly,
                    Err(_)   => Hourly::default(),
                };

                // Send the result back to the application.
                let _ = tx_res.send(hourly);
            }
        });

        Self {
            tx: tx_req,
            rx: rx_res,
            current: Hourly::default(),
        }
    }
}
