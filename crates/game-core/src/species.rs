use crate::board::Terrain;
use serde::{Deserialize, Serialize};

/// Featured Western Ghats species. Habitat needs are deliberately simplified for
/// play; the Field Journal text in `content/species/` carries the real ecology
/// and sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Species {
    /// *Macaca silenus* — canopy-dwelling, fruit-eating, endangered; avoids open ground.
    LionTailedMacaque,
    /// *Ratufa indica* — large arboreal squirrel that travels through connected canopy.
    MalabarGiantSquirrel,
    /// *Buceros bicornis* — needs tall old nest trees and fruiting figs; flies across gaps.
    GreatHornbill,
    /// *Rhacophorus malabaricus* — breeds in foam nests on leaves overhanging water.
    MalabarGlidingFrog,
    /// *Nilgiritragus hylocrius* — grazes montane grassland, escapes to cliffs.
    NilgiriTahr,
}

impl Species {
    pub const ALL: [Species; 5] = [
        Species::LionTailedMacaque,
        Species::MalabarGiantSquirrel,
        Species::GreatHornbill,
        Species::MalabarGlidingFrog,
        Species::NilgiriTahr,
    ];

    pub fn common_name(self) -> &'static str {
        match self {
            Species::LionTailedMacaque => "Lion-tailed macaque",
            Species::MalabarGiantSquirrel => "Malabar giant squirrel",
            Species::GreatHornbill => "Great hornbill",
            Species::MalabarGlidingFrog => "Malabar gliding frog",
            Species::NilgiriTahr => "Nilgiri tahr",
        }
    }

    /// Flying species are not limited by ground or canopy routes.
    pub fn flies(self) -> bool {
        self == Species::GreatHornbill
    }

    /// Whether this species can live in or move through a cell of this terrain.
    pub fn route_suitable(self, t: Terrain) -> bool {
        match self {
            Species::LionTailedMacaque | Species::MalabarGiantSquirrel => {
                matches!(t, Terrain::Forest | Terrain::Grove | Terrain::Bridge)
            }
            // Frogs need moist forest floor and cannot use a rope bridge.
            Species::MalabarGlidingFrog => matches!(t, Terrain::Forest | Terrain::Grove),
            Species::NilgiriTahr => matches!(t, Terrain::Grassland | Terrain::Rock),
            Species::GreatHornbill => t != Terrain::River,
        }
    }
}
