pub mod input;

use std::{marker::PhantomData, path::PathBuf};

use crate::{
    geometry::{Hyperplane, Point},
    lang::{self, Language, Options},
    polytope::concrete::Concrete,
    polytope::Polytope,
    Float, OffOptions,
};

use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiSettings};
use rfd::FileDialog;

/// Guarantees that file dialogs will be opened on the main thread, used to
/// circumvent a MacOS limitation that all GUI operations must be done on the
/// main thread.
pub struct MainThreadToken(PhantomData<*const ()>);

impl MainThreadToken {
    /// Initializes a new token.
    pub fn new() -> Self {
        Self(Default::default())
    }

    /// Auxiliary function to create a new file dialog.
    fn new_file_dialog() -> FileDialog {
        FileDialog::new()
            .add_filter("OFF File", &["off"])
            .add_filter("GGB file", &["ggb"])
    }

    /// Returns the path given by an open file dialog.
    fn pick_file(&self) -> Option<PathBuf> {
        Self::new_file_dialog().pick_file()
    }

    /// Returns the path given by a save file dialog.
    fn save_file(&self, name: &str) -> Option<PathBuf> {
        Self::new_file_dialog().set_file_name(name).save_file()
    }
}

enum FileDialogMode {
    Disabled,
    Open,
    Save,
}

impl Default for FileDialogMode {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Default)]
pub struct FileDialogState {
    mode: FileDialogMode,
    name: Option<String>,
}

impl FileDialogState {
    pub fn open(&mut self) {
        self.mode = FileDialogMode::Open;
    }

    pub fn save(&mut self, name: String) {
        self.mode = FileDialogMode::Save;
        self.name = Some(name);
    }
}

/// Stores whether the cross-section view is active.
pub struct CrossSectionActive(pub bool);

impl CrossSectionActive {
    /// Flips the state.
    pub fn flip(&mut self) {
        self.0 = !self.0;
    }
}

/// Stores the state of the cross-section view.
pub struct CrossSectionState {
    /// The polytope from which the cross-section originates.
    original_polytope: Option<Concrete>,

    /// The position of the slicing hyperplane.
    hyperplane_pos: Float,

    /// Whether the cross-section is flattened into a dimension lower.
    flatten: bool,
}

impl Default for CrossSectionState {
    fn default() -> Self {
        Self {
            original_polytope: None,
            hyperplane_pos: 0.0,
            flatten: true,
        }
    }
}

/// The system in charge of the UI.
pub fn ui(
    egui_ctx: ResMut<EguiContext>,
    mut query: Query<&mut Concrete>,
    mut section_state: ResMut<CrossSectionState>,
    mut section_active: ResMut<CrossSectionActive>,
    mut file_dialog_state: ResMut<FileDialogState>,
) {
    let ctx = egui_ctx.ctx();

    egui::TopPanel::top("top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu(ui, "File", |ui| {
                // Loads a file.
                if ui.button("Load").clicked() {
                    file_dialog_state.open();
                }

                // Saves a file.
                if ui.button("Save").clicked() {
                    if let Some(p) = query.iter_mut().next() {
                        file_dialog_state.save(lang::En::parse(p.name(), Default::default()));
                    }
                }

                // Quits the application.
                if ui.button("Exit").clicked() {
                    std::process::exit(0);
                }
            });
        });

        ui.columns(6, |columns| {
            // Converts the active polytope into its dual.
            if columns[0].button("Dual").clicked() {
                for mut p in query.iter_mut() {
                    match p.dual_mut() {
                        Ok(_) => println!("Dual succeeded."),
                        Err(idx) => println!(
                            "Dual failed: Facet {} passes through inversion center.",
                            idx
                        ),
                    }

                    // If we're currently viewing a cross-section, it gets "fixed"
                    // as the active polytope.
                    section_state.original_polytope = None;
                    section_active.0 = false;

                    // Crashes for some reason.
                    // println!("{}", &p.concrete.to_src(off::OffOptions { comments: true }));
                }
            }

            // Converts the active polytope into any of its facets.
            if columns[1].button("Facet").clicked() {
                for mut p in query.iter_mut() {
                    println!("Facet");

                    if let Some(mut facet) = p.facet(0) {
                        facet.flatten();
                        facet.recenter();
                        *p = facet;
                        println!("Facet succeeded.")
                    } else {
                        println!("Facet failed.")
                    }

                    // If we're currently viewing a cross-section, it gets "fixed"
                    // as the active polytope.
                    section_state.original_polytope = None;
                    section_active.0 = false;
                }
            }

            // Converts the active polytope into any of its verfs.
            if columns[2].button("Verf").clicked() {
                for mut p in query.iter_mut() {
                    println!("Verf");

                    if let Some(mut facet) = p.verf(0) {
                        facet.flatten();
                        facet.recenter();
                        *p = facet;
                        println!("Verf succeeded.")
                    } else {
                        println!("Verf failed.")
                    }

                    // If we're currently viewing a cross-section, it gets "fixed"
                    // as the active polytope.
                    section_state.original_polytope = None;
                    section_active.0 = false;
                }
            }

            // Exports the active polytope as an OFF file (not yet functional!)
            if columns[3].button("Print OFF").clicked() {
                for p in query.iter_mut() {
                    println!("{}", p.to_off(OffOptions::default()));
                }
            }

            // Gets the volume of the polytope.
            if columns[4].button("Volume").clicked() {
                for p in query.iter_mut() {
                    if let Some(vol) = p.volume() {
                        println!("The volume is {}.", vol);
                    } else {
                        println!("The polytope has no volume.");
                    }
                }
            }

            // Toggles cross-section mode.
            if columns[5].button("Cross-section").clicked() {
                section_active.flip();
            }
        });

        ui.spacing_mut().slider_width = 800.0;

        // Sets the slider range to the range of x coordinates in the polytope.
        let mut new_hyperplane_pos = section_state.hyperplane_pos;
        let (x_min, x_max) = section_state
            .original_polytope
            .as_ref()
            .map(|p| p.x_minmax())
            .flatten()
            .unwrap_or((-1.0, 1.0));

        ui.add(
            egui::Slider::new(
                &mut new_hyperplane_pos,
                (x_min + 0.00001)..=(x_max - 0.00001),
            )
            .text("Slice depth"),
        );

        #[allow(clippy::float_cmp)]
        // Updates the slicing depth for the polytope, but only when needed.
        if section_state.hyperplane_pos != new_hyperplane_pos {
            section_state.hyperplane_pos = new_hyperplane_pos;
        }

        // Updates the flattening setting.
        let mut new_flatten = section_state.flatten;
        ui.add(egui::Checkbox::new(&mut new_flatten, "Flatten"));

        if section_state.flatten != new_flatten {
            section_state.flatten = new_flatten;
        }
    });
}

pub fn file_dialog(
    mut query: Query<&mut Concrete>,
    file_dialog_state: ResMut<FileDialogState>,
    token: NonSend<MainThreadToken>,
) {
    if file_dialog_state.is_changed() {
        match file_dialog_state.mode {
            FileDialogMode::Save => {
                if let Some(path) = token.save_file(file_dialog_state.name.as_ref().unwrap()) {
                    for p in query.iter_mut() {
                        std::fs::write(path.clone(), p.to_off(OffOptions::default())).unwrap();
                    }
                }
            }
            FileDialogMode::Open => {
                if let Some(path) = token.pick_file() {
                    for mut p in query.iter_mut() {
                        *p = Concrete::from_path(&path).unwrap();
                        p.recenter();
                    }
                }
            }
            _ => {}
        }
    }
}

/// Resizes the UI when the screen is resized.
pub fn update_scale_factor(mut egui_settings: ResMut<EguiSettings>, windows: Res<Windows>) {
    if let Some(window) = windows.get_primary() {
        egui_settings.scale_factor = 1.0 / window.scale_factor();
    }
}

/// Updates polytopes after an operation.
pub fn update_changed_polytopes(
    mut meshes: ResMut<Assets<Mesh>>,
    polies: Query<(&Concrete, &Handle<Mesh>, &Children), Changed<Concrete>>,
    wfs: Query<&Handle<Mesh>, Without<Concrete>>,
    mut windows: ResMut<Windows>,
) {
    for (poly, mesh_handle, children) in polies.iter() {
        let mesh: &mut Mesh = meshes.get_mut(mesh_handle).unwrap();
        *mesh = poly.get_mesh();

        windows
            .get_primary_mut()
            .unwrap()
            .set_title(lang::En::parse(poly.name(), Options::default()));

        for child in children.iter() {
            if let Ok(wf_handle) = wfs.get_component::<Handle<Mesh>>(*child) {
                let wf: &mut Mesh = meshes.get_mut(wf_handle).unwrap();
                *wf = poly.get_wireframe();

                break;
            }
        }
    }
}

/// Shows or hides the cross-section view.
pub fn update_cross_section_state(
    mut query: Query<&mut Concrete>,
    mut state: ResMut<CrossSectionState>,
    active: Res<CrossSectionActive>,
) {
    if active.is_changed() {
        if active.0 {
            state.original_polytope = Some(query.iter_mut().next().unwrap().clone());
        } else if let Some(p) = state.original_polytope.take() {
            *query.iter_mut().next().unwrap() = p;
        }
    }
}

/// Updates the cross-section shown.
pub fn update_cross_section(
    mut query: Query<&mut Concrete>,
    state: Res<CrossSectionState>,
    active: Res<CrossSectionActive>,
) {
    if state.is_changed() && active.0 {
        for mut p in query.iter_mut() {
            let r = state.original_polytope.clone().unwrap();
            let hyp_pos = state.hyperplane_pos + 0.0000001; // Botch fix for degeneracies.

            if let Some(dim) = r.dim() {
                let hyperplane = Hyperplane::x(dim, hyp_pos);
                let mut slice = r.slice(&hyperplane);

                if state.flatten {
                    slice.flatten_into(&hyperplane.subspace);
                    slice.recenter_with(
                        &hyperplane.flatten(&hyperplane.project(&Point::zeros(dim))),
                    );
                }

                *p = slice;
            }
        }
    }
}
