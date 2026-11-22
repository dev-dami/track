# Porting the cargo test suites to tests/run.sh

Card S2. Every `#[test]` (and `#[tokio::test]`) in `tests/*.rs`, its class,
and where it lives now. The `.rs` suites stay until card S5 deletes them.

Classes:

- **A**: tests trackc, the runtimes, the native backend, the bootstrap or
  the programs. Ported.
- **B**: also tested the Rust stage0 (its parser, checker, codegen, or
  "both compilers agree" parity). The trackc half is ported, the stage0
  half dropped. Where parity with stage0 was the point, goldens recorded
  from stage0 stand in for it (`tests/cases/examples/*.out`).
- **C**: tests yard or the LSP. Covered by `tests/yard/run.sh` (PR #36)
  and `tests/lsp/run.sh` (PR #37); the case is named.
- **D**: editor assets. Ported when the assets survive the Rust deletion.

Totals: 165 tests (162 `#[test]`, 3 `#[tokio::test]` in lsp_tests.rs).
A 80 ported; B 63 (60 trackc halves ported, 3 dropped); C 12 mapped
(9 yard, 3 LSP); D 10 ported. No class-A test is dropped.

Known differences from the Rust checks:

- A signal death is checked as exit status 128+N (SIGFPE = 136); a program
  calling `exit(136)` would also pass.
- `editor_tests` / `headless_highlight_tests` read JSON with `jq` and run
  grammar regexes with `grep -P`; the nvim checks skip without nvim, as
  before.

Run: `tests/run.sh [-j N]` (see its header). Locations are
`tests/sh/<suite>.sh::<function>` (bare `lang.sh::x` means
`tests/sh/lang.sh::t_x`); data lives in `tests/cases/` and the existing
`tests/<dir>/` goldens.

| Rust test | Class | New location | Note / reason dropped |
|---|---|---|---|
| checker_tests.rs::test_for_over_scalar_is_rejected | B | checker_verdicts rows checker_for_over_scalar_is_rejected + checker_for_over_range_and_array_ok | |
| checker_tests.rs::test_linear_checker_borrow_and_escape | B | checker_verdicts row checker_borrow_and_escape | |
| checker_tests.rs::test_linear_checker_enum_and_union_matching | B | checker_verdicts row checker_enum_and_union_matching | |
| checker_tests.rs::test_linear_checker_escape_error | B | checker_verdicts row checker_escape_error | |
| checker_tests.rs::test_linear_checker_extended_char_str_path_api | B | checker_verdicts row checker_extended_char_str_path_api | |
| checker_tests.rs::test_linear_checker_lens_block_locked_error | B | checker_verdicts row checker_lens_block_locked_error | "frozen" |
| checker_tests.rs::test_linear_checker_lens_block_valid_and_locked | B | checker_verdicts row checker_lens_block_valid_and_locked | |
| checker_tests.rs::test_linear_checker_linear_type_transfer | B | checker_verdicts row checker_linear_type_transfer | |
| checker_tests.rs::test_linear_checker_type_alias_and_os_api | B | checker_verdicts row checker_type_alias_and_os_api | |
| checker_tests.rs::test_linear_checker_typo_suggestion | B | checker_verdicts row checker_typo_suggestion | "Did you mean 'print'?" |
| checker_tests.rs::test_linear_checker_use_after_free_error | B | checker_verdicts row checker_use_after_free_error | |
| checker_tests.rs::test_linear_checker_valid_code | B | checker_verdicts row checker_valid_code | trackc half of a stage0 checker test |
| cli_tests.rs::test_cli_build_and_run_generics | A | cli.sh::cli_build_and_run_generics | stricter: runs the exe, stdout 7 |
| cli_tests.rs::test_cli_check_command | A | cli.sh::cli_check_command | "Check passed" |
| cli_tests.rs::test_cli_check_const_across_functions | A | lang.sh::checker_verdicts row cli_check_const_across_functions | |
| cli_tests.rs::test_cli_check_generics_arity_mismatch | A | lang.sh::checker_verdicts row cli_check_generics_arity_mismatch | stricter: pins E0200 arity text |
| cli_tests.rs::test_cli_check_generics_valid | A | lang.sh::checker_verdicts row cli_check_generics_valid | |
| cli_tests.rs::test_cli_check_invalid_syntax_fails | A | lang.sh::checker_verdicts row cli_check_invalid_syntax_fails | stricter: pins E0100 |
| cli_tests.rs::test_cli_duplicate_function_is_a_diagnostic | A | cli.sh::cli_duplicate_function_is_a_diagnostic | check+build exit 1, E0201 text |
| cli_tests.rs::test_cli_duplicate_function_through_import_is_a_diagnostic | A | cli.sh::cli_duplicate_function_through_import_is_a_diagnostic | |
| cli_tests.rs::test_cli_help | A | tests/sh/cli.sh::cli_help | Rust `track` CLI -> trackc, same commands/messages |
| cli_tests.rs::test_cli_invalid_command | A | cli.sh::cli_invalid_command | "Unknown command" |
| cli_tests.rs::test_cli_version_reports_0_6 | A | cli.sh::cli_version_reports_1_0_0 | asserts 1.0.0 as the Rust test did |
| codegen_semantics_tests.rs::array_index_out_of_bounds_aborts | A | lang.sh::array_index_out_of_bounds_aborts | exit 134 + stderr |
| codegen_semantics_tests.rs::block_bindings_do_not_leak_into_the_enclosing_scope | A | lang.sh::block_bindings_do_not_leak_into_the_enclosing_scope | |
| codegen_semantics_tests.rs::constants_fold_comparisons_and_variants | A | lang.sh::constants_fold_comparisons_and_variants | |
| codegen_semantics_tests.rs::division_by_zero_flushes_and_raises_sigfpe | A | lang.sh::division_by_zero_flushes_and_raises_sigfpe | status 136 (SIGFPE) |
| codegen_semantics_tests.rs::executables_are_position_independent | A | lang.sh::executables_are_position_independent | ET_DYN via od |
| codegen_semantics_tests.rs::for_in_binds_array_elements | A | lang.sh::for_in_binds_array_elements | |
| codegen_semantics_tests.rs::if_and_match_produce_values | A | lang.sh::if_and_match_produce_values | |
| codegen_semantics_tests.rs::int64_min_div_minus_one_flushes_and_raises_sigfpe | A | lang.sh::int64_min_div_minus_one_flushes_and_raises_sigfpe | |
| codegen_semantics_tests.rs::logical_operators_short_circuit_and_normalise | A | lang.sh::logical_operators_short_circuit_and_normalise | sources in cases/lang/<stem>.trk |
| codegen_semantics_tests.rs::negative_runtime_results_keep_their_sign | A | lang.sh::negative_runtime_results_keep_their_sign | |
| codegen_semantics_tests.rs::program_arguments_reach_os_args | A | lang.sh::program_arguments_reach_os_args | |
| codegen_semantics_tests.rs::return_inside_match_while_and_nested_if | A | lang.sh::return_inside_match_while_and_nested_if | |
| codegen_semantics_tests.rs::returned_tuple_variables_do_not_dangle | A | lang.sh::returned_tuple_variables_do_not_dangle | |
| codegen_semantics_tests.rs::string_patterns_compare_contents | A | lang.sh::string_patterns_compare_contents | |
| codegen_semantics_tests.rs::structs_and_arrays_have_runtime_values | A | lang.sh::structs_and_arrays_have_runtime_values | |
| codegen_semantics_tests.rs::union_payloads_and_enum_discriminants | A | lang.sh::union_payloads_and_enum_discriminants | |
| codegen_tests.rs::test_codegen_object_file_emission | B | — | dropped: stage0 CodeGen::write_object_file only; `print(42)` build covered by cli/lang build tests |
| codegen_tests.rs::test_codegen_simple_function | B | — | dropped: drives stage0 Cranelift CodeGen API on a hand-built Rust AST; no trackc equivalent (trackc builds are covered by every build test) |
| codegen_tests.rs::test_codegen_string_literals_and_print | B | — | dropped: stage0 object emission only; string print covered by examples/hello in lang.sh::valid_examples_compile_and_run |
| editor_tests.rs::test_nvim_plugin_files | D | tests/sh/tools.sh::t_nvim_plugin_files | luac -p step skips when luac is absent, as in Rust |
| editor_tests.rs::test_vscode_language_configuration | D | tests/sh/tools.sh::t_vscode_language_configuration | repo asset |
| editor_tests.rs::test_vscode_package_json | D | tests/sh/tools.sh::t_vscode_package_json | editor/vscode/package.json survives; JSON via jq |
| editor_tests.rs::test_vscode_textmate_grammar | D | tests/sh/tools.sh::t_vscode_textmate_grammar | repo asset |
| headless_highlight_tests.rs::test_nvim_devicon_svg_not_needed_headless | D | tests/sh/tools.sh::t_nvim_devicon_svg_not_needed_headless | repo asset |
| headless_highlight_tests.rs::test_nvim_headless_filetype_and_syntax | D | tests/sh/tools.sh::t_nvim_headless_filetype_and_syntax | skips without nvim (as in Rust) |
| headless_highlight_tests.rs::test_nvim_headless_lua_syntax_ok | D | tests/sh/tools.sh::t_nvim_headless_lua_syntax_ok | skips without nvim |
| headless_highlight_tests.rs::test_nvim_syntax_highlight_sample_headless | D | tests/sh/tools.sh::t_nvim_syntax_highlight_sample_headless | skips without nvim |
| headless_highlight_tests.rs::test_vscode_grammar_headless_tokenization | D | tests/sh/tools.sh::t_vscode_grammar_headless_tokenization | regexes run under grep -P (PCRE) instead of the regex crate; patterns invalid in PCRE are skipped as before (PCRE accepts lookaheads the regex crate skipped) |
| headless_highlight_tests.rs::test_vscode_grammar_valid_json_headless | D | tests/sh/tools.sh::t_vscode_grammar_valid_json_headless | both grammar copies; exactly-one-JSON-value check |
| integration_tests.rs::test_const_resolution_across_functions | A | lang.sh::const_resolution_across_functions | stdout pinned exactly |
| integration_tests.rs::test_error_predicates_disambiguate_sentinels | A | lang.sh::error_predicates_disambiguate_sentinels | stricter: whole stdout pinned |
| integration_tests.rs::test_explicit_error_handling_convention | A | lang.sh::explicit_error_handling_convention | cases/lang/error_handling.trk |
| integration_tests.rs::test_for_in_loop_execution | A | lang.sh::for_in_loop_execution | stricter: stdout 0..2 pinned |
| integration_tests.rs::test_generics_monomorphization | A | lang.sh::generics_monomorphization | |
| integration_tests.rs::test_invalid_examples_rejection | A | lang.sh::invalid_examples_rejection | trackc check exit 1 + same substrings |
| integration_tests.rs::test_logical_not_operator | A | lang.sh::logical_not_operator | cases/lang/lnot.trk |
| integration_tests.rs::test_memory_boundary_exceeded_crash | A | lang.sh::memory_boundary_exceeded_crash | cases/lang/mem_test.trk |
| integration_tests.rs::test_valid_examples_compilation_and_execution | A | tests/sh/lang.sh::valid_examples_compile_and_run | trackc build + run, exit 0 |
| lexer_tests.rs::test_tokenize_comments_and_whitespace | B | lexer_token_dumps (lex_comments.tokens) | |
| lexer_tests.rs::test_tokenize_invalid_character | B | lang.sh::lexer_invalid_character | trackc: "error[E0001]: unexpected character" at 1:9 (no quoted char) |
| lexer_tests.rs::test_tokenize_keywords | B | lang.sh::lexer_token_dumps (cases/lang/lex_keywords.tokens) | Rust Token enum dropped; `trackc tokens` golden lists the same kinds |
| lexer_tests.rs::test_tokenize_literals_and_idents | B | lexer_token_dumps (lex_literals.tokens) | escapes decoded (\r shown as \015) |
| lexer_tests.rs::test_tokenize_operators_and_punctuation | B | lexer_token_dumps (lex_operators.tokens) | 33 tokens, same order |
| lexer_tests.rs::test_tokenize_types | B | lexer_token_dumps (lex_types.tokens) | |
| logic_and_soundness_strict_tests.rs::test_strict_bitwise_or_vs_logical_or_ast_precedence | B | lang.sh::bitwise_vs_logical_or_precedence | Rust AST half dropped; trackc half checks the grouping by running (prints 1 only for (a|b)||(c&d)) |
| logic_and_soundness_strict_tests.rs::test_strict_borrow_escape_from_lens_block | B | checker_verdicts row strict_borrow_escape_from_lens_block | |
| logic_and_soundness_strict_tests.rs::test_strict_double_move_in_loop_rejection | B | checker_verdicts row strict_double_move_in_loop_rejection | |
| logic_and_soundness_strict_tests.rs::test_strict_mutability_reassignment_rejection | B | checker_verdicts row strict_mutability_reassignment_rejection | same text "Cannot mutate immutable variable 'x'" |
| logic_and_soundness_strict_tests.rs::test_strict_mutable_variable_reassignment_allowed | B | checker_verdicts row strict_mutable_variable_reassignment_allowed | |
| logic_and_soundness_strict_tests.rs::test_strict_unresolved_type_alias_detection | B | checker_verdicts row strict_unresolved_type_alias_detection | Rust AST half dropped; trackc accepts the alias (parses) |
| lsp_tests.rs::test_lsp_completion (#[tokio::test]) | C | PR #37 tests/lsp/cases/basic.in (id 2: same text, position 1:6, label "let") | same source and position |
| lsp_tests.rs::test_lsp_hover (#[tokio::test]) | C | PR #37 tests/lsp/cases/basic.in (id 3: "Print a value to stdout") | same source and position |
| lsp_tests.rs::test_lsp_initialization (#[tokio::test]) | C | PR #37 tests/lsp/cases/basic.in (id 1: completionProvider + hoverProvider in capabilities) | not counted by `grep '#[test]'` |
| native_tests.rs::native_and_c_print_the_expected_output | A | tests/sh/native.sh::t_native_and_c_print_the_expected_output | a signal N is exit 128+N (SIGFPE = 136) |
| native_tests.rs::native_and_track_rt_reject_what_check_rejects | A | tests/sh/native.sh::t_native_and_track_rt_reject_what_check_rejects | |
| native_tests.rs::native_build_needs_no_c_toolchain | A | tests/sh/native.sh::t_native_build_needs_no_c_toolchain | |
| native_tests.rs::native_compiler_reaches_a_fixed_point | A | tests/sh/native.sh::t_native_compiler_reaches_a_fixed_point | stage1 is $TRACKC (seed bootstrap) instead of the yard-built trackc |
| native_tests.rs::native_executables_have_a_non_executable_stack | A | tests/sh/native.sh::t_native_executables_have_a_non_executable_stack | ELF header read with od |
| native_tests.rs::native_flag_is_honoured_in_any_position | A | tests/sh/native.sh::t_native_flag_is_honoured_in_any_position | |
| native_tests.rs::native_identical_string_literals_compare_equal | A | tests/sh/native.sh::t_native_identical_string_literals_compare_equal | |
| native_tests.rs::native_program_without_main_builds | A | tests/sh/native.sh::t_native_program_without_main_builds | |
| native_tests.rs::native_programs_match_the_c_backend | A | tests/sh/native.sh::t_native_programs_match_the_c_backend | |
| native_tests.rs::native_tuples_outlive_their_frame | A | tests/sh/native.sh::t_native_tuples_outlive_their_frame | |
| parser_tests.rs::test_parse_array_type_and_literal | B | parser_sources_parse row parse_array_type_and_literal | |
| parser_tests.rs::test_parse_enum_and_union | B | parser_sources_parse row parse_enum_and_union | |
| parser_tests.rs::test_parse_fn_def | B | lang.sh::parser_sources_parse row parse_fn_def | stage0 AST equality dropped (no trackc AST dump); trackc half: parses with no E0001/E01xx, verdict pinned |
| parser_tests.rs::test_parse_for_in_loop | B | parser_sources_parse row parse_for_in_loop | |
| parser_tests.rs::test_parse_generic_fn_multi_params | B | parser_sources_parse row parse_generic_fn_multi_params | |
| parser_tests.rs::test_parse_generic_fn_no_return | B | parser_sources_parse row parse_generic_fn_no_return | |
| parser_tests.rs::test_parse_generic_fn_single_param | B | parser_sources_parse row parse_generic_fn_single_param | |
| parser_tests.rs::test_parse_let_def | B | parser_sources_parse row parse_let_def | same |
| parser_tests.rs::test_parse_macro_def_and_call | B | parser_sources_parse row parse_macro_def_and_call | |
| parser_tests.rs::test_parse_match | B | parser_sources_parse row parse_match | parses; undeclared 'val' pinned |
| parser_tests.rs::test_parse_slice_type_and_indexing | B | parser_sources_parse row parse_slice_type_and_indexing | parses; E0200 undeclared 'arr' pinned |
| parser_tests.rs::test_parse_struct_and_lens | B | parser_sources_parse row parse_struct_and_lens | |
| parser_tests.rs::test_parse_type_alias | B | parser_sources_parse row parse_type_alias | |
| parser_tests.rs::test_parse_use_and_const | B | parser_sources_parse row parse_use_and_const | parses; E0011 unknown module pinned |
| parser_tests.rs::test_parse_use_statement | B | parser_sources_parse row parse_use_statement | parses; std/io export error pinned |
| performance_tests.rs::test_performance_full_pipeline_throughput | B | lang.sh::performance_full_pipeline_throughput | same 500-fn source through `trackc check` < 5 s |
| performance_tests.rs::test_performance_lexer_throughput | B | lang.sh::performance_lexer_throughput | stage0 lexer timing dropped; same input/5 s cap on `trackc tokens`, >=60000 tokens |
| performance_tests.rs::test_performance_parser_throughput | B | lang.sh::performance_parser_throughput | stage0 parser timing dropped; 1000 fns (distinct names, else E0201) through `trackc check` < 5 s |
| regression_tests.rs::test_regression_complex_match_guards | B | checker_verdicts row regression_complex_match_guards | |
| regression_tests.rs::test_regression_multiple_macro_calls | B | checker_verdicts row regression_multiple_macro_calls | |
| regression_tests.rs::test_regression_nested_expressions | B | lang.sh::checker_verdicts row regression_nested_expressions | trackc half: same accept verdict |
| regression_tests.rs::test_regression_nested_if_else_chain | B | checker_verdicts row regression_nested_if_else_chain | |
| regression_tests.rs::test_regression_nested_lens_blocks | B | checker_verdicts row regression_nested_lens_blocks | |
| regression_tests.rs::test_regression_shadowed_variables | B | checker_verdicts row regression_shadowed_variables | |
| regression_tests.rs::test_regression_top_level_variables_and_deref | B | checker_verdicts row regression_top_level_variables_and_deref | |
| selfhost_tests.rs::autofree_frees_heap_cells | A | tests/sh/lifecycle.sh::t_autofree_frees_heap_cells | `dealloc(l` in C, ASan detect_leaks=0 |
| selfhost_tests.rs::bootstrap_reaches_a_fixed_point | A | tests/sh/selfhost.sh::t_bootstrap_reaches_a_fixed_point | stage1 is $TRACKC (seed bootstrap) instead of yard-built |
| selfhost_tests.rs::build_creates_missing_output_directories | A | tests/sh/selfhost.sh::t_build_creates_missing_output_directories | |
| selfhost_tests.rs::checkers_agree_on_accepted_and_rejected_programs | B | tests/sh/selfhost.sh::t_checkers_agree_on_accepted_and_rejected_programs | Rust checker half dropped; 34 trackc verdicts in tests/cases/verdicts/{accept,reject} |
| selfhost_tests.rs::compiler_sources_are_warning_clean | A | tests/sh/selfhost.sh::t_compiler_sources_are_warning_clean | |
| selfhost_tests.rs::const_divrem_traps_are_e0200_in_both_checkers | B | tests/sh/selfhost.sh::t_const_divrem_traps_are_e0200_in_both_checkers | Rust checker half dropped; 19 cases in tests/cases/const_divrem (exit 1 + stderr prefix) |
| selfhost_tests.rs::diagnostics_point_at_the_source | A | tests/sh/selfhost.sh::t_diagnostics_point_at_the_source | |
| selfhost_tests.rs::duplicate_functions_are_rejected | A | tests/sh/lifecycle.sh::t_duplicate_functions_are_rejected | check+build, one file and import |
| selfhost_tests.rs::examples_behave_the_same_under_both_compilers | B | tests/sh/selfhost.sh::t_examples_behave_the_same_under_both_compilers | stage0 build dropped; stage0 behaviour frozen in tests/cases/examples/*.out/.code (captured from the Rust compiler), trackc compared to it; _err examples must be rejected |
| selfhost_tests.rs::examples_warn_only_where_intended | A | tests/sh/selfhost.sh::t_examples_warn_only_where_intended | |
| selfhost_tests.rs::for_in_exit_and_div_trap_on_both_runtimes | A | tests/sh/lifecycle.sh::t_for_in_exit_and_div_trap_on_both_runtimes | signal checked as status 136 |
| selfhost_tests.rs::handles_free_fresh_values | A | tests/sh/lifecycle.sh::t_handles_free_fresh_values | C+native .out, free fns in C, coll_move exact block, kept-not-freed, ASan |
| selfhost_tests.rs::int64_min_div_minus_one_flushes_on_every_backend | A | tests/sh/lifecycle.sh::t_int64_min_div_minus_one_flushes_on_every_backend | C, --rt=track, --native; status 136 |
| selfhost_tests.rs::life_dumps_match_goldens | A | tests/sh/lifecycle.sh::t_life_dumps_match_goldens | tests/life/*.life goldens, UPDATE_LIFE kept |
| selfhost_tests.rs::life_place_dumps_match_goldens | A | tests/sh/lifecycle.sh::t_life_place_dumps_match_goldens | .place goldens + 5 rule lines |
| selfhost_tests.rs::lifecycle_hard_errors_print_the_catalogue_texts | A | tests/sh/selfhost.sh::t_lifecycle_hard_errors_print_the_catalogue_texts | 19 cases in tests/cases/life_errors |
| selfhost_tests.rs::lifecycle_lookalikes_check_clean | A | tests/sh/selfhost.sh::t_lifecycle_lookalikes_check_clean | 10 cases in tests/cases/life_clean |
| selfhost_tests.rs::lifecycle_warnings_match_the_catalogue | A | tests/sh/selfhost.sh::t_lifecycle_warnings_match_the_catalogue | near misses in tests/cases/life_warn_clean |
| selfhost_tests.rs::native_binaries_carry_a_symbol_table | A | tests/sh/selfhost.sh::t_native_binaries_carry_a_symbol_table | skips without nm, as before |
| selfhost_tests.rs::native_test_suite_passes_under_trackc | A | tests/sh/selfhost.sh::t_native_test_suite_passes_under_trackc | |
| selfhost_tests.rs::non_escaping_tuples_use_stack_slots | A | tests/sh/selfhost.sh::t_non_escaping_tuples_use_stack_slots | |
| selfhost_tests.rs::program_without_main_runs_on_both_runtimes | A | tests/sh/selfhost.sh::t_program_without_main_runs_on_both_runtimes | |
| selfhost_tests.rs::ret_slots_write_caller_cells | A | tests/sh/lifecycle.sh::t_ret_slots_write_caller_cells | `__rs(` in C, 11 rule lines, ASan |
| selfhost_tests.rs::returns_hidden_in_expressions_are_seen | A | tests/sh/lifecycle.sh::t_returns_hidden_in_expressions_are_seen | banned-plan checks, ASan |
| selfhost_tests.rs::runtime_layout_consts_stay_reserved | A | tests/sh/lifecycle.sh::t_runtime_layout_consts_stay_reserved | E0201 under --native |
| selfhost_tests.rs::runtime_object_is_cached_and_builds_stay_identical | A | tests/sh/selfhost.sh::t_runtime_object_is_cached_and_builds_stay_identical | |
| selfhost_tests.rs::stack_cells_run_and_plan | A | tests/sh/lifecycle.sh::t_stack_cells_run_and_plan | .out/.place, `tk_w s0[` in C |
| selfhost_tests.rs::strfield_frees_owned_str_fields | A | tests/sh/lifecycle.sh::t_strfield_frees_owned_str_fields | ` strs ` ownership checks, LSan for 5 |
| selfhost_tests.rs::string_plus_string_is_a_type_error | A | tests/sh/lifecycle.sh::t_string_plus_string_is_a_type_error | 3 rejects + ptr arith accepted |
| selfhost_tests.rs::temps_free_fresh_cell_temporaries | A | tests/sh/lifecycle.sh::t_temps_free_fresh_cell_temporaries | accdeep native, LSan for the 8 all-die programs |
| selfhost_tests.rs::temps_free_fresh_temporaries | A | tests/sh/lifecycle.sh::t_temps_free_fresh_temporaries | counts per block, ASan+LSan for discard/nested/print_loop |
| selfhost_tests.rs::track_runtime_matches_the_c_runtime | A | tests/sh/selfhost.sh::t_track_runtime_matches_the_c_runtime | |
| selfhost_tests.rs::trackc_closes_checker_holes | A | tests/sh/selfhost.sh::t_trackc_closes_checker_holes | 46 cases in tests/cases/trackc_verdicts |
| selfhost_tests.rs::trackc_on_the_track_runtime_self_compiles | A | tests/sh/selfhost.sh::t_trackc_on_the_track_runtime_self_compiles | |
| selfhost_tests.rs::trackc_programs_print_the_expected_output | A | tests/sh/selfhost.sh::t_trackc_programs_print_the_expected_output | 10 cases in tests/cases/run_outputs |
| selfhost_tests.rs::user_names_never_clash_with_runtime_names | A | tests/sh/lifecycle.sh::t_user_names_never_clash_with_runtime_names | C, --rt=track, --native |
| selfhost_tests.rs::warnings_print_sorted_and_only_deny_warnings_fails | A | tests/sh/selfhost.sh::t_warnings_print_sorted_and_only_deny_warnings_fails | |
| soundness_tests.rs::test_soundness_double_move_rejected | B | checker_verdicts row soundness_double_move_rejected | |
| soundness_tests.rs::test_soundness_inconsistent_branch_move_rejected | B | checker_verdicts row soundness_inconsistent_branch_move_rejected | "inconsistent state" |
| soundness_tests.rs::test_soundness_lens_alias_not_visible_after_block | B | checker_verdicts row soundness_lens_alias_not_visible_after_block | |
| soundness_tests.rs::test_soundness_lens_escape_rejected | B | checker_verdicts row soundness_lens_escape_rejected | "Lens ... escape" |
| soundness_tests.rs::test_soundness_lens_released_on_early_return | B | checker_verdicts row soundness_lens_released_on_early_return | |
| soundness_tests.rs::test_soundness_loop_consumed_variable_use_rejected | B | checker_verdicts row soundness_loop_consumed_variable_use_rejected | trackc half; pins trackc's first line (mentions 'v') |
| soundness_tests.rs::test_soundness_loop_reinitialization_accepted | B | checker_verdicts row soundness_loop_reinitialization_accepted | |
| soundness_tests.rs::test_soundness_nested_lens_same_target_rejected | B | checker_verdicts row soundness_nested_lens_same_target_rejected | |
| yard_tests.rs::test_yard_build_errors_follow_source_order | C | PR #36 case `build_errors_follow_source_order` | Rust calls ParallelBuilder directly; PR runs `yard build` 4x and checks a.trk is the first error |
| yard_tests.rs::test_yard_build_generics_project | C | PR #36 case `build_generics_project` | 1:1 |
| yard_tests.rs::test_yard_dependency_resolution_is_sorted | C | PR #36 case `dependency_resolution_is_sorted` | Rust calls resolver::resolve; PR checks the "Using dependency" order of `yard build` |
| yard_tests.rs::test_yard_init_add_check_in_temp_dir | C | PR #36 case `init_add_check_in_temp_dir` | 1:1 |
| yard_tests.rs::test_yard_lint_and_clean | C | PR #36 case `lint_and_clean` | 1:1 |
| yard_tests.rs::test_yard_manifest_serialization | C | PR #36 tests/yard/run.sh case `manifest_serialization` | Rust test hits the Rust Manifest/toml API; PR checks the same fields through `yard init` + `add` round trip |
| yard_tests.rs::test_yard_nested_modules_have_isolated_stable_artifacts | C | PR #36 case `nested_modules_have_isolated_stable_artifacts` | 1:1 |
| yard_tests.rs::test_yard_parallel_build_and_incremental_cache | C | PR #36 case `parallel_build_and_incremental_cache` | 1:1 |
| yard_tests.rs::test_yard_version_reports | C | PR #36 case `version_reports` | 1:1 |
