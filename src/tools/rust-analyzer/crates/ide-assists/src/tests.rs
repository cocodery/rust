mod generated;

use expect_test::expect;
use hir::Semantics;
use ide_db::{
    EditionedFileId, FileRange, RootDatabase, SnippetCap,
    assists::ExprFillDefaultMode,
    base_db::SourceDatabase,
    imports::insert_use::{ImportGranularity, InsertUseConfig},
    source_change::FileSystemEdit,
};
use stdx::{format_to, trim_indent};
use syntax::TextRange;
use test_fixture::WithFixture;
use test_utils::{assert_eq_text, extract_offset};

use crate::{
    Assist, AssistConfig, AssistContext, AssistKind, AssistResolveStrategy, Assists, SingleResolve,
    assists, handlers::Handler,
};

pub(crate) const TEST_CONFIG: AssistConfig = AssistConfig {
    snippet_cap: SnippetCap::new(true),
    allowed: None,
    insert_use: InsertUseConfig {
        granularity: ImportGranularity::Crate,
        prefix_kind: hir::PrefixKind::Plain,
        enforce_granularity: true,
        group: true,
        skip_glob_imports: true,
    },
    prefer_no_std: false,
    prefer_prelude: true,
    prefer_absolute: false,
    assist_emit_must_use: false,
    term_search_fuel: 400,
    term_search_borrowck: true,
    code_action_grouping: true,
    expr_fill_default: ExprFillDefaultMode::Todo,
    prefer_self_ty: false,
};

pub(crate) const TEST_CONFIG_NO_GROUPING: AssistConfig = AssistConfig {
    snippet_cap: SnippetCap::new(true),
    allowed: None,
    insert_use: InsertUseConfig {
        granularity: ImportGranularity::Crate,
        prefix_kind: hir::PrefixKind::Plain,
        enforce_granularity: true,
        group: true,
        skip_glob_imports: true,
    },
    prefer_no_std: false,
    prefer_prelude: true,
    prefer_absolute: false,
    assist_emit_must_use: false,
    term_search_fuel: 400,
    term_search_borrowck: true,
    code_action_grouping: false,
    expr_fill_default: ExprFillDefaultMode::Todo,
    prefer_self_ty: false,
};

pub(crate) const TEST_CONFIG_NO_SNIPPET_CAP: AssistConfig = AssistConfig {
    snippet_cap: None,
    allowed: None,
    insert_use: InsertUseConfig {
        granularity: ImportGranularity::Crate,
        prefix_kind: hir::PrefixKind::Plain,
        enforce_granularity: true,
        group: true,
        skip_glob_imports: true,
    },
    prefer_no_std: false,
    prefer_prelude: true,
    prefer_absolute: false,
    assist_emit_must_use: false,
    term_search_fuel: 400,
    term_search_borrowck: true,
    code_action_grouping: true,
    expr_fill_default: ExprFillDefaultMode::Todo,
    prefer_self_ty: false,
};

pub(crate) const TEST_CONFIG_IMPORT_ONE: AssistConfig = AssistConfig {
    snippet_cap: SnippetCap::new(true),
    allowed: None,
    insert_use: InsertUseConfig {
        granularity: ImportGranularity::One,
        prefix_kind: hir::PrefixKind::Plain,
        enforce_granularity: true,
        group: true,
        skip_glob_imports: true,
    },
    prefer_no_std: false,
    prefer_prelude: true,
    prefer_absolute: false,
    assist_emit_must_use: false,
    term_search_fuel: 400,
    term_search_borrowck: true,
    code_action_grouping: true,
    expr_fill_default: ExprFillDefaultMode::Todo,
    prefer_self_ty: false,
};

pub(crate) fn with_single_file(text: &str) -> (RootDatabase, EditionedFileId) {
    RootDatabase::with_single_file(text)
}

#[track_caller]
pub(crate) fn check_assist(
    assist: Handler,
    #[rust_analyzer::rust_fixture] ra_fixture_before: &str,
    #[rust_analyzer::rust_fixture] ra_fixture_after: &str,
) {
    let ra_fixture_after = trim_indent(ra_fixture_after);
    check(assist, ra_fixture_before, ExpectedResult::After(&ra_fixture_after), None);
}

#[track_caller]
pub(crate) fn check_assist_with_config(
    assist: Handler,
    config: AssistConfig,
    #[rust_analyzer::rust_fixture] ra_fixture_before: &str,
    #[rust_analyzer::rust_fixture] ra_fixture_after: &str,
) {
    let ra_fixture_after = trim_indent(ra_fixture_after);
    check_with_config(
        config,
        assist,
        ra_fixture_before,
        ExpectedResult::After(&ra_fixture_after),
        None,
    );
}

#[track_caller]
pub(crate) fn check_assist_no_snippet_cap(
    assist: Handler,
    #[rust_analyzer::rust_fixture] ra_fixture_before: &str,
    #[rust_analyzer::rust_fixture] ra_fixture_after: &str,
) {
    let ra_fixture_after = trim_indent(ra_fixture_after);
    check_with_config(
        TEST_CONFIG_NO_SNIPPET_CAP,
        assist,
        ra_fixture_before,
        ExpectedResult::After(&ra_fixture_after),
        None,
    );
}

#[track_caller]
pub(crate) fn check_assist_import_one(
    assist: Handler,
    #[rust_analyzer::rust_fixture] ra_fixture_before: &str,
    #[rust_analyzer::rust_fixture] ra_fixture_after: &str,
) {
    let ra_fixture_after = trim_indent(ra_fixture_after);
    check_with_config(
        TEST_CONFIG_IMPORT_ONE,
        assist,
        ra_fixture_before,
        ExpectedResult::After(&ra_fixture_after),
        None,
    );
}

// There is no way to choose what assist within a group you want to test against,
// so this is here to allow you choose.
pub(crate) fn check_assist_by_label(
    assist: Handler,
    #[rust_analyzer::rust_fixture] ra_fixture_before: &str,
    #[rust_analyzer::rust_fixture] ra_fixture_after: &str,
    label: &str,
) {
    let ra_fixture_after = trim_indent(ra_fixture_after);
    check(assist, ra_fixture_before, ExpectedResult::After(&ra_fixture_after), Some(label));
}

// FIXME: instead of having a separate function here, maybe use
// `extract_ranges` and mark the target as `<target> </target>` in the
// fixture?
#[track_caller]
pub(crate) fn check_assist_target(
    assist: Handler,
    #[rust_analyzer::rust_fixture] ra_fixture: &str,
    target: &str,
) {
    check(assist, ra_fixture, ExpectedResult::Target(target), None);
}

#[track_caller]
pub(crate) fn check_assist_not_applicable(
    assist: Handler,
    #[rust_analyzer::rust_fixture] ra_fixture: &str,
) {
    check(assist, ra_fixture, ExpectedResult::NotApplicable, None);
}

#[track_caller]
pub(crate) fn check_assist_not_applicable_by_label(
    assist: Handler,
    #[rust_analyzer::rust_fixture] ra_fixture: &str,
    label: &str,
) {
    check(assist, ra_fixture, ExpectedResult::NotApplicable, Some(label));
}

#[track_caller]
pub(crate) fn check_assist_not_applicable_for_import_one(
    assist: Handler,
    #[rust_analyzer::rust_fixture] ra_fixture: &str,
) {
    check_with_config(
        TEST_CONFIG_IMPORT_ONE,
        assist,
        ra_fixture,
        ExpectedResult::NotApplicable,
        None,
    );
}

#[track_caller]
pub(crate) fn check_assist_not_applicable_no_grouping(
    assist: Handler,
    #[rust_analyzer::rust_fixture] ra_fixture: &str,
) {
    check_with_config(
        TEST_CONFIG_NO_GROUPING,
        assist,
        ra_fixture,
        ExpectedResult::NotApplicable,
        None,
    );
}

/// Check assist in unresolved state. Useful to check assists for lazy computation.
#[track_caller]
pub(crate) fn check_assist_unresolved(
    assist: Handler,
    #[rust_analyzer::rust_fixture] ra_fixture: &str,
) {
    check(assist, ra_fixture, ExpectedResult::Unresolved, None);
}

#[track_caller]
fn check_doc_test(assist_id: &str, before: &str, after: &str) {
    let after = trim_indent(after);
    let (db, file_id, selection) = RootDatabase::with_range_or_offset(before);
    let before = db.file_text(file_id.file_id(&db)).text(&db).to_string();
    let frange = ide_db::FileRange { file_id: file_id.file_id(&db), range: selection.into() };

    let assist = assists(&db, &TEST_CONFIG, AssistResolveStrategy::All, frange)
        .into_iter()
        .find(|assist| assist.id.0 == assist_id)
        .unwrap_or_else(|| {
            panic!(
                "\n\nAssist is not applicable: {}\nAvailable assists: {}",
                assist_id,
                assists(&db, &TEST_CONFIG, AssistResolveStrategy::None, frange)
                    .into_iter()
                    .map(|assist| assist.id.0)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });

    let actual = {
        let source_change = assist
            .source_change
            .filter(|it| !it.source_file_edits.is_empty() || !it.file_system_edits.is_empty())
            .expect("Assist did not contain any source changes");
        let mut actual = before;
        if let Some((source_file_edit, snippet_edit)) =
            source_change.get_source_and_snippet_edit(file_id.file_id(&db))
        {
            source_file_edit.apply(&mut actual);
            if let Some(snippet_edit) = snippet_edit {
                snippet_edit.apply(&mut actual);
            }
        }
        actual
    };
    assert_eq_text!(&after, &actual);
}

enum ExpectedResult<'a> {
    NotApplicable,
    Unresolved,
    After(&'a str),
    Target(&'a str),
}

#[track_caller]
fn check(handler: Handler, before: &str, expected: ExpectedResult<'_>, assist_label: Option<&str>) {
    check_with_config(TEST_CONFIG, handler, before, expected, assist_label);
}

#[track_caller]
fn check_with_config(
    config: AssistConfig,
    handler: Handler,
    before: &str,
    expected: ExpectedResult<'_>,
    assist_label: Option<&str>,
) {
    let (mut db, file_with_caret_id, range_or_offset) = RootDatabase::with_range_or_offset(before);
    db.enable_proc_attr_macros();
    let text_without_caret = db.file_text(file_with_caret_id.file_id(&db)).text(&db).to_string();

    let frange = hir::FileRange { file_id: file_with_caret_id, range: range_or_offset.into() };

    let sema = Semantics::new(&db);
    let ctx = AssistContext::new(sema, &config, frange);
    let resolve = match expected {
        ExpectedResult::Unresolved => AssistResolveStrategy::None,
        _ => AssistResolveStrategy::All,
    };
    let mut acc = Assists::new(&ctx, resolve);
    handler(&mut acc, &ctx);
    let mut res = acc.finish();

    let assist = match assist_label {
        Some(label) => res.into_iter().find(|resolved| resolved.label == label),
        None if res.is_empty() => None,
        // Pick the first as that is the one with the highest priority
        None => Some(res.swap_remove(0)),
    };

    match (assist, expected) {
        (Some(assist), ExpectedResult::After(after)) => {
            let source_change = assist
                .source_change
                .filter(|it| !it.source_file_edits.is_empty() || !it.file_system_edits.is_empty())
                .expect("Assist did not contain any source changes");
            let skip_header = source_change.source_file_edits.len() == 1
                && source_change.file_system_edits.is_empty();

            let mut buf = String::new();
            for (file_id, (edit, snippet_edit)) in source_change.source_file_edits {
                let mut text = db.file_text(file_id).text(&db).as_ref().to_owned();
                edit.apply(&mut text);
                if let Some(snippet_edit) = snippet_edit {
                    snippet_edit.apply(&mut text);
                }
                if !skip_header {
                    let source_root_id = db.file_source_root(file_id).source_root_id(&db);
                    let sr = db.source_root(source_root_id).source_root(&db);
                    let path = sr.path_for_file(&file_id).unwrap();
                    format_to!(buf, "//- {}\n", path)
                }
                buf.push_str(&text);
            }

            for file_system_edit in source_change.file_system_edits {
                let (dst, contents) = match file_system_edit {
                    FileSystemEdit::CreateFile { dst, initial_contents } => (dst, initial_contents),
                    FileSystemEdit::MoveFile { src, dst } => {
                        (dst, db.file_text(src).text(&db).as_ref().to_owned())
                    }
                    FileSystemEdit::MoveDir { src, src_id, dst } => {
                        // temporary placeholder for MoveDir since we are not using MoveDir in ide assists yet.
                        (dst, format!("{src_id:?}\n{src:?}"))
                    }
                };

                let source_root_id = db.file_source_root(dst.anchor).source_root_id(&db);
                let sr = db.source_root(source_root_id).source_root(&db);
                let mut base = sr.path_for_file(&dst.anchor).unwrap().clone();
                base.pop();
                let created_file_path = base.join(&dst.path).unwrap();
                format_to!(buf, "//- {}\n", created_file_path);
                buf.push_str(&contents);
            }

            assert_eq_text!(after, &buf);
        }
        (Some(assist), ExpectedResult::Target(target)) => {
            let range = assist.target;
            assert_eq_text!(&text_without_caret[range], target);
        }
        (Some(assist), ExpectedResult::Unresolved) => assert!(
            assist.source_change.is_none(),
            "unresolved assist should not contain source changes"
        ),
        (Some(_), ExpectedResult::NotApplicable) => panic!("assist should not be applicable!"),
        (
            None,
            ExpectedResult::After(_) | ExpectedResult::Target(_) | ExpectedResult::Unresolved,
        ) => {
            panic!("code action is not applicable")
        }
        (None, ExpectedResult::NotApplicable) => (),
    };
}

fn labels(assists: &[Assist]) -> String {
    let mut labels = assists
        .iter()
        .map(|assist| {
            let mut label = match &assist.group {
                Some(g) => g.0.clone(),
                None => assist.label.to_string(),
            };
            label.push('\n');
            label
        })
        .collect::<Vec<_>>();
    labels.dedup();
    labels.into_iter().collect::<String>()
}

#[test]
fn assist_order_field_struct() {
    let before = "struct Foo { $0bar: u32 }";
    let (before_cursor_pos, before) = extract_offset(before);
    let (db, file_id) = with_single_file(&before);
    let frange =
        FileRange { file_id: file_id.file_id(&db), range: TextRange::empty(before_cursor_pos) };
    let assists = assists(&db, &TEST_CONFIG, AssistResolveStrategy::None, frange);
    let mut assists = assists.iter();

    assert_eq!(assists.next().expect("expected assist").label, "Change visibility to pub(crate)");
    assert_eq!(assists.next().expect("expected assist").label, "Generate a getter method");
    assert_eq!(assists.next().expect("expected assist").label, "Generate a mut getter method");
    assert_eq!(assists.next().expect("expected assist").label, "Generate a setter method");
    assert_eq!(assists.next().expect("expected assist").label, "Add `#[derive]`");
    assert_eq!(assists.next().expect("expected assist").label, "Generate `new`");
    assert_eq!(assists.next().map(|it| it.label.to_string()), None);
}

#[test]
fn assist_order_if_expr() {
    let (db, frange) = RootDatabase::with_range(
        r#"
pub fn test_some_range(a: int) -> bool {
    if let 2..6 = $05$0 {
        true
    } else {
        false
    }
}
"#,
    );

    let assists = assists(
        &db,
        &TEST_CONFIG,
        AssistResolveStrategy::None,
        FileRange { file_id: frange.file_id.file_id(&db), range: frange.range },
    );
    let expected = labels(&assists);

    expect![[r#"
        Convert integer base
        Extract into...
        Replace if let with match
    "#]]
    .assert_eq(&expected);
}

#[test]
fn assist_filter_works() {
    let (db, frange) = RootDatabase::with_range(
        r#"
pub fn test_some_range(a: int) -> bool {
    if let 2..6 = $05$0 {
        true
    } else {
        false
    }
}
"#,
    );
    {
        let mut cfg = TEST_CONFIG;
        cfg.allowed = Some(vec![AssistKind::Refactor]);

        let assists = assists(
            &db,
            &cfg,
            AssistResolveStrategy::None,
            FileRange { file_id: frange.file_id.file_id(&db), range: frange.range },
        );
        let expected = labels(&assists);

        expect![[r#"
            Convert integer base
            Extract into...
            Replace if let with match
        "#]]
        .assert_eq(&expected);
    }

    {
        let mut cfg = TEST_CONFIG;
        cfg.allowed = Some(vec![AssistKind::RefactorExtract]);
        let assists = assists(
            &db,
            &cfg,
            AssistResolveStrategy::None,
            FileRange { file_id: frange.file_id.file_id(&db), range: frange.range },
        );
        let expected = labels(&assists);

        expect![[r#"
            Extract into...
        "#]]
        .assert_eq(&expected);
    }

    {
        let mut cfg = TEST_CONFIG;
        cfg.allowed = Some(vec![AssistKind::QuickFix]);
        let assists = assists(
            &db,
            &cfg,
            AssistResolveStrategy::None,
            FileRange { file_id: frange.file_id.file_id(&db), range: frange.range },
        );
        let expected = labels(&assists);

        expect![[r#""#]].assert_eq(&expected);
    }
}

#[test]
fn various_resolve_strategies() {
    let (db, frange) = RootDatabase::with_range(
        r#"
pub fn test_some_range(a: int) -> bool {
    if let 2..6 = $05$0 {
        true
    } else {
        false
    }
}
"#,
    );

    let mut cfg = TEST_CONFIG;
    cfg.allowed = Some(vec![AssistKind::RefactorExtract]);

    {
        let assists = assists(
            &db,
            &cfg,
            AssistResolveStrategy::None,
            FileRange { file_id: frange.file_id.file_id(&db), range: frange.range },
        );
        assert_eq!(4, assists.len());
        let mut assists = assists.into_iter();

        let extract_into_variable_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_variable",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into variable",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_variable_assist);

        let extract_into_constant_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_constant",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into constant",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_constant_assist);

        let extract_into_static_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_static",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into static",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_static_assist);

        let extract_into_function_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_function",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into function",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_function_assist);
    }

    {
        let assists = assists(
            &db,
            &cfg,
            AssistResolveStrategy::Single(SingleResolve {
                assist_id: "SOMETHING_MISMATCHING".to_owned(),
                assist_kind: AssistKind::RefactorExtract,
                assist_subtype: None,
            }),
            FileRange { file_id: frange.file_id.file_id(&db), range: frange.range },
        );
        assert_eq!(4, assists.len());
        let mut assists = assists.into_iter();

        let extract_into_variable_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_variable",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into variable",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_variable_assist);

        let extract_into_constant_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_constant",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into constant",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_constant_assist);

        let extract_into_static_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_static",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into static",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_static_assist);

        let extract_into_function_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_function",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into function",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_function_assist);
    }

    {
        let assists = assists(
            &db,
            &cfg,
            AssistResolveStrategy::Single(SingleResolve {
                assist_id: "extract_variable".to_owned(),
                assist_kind: AssistKind::RefactorExtract,
                assist_subtype: None,
            }),
            FileRange { file_id: frange.file_id.file_id(&db), range: frange.range },
        );
        assert_eq!(4, assists.len());
        let mut assists = assists.into_iter();

        let extract_into_variable_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_variable",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into variable",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: Some(
                    SourceChange {
                        source_file_edits: {
                            FileId(
                                0,
                            ): (
                                TextEdit {
                                    indels: [
                                        Indel {
                                            insert: "let",
                                            delete: 45..47,
                                        },
                                        Indel {
                                            insert: "var_name",
                                            delete: 48..60,
                                        },
                                        Indel {
                                            insert: "=",
                                            delete: 61..81,
                                        },
                                        Indel {
                                            insert: "5;\n    if let 2..6 = var_name {\n        true\n    } else {\n        false\n    }",
                                            delete: 82..108,
                                        },
                                    ],
                                    annotation: None,
                                },
                                Some(
                                    SnippetEdit(
                                        [
                                            (
                                                0,
                                                49..49,
                                            ),
                                        ],
                                    ),
                                ),
                            ),
                        },
                        file_system_edits: [],
                        is_snippet: true,
                        annotations: {},
                        next_annotation_id: 0,
                    },
                ),
                command: Some(
                    Rename,
                ),
            }
        "#]]
        .assert_debug_eq(&extract_into_variable_assist);

        let extract_into_constant_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_constant",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into constant",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_constant_assist);

        let extract_into_static_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_static",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into static",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_static_assist);

        let extract_into_function_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_function",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into function",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: None,
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_function_assist);
    }

    {
        let assists = assists(
            &db,
            &cfg,
            AssistResolveStrategy::All,
            FileRange { file_id: frange.file_id.file_id(&db), range: frange.range },
        );
        assert_eq!(4, assists.len());
        let mut assists = assists.into_iter();

        let extract_into_variable_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_variable",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into variable",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: Some(
                    SourceChange {
                        source_file_edits: {
                            FileId(
                                0,
                            ): (
                                TextEdit {
                                    indels: [
                                        Indel {
                                            insert: "let",
                                            delete: 45..47,
                                        },
                                        Indel {
                                            insert: "var_name",
                                            delete: 48..60,
                                        },
                                        Indel {
                                            insert: "=",
                                            delete: 61..81,
                                        },
                                        Indel {
                                            insert: "5;\n    if let 2..6 = var_name {\n        true\n    } else {\n        false\n    }",
                                            delete: 82..108,
                                        },
                                    ],
                                    annotation: None,
                                },
                                Some(
                                    SnippetEdit(
                                        [
                                            (
                                                0,
                                                49..49,
                                            ),
                                        ],
                                    ),
                                ),
                            ),
                        },
                        file_system_edits: [],
                        is_snippet: true,
                        annotations: {},
                        next_annotation_id: 0,
                    },
                ),
                command: Some(
                    Rename,
                ),
            }
        "#]]
        .assert_debug_eq(&extract_into_variable_assist);

        let extract_into_constant_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_constant",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into constant",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: Some(
                    SourceChange {
                        source_file_edits: {
                            FileId(
                                0,
                            ): (
                                TextEdit {
                                    indels: [
                                        Indel {
                                            insert: "const",
                                            delete: 45..47,
                                        },
                                        Indel {
                                            insert: "VAR_NAME:",
                                            delete: 48..60,
                                        },
                                        Indel {
                                            insert: "i32",
                                            delete: 61..81,
                                        },
                                        Indel {
                                            insert: "=",
                                            delete: 82..86,
                                        },
                                        Indel {
                                            insert: "5;\n    if let 2..6 = VAR_NAME {\n        true\n    } else {\n        false\n    }",
                                            delete: 87..108,
                                        },
                                    ],
                                    annotation: None,
                                },
                                Some(
                                    SnippetEdit(
                                        [
                                            (
                                                0,
                                                51..51,
                                            ),
                                        ],
                                    ),
                                ),
                            ),
                        },
                        file_system_edits: [],
                        is_snippet: true,
                        annotations: {},
                        next_annotation_id: 0,
                    },
                ),
                command: Some(
                    Rename,
                ),
            }
        "#]]
        .assert_debug_eq(&extract_into_constant_assist);

        let extract_into_static_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_static",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into static",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: Some(
                    SourceChange {
                        source_file_edits: {
                            FileId(
                                0,
                            ): (
                                TextEdit {
                                    indels: [
                                        Indel {
                                            insert: "static",
                                            delete: 45..47,
                                        },
                                        Indel {
                                            insert: "VAR_NAME:",
                                            delete: 48..60,
                                        },
                                        Indel {
                                            insert: "i32",
                                            delete: 61..81,
                                        },
                                        Indel {
                                            insert: "=",
                                            delete: 82..86,
                                        },
                                        Indel {
                                            insert: "5;\n    if let 2..6 = VAR_NAME {\n        true\n    } else {\n        false\n    }",
                                            delete: 87..108,
                                        },
                                    ],
                                    annotation: None,
                                },
                                Some(
                                    SnippetEdit(
                                        [
                                            (
                                                0,
                                                52..52,
                                            ),
                                        ],
                                    ),
                                ),
                            ),
                        },
                        file_system_edits: [],
                        is_snippet: true,
                        annotations: {},
                        next_annotation_id: 0,
                    },
                ),
                command: Some(
                    Rename,
                ),
            }
        "#]]
        .assert_debug_eq(&extract_into_static_assist);

        let extract_into_function_assist = assists.next().unwrap();
        expect![[r#"
            Assist {
                id: AssistId(
                    "extract_function",
                    RefactorExtract,
                    None,
                ),
                label: "Extract into function",
                group: Some(
                    GroupLabel(
                        "Extract into...",
                    ),
                ),
                target: 59..60,
                source_change: Some(
                    SourceChange {
                        source_file_edits: {
                            FileId(
                                0,
                            ): (
                                TextEdit {
                                    indels: [
                                        Indel {
                                            insert: "fun_name()",
                                            delete: 59..60,
                                        },
                                        Indel {
                                            insert: "\n\nfn fun_name() -> i32 {\n    5\n}",
                                            delete: 110..110,
                                        },
                                    ],
                                    annotation: None,
                                },
                                Some(
                                    SnippetEdit(
                                        [
                                            (
                                                0,
                                                124..124,
                                            ),
                                        ],
                                    ),
                                ),
                            ),
                        },
                        file_system_edits: [],
                        is_snippet: true,
                        annotations: {},
                        next_annotation_id: 0,
                    },
                ),
                command: None,
            }
        "#]]
        .assert_debug_eq(&extract_into_function_assist);
    }
}
