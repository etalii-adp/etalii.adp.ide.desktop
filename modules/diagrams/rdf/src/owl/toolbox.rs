//! The ontology palette (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Owl/OwlToolboxProvider.cs` at `8b6ccfc`: Class, Object property, Datatype property and Individual, each naming the
//! add action its drop commits. There is deliberately no restriction entry: a restriction needs a structure no drop
//! can state.

use adp_diagram::ToolboxItem;

use crate::actions;
use crate::toolbox::item;

/// The ontology palette.
pub fn items() -> Vec<ToolboxItem> {
    vec![
        item(
            "owl.toolbox.class",
            "Class",
            "mdi-shape-circle-plus",
            "A named class. Drop it where it should sit; you will be asked for its name.",
            actions::ADD_CLASS,
        ),
        item(
            "owl.toolbox.object-property",
            "Object property",
            "mdi-ray-start-arrow",
            "A property relating individuals to individuals - drawn as an edge once it has a domain and a range.",
            actions::ADD_OBJECT_PROPERTY,
        ),
        item(
            "owl.toolbox.datatype-property",
            "Datatype property",
            "mdi-form-textbox",
            "A property relating individuals to data values - drawn as an edge to its datatype.",
            actions::ADD_DATATYPE_PROPERTY,
        ),
        item(
            "owl.toolbox.individual",
            "Individual",
            "mdi-account-outline",
            "A named individual - a member of the classes this ontology describes.",
            actions::ADD_INDIVIDUAL,
        ),
    ]
}
