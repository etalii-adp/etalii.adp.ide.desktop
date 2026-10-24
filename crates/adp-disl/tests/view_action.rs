//! The `view` action (DISL section 9.4): a viewer's view of an element, handed back to the host with the values it
//! sets, and nothing written to the model (etalii.adp spec 035, T127).

mod common;

use adp_cel::Value;
use adp_disl::behavior::{ids, operations};
use adp_disl::{DislDiagram, HostAction};
use common::{attrs, loaded, with};

#[test]
fn a_view_action_hands_the_viewers_view_back_and_changes_no_model() {
    let specification = loaded(&with(
        r#""metamodel": { "types": { "Topic": { "attributes": { "name": { "type": "string" } } } } },
        "behavior": {
          "operations": {
            "fold": { "label": "Fold", "for": ["Topic"], "actions": [ { "view": { "collapsed": "self.name != ''" } } ] },
            "foldOther": { "label": "Fold other", "for": ["Topic"], "actions": [ { "view": { "collapsed": "false" }, "target": "self" } ] }
          }
        }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    let topic = diagram
        .add_node("Topic", "t", attrs([("name", "Plan".into())]))
        .unwrap();
    for operation in ["fold", "foldOther"] {
        let transaction = operations::run(
            &specification,
            operation,
            &diagram,
            Some(&topic),
            &ids::fixed(&[]),
            None,
            None,
        );
        assert!(transaction.refusal.is_none(), "{:?}", transaction.refusal);
        assert!(transaction.changes.is_empty());
        let [HostAction::View { element_id, values }] = &transaction.host_actions[..] else {
            panic!("{:?}", transaction.host_actions);
        };
        assert_eq!(element_id, "t");
        assert!(matches!(values.get("collapsed"), Some(Value::Bool(_))));
    }
}
