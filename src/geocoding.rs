//! The open-meteo geocoding API.

use std::sync::mpsc;
use serde::{Serialize, Deserialize};

/// Deserialized place information as returned from the server.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Place {
    /// The name of this place.
    name: String,

    /// The latitude of this place.
    latitude: LatLon,

    /// The longitude of this place.
    longitude: LatLon,

    /// The country of this place.
    country: Option<String>,

    /// The first level administrative area this location resides in.
    ///
    /// The API has admin1-4, but the first level should be enough.
    admin1: Option<String>,
}

impl From<PlaceWire> for Place {
    fn from(wire: PlaceWire) -> Self {
        let PlaceWire { latitude, longitude, name, country, admin1 } = wire;

        Self {
            latitude: LatLon::quantize(latitude),
            longitude: LatLon::quantize(longitude),
            name,
            country,
            admin1,
        }
    }
}

impl Place {
    /// Get the string representation of this place.
    pub fn to_string(&self) -> String {
        // Format the country part.
        let country = if let Some(country) = &self.country {
            format!("{}, ", country)
        } else {
            "".to_string()
        };

        // Format the administrative area part.
        let admin = if let Some(admin) = &self.admin1 {
            format!("{} | ", admin)
        } else {
            "".to_string()
        };

        // Format the whole string.
        format!("{} ({}{}{}, {})",
            self.name, country, admin, self.latitude(), self.longitude())
    }

    /// Get this place's latitude.
    pub fn latitude(&self) -> f64 {
        self.latitude.dequantize()
    }

    /// Get this place's longitude.
    pub fn longitude(&self) -> f64 {
        self.longitude.dequantize()
    }
}

/// The integer representation of latitude/longitude.
///
/// Used to convert the `f64` latitude/longitude returned from the server into
/// an integer with a stable bit pattern that can be used for hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
struct LatLon(i64);

impl LatLon {
    /// The open-meteo geocoding API uses coordinates with resolution to
    /// 5 significant digits and so this is the scale we will use to convert the
    /// float to integer.
    const SCALE: f64 = 1e5;

    /// Quantize a `float` as integer.
    fn quantize(float: f64) -> Self {
        Self((float * Self::SCALE).round() as i64)
    }

    /// Dequantize this value as its floating point representation.
    fn dequantize(&self) -> f64 {
        (self.0 as f64) / Self::SCALE
    }
}

/// Raw place information returned from the server.
#[derive(Debug, Clone, Deserialize)]
struct PlaceWire {
    name: String,
    latitude: f64,
    longitude: f64,
    country: Option<String>,
    admin1: Option<String>,
}

/// A list of places matching the geocoding query, returned from the server.
#[derive(Debug, Clone, Deserialize)]
struct GeoResponse {
    results: Option<Vec<PlaceWire>>,
}

/// The system responsible for communicating with the remote open-meteo
/// geocoding API.
#[derive(Debug)]
pub struct GeoCoding {
    /// The transmitting end of the channel where requests are sent to the
    /// server.
    tx: mpsc::Sender<String>,

    /// The receiving end of the channel where responses from the server are
    /// received.
    rx: mpsc::Receiver<Vec<Place>>,

    /// The query that the user is currently filling in the UI.
    pub current_query: String,

    /// Results that have been received from the last request.
    pub search_results: Vec<Place>,
}

impl GeoCoding {
    /// Using `request`, get the URL of the endpoint that will service it.
    fn endpoint_url(request: &str) -> String {
        let params = vec![
            "count=10".to_string(),
            format!("name={}", urlencoding::encode(&request)),
        ];

        format!(
            "http://geocoding-api.open-meteo.com/v1/search?{}",
            params.join("&"))
    }

    /// Spawn the background task that will handle networking, and return the
    /// interface that can be used to communicate with the task.
    pub fn spawn_background_task() -> Self {
        // Create the channels for communicating with this task.
        let (tx_req, rx_req) = mpsc::channel::<String>();
        let (tx_res, rx_res) = mpsc::channel::<Vec<Place>>();

        std::thread::spawn(move || {
            // Create the client we will use to make requests.
            let client = reqwest::blocking::Client::new();

            // Listen for queries from the app.
            while let Ok(query) = rx_req.recv() {
                // Get the endpoint URL for this request.
                let url = Self::endpoint_url(&query);

                // Send the request to the server and attempt to parse the json.
                let result = client.get(url).send()
                    .and_then(|r| r.json::<GeoResponse>());

                // Normalize the results in case none were sent.
                let places: Vec<PlaceWire> = match result {
                    Ok(resp) => resp.results.unwrap_or_default(),
                    Err(_)   => vec![],
                };

                // Convert the wire for into app form.
                let places: Vec<Place> = places.into_iter()
                    .map(|wire| Place::from(wire))
                    .collect();

                // Send the result back to the application.
                let _ = tx_res.send(places);
            }
        });

        Self {
            tx: tx_req,
            rx: rx_res,
            current_query: String::new(),
            search_results: Vec::new(),
        }
    }

    /// Send the current query to the server.
    pub fn send_query(&mut self) {
        let query = self.current_query.trim();

        if !query.is_empty() {
            let _ = self.tx.send(query.to_string());
        }
    }

    /// Drain responses from the remote server and save them in
    /// `self.search_results`.
    pub fn drain_responses(&mut self) {
        while let Ok(results) = self.rx.try_recv() {
            self.search_results = results;
        }
    }
}
