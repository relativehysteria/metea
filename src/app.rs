use std::collections::HashSet;
use serde::{Serialize, Deserialize};
use crate::geocoding::{GeoCoding, Place};
use crate::weather::Weather;
use crate::InternalStorage;

/// State of the application that will be persisted across runs.
#[derive(Default, Serialize, Deserialize)]
struct PersistedState {
    // TODO: This should be newtyped.
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

/// The screen that is currently shown.
enum Screen {
    Selection,
    Weather(Place),
}

/// The android metea application.
pub struct App {
    /// Interface to the application's internal storage.
    internal_storage: InternalStorage,

    /// State of the application that is persisted across runs.
    state: PersistedState,

    /// Interface to the open-meteo geocoding API.
    geocoding: GeoCoding,

    /// Interface to the open-meteo weather API.
    weather: Weather,

    /// The screen that should be currently shown.
    screen: Screen,
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
            screen: Screen::Selection,
            geocoding: GeoCoding::spawn_background_task(),
            weather: Weather::spawn_background_task(),
        }
    }

    /// Draw the weather screen.
    fn weather_screen(&mut self, ui: &mut egui::Ui, place: Place) {
        let place_string = place.to_string_coords();

        // Send the weather query for this place in case we haven't done so yet.
        if !self.weather.current.contains_key(&place_string) {
            self.weather.send_query(place.clone(), ui.ctx().clone());
        }

        // Receive results from the weather endpoint.
        self.weather.drain_responses();

        // Show the place title.
        ui.vertical_centered(|ui| {
            let title = egui::RichText::new(place.to_string()).heading();
            let title = ui.label(title);

            // Show a popup to let the user mutate the place, e.g. remove it.
            egui::Popup::menu(&title).align(egui::RectAlign::BOTTOM).show(|ui| {
                if ui.button("REMOVE").clicked() {
                    self.state.places.remove(&place_string);
                    let _ = self.state.save(&self.internal_storage);
                    self.screen = Screen::Selection;
                }
            });
        });

        ui.add_space(20.0);

        // Print the data for now.
        let hourly = self.weather.current.get(&place_string)
            .and_then(|o| o.as_ref());

        match hourly {
            Some(hourly) => { ui.label(format!("{hourly:?}")); },
            None => {},
        }
    }

    /// Draw the selection screen.
    fn selection_screen(&mut self, ui: &mut egui::Ui) {
        // Receive results from the geocoding endpoint.
        self.geocoding.drain_responses();

        ui.vertical_centered(|ui| {
            // Create the text input field.
            let input = egui::TextEdit::singleline(
                    &mut self.geocoding.current_query)
                .font(egui::TextStyle::Heading);
            let text_input = ui.add(input);

            // Save on Enter (when focus is in field).
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

            // Once the user presses enter, send the geocoding request.
            if text_input.lost_focus() && enter_pressed {
                self.geocoding.send_query();
                text_input.request_focus();
            }

            ui.add_space(20.0);

            // If we got some geocoding results, show them and allow the
            // user selection. Once one result is selected, it is saved and
            // the results are cleared.
            let mut clear = text_input.lost_focus();
            if !self.geocoding.search_results.is_empty() {
                ui.label("Results:");

                for place in &self.geocoding.search_results {
                    let place = place.to_string_coords();
                    let label = egui::RichText::new(&place).heading();

                    // Result selected; save it and clear the results.
                    if ui.button(label).clicked() {
                        self.state.places.insert(place);
                        let _ = self.state.save(&self.internal_storage);
                        clear = true;
                        break;
                    }
                    ui.add_space(10.0);
                }

                ui.add_space(10.0);
                ui.separator();

                // The user has either selected a result or the text input lost
                // focus; in either case, clear the search results.
                if clear { self.geocoding.search_results.clear(); }
            }

            ui.add_space(20.0);

            // Show saved coordinates.
            if !self.state.places.is_empty() {
                for place in &self.state.places {
                    let label = egui::RichText::new(place).heading();
                    if ui.label(label).clicked() {
                        self.screen = Screen::Weather(Place::from_string(
                                place).unwrap());
                    }

                    ui.add_space(10.0);
                }
            }
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Safe space hack.
        // https://github.com/rust-windowing/winit/issues/3910
        egui::Panel::top("safe_space_hack").show_inside(ui, |ui| {
            ui.set_height(32.0);
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match &self.screen {
                    Screen::Selection => self.selection_screen(ui),
                    Screen::Weather(place) =>
                        self.weather_screen(ui, place.clone()),
                }
            });
        });
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::BrowserBack)) {
            match self.screen {
                // XXX: A very brutal hack for now. Sending a minimize command
                //      to the viewport doesn't work; I'll figure it out later.
                Screen::Selection  => std::process::exit(0),
                Screen::Weather(_) => self.screen = Screen::Selection,
            }
        }
    }
}
