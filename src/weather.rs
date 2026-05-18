//! The open-meteo weather API.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use serde::{Deserialize, Deserializer};
use chrono::NaiveDateTime;
use egui_plot::{Plot, Legend, Line, PlotPoints};
use crate::geocoding::Place;

/// The number of days to include in the forecast.
const FORECAST_DAYS: u8 = 3;

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
// * Low clouds (0-2 km): strongest impact on sunlight and gloominess
// * Mid clouds (2-6 km): soften sunlight, partial shading, textured skies
// * High clouds (6+ km): thin/translucent clouds, halos, filters sunlight
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
//
// Light Radiation
// * (GHI) Global Horizontal Irradiance
//   - total sunlight reaching a flat horizontal surface
//   - includes both direct sunlight and diffuse sky light
//   - good indicator of overall daylight brightness and solar energy
// * (DHI) Diffuse Horizontal Irradiance
//   - sunlight scattered by the atmosphere, clouds, haze and dust
//   - produces soft lighting and reduced shadows
//   - penetrates deeper into plant canopies and creates more illumination
// * (DNI) Direct Normal Irradiance
//   - direct beam sunlight coming from straight from the sun
//   - high DNI -> sharp shadows, intense sunlight, strong solar heating
//   - important for tracking solar panels, photography, clear-sky conditions
// * (GTI) Global Tilted Irradiance
//   - total sunlight reaching a tilted surface
//   - depends on panel tilt and orientation
//   - most relevant for estimating actual solar panel energy production

/// Weather dataset, returned from the server.
#[derive(Debug, Clone, Deserialize)]
struct WeatherData {
    hourly: Hourly,
}

/// Generate the `Hourly` weather struct and its plotting implementation.
///
/// This macro defines:
/// - the struct fields,
/// - API URL argument generation,
/// - field display labels,
/// - and grouped egui plots,
///
/// from a single declarative definition.
///
/// # Syntax
///
/// ```ignore
/// hourly_fields! {
///     fields {
///         field_name => "display label",
///     }
///
///     plots {
///         "plot title" => [field_name, other_field],
///     }
/// }
/// ```
macro_rules! hourly_fields {
    (
        fields {
            $(
                $field:ident => $label:literal
            ),* $(,)?
        }

        plots {
            $(
                $plot_name:literal => [
                    $( $plot_field:ident ),* $(,)?
                ]
            ),* $(,)?
        }
    ) => {
        /// Hourly values from the dataset, returned from the server.
        #[derive(Debug, Default, Clone, Deserialize)]
        pub struct Hourly {
            #[serde(deserialize_with = "deserialize_naive_datetime")]
            pub time: Vec<NaiveDateTime>,

            $(
                pub $field: Vec<f32>,
            )*
        }

        impl Hourly {
            /// Get the API URL arguments that will be required to parse this
            /// struct.
            fn url_args() -> String {
                vec![
                    $(
                        stringify!($field),
                    )*
                ].join(",")
            }

            /// Get the plot label for a specific field.
            fn label(field: &str) -> &'static str {
                match field {
                    $(
                        stringify!($field) => $label,
                    )*
                    _ => unreachable!(),
                }
            }

            /// Draw all plots.
            pub fn draw_plots(&self, ui: &mut egui::Ui) {
                $(
                    Self::plot($plot_name).show(ui, |ui| {
                        $(
                            ui.line(Line::new(
                                Self::label(stringify!($plot_field)),
                                Self::make_points(&self.$plot_field),
                            ));
                        )*
                    });
                )*
            }

            /// Create a plot for `name`.
            fn plot<'a>(name: &'a str) -> Plot<'a> {
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
                        if hour.is_positive() {
                            format!("{:02}", hour)
                        } else {
                            "".to_string()
                        }
                    })
                    .x_grid_spacer(|input| {
                        let mut marks = Vec::new();

                        let start = (input.bounds.0 / 6.0).floor() as i64 * 6;
                        let end = input.bounds.1.ceil() as i64;

                        let mut x = start;

                        while x <= end {
                            let step = 10.0
                                +  5.0 * ((x % 6  == 0) as u8 as f64)
                                + 15.0 * ((x % 12 == 0) as u8 as f64)
                                + 30.0 * ((x % 24 == 0) as u8 as f64);

                            marks.push(egui_plot::GridMark {
                                value: x as f64,
                                step_size: step,
                            });

                            x += 6;
                        }

                        marks
                    })
            }

            /// Create plot points for `values`.
            fn make_points<'a>(values: &'a [f32]) -> PlotPoints<'a> {
                values.iter()
                    .enumerate()
                    .map(|(idx, value)| [idx as f64, *value as f64])
                    .collect()
            }
        }
    };
}

hourly_fields! {
    fields {
        temperature_2m                   => "temperature",
        apparent_temperature             => "apparent temperature",
        wind_speed_10m                   => "wind speed",
        wind_gusts_10m                   => "wind gusts",
        precipitation                    => "precipitation",
        dew_point_2m                     => "dew point",
        cloud_cover_low                  => "cloud cover low",
        cloud_cover_mid                  => "cloud cover mid",
        cloud_cover_high                 => "cloud cover high",
        soil_moisture_0_to_1cm           => "soil moisture (0-1cm)",
        soil_moisture_1_to_3cm           => "soil moisture (1-3cm)",
        soil_moisture_3_to_9cm           => "soil moisture (3-9cm)",
        soil_moisture_9_to_27cm          => "soil moisture (9-27cm)",
        shortwave_radiation_instant      => "global horizontal irradiance",
        direct_normal_irradiance_instant => "direct normal irradiance",
        diffuse_radiation_instant        => "diffuse horizontal irradiance",
    }

    plots {
        "temperature" => [
            temperature_2m,
            apparent_temperature,
        ],

        "wind" => [
            wind_speed_10m,
            wind_gusts_10m,
        ],

        "dew point" => [
            dew_point_2m,
        ],

        "precipitation" => [
            precipitation,
        ],

        "soil moisture" => [
            soil_moisture_0_to_1cm,
            soil_moisture_1_to_3cm,
            soil_moisture_3_to_9cm,
            soil_moisture_9_to_27cm,
        ],

        "solar radiation" => [
            shortwave_radiation_instant,
            direct_normal_irradiance_instant,
            diffuse_radiation_instant,
        ],

        "cloud cover" => [
            cloud_cover_low,
            cloud_cover_mid,
            cloud_cover_high,
        ],
    }
}

// TODO: HourlyResult here is a little weird, especially because
// `Weather.current` doesn't encode anything.
// Use newtype patterns for `PlaceString` or something like that..
// Also `HourlyResult` is a bad name.

/// Result sent from the background task back to the weather interface.
struct HourlyResult {
    place: Place,
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
    pub outgoing: HashSet<Place>,

    /// The current dataset.
    ///
    /// If the value is `None`, it means that we've attempted to send a request
    /// but received no response yet.
    pub current: HashMap<Place, Option<Hourly>>,
}

impl Weather {
    /// Using `place`, get the URL of the endpoint that will service it.
    fn endpoint_url(place: &Place, forecast_days: u8) -> String {
        let params = [
            "timezone=auto".to_string(),
            format!("forecast_days={}", forecast_days),
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
                let url = Self::endpoint_url(&place, FORECAST_DAYS);

                // Send the request to the server and attempt to parse the json.
                let result = client.get(url).send()
                    .and_then(|r| r.json::<WeatherData>())
                    .ok()
                    .map(|data| data.hourly);

                // Encode the result and send it back to the client.
                let _ = tx_res.send(HourlyResult { place, data: result, });

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
        // Get the current dataset.
        let current = self.current
            .entry(place.clone())
            .or_insert(None);

        // If there's one in the cache, don't do anything.
        if current.is_some() { return; }

        // Only send a request if there's no outgoing request yet.
        if !self.outgoing.contains(&place) {
            self.send_query(place, ctx);
        }
    }

    /// Send a query for `place` to the server.
    pub fn send_query(&mut self, place: Place, ctx: egui::Context) {
        // Keep track of this outgoing query.
        self.outgoing.insert(place.clone());

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
