//! A single voice/speaker/model binding table; entity names retain map ownership.
#[derive(Debug)]
pub struct SpeakerSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub voice: &'static str,
    pub models: &'static [&'static str],
}
pub static SPEAKERS: &[SpeakerSpec] = &[
    SpeakerSpec {
        id: "alice",
        label: "Alice",
        voice: "sound/character/alice/",
        models: &["alice"],
    },
    SpeakerSpec {
        id: "cheshire",
        label: "Cheshire Cat",
        voice: "sound/character/cheshire_cat/",
        models: &["c_cheshire"],
    },
    SpeakerSpec {
        id: "elder",
        label: "Elder Gnome",
        voice: "sound/character/gnome/elder/",
        models: &["c_gnomeold"],
    },
    SpeakerSpec {
        id: "village",
        label: "Village Gnome",
        voice: "sound/character/gnome/torch/",
        models: &["c_torchgnome"],
    },
    SpeakerSpec {
        id: "rabbit",
        label: "White Rabbit",
        voice: "sound/character/white_rabbit/",
        models: &["c_whiterabbit"],
    },
    SpeakerSpec {
        id: "duchess",
        label: "Duchess",
        voice: "sound/character/duchess/",
        models: &["c_duchess"],
    },
    SpeakerSpec {
        id: "bill",
        label: "Bill the Lizard",
        voice: "sound/character/bill/",
        models: &["c_bill"],
    },
    SpeakerSpec {
        id: "turtle",
        label: "Mock Turtle",
        voice: "sound/character/mock_turtle/",
        models: &["c_mockturtle"],
    },
    SpeakerSpec {
        id: "caterpillar",
        label: "Caterpillar",
        voice: "sound/character/caterpillar/",
        models: &["c_caterpillar"],
    },
    SpeakerSpec {
        id: "centipede",
        label: "Centipede",
        voice: "sound/character/centipede/",
        models: &["c_centipede"],
    },
    SpeakerSpec {
        id: "gryphon",
        label: "Gryphon",
        voice: "sound/character/gryphon/",
        models: &["c_gryphon"],
    },
    SpeakerSpec {
        id: "king",
        label: "Chess King",
        voice: "sound/character/chess_piece/",
        models: &["c_chess_king"],
    },
    SpeakerSpec {
        id: "hatter",
        label: "Mad Hatter",
        voice: "sound/character/mad_hatter/",
        models: &["c_madhatter"],
    },
    SpeakerSpec {
        id: "tweedle",
        label: "Tweedle",
        voice: "sound/character/tweedle/",
        models: &["c_tweedle_dee", "c_tweedle_dum"],
    },
    SpeakerSpec {
        id: "hare",
        label: "March Hare",
        voice: "sound/character/march_hare/",
        models: &["c_marchhare"],
    },
    SpeakerSpec {
        id: "dormouse",
        label: "Dormouse",
        voice: "sound/character/dormouse/",
        models: &["c_dormouse"],
    },
    SpeakerSpec {
        id: "jabberwock",
        label: "Jabberwock",
        voice: "sound/character/jabberwock/",
        models: &["c_jabberwock"],
    },
    SpeakerSpec {
        id: "queen",
        label: "Queen",
        voice: "sound/character/queen/",
        models: &["c_queen1", "c_q2_body"],
    },
];
pub fn for_voice(path: &str) -> Option<&'static SpeakerSpec> {
    SPEAKERS.iter().find(|s| path.starts_with(s.voice))
}
pub fn models() -> impl Iterator<Item = &'static str> {
    SPEAKERS.iter().flat_map(|s| s.models.iter().copied())
}
/// Queen's first form has a rigid mask and no supplied mouth joint. Keep it
/// neutral rather than borrowing a head bone; the second form has tag_mouth.
pub fn neutral_model(model: &str) -> bool {
    model == "c_queen1"
}
pub fn focus(model: &str) -> &'static str {
    if neutral_model(model) {
        "Bip01 Head"
    } else if model == "c_jabberwock" {
        "Tag_mouth"
    } else {
        "tag_mouth"
    }
}
pub fn portrait_distance(model: &str) -> f32 {
    match model {
        "c_centipede" | "c_queen1" | "c_q2_body" => 350.,
        "c_jabberwock" => 220.,
        "c_gryphon" => 150.,
        _ => 75.,
    }
}
pub fn is_alice(actor: &str) -> bool {
    ["alice", "player", "fakeplayer"].contains(&actor)
}
pub fn same_actor(a: &str, b: &str) -> bool {
    a == b || (is_alice(a) && is_alice(b))
}
pub fn bind_actor<'a>(actor: &'a str, voice: &str) -> &'a str {
    // Reviewed misassignment in the school book scene. Do not animate Alice
    // with another character's recording or merge all Cat entity identities.
    if actor == "fakeplayer" && for_voice(voice).is_some_and(|s| s.id == "cheshire") {
        "book_cat"
    } else if actor == "fakeplayer" && for_voice(voice).is_some_and(|s| s.id == "gryphon") {
        "gryphon_actor1"
    } else if is_alice(actor)
        || (actor == "queen" && for_voice(voice).is_some_and(|s| s.id == "alice"))
    {
        "alice"
    } else {
        actor
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aliases_do_not_merge_different_placed_characters() {
        for a in ["alice", "player", "fakeplayer"] {
            for b in ["alice", "player", "fakeplayer"] {
                assert!(same_actor(a, b));
            }
        }
        assert!(!same_actor("book_cat", "shelf_cat"));
        assert!(!same_actor("old_gnome_1", "old_gnome_2"));
        assert_eq!(
            bind_actor("fakeplayer", "sound/character/cheshire_cat/vo/test.wav"),
            "book_cat"
        );
        assert_eq!(
            bind_actor("fakeplayer", "sound/character/alice/vo/test.wav"),
            "alice"
        );
        assert_eq!(
            bind_actor("fakeplayer", "sound/character/gryphon/vo/test.wav"),
            "gryphon_actor1"
        );
        assert!(for_voice("sound/character/unknown/vo/test.wav").is_none());
        let mut ids = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();
        for s in SPEAKERS {
            assert!(ids.insert(s.id));
            for model in s.models {
                assert!(names.insert(model));
            }
        }
    }
}
