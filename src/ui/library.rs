use std::{
    ffi::{OsStr, OsString},
    fs, io,
    path::PathBuf,
};

use bevy_egui::egui::{self, Ui};
use serde::{Deserialize, Serialize};
use strum_macros::Display;

use crate::{
    lang::{
        name::{Con, Name as LangName},
        SelectedLanguage,
    },
    polytope::{concrete::Concrete, r#abstract::rank::Rank},
};

/// Represents any of the special polytopes in Miratope's library, namely those
/// families that are generated by code.
#[derive(Clone, Serialize, Deserialize, Debug, Display)]
pub enum SpecialLibrary {
    /// Allows one to select a regular polygon.
    #[strum(serialize = "Regular polygon")]
    Polygon(usize, usize),

    /// Allows one to select a (uniform 3D) antiprism.
    Antiprism(usize, usize),

    /// Allows one to select a simplex.
    Simplex(Rank),

    /// Allows one to select a hypercube.
    Hypercube(Rank),

    /// Allows one to select an orthoplex.
    Orthoplex(Rank),
}

impl SpecialLibrary {
    /// Shows the special component of the library.
    pub fn show(&mut self, ui: &mut Ui, _selected_language: SelectedLanguage) -> ShowResult {
        let text = self.to_string();

        match self {
            // An {n/d} regular polygon.
            Self::Polygon(n, d) => {
                let mut clicked = false;

                ui.horizontal(|ui| {
                    clicked = ui.button(text).clicked();

                    // Number of sides.
                    ui.label("n:");
                    ui.add(
                        egui::DragValue::new(n)
                            .speed(0.25)
                            .clamp_range(2..=usize::MAX),
                    );

                    // Turning number.
                    ui.label("d:");
                    ui.add(egui::DragValue::new(d).speed(0.25).clamp_range(1..=*n / 2));
                });

                if clicked {
                    ShowResult::Special(self.clone())
                } else {
                    ShowResult::None
                }
            }
            Self::Antiprism(n, d) => {
                let mut clicked = false;

                ui.horizontal(|ui| {
                    clicked = ui.button(text).clicked();

                    // Number of sides.
                    ui.label("n:");
                    ui.add(
                        egui::DragValue::new(n)
                            .speed(0.25)
                            .clamp_range(2..=usize::MAX),
                    );

                    // Turning number.
                    ui.label("d:");
                    ui.add(
                        egui::DragValue::new(d)
                            .speed(0.25)
                            .clamp_range(1..=(*n * 2 / 3)),
                    );
                });

                if clicked {
                    ShowResult::Special(self.clone())
                } else {
                    ShowResult::None
                }
            }
            Self::Simplex(rank) | Self::Hypercube(rank) | Self::Orthoplex(rank) => {
                let mut clicked = false;

                ui.horizontal(|ui| {
                    clicked = ui.button(text).clicked();

                    // Rank.
                    ui.label("Rank:");
                    ui.add(egui::DragValue::new(rank).speed(0.1).clamp_range(-1..=20));
                });

                if clicked {
                    ShowResult::Special(self.clone())
                } else {
                    ShowResult::None
                }
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Name {
    /// A name in its language-independent representation.
    Name(LangName<Con>),

    /// A literal string name.
    Literal(String),
}

impl Name {
    /// This is running at 60 FPS but name parsing isn't blazing fast. Maybe
    /// do some sort of cacheing in the future?
    pub fn parse(&self, selected_language: SelectedLanguage) -> String {
        match self {
            Self::Name(name) => selected_language.parse_uppercase(name, Default::default()),
            Self::Literal(name) => name.clone(),
        }
    }
}

/// Represents any of the files or folders that make up the Miratope library.
#[derive(Serialize, Deserialize)]
pub enum Library {
    /// A folder whose contents have not yet been read.
    UnloadedFolder {
        folder_name: String,
        name: Name,
    },

    /// A folder whose contents have been read.
    LoadedFolder {
        folder_name: String,
        name: Name,
        contents: Vec<Library>,
    },

    /// A file that can be loaded into Miratope.
    File {
        file_name: String,
        name: Name,
    },

    Special(SpecialLibrary),
}

/// The result of showing the Miratope library every frame.
pub enum ShowResult {
    /// Nothing happened this frame.
    None,

    /// We asked to load a file.
    Load(OsString),

    /// We asked to load a special polytope.
    Special(SpecialLibrary),
}

impl std::ops::BitOr for ShowResult {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        if let Self::None = self {
            rhs
        } else {
            self
        }
    }
}

impl std::ops::BitOrAssign for ShowResult {
    fn bitor_assign(&mut self, rhs: Self) {
        if !matches!(rhs, Self::None) {
            *self = rhs;
        }
    }
}

pub fn path_to_str(path: PathBuf) -> String {
    String::from(path.file_name().unwrap().to_str().unwrap())
}

impl Library {
    /// Loads the data from a file at a given path.
    pub fn new_file(path: &impl AsRef<OsStr>) -> Self {
        let path = PathBuf::from(&path);
        let name = if let Some(name) = Concrete::name_from_off(&path) {
            Name::Name(name)
        } else {
            Name::Literal(String::from(
                path.file_stem().map(|f| f.to_str()).flatten().unwrap_or(""),
            ))
        };

        Self::File {
            file_name: path_to_str(path),
            name,
        }
    }

    /// Creates a new unloaded folder from a given path.
    pub fn new_folder(path: &impl AsRef<OsStr>) -> Self {
        let path = PathBuf::from(&path);
        assert!(path.is_dir(), "Path {:?} not a directory!", path);

        // Attempts to read from the .name file.
        if let Ok(Ok(name)) = fs::read(path.join(".name"))
            .map(|file| ron::from_str(&String::from_utf8(file).unwrap()))
        {
            Self::UnloadedFolder {
                folder_name: path_to_str(path),
                name,
            }
        }
        // Else, takes the name from the folder itself.
        else {
            let name = Name::Literal(String::from(
                path.file_name()
                    .map(|name| name.to_str())
                    .flatten()
                    .unwrap_or(""),
            ));

            Self::UnloadedFolder {
                folder_name: String::from(path.file_name().unwrap().to_str().unwrap()),
                name,
            }
        }
    }

    /// Reads a folder's data from the `.folder` file. If it doesn't exist, it
    /// defaults to loading the folder's name and its data in alphabetical order.
    /// If that also fails, it returns an `Err`.
    pub fn folder_contents(path: &impl AsRef<OsStr>) -> io::Result<Vec<Self>> {
        let path = PathBuf::from(&path);
        assert!(path.is_dir(), "Path {:?} not a directory!", path);

        // Attempts to read from the .folder file.
        Ok(
            if let Some(Ok(folder)) = fs::read(path.join(".folder"))
                .ok()
                .map(|file| ron::from_str(&String::from_utf8(file).unwrap()))
            {
                folder
            }
            // Otherwise, just manually goes through the files.
            else {
                let mut contents = Vec::new();

                for entry in fs::read_dir(path.clone())? {
                    let path = &entry?.path();

                    // Adds a new unloaded folder.
                    if path.is_dir() {
                        contents.push(Self::new_folder(path));
                    }
                    // Adds a new file.
                    else {
                        let ext = path.extension();

                        if ext == Some(OsStr::new("off")) || ext == Some(OsStr::new("ggb")) {
                            contents.push(Self::new_file(path));
                        }
                    }
                }

                // We cache these contents for future use.
                fs::write(path.join(".folder"), ron::to_string(&contents).unwrap()).unwrap();
                println!(".folder file overwritten!");

                contents
            },
        )
    }

    /// Shows the library from the root.
    pub fn show_root(&mut self, ui: &mut Ui, selected_language: SelectedLanguage) -> ShowResult {
        self.show(ui, PathBuf::new(), selected_language)
    }

    /// Shows the library.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        mut path: PathBuf,
        selected_language: SelectedLanguage,
    ) -> ShowResult {
        match self {
            // Shows a collapsing drop-down, and loads the folder in case it's clicked.
            Self::UnloadedFolder { folder_name, name } => {
                // Clones so that the closure doesn't require unique access.
                let folder_name = folder_name.clone();
                let name = name.clone();

                path.push(folder_name);
                let mut res = ShowResult::None;

                ui.collapsing(name.parse(selected_language), |ui| {
                    let mut contents = Self::folder_contents(&path).unwrap();

                    // Contents of drop down.
                    for lib in contents.iter_mut() {
                        res |= lib.show(ui, path.clone(), selected_language);
                    }

                    // Opens the folder.
                    *self = Self::LoadedFolder {
                        folder_name: path_to_str(path),
                        name,
                        contents,
                    };
                });

                res
            }
            // Shows a drop-down with all of the files and folders.
            Self::LoadedFolder {
                folder_name,
                name,
                contents,
            } => {
                path.push(&folder_name);
                let mut res = ShowResult::None;

                ui.collapsing(name.parse(selected_language), |ui| {
                    for lib in contents.iter_mut() {
                        res |= lib.show(ui, path.clone(), selected_language);
                    }
                });

                res
            }
            // Shows a button that loads the file if clicked.
            Self::File { file_name, name } => {
                path.push(file_name);

                if ui.button(name.parse(selected_language)).clicked() {
                    ShowResult::Load(path.into_os_string())
                } else {
                    ShowResult::None
                }
            }
            Self::Special(special) => special.show(ui, selected_language),
        }
    }
}
