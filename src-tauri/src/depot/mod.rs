pub mod catalogue;
pub mod comptes;
pub mod compteurs;
pub mod connexion;
pub mod horloge;
pub mod migrations;
pub mod parametres;
#[cfg(test)]
mod parcours_tests;
pub mod rapports;
pub mod sessions;
pub mod traduction;
pub mod ventes;

pub use connexion::Depot;
