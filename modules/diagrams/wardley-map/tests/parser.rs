//! The `.owm` reader (etalii.adp spec 030, task T070), ported from the standalone host's `WardleyParser.Tests.cs` and
//! `WardleyParser.Vocabulary.Tests.cs` at `8b6ccfc`. `Parse_RejectsNull` has no counterpart: a Rust reference is never
//! null.

mod common;

use adp_diagram_wardley_map::lines::LineDocument;
use adp_diagram_wardley_map::model::{
    ParseProblem, WardleyAttitudeKind, WardleyCoordinate, WardleyDecorator, WardleyElementKind, WardleyLabelOffset,
    WardleyLinkKind, WardleyMapSize, WardleyPipelineForm,
};
use adp_diagram_wardley_map::{empty_document, parser};
use common::{corpus, fixture, parse_fixture, parse_text};

#[test]
fn parse_never_throws_for_anything_in_the_corpus() {
    let names = corpus();
    assert_eq!(names.len(), 13);
    for name in names {
        // Parsing and judging are separate: a map with a dangling link or an unknown keyword still reads.
        parse_fixture(&name);
    }
}

#[test]
fn outcome_reports_each_line_it_passed_over_and_not_the_statements_it_skips_by_design() {
    let (map, problems) = parser::outcome(&LineDocument::parse(
        "title A map\ncomponent Kettle [0.5, 0.4]\nevolve\ny_axis Value chain->Invisible->Visible\n// a comment\n\nthis is nothing\n",
    ));
    assert_eq!(
        problems,
        vec![
            ParseProblem {
                line: 2,
                message: "This evolve statement could not be read, so it was passed over.".to_owned()
            },
            ParseProblem {
                line: 6,
                message: "This line is not a statement of a Wardley map, so it was passed over.".to_owned()
            },
        ]
    );
    assert_eq!(map.components.len(), 1);
    assert_eq!(map.components[0].name, "Kettle");
}

#[test]
fn outcome_of_the_corpus_reports_only_the_statement_no_parser_knows() {
    for name in corpus() {
        let (_, problems) = parser::outcome(&LineDocument::parse(&fixture(&name)));
        // The unmodelled-statements fixture holds one line no Wardley parser knows, on its 14th line.
        if name == "unmodelled-statements.owm" {
            assert_eq!(problems.len(), 1, "{name}");
            assert_eq!(problems[0].line, 13);
        } else {
            assert!(problems.is_empty(), "{name}: {problems:?}");
        }
    }
}

#[test]
fn outcome_reports_a_line_inside_a_pipeline_that_is_not_a_child() {
    let (_, problems) = parser::outcome(&LineDocument::parse(
        "component Kettle [0.5, 0.4]\npipeline Kettle\n{\n  component Electric [0.6]\n  nothing at all\n}\n",
    ));
    assert_eq!(
        problems,
        vec![ParseProblem {
            line: 4,
            message: "This line inside a pipeline is not a component of it, so it was passed over.".to_owned()
        }]
    );
}

#[test]
fn parse_reads_the_three_statement_kinds() {
    let map = parse_text("component Alpha [0.80, 0.40]\nanchor Customer [0.95, 0.63]\nsubmap Billing [0.45, 0.68]");
    assert_eq!(
        map.components.iter().map(|c| c.kind).collect::<Vec<_>>(),
        [
            WardleyElementKind::Component,
            WardleyElementKind::Anchor,
            WardleyElementKind::Submap
        ]
    );
}

#[test]
fn parse_reads_market_and_ecosystem_as_decorators_not_as_kinds() {
    let map = parse_text("component BetaMarket [0.66, 0.88] (market)\ncomponent GammaEco [0.40, 0.47] (ecosystem)");
    assert!(map.components.iter().all(|c| c.kind == WardleyElementKind::Component));
    assert_eq!(map.components[0].decorators, [WardleyDecorator::Market]);
    assert_eq!(map.components[1].decorators, [WardleyDecorator::Ecosystem]);
}

#[test]
fn parse_ignores_a_market_keyword_statement_because_the_format_has_no_such_statement() {
    let map = parse_text("component Alpha [0.80, 0.40]\nmarket BetaMarket [0.66, 0.88]");
    assert_eq!(
        map.components.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["Alpha"]
    );
}

#[test]
fn parse_reads_coordinates_as_visibility_then_maturity() {
    let map = parse_text("component Alpha [0.80, 0.40]");
    assert_eq!(map.components[0].position.visibility, 0.80);
    assert_eq!(map.components[0].position.maturity, 0.40);
}

#[test]
fn parse_reads_all_five_decorators_and_inertia() {
    let map = parse_text(
        "component A [0.5, 0.5] (build)\ncomponent B [0.5, 0.5] (buy)\ncomponent C [0.5, 0.5] (outsource)\ncomponent D [0.5, 0.5] (market)\ncomponent E [0.5, 0.5] (ecosystem)\ncomponent F [0.5, 0.5] inertia",
    );
    assert_eq!(map.components[0].decorators, [WardleyDecorator::Build]);
    assert_eq!(map.components[1].decorators, [WardleyDecorator::Buy]);
    assert_eq!(map.components[2].decorators, [WardleyDecorator::Outsource]);
    assert_eq!(map.components[3].decorators, [WardleyDecorator::Market]);
    assert_eq!(map.components[4].decorators, [WardleyDecorator::Ecosystem]);
    // inertia is a boolean, not a decorator.
    assert!(map.components[5].inertia);
    assert!(map.components[5].decorators.is_empty());
}

#[test]
fn parse_ignores_an_unknown_decorator_rather_than_refusing_the_line() {
    let map = parse_text("component Future [0.5, 0.5] (someDecoratorFromLater)");
    assert_eq!(map.components.len(), 1);
    assert!(map.components[0].decorators.is_empty());
}

#[test]
fn parse_reads_link_forms() {
    for (text, kind, context) in [
        ("A->B", WardleyLinkKind::Dependency, ""),
        ("A+>B", WardleyLinkKind::Flow, ""),
        (
            "A->B; via the plugin SDK",
            WardleyLinkKind::Dependency,
            "via the plugin SDK",
        ),
    ] {
        let map = parse_text(text);
        assert_eq!(map.links.len(), 1, "{text}");
        let link = &map.links[0];
        assert_eq!(link.source, "A");
        assert_eq!(link.target, "B");
        assert_eq!(link.kind, kind);
        assert_eq!(link.context, context);
    }
}

#[test]
fn parse_reads_link_endpoints_with_spaces_in_their_names() {
    let map = parse_text("Cup of Tea->Hot Water");
    assert_eq!(map.links.len(), 1);
    assert_eq!(map.links[0].source, "Cup of Tea");
    assert_eq!(map.links[0].target, "Hot Water");
}

#[test]
fn parse_reads_the_nested_pipeline_form_with_children_carrying_maturity_only() {
    let map = parse_text(
        "component Kettle [0.43, 0.35]\npipeline Kettle\n{\n  component Campfire Kettle [0.15]\n  component Electric Kettle [0.63]\n}",
    );
    assert_eq!(map.pipelines.len(), 1);
    let pipeline = &map.pipelines[0];
    assert_eq!(pipeline.parent, "Kettle");
    assert_eq!(pipeline.form, WardleyPipelineForm::Nested);
    assert_eq!(
        pipeline.children.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["Campfire Kettle", "Electric Kettle"]
    );
    assert_eq!(
        pipeline.children.iter().map(|c| c.maturity).collect::<Vec<_>>(),
        [0.15, 0.63]
    );
}

#[test]
fn parse_reads_the_legacy_pipeline_form_and_remembers_it_was_legacy() {
    let map = parse_text("pipeline Power [0.30, 0.85]");
    assert_eq!(map.pipelines.len(), 1);
    let pipeline = &map.pipelines[0];
    assert_eq!(pipeline.parent, "Power");
    assert_eq!(pipeline.form, WardleyPipelineForm::Legacy);
    assert!(pipeline.children.is_empty());
    assert_eq!(pipeline.legacy_extent, Some(WardleyCoordinate::new(0.30, 0.85)));
}

#[test]
fn parse_reads_both_pipeline_forms_in_one_document() {
    let map = parse_fixture("pipelines-both-forms.owm");
    assert_eq!(map.pipelines.len(), 3);
    assert!(map.pipelines.iter().any(|p| p.form == WardleyPipelineForm::Nested));
    assert!(map.pipelines.iter().any(|p| p.form == WardleyPipelineForm::Legacy));
}

#[test]
fn parse_tolerates_an_empty_pipeline() {
    let map = parse_text("component Water [0.38, 0.82]\npipeline Water\n{\n}");
    assert_eq!(map.pipelines.len(), 1);
    assert!(map.pipelines[0].children.is_empty());
}

#[test]
fn parse_reads_label_offsets_and_urls() {
    let map = parse_text("component Kettle [0.43, 0.35] label [-57, 4]\nsubmap Site [0.83, 0.50] url(siteUrl)");
    // The offset is in pixels, and negative: a property of the format.
    assert_eq!(
        map.components[0].label_offset,
        Some(WardleyLabelOffset { x: -57.0, y: 4.0 })
    );
    assert_eq!(map.components[1].url, "siteUrl");
}

#[test]
fn parse_reads_the_map_level_statements() {
    let map = parse_text("title Strategy vocabulary\nsize [900,700]\nstyle wardley");
    assert_eq!(map.title, "Strategy vocabulary");
    assert_eq!(
        map.size,
        Some(WardleyMapSize {
            width: 900.0,
            height: 700.0
        })
    );
    assert_eq!(map.style, "wardley");
}

#[test]
fn parse_records_the_declaring_line_one_based() {
    let map = parse_text("title Lines\ncomponent Alpha [0.80, 0.40]");
    assert_eq!(map.components[0].line, 2);
}

#[test]
fn parse_ignores_comments_including_trailing_ones() {
    let map = parse_fixture("comments-everywhere.owm");
    assert_eq!(
        map.components.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["Customer", "Cup of Tea", "Kettle"]
    );
    assert_eq!(map.links.len(), 2);
    assert_eq!(map.links[1].target, "Kettle");
}

#[test]
fn parse_skips_statements_it_does_not_model_without_losing_the_ones_it_does() {
    let map = parse_fixture("unmodelled-statements.owm");
    assert!(map.components.iter().any(|c| c.name == "Cup of Tea"));
    assert_eq!(map.links.len(), 1);
}

#[test]
fn parse_reads_the_tea_shop_end_to_end() {
    let map = parse_fixture("tea-shop.owm");
    assert_eq!(map.title, "Tea shop");
    assert_eq!(
        map.components
            .iter()
            .filter(|c| c.kind == WardleyElementKind::Anchor)
            .count(),
        2
    );
    assert_eq!(
        map.components
            .iter()
            .filter(|c| c.kind == WardleyElementKind::Component)
            .count(),
        7
    );
    assert_eq!(map.links.len(), 8);
}

#[test]
fn parse_reads_the_strategy_vocabulary_end_to_end() {
    let map = parse_fixture("strategy-vocabulary.owm");
    for decorator in WardleyDecorator::ALL {
        assert!(
            map.components.iter().any(|c| c.decorators.contains(&decorator)),
            "{decorator:?}"
        );
    }
    assert!(map.components.iter().any(|c| c.inertia));
    assert!(map.links.iter().any(|l| l.kind == WardleyLinkKind::Flow));
    assert!(map.links.iter().any(|l| !l.context.is_empty()));
}

#[test]
fn parse_does_not_mistake_a_keyword_statement_for_a_link_even_when_it_contains_an_arrow() {
    for text in [
        "y_axis Value chain->Invisible->Visible",
        "evolve Datacentre->Cloud Hosting 0.83",
    ] {
        assert!(parse_text(text).links.is_empty(), "{text}");
    }
}

#[test]
fn parse_reads_an_empty_document_as_an_empty_map() {
    let map = parse_text("");
    assert!(map.components.is_empty());
    assert!(map.links.is_empty());
    assert_eq!(map.title, "");
}

#[test]
fn parse_reads_the_new_empty_document() {
    let map = parse_text(&empty_document("map"));
    assert_eq!(map.title, "map");
    assert!(map.components.is_empty());
}

#[test]
fn parse_uses_invariant_number_parsing() {
    let map = parse_text("component Alpha [0.79, 0.61]");
    assert_eq!(map.components[0].position.visibility, 0.79);
}

// ---- WardleyParser.Vocabulary.Tests.cs ------------------------------------------------------------------------------

#[test]
fn parse_reads_evolve_without_a_rename() {
    let map = parse_text("evolve Fraud Scoring 0.72");
    assert_eq!(map.evolves.len(), 1);
    assert_eq!(map.evolves[0].name, "Fraud Scoring");
    assert_eq!(map.evolves[0].maturity, 0.72);
    assert_eq!(map.evolves[0].override_name, "");
}

#[test]
fn parse_reads_evolve_with_a_rename_and_keeps_the_arrival_name() {
    let map = parse_text("evolve Datacentre->Cloud Hosting 0.83");
    assert_eq!(map.evolves.len(), 1);
    assert_eq!(map.evolves[0].name, "Datacentre");
    assert_eq!(map.evolves[0].override_name, "Cloud Hosting");
    assert_eq!(map.evolves[0].maturity, 0.83);
}

#[test]
fn parse_does_not_read_an_evolve_as_a_link_despite_the_arrow() {
    assert!(parse_text("evolve Datacentre->Cloud Hosting 0.83").links.is_empty());
}

#[test]
fn parse_reads_the_three_attitudes() {
    for (keyword, expected) in [
        ("pioneers", WardleyAttitudeKind::Pioneers),
        ("settlers", WardleyAttitudeKind::Settlers),
        ("townplanners", WardleyAttitudeKind::TownPlanners),
    ] {
        let map = parse_text(&format!("{keyword} [0.30, 0.20, 0.55, 0.45]"));
        assert_eq!(map.attitudes.len(), 1);
        assert_eq!(map.attitudes[0].kind, expected);
    }
}

#[test]
fn parse_reads_an_attitudes_four_numbers_as_visibility_first() {
    let map = parse_text("pioneers [0.30, 0.20, 0.55, 0.45]");
    assert_eq!(map.attitudes[0].from, WardleyCoordinate::new(0.30, 0.20));
    assert_eq!(map.attitudes[0].to, WardleyCoordinate::new(0.55, 0.45));
}

#[test]
fn parse_reads_accelerators_and_deaccelerators_into_one_collection() {
    let map = parse_text("accelerator Regulatory pressure [0.70, 0.60]\ndeaccelerator Legacy contracts [0.45, 0.30]");
    assert_eq!(map.accelerators.len(), 2);
    assert!(!map.accelerators[0].is_deaccelerator);
    assert_eq!(map.accelerators[0].name, "Regulatory pressure");
    assert!(map.accelerators[1].is_deaccelerator);
    assert_eq!(map.accelerators[1].position, WardleyCoordinate::new(0.45, 0.30));
}

#[test]
fn parse_reads_a_note() {
    let map = parse_text("note Renewal is due in March [0.22, 0.30]");
    assert_eq!(map.notes.len(), 1);
    assert_eq!(map.notes[0].text, "Renewal is due in March");
    assert_eq!(map.notes[0].position, WardleyCoordinate::new(0.22, 0.30));
}

#[test]
fn parse_reads_an_annotation_pinned_at_one_place() {
    let map = parse_text("annotation 2 [0.48, 0.85] A single-position annotation");
    assert_eq!(map.annotations.len(), 1);
    let annotation = &map.annotations[0];
    assert_eq!(annotation.number, 2);
    assert_eq!(annotation.occurrences, [WardleyCoordinate::new(0.48, 0.85)]);
    assert_eq!(annotation.text, "A single-position annotation");
}

#[test]
fn parse_reads_an_annotation_pinned_at_several_places_losing_none() {
    let map = parse_text("annotation 1 [[0.43,0.49],[0.08,0.79]] Standardising power");
    let annotation = &map.annotations[0];
    assert_eq!(annotation.number, 1);
    assert_eq!(
        annotation.occurrences,
        [WardleyCoordinate::new(0.43, 0.49), WardleyCoordinate::new(0.08, 0.79)]
    );
    assert_eq!(annotation.text, "Standardising power");
}

#[test]
fn parse_reads_the_annotations_block_position_without_mistaking_it_for_an_annotation() {
    let map = parse_text("annotation 1 [0.43, 0.49] Text\nannotations [0.60, 0.02]");
    assert_eq!(map.annotations.len(), 1);
    assert_eq!(map.annotations_position, Some(WardleyCoordinate::new(0.60, 0.02)));
}

#[test]
fn parse_reads_a_url_definition() {
    let map = parse_text("url fulfilmentMap [https://onlinewardleymaps.com/#clone:example]");
    assert_eq!(map.urls.len(), 1);
    assert_eq!(map.urls[0].name, "fulfilmentMap");
    assert_eq!(map.urls[0].address, "https://onlinewardleymaps.com/#clone:example");
}

#[test]
fn parse_does_not_mistake_a_url_scheme_for_a_comment() {
    let map = parse_text("url siteUrl [https://example.com/map] // and a real comment");
    assert_eq!(map.urls[0].address, "https://example.com/map");
}

#[test]
fn parse_starts_a_comment_at_a_slash_pair_inside_a_quoted_name() {
    let map = parse_text("component \"Cache // fast\" [0.40, 0.60]\r\ncomponent Plain [0.30, 0.20]\r\n");
    assert_eq!(map.components.len(), 1);
    assert_eq!(map.components[0].name, "Plain");
}

#[test]
fn parse_links_a_submap_to_its_url_by_name() {
    let map = parse_fixture("submaps-and-urls.owm");
    assert_eq!(map.urls.len(), 2);
    let submaps: Vec<_> = map
        .components
        .iter()
        .filter(|c| c.kind == WardleyElementKind::Submap)
        .collect();
    assert_eq!(submaps.len(), 2);
    for submap in submaps {
        assert!(map.urls.iter().any(|u| u.name == submap.url), "{}", submap.name);
    }
}

#[test]
fn parse_reads_the_annotations_and_labels_fixture_whole() {
    let map = parse_fixture("annotations-and-labels.owm");
    assert_eq!(map.annotations.len(), 2);
    assert_eq!(
        map.annotations
            .iter()
            .find(|a| a.number == 1)
            .unwrap()
            .occurrences
            .len(),
        2
    );
    assert_eq!(
        map.annotations
            .iter()
            .find(|a| a.number == 2)
            .unwrap()
            .occurrences
            .len(),
        1
    );
    assert!(map.annotations_position.is_some());
    assert_eq!(map.notes.len(), 1);
    assert_eq!(map.components.iter().filter(|c| c.label_offset.is_some()).count(), 3);
}

#[test]
fn parse_reads_the_strategy_vocabulary_fixture_whole() {
    let map = parse_fixture("strategy-vocabulary.owm");
    assert_eq!(map.evolves.len(), 2);
    assert!(map.evolves.iter().any(|e| e.override_name == "Cloud Hosting"));
    assert_eq!(map.attitudes.len(), 3);
    assert_eq!(map.accelerators.len(), 2);
    assert!(map.accelerators.iter().any(|a| a.is_deaccelerator));
    assert_eq!(map.notes.len(), 1);
    assert_eq!(
        map.size,
        Some(WardleyMapSize {
            width: 900.0,
            height: 700.0
        })
    );
}

#[test]
fn parse_leaves_the_vocabulary_empty_for_a_map_that_uses_none_of_it() {
    let map = parse_fixture("minimal.owm");
    assert!(map.evolves.is_empty());
    assert!(map.attitudes.is_empty());
    assert!(map.accelerators.is_empty());
    assert!(map.notes.is_empty());
    assert!(map.annotations.is_empty());
    assert!(map.urls.is_empty());
    assert!(map.annotations_position.is_none());
}
