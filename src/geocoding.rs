//! The open-meteo geocoding API.

use std::sync::mpsc;
use serde::Deserialize;

/// A list of places matching the query, returned from the server.
#[derive(Debug, Clone, Deserialize)]
pub struct GeoResponse {
    results: Option<Vec<Place>>,
}

/// Place information returned from the server.
#[derive(Debug, Clone, Deserialize)]
pub struct Place {
    /// The name of this place.
    name: String,

    /// The latitude of this place.
    latitude: f64,

    /// The longitude of this place.
    longitude: f64,

    /// The country of this place.
    country: Option<String>,
}

impl Place {
    /// Convert the formatted string form to this struct.
    fn from_string(string: &str) -> Option<Self> {
        // Split "name (....)"
        let (name_part, rest) = string.split_once(" (")?;
        let name = name_part.trim().to_string();
        let rest = rest.strip_suffix(')')?;

        // Split "country | lat, long"
        let (country_part, coords_part) = rest.split_once(" | ")?;
        let country_part = country_part.trim();

        // Parse country (empty => None)
        let country = if country_part.is_empty() {
            None
        } else {
            Some(country_part.to_string())
        };

        // Split coordinates
        let (lat_str, lon_str) = coords_part.split_once(',')?;

        let latitude = lat_str.trim().parse::<f64>().ok()?;
        let longitude = lon_str.trim().parse::<f64>().ok()?;

        Some(Self {
            name,
            country,
            latitude,
            longitude,
        })
    }

    /// Convert this struct into its formatted string form.
    pub fn to_string(&self) -> String {
        format!(
            "{} ({} | {}, {})",
            self.name,
            self.country.clone().unwrap_or_default(),
            self.latitude,
            self.longitude,
        )
    }
}

/// The system responsible for sending geocoding requests and receiving
/// responses from the server.
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
        let partial_url =
            "http://geocoding-api.open-meteo.com/v1/search?count=10";

        format!("{}&name={}", partial_url, urlencoding::encode(&request))
    }

    /// Spawn the background task that will handle networking and return the
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
                let places: Vec<Place> = match result {
                    Ok(resp) => resp.results.unwrap_or_default(),
                    Err(_)   => vec![],
                };

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
