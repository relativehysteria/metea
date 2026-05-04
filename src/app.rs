use std::collections::HashSet;
use std::path::{PathBuf, Path};
use serde::{Serialize, Deserialize};
use crate::geocoding::GeoCoding;
use crate::InternalStorage;

/// State of the application that will be persisted across runs.
#[derive(Default, Serialize, Deserialize)]
struct PersistedState {
    /// Saved places retrieved from the geocoding API.
    places: HashSet<String>,
}

impl PersistedState {
    /// Load the state from the internal storage.
    fn load(storage: &InternalStorage) -> Option<Self> {
        let data = std::fs::read(storage.places()).ok()?;
        serde_json::from_slice(&data).ok()
    }

    /// Save the state to the internal storage.
    fn save(&self, storage: &InternalStorage) -> std::io::Result<()> {
        let data = serde_json::to_vec_pretty(self).unwrap();
        storage.write_atomic(&storage.places(), &data)
    }
}

/// The android metea application.
pub struct App {
    /// Interface to the application's internal storage.
    internal_storage: InternalStorage,

    /// State of the application that is persisted across runs.
    state: PersistedState,

    /// Interface to the open-meteo geocoding API.
    geocoding: GeoCoding,
}

impl App {
    /// Create the application state.
    pub fn new(
        _cc: &eframe::CreationContext,
        internal_storage: InternalStorage,
    ) -> Self {
        let state = PersistedState::load(&internal_storage)
            .unwrap_or_default();

        Self {
            internal_storage,
            state,
            geocoding: GeoCoding::spawn_background_task(),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Safe space hack.
        // https://github.com/rust-windowing/winit/issues/3910
        egui::Panel::top("safe_space_hack").show_inside(ui, |ui| {
            ui.set_height(32.0);
        });

        // Receive results from the geocoding endpoint.
        self.geocoding.drain_responses();

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical_centered(|ui| {
                // Create the text input field.
                let input = egui::TextEdit::singleline(
                        &mut self.geocoding.current_query)
                    .font(egui::TextStyle::Heading);

                let response = ui.add(input);

                // Save on Enter (when focus is in field).
                let enter_pressed =
                    ui.input(|i| i.key_pressed(egui::Key::Enter));

                // Once the user presses enter, send the geocoding request.
                if response.lost_focus() && enter_pressed {
                    self.geocoding.send_query();
                    response.request_focus();
                }

                ui.add_space(20.0);

                // If we got some geocoding results, show them and allow the
                // user selection. Once one result is selected, it is saved and
                // the results are cleared.
                let mut clear = false;
                if !self.geocoding.search_results.is_empty() {
                    ui.label("Results:");

                    for place in &self.geocoding.search_results {
                        let label = place.to_string();

                        // Result selected; save it and clear the results.
                        if ui.button(&label).clicked() {
                            self.state.places.insert(label);
                            let _ = self.state.save(&self.internal_storage);
                            clear = true;
                            break;
                        }
                    }

                    // The user has selected a result; clear the buffer.
                    if clear {
                        self.geocoding.search_results.clear();
                    }
                }

                ui.add_space(20.0);

                // Show saved coordinates.
                if !self.state.places.is_empty() {
                    for place in &self.state.places {
                        ui.label(egui::RichText::new(place).heading());
                        ui.add_space(10.0);
                    }
                }
            });
        });
    }
}
