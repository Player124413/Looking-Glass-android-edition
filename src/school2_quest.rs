//! Ordered, idempotent school-two quest rules shared by play and diagnostics.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Stage {
    #[default]
    Explore,
    MushroomDialogue,
    Battle,
    Laboratory,
    Rescue,
    SpiceDialogue,
    Jumbogrow,
    Growing,
    Lollipop,
    FinalDialogue,
    Mixing,
    Rewards,
    Complete,
}
#[derive(Clone, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Items {
    pub mushroom: bool,
    pub spice: bool,
    pub jumbogrow: bool,
    pub lollipop: bool,
    pub potion: bool,
    pub star: bool,
}
impl Items {
    pub fn summary(&self) -> String {
        [
            (self.mushroom, "Mushroom"),
            (self.spice, "Spice Drops"),
            (self.jumbogrow, "Jumbogrow"),
            (self.lollipop, "Lollipop"),
            (self.potion, "Drink Me potion"),
            (self.star, "Lucky Star"),
        ]
        .into_iter()
        .filter_map(|(has, name)| has.then_some(name))
        .collect::<Vec<_>>()
        .join(" / ")
    }
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Quest {
    pub stage: Stage,
    pub items: Items,
    pub time: f32,
}
impl Quest {
    pub fn enter(&mut self, stage: Stage) {
        self.stage = stage;
        self.time = 0.;
    }
    pub fn begin(&mut self, event: &str) -> bool {
        let next = match (self.stage, event) {
            (Stage::Explore, "Old_Gnome_Mushroom") => Stage::MushroomDialogue,
            (Stage::Laboratory, "Kill_The_Gnome") => Stage::Rescue,
            (Stage::Jumbogrow, "Skool2_GrowLollypop") if self.items.jumbogrow => Stage::Growing,
            (Stage::Lollipop, "Skool2_LastGnome_Cinema")
                if self.items.lollipop && self.items.mushroom && self.items.spice =>
            {
                Stage::FinalDialogue
            }
            _ => return false,
        };
        if next == Stage::Growing {
            self.items.jumbogrow = false;
        }
        self.enter(next);
        true
    }
    pub fn dialogue_finished(&mut self, event: &str) -> bool {
        let next = match (self.stage, event) {
            (Stage::MushroomDialogue, "Old_Gnome_Mushroom") => {
                self.items.mushroom = true;
                Stage::Battle
            }
            (Stage::SpiceDialogue, "Old_Gnome_SpiceDrops") => {
                self.items.spice = true;
                Stage::Jumbogrow
            }
            (Stage::FinalDialogue, "Skool2_LastGnome_Cinema") => Stage::Mixing,
            _ => return false,
        };
        self.enter(next);
        true
    }
    pub fn tick(
        &mut self,
        dt: f32,
        boojums_dead: usize,
        guards_dead: usize,
    ) -> Option<&'static str> {
        if !dt.is_finite() || dt <= 0. {
            return None;
        }
        self.time += dt.min(0.1);
        match self.stage {
            Stage::Battle if boojums_dead == 3 => {
                self.enter(Stage::Laboratory);
            }
            Stage::Rescue if guards_dead == 2 => {
                self.enter(Stage::SpiceDialogue);
                return Some("Old_Gnome_SpiceDrops");
            }
            Stage::Growing if self.time >= 5.5 => {
                self.finish_growing();
            }
            Stage::Mixing if self.time >= 5. => {
                self.finish_mixing();
            }
            _ => {}
        }
        None
    }
    // Shared commits for timed legacy saves and the staged watch/skip paths.
    pub fn finish_growing(&mut self) -> bool {
        if self.stage != Stage::Growing {
            return false;
        }
        self.enter(Stage::Lollipop);
        true
    }
    pub fn finish_mixing(&mut self) -> bool {
        if self.stage != Stage::Mixing {
            return false;
        }
        self.items.mushroom = false;
        self.items.spice = false;
        self.items.lollipop = false;
        self.enter(Stage::Rewards);
        true
    }
    pub fn collect(&mut self, name: &str) -> bool {
        let item = match (self.stage, name) {
            (Stage::Jumbogrow, "jumbo_shelf1") => &mut self.items.jumbogrow,
            (Stage::Lollipop, "ig_lollypop") => &mut self.items.lollipop,
            (Stage::Rewards, "shrink_potion") => &mut self.items.potion,
            (Stage::Rewards, "lucky_star") => &mut self.items.star,
            _ => return false,
        };
        if *item {
            return false;
        }
        *item = true;
        if self.items.potion && self.items.star {
            self.enter(Stage::Complete);
        }
        true
    }
    pub fn objective(&self) -> String {
        match self.stage {
            Stage::Explore=>"School: use the gym lever, climb the bleachers and reach the Elder Gnome.",
            Stage::MushroomDialogue=>"Listen to the Elder Gnome / E advances dialogue.",
            Stage::Battle=>"School: defeat the three Boojums in the gym. Cards and the thrown Blade can reach them.",
            Stage::Laboratory|Stage::Rescue=>"School: return to the hall, climb the broken central stairs and reach the laboratory and defeat the two guards threatening the Gnome.",
            Stage::SpiceDialogue=>"Listen to the Gnome's directions / E advances dialogue.",
            Stage::Jumbogrow if !self.items.jumbogrow=>"School: collect Jumbogrow from the opened laboratory cabinet.",
            Stage::Jumbogrow=>"School: take Jumbogrow through a corridor mirror to the west classrooms and greenhouse lollipop.",
            Stage::Growing=>"The lollipop is growing. Wait for it to reach full size.",
            Stage::Lollipop if !self.items.lollipop=>"School: collect the grown lollipop.",
            Stage::Lollipop=>"School: bring the lollipop back to the Elder Gnome in the laboratory.",
            Stage::FinalDialogue|Stage::Mixing=>"The Gnome is preparing the shrinking potion.",
            Stage::Rewards=>"School: collect both the Drink Me potion and Lucky Star near the condenser.",
            Stage::Complete=>"School: the return portal is open. Take the potion and star back to the first school.",
        }.into()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn finish(q: &mut Quest) {
        assert!(q.begin("Old_Gnome_Mushroom"));
        assert!(q.dialogue_finished("Old_Gnome_Mushroom"));
        q.tick(0.1, 3, 0);
        assert!(q.begin("Kill_The_Gnome"));
        assert_eq!(q.tick(0.1, 3, 2), Some("Old_Gnome_SpiceDrops"));
        assert!(q.dialogue_finished("Old_Gnome_SpiceDrops"));
    }
    #[test]
    fn ordered_ingredients_transform_and_distinct_rewards_gate_exit() {
        let mut q = Quest::default();
        for item in ["jumbo_shelf1", "ig_lollypop", "shrink_potion", "lucky_star"] {
            assert!(!q.collect(item));
        }
        assert!(!q.begin("Kill_The_Gnome"));
        assert!(!q.begin("Skool2_LastGnome_Cinema"));
        finish(&mut q);
        assert!(q.items.mushroom && q.items.spice);
        assert!(!q.begin("Skool2_GrowLollypop"));
        assert!(q.collect("jumbo_shelf1"));
        assert!(!q.collect("jumbo_shelf1"));
        assert!(q.begin("Skool2_GrowLollypop"));
        assert!(!q.items.jumbogrow);
        assert!(!q.collect("ig_lollypop"));
        for _ in 0..56 {
            q.tick(0.1, 3, 2);
        }
        assert_eq!(q.stage, Stage::Lollipop);
        assert!(!q.begin("Skool2_LastGnome_Cinema"));
        assert!(q.collect("ig_lollypop"));
        assert!(q.begin("Skool2_LastGnome_Cinema"));
        assert!(!q.collect("shrink_potion"));
        assert!(q.dialogue_finished("Skool2_LastGnome_Cinema"));
        for _ in 0..51 {
            q.tick(0.1, 3, 2);
        }
        assert_eq!(q.stage, Stage::Rewards);
        assert!(!q.items.mushroom && !q.items.spice && !q.items.lollipop);
        assert!(q.collect("shrink_potion"));
        assert!(!q.collect("shrink_potion"));
        assert_eq!(q.stage, Stage::Rewards);
        assert!(q.collect("lucky_star"));
        assert_eq!(q.stage, Stage::Complete);
        assert!(!q.collect("lucky_star"));
        assert!(!q.begin("Old_Gnome_Mushroom"));
    }
    #[test]
    fn pause_repeated_callbacks_and_partial_fights_do_not_advance() {
        let mut q = Quest::default();
        q.begin("Old_Gnome_Mushroom");
        assert!(!q.dialogue_finished("Old_Gnome_SpiceDrops"));
        q.dialogue_finished("Old_Gnome_Mushroom");
        assert!(!q.dialogue_finished("Old_Gnome_Mushroom"));
        q.tick(0.1, 2, 2);
        assert_eq!(q.stage, Stage::Battle);
        q.tick(0., 3, 2);
        assert_eq!(q.stage, Stage::Battle);
        q.tick(0.1, 3, 2);
        assert_eq!(q.stage, Stage::Laboratory);
        q.begin("Kill_The_Gnome");
        q.tick(0.1, 3, 1);
        assert_eq!(q.stage, Stage::Rescue);
        let mut recovered = q.clone();
        recovered.tick(0., 3, 2);
        assert_eq!(recovered.stage, Stage::Rescue);
        assert_eq!(recovered.tick(0.1, 3, 2), Some("Old_Gnome_SpiceDrops"));
        assert_eq!(recovered.tick(0.1, 3, 2), None);
    }
}
