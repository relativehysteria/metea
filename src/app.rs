use std::collections::HashSet;
use std::path::PathBuf;
use crate::geocoding::GeoCoding;

/// The android metea application.
pub struct App {
    /// Application's persistent internal storage which will be used as a cache.
    internal_storage: Option<PathBuf>,

    /// Vector of places that have been saved by the user.
    saved_places: HashSet<String>,

    /// Interface to the open-meteo geocoding API.
    geocoding: GeoCoding,
}

impl App {
    /// Create the application state.
    pub fn new(
        _cc: &eframe::CreationContext,
        internal_storage: Option<PathBuf>,
    ) -> Self {
        Self {
            internal_storage,
            saved_places: HashSet::new(),
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

                // Save on Enter (when focus is in field)
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

                        // Result selected; save it and request search result
                        // clear.
                        if ui.button(&label).clicked() {
                            self.saved_places.insert(label);
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
                if !self.saved_places.is_empty() {
                    for place in &self.saved_places {
                        ui.label(egui::RichText::new(place).heading());
                        ui.add_space(10.0);
                    }
                }
            });
        });
    }
}
