//! The open-meteo weather API.

// TODO: unfuck all of this.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use serde::{Deserialize, Deserializer};
use chrono::NaiveDateTime;
use egui_plot::{Plot, Legend, Line, PlotPoints};
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
    #[serde(deserialize_with = "deserialize_naive_datetime")]
    pub time:                 Vec<NaiveDateTime>,
    pub temperature_2m:       Vec<f32>,
    pub apparent_temperature: Vec<f32>,
    pub wind_speed_10m:       Vec<f32>,
    pub wind_gusts_10m:       Vec<f32>,
    pub precipitation:        Vec<f32>,
    pub dew_point_2m:         Vec<f32>,
    pub cloud_cover_low:      Vec<f32>,
    pub cloud_cover_mid:      Vec<f32>,
    pub cloud_cover_high:     Vec<f32>,
}

impl Hourly {
    pub fn draw_plots(&self, ui: &mut egui::Ui) {
        let temp = Line::new("temperature",
            Self::make_points(&self.temperature_2m));
        let apparent = Line::new("apparent temperature",
            Self::make_points(&self.apparent_temperature));
        let wind_speed = Line::new("wind speed",
            Self::make_points(&self.wind_speed_10m));
        let wind_gusts = Line::new("wind gusts",
            Self::make_points(&self.wind_gusts_10m));
        let precipitation = Line::new("precipitation",
            Self::make_points(&self.precipitation));
        let dew = Line::new("dew point",
            Self::make_points(&self.dew_point_2m));
        let cloud_low = Line::new("cloud cover low",
            Self::make_points(&self.cloud_cover_low));
        let cloud_mid = Line::new("cloud cover mid",
            Self::make_points(&self.cloud_cover_mid));
        let cloud_high = Line::new("cloud cover high",
            Self::make_points(&self.cloud_cover_high));

        Self::plot("temperature").show(ui, |ui| {
            ui.line(temp);
            ui.line(apparent);
        });

        Self::plot("wind").show(ui, |ui| {
            ui.line(wind_speed);
            ui.line(wind_gusts);
        });

        Self::plot("precipitation").show(ui, |ui| {
            ui.line(precipitation);
        });

        Self::plot("dew point").show(ui, |ui| {
            ui.line(dew);
        });

        Self::plot("cloud low").show(ui, |ui| {
            ui.line(cloud_low);
            ui.line(cloud_mid);
            ui.line(cloud_high);
        });
    }

    fn plot<'a>(name: &'a str) -> Plot<'a> {
        // TODO: show every 6 hours

        Plot::new(name)
            .legend(Legend::default())
            .height(200.0)
            .allow_axis_zoom_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_drag(false)
            .sense(egui::Sense::empty())
            .x_axis_formatter(|x, _| {
                let hour = x.value as i64 % 24;
                format!("{:02}:00", hour)
            })
    }

    fn make_points<'a>(values: &'a [f32]) -> PlotPoints<'a> {
        values.iter().enumerate().map(|(idx, value)| {
            [idx as f64, *value as f64]
        }).collect()
    }
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

// TODO: HourlyResult here is a little weird, especially because
// `Weather.current` doesn't encode anything.
// Use newtype patterns for `PlaceString` or something like that..
// Also `HourlyResult` is a bad name.

/// Result sent from the background task back to the weather interface.
struct HourlyResult {
    place: String,
    data: Option<Hourly>,
}

struct HourlyRequest {
    place: Place,
    ctx: egui::Context,
}

/// The system responsible for communicating with the remote open-meteo
/// weather API.
#[derive(Debug)]
pub struct Weather {
    /// The transmitting end of the channel where requests are sent to the
    /// server.
    tx: mpsc::Sender<HourlyRequest>,

    /// The receiving end of the channel where responses from the server are
    /// received.
    rx: mpsc::Receiver<HourlyResult>,

    /// A list of places that have currently outgoing requests.
    pub outgoing: HashSet<String>,

    /// The current dataset.
    ///
    /// This can either be the dataset received from the remote server, or
    /// dataset that was loaded from the disk cache.
    ///
    /// The string here is the string representation of a place.
    ///
    /// If the value is `None`, it means that we've attempted to send a request
    /// but received no response.
    pub current: HashMap<String, Option<Hourly>>,
}

impl Weather {
    /// Using `place`, get the URL of the endpoint that will service it.
    fn endpoint_url(place: &Place) -> String {
        let params = vec![
            "timezone=auto".to_string(),
            "forecast_days=3".to_string(),
            format!("hourly={}", Hourly::url_args()),
            format!("latitude={:.4}", place.latitude()),
            format!("longitude={:.4}", place.longitude()),
        ];

        format!("http://api.open-meteo.com/v1/forecast?{}", params.join("&"))
    }

    /// Spawn the background task that will handle networking, and return the
    /// interface that can be used to communicate with the task.
    pub fn spawn_background_task() -> Self {
        // Create the channels for communicating with this task.
        let (tx_req, rx_req) = mpsc::channel::<HourlyRequest>();
        let (tx_res, rx_res) = mpsc::channel::<HourlyResult>();

        std::thread::spawn(move || {
            // Create the client we will use to make requests.
            let client = reqwest::blocking::Client::new();

            // Listen for queries from the app.
            while let Ok(request) = rx_req.recv() {
                let place = request.place;

                // Get the endpoint URL for this request.
                let url = Self::endpoint_url(&place);

                // Send the request to the server and attempt to parse the json.
                let result = client.get(url).send()
                    .and_then(|r| r.json::<WeatherData>())
                    .ok()
                    .map(|data| data.hourly);

                // Encode the result and send it back to the client.
                let _ = tx_res.send(HourlyResult {
                    place: place.to_string_coords(),
                    data: result,
                });

                // Request a repaint. This is a little bit of a hack; the first
                // request may trigger a repaint before the result is received,
                // so we're waging that the second one will trigger it more
                // correctly.
                request.ctx.request_repaint();
                request.ctx.request_repaint_after_secs(0.5);
            }
        });

        Self {
            tx: tx_req,
            rx: rx_res,
            current: HashMap::new(),
            outgoing: HashSet::new(),
        }
    }

    /// Send a request to the remote server if we don't have this `place` cached
    /// yet.
    ///
    /// `ctx` will be used to request a repaint of the viewport once the
    /// response (could be an error) is received.
    pub fn request_if_not_cached(&mut self, place: Place, ctx: egui::Context) {
        let place_string = place.to_string_coords();

        // Get the current dataset.
        let current = self.current
            .entry(place_string.clone())
            .or_insert(None);

        // If there's one in the cache, don't do anything.
        if current.is_some() { return; }

        // Only send a request if there's no outgoing request yet.
        if !self.outgoing.contains(&place_string) {
            self.send_query(place.clone(), ctx);
        }
    }

    /// Send a query for `place` to the server.
    pub fn send_query(&mut self, place: Place, ctx: egui::Context) {
        // Keep track of this outgoing query.
        self.outgoing.insert(place.to_string_coords());

        // Send the query.
        let _ = self.tx.send(HourlyRequest { place, ctx, });
    }

    /// Drain responses from the remote server and save them in `self.current`
    pub fn drain_responses(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            self.outgoing.remove(&result.place);
            self.current.insert(result.place, result.data);
        }
    }
}

/// Deserialize the time strings returned from the server as a vector of
/// datetimes.
fn deserialize_naive_datetime<'de, D>(
    deserializer: D
) -> Result<Vec<NaiveDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Vec<String> = <Vec<String>>::deserialize(deserializer)?;

    s.into_iter()
        .map(|s| {
            NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M")
                .map_err(serde::de::Error::custom)
        })
        .collect()
}
